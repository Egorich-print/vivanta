// ---------------------------------------------------------------------------
// MemoryObject — the architectural centre of the memory model
//
// Lifecycle:
//
//   Created ──► Allocated ──► Mapped
//                      │            │
//                      ├── Shared   │
//                      │            │
//                      └── Revoked ◄┘
// ---------------------------------------------------------------------------

use crate::capability::MemoryCapability;
use crate::resource::ResourceId;

pub type MemoryObjectId = u64;

/// Maximum number of simultaneous virtual mappings per object.
const MAX_MAPPINGS: usize = 4;

/// A single virtual mapping of this object.
#[derive(Debug, Clone, Copy)]
pub struct VirtualMapping {
    pub vaddr: u64,
    pub size: u64,
}

/// Current stage in the object's lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryObjectState {
    /// ID reserved but no physical allocation yet.
    Created,
    /// Backend has allocated physical storage.
    Allocated,
    /// At least one virtual mapping exists.
    Mapped,
    /// Object was shared with another owner (state unchanged).
    Shared,
    /// Object has been revoked; future access should be denied.
    Revoked,
}

/// Error returned when an operation is invalid for the current state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectError {
    InvalidTransition,
    NoStorage,
    MappingLimitReached,
    AlreadyMapped,
    NotMapped,
    Revoked,
}

/// A share-handle returned by `share()`.
#[derive(Debug, Clone, Copy)]
pub struct ShareHandle {
    pub object_id: MemoryObjectId,
    pub capability: MemoryCapability,
}

/// A MemoryObject represents a logical region of memory.
///
/// It ties together:
///   - which backend provides the physical pages (resource_id)
///   - zero or more virtual mappings
///   - who has access (capability)
///
/// MemoryObject is the unit of:
///   - allocation (create)
///   - sharing (clone / share)
///   - mapping (map / unmap)
///   - revocation (revoke)
pub struct MemoryObject {
    pub id: MemoryObjectId,
    pub size: u64,
    pub resource_id: ResourceId,
    pub state: MemoryObjectState,
    pub(super) mappings: [Option<VirtualMapping>; MAX_MAPPINGS],
    pub capability: MemoryCapability,
}

impl MemoryObject {
    /// Create a new object in `Created` state. No physical storage yet.
    pub fn new(
        id: MemoryObjectId,
        size: u64,
        resource_id: ResourceId,
        capability: MemoryCapability,
    ) -> Self {
        MemoryObject {
            id,
            size,
            resource_id,
            state: MemoryObjectState::Created,
            mappings: [None; MAX_MAPPINGS],
            capability,
        }
    }

    // ------------------------------------------------------------------
    // Lifecycle transitions
    // ------------------------------------------------------------------

    /// Transition from Created to Allocated. Marks storage as ready.
    pub fn mark_allocated(&mut self) -> Result<(), ObjectError> {
        match self.state {
            MemoryObjectState::Created => {
                self.state = MemoryObjectState::Allocated;
                Ok(())
            }
            _ => Err(ObjectError::InvalidTransition),
        }
    }

    /// True if the object has been revoked.
    pub fn is_revoked(&self) -> bool {
        self.state == MemoryObjectState::Revoked
    }

    // ------------------------------------------------------------------
    // Mapping
    // ------------------------------------------------------------------

    /// Add a virtual mapping. Returns the slot index.
    pub fn map(&mut self, vaddr: u64, size: u64) -> Result<usize, ObjectError> {
        if self.is_revoked() {
            return Err(ObjectError::Revoked);
        }
        if self.state == MemoryObjectState::Created {
            return Err(ObjectError::NoStorage);
        }
        // Find an empty slot.
        for i in 0..MAX_MAPPINGS {
            if self.mappings[i].is_none() {
                self.mappings[i] = Some(VirtualMapping { vaddr, size });
                self.state = MemoryObjectState::Mapped;
                return Ok(i);
            }
        }
        Err(ObjectError::MappingLimitReached)
    }

    /// Remove a virtual mapping by slot index.
    pub fn unmap(&mut self, slot: usize) -> Result<(), ObjectError> {
        if self.is_revoked() {
            return Err(ObjectError::Revoked);
        }
        let existing = self.mappings.get(slot).ok_or(ObjectError::NotMapped)?;
        if existing.is_none() {
            return Err(ObjectError::NotMapped);
        }
        self.mappings[slot] = None;
        // Downgrade state if no mappings remain.
        if self.mappings.iter().all(|m| m.is_none()) {
            self.state = MemoryObjectState::Allocated;
        }
        Ok(())
    }

    /// Number of active virtual mappings.
    pub fn mapping_count(&self) -> usize {
        self.mappings.iter().filter(|m| m.is_some()).count()
    }

    /// Iterator over active mappings.
    pub fn all_mappings(&self) -> impl Iterator<Item = &VirtualMapping> {
        self.mappings.iter().filter_map(|m| m.as_ref())
    }

    // ------------------------------------------------------------------
    // Clone — creates a new object sharing the same backend storage
    // ------------------------------------------------------------------

    /// Clone this object.
    ///
    /// The clone shares the same backend storage (same resource_id).
    /// Both objects see the same content. Not copy-on-write yet.
    pub fn clone(&self, new_id: MemoryObjectId, new_cap: MemoryCapability) -> Self {
        MemoryObject {
            id: new_id,
            size: self.size,
            resource_id: self.resource_id,
            state: MemoryObjectState::Allocated,
            mappings: [None; MAX_MAPPINGS],
            capability: new_cap,
        }
    }

    // ------------------------------------------------------------------
    // Share — grants access to another owner
    // ------------------------------------------------------------------

    /// Create a share-handle with a new capability for another owner.
    /// The share handle allows the recipient to map/unmap this object.
    pub fn share(&mut self, new_cap: MemoryCapability) -> ShareHandle {
        self.state = MemoryObjectState::Shared;
        ShareHandle {
            object_id: self.id,
            capability: new_cap,
        }
    }

    // ------------------------------------------------------------------
    // Revoke — invalidate the object
    // ------------------------------------------------------------------

    /// Revoke the object. Future accesses should be denied.
    pub fn revoke(&mut self) {
        self.state = MemoryObjectState::Revoked;
        // Clear all mappings.
        for slot in &mut self.mappings {
            *slot = None;
        }
    }

    // ------------------------------------------------------------------
    // Query
    // ------------------------------------------------------------------

    /// First mapped virtual address, if any.
    pub fn vaddr(&self) -> Option<u64> {
        self.mappings
            .iter()
            .find_map(|m| m.as_ref().map(|x| x.vaddr))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capability::{MemRights, MemoryCapability};

    fn cap(id: u64, object: u64) -> MemoryCapability {
        MemoryCapability {
            id,
            object,
            rights: MemRights::FULL,
            owner: 1,
        }
    }

    fn obj(id: u64) -> MemoryObject {
        MemoryObject::new(id, 4096, 0, cap(id, id))
    }

    #[test]
    fn lifecycle_created_allocated_mapped() {
        let mut o = obj(1);
        assert_eq!(o.state, MemoryObjectState::Created);
        assert!(
            o.map(0x1000, 4096).is_err(),
            "map before allocation must fail"
        );
        o.mark_allocated().unwrap();
        assert_eq!(o.map(0x1000, 4096).unwrap(), 0);
        assert_eq!(o.state, MemoryObjectState::Mapped);
        assert_eq!(o.mapping_count(), 1);
    }

    #[test]
    fn mapping_slots_reusable() {
        let mut o = obj(2);
        o.mark_allocated().unwrap();
        let s0 = o.map(0x1000, 4096).unwrap();
        let s1 = o.map(0x2000, 4096).unwrap();
        assert_eq!(o.mapping_count(), 2);
        o.unmap(s0).unwrap();
        assert_eq!(o.mapping_count(), 1);
        // Slot 0 is reusable for a new mapping.
        let s2 = o.map(0x3000, 4096).unwrap();
        assert_eq!(s2, 0);
        o.unmap(s1).unwrap();
        o.unmap(s2).unwrap();
        assert_eq!(
            o.state,
            MemoryObjectState::Allocated,
            "no mappings -> downgrade"
        );
    }

    #[test]
    fn mapping_limit_returns_error() {
        let mut o = obj(3);
        o.mark_allocated().unwrap();
        for i in 0..MAX_MAPPINGS {
            o.map(0x1000 + i as u64 * 4096, 4096).unwrap();
        }
        assert_eq!(o.map(0x9000, 4096), Err(ObjectError::MappingLimitReached));
    }

    #[test]
    fn revoke_clears_mappings_and_blocks_ops() {
        let mut o = obj(4);
        o.mark_allocated().unwrap();
        o.map(0x1000, 4096).unwrap();
        o.revoke();
        assert!(o.is_revoked());
        assert_eq!(o.mapping_count(), 0);
        assert_eq!(o.map(0x2000, 4096), Err(ObjectError::Revoked));
        assert_eq!(o.unmap(0), Err(ObjectError::Revoked));
    }

    #[test]
    fn clone_keeps_storage_but_no_mappings() {
        let mut o = obj(5);
        o.mark_allocated().unwrap();
        o.map(0x1000, 4096).unwrap();
        let c = o.clone(6, cap(6, 6));
        assert_eq!(c.resource_id, o.resource_id);
        assert_eq!(c.size, o.size);
        assert_eq!(c.mapping_count(), 0);
        assert_eq!(c.state, MemoryObjectState::Allocated);
    }

    #[test]
    fn share_flags_shared_state() {
        let mut o = obj(7);
        o.mark_allocated().unwrap();
        let handle = o.share(cap(8, 7));
        assert_eq!(handle.object_id, 7);
        assert_eq!(o.state, MemoryObjectState::Shared);
    }

    #[test]
    fn vaddr_returns_first_mapping() {
        let mut o = obj(9);
        o.mark_allocated().unwrap();
        assert_eq!(o.vaddr(), None);
        o.map(0xABCD, 4096).unwrap();
        assert_eq!(o.vaddr(), Some(0xABCD));
    }
}
