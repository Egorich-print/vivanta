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
#[derive(Debug, Clone, Copy)]
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
        self.mappings.iter().find_map(|m| m.as_ref().map(|x| x.vaddr))
    }
}