use crate::memory::capability::MemoryCapability;
use crate::memory::resource::{PhysAddr, ResourceId};
use vivanta_arch_api::mmu::{MappingFlags, PageTableAllocator};

/// Adapter that wraps the kernel's `MemoryResourceManager` into a `PageTableAllocator`.
pub struct MrmPageTableAllocator {
    mrm: *mut crate::memory::MemoryResourceManager,
}

impl MrmPageTableAllocator {
    pub unsafe fn new(mrm: *mut crate::memory::MemoryResourceManager) -> Self {
        MrmPageTableAllocator { mrm }
    }
}

impl PageTableAllocator for MrmPageTableAllocator {
    fn alloc_page_table_frame(&mut self) -> u64 {
        use crate::memory::{AllocationRequirements, MemoryBackend};
        let req = AllocationRequirements::new(4096);
        unsafe {
            (*self.mrm).allocate(&req, 0)
                .expect("alloc_page_table_frame: OOM")
                .phys_addr
                .expect("alloc_page_table_frame: no phys addr")
        }
    }
}

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
    Created,
    Allocated,
    Mapped,
    Shared,
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
pub struct MemoryObject {
    pub id: MemoryObjectId,
    pub size: u64,
    pub resource_id: ResourceId,
    pub state: MemoryObjectState,
    pub phys_addr: Option<PhysAddr>,
    pub(super) mappings: [Option<VirtualMapping>; MAX_MAPPINGS],
    pub capability: MemoryCapability,
}

impl MemoryObject {
    pub fn new(id: MemoryObjectId, size: u64, resource_id: ResourceId, capability: MemoryCapability) -> Self {
        MemoryObject {
            id,
            size,
            resource_id,
            state: MemoryObjectState::Created,
            phys_addr: None,
            mappings: [None; MAX_MAPPINGS],
            capability,
        }
    }

    /// Set the physical address after backend allocation succeeds.
    pub fn set_phys_addr(&mut self, addr: PhysAddr) {
        self.phys_addr = Some(addr);
    }

    // ------------------------------------------------------------------
    // Lifecycle transitions
    // ------------------------------------------------------------------

    pub fn mark_allocated(&mut self) -> Result<(), ObjectError> {
        match self.state {
            MemoryObjectState::Created => {
                self.state = MemoryObjectState::Allocated;
                Ok(())
            }
            _ => Err(ObjectError::InvalidTransition),
        }
    }

    pub fn is_revoked(&self) -> bool {
        self.state == MemoryObjectState::Revoked
    }

    // ------------------------------------------------------------------
    // Mapping — NOW PROGRAMS REAL MMU via arch-api
    // ------------------------------------------------------------------

    /// Map this object at a virtual address in the given address space.
    /// Programs the live page table via AddressSpace.map_pages().
    pub fn map(
        &mut self,
        vaddr: u64,
        size: u64,
        aspace: &mut crate::vmm::AddressSpace,
        alloc: &mut dyn PageTableAllocator,
    ) -> Result<usize, ObjectError> {
        if self.is_revoked() {
            return Err(ObjectError::Revoked);
        }
        if self.state == MemoryObjectState::Created {
            return Err(ObjectError::NoStorage);
        }
        let phys = self.phys_addr.ok_or(ObjectError::NoStorage)?;

        let flags = rights_to_flags(self.capability.rights);
        aspace.map_pages(vaddr, phys, size, flags, alloc, self.id)
            .map_err(|_| ObjectError::MappingLimitReached)?;

        for i in 0..MAX_MAPPINGS {
            if self.mappings[i].is_none() {
                self.mappings[i] = Some(VirtualMapping { vaddr, size });
                self.state = MemoryObjectState::Mapped;
                return Ok(i);
            }
        }
        Err(ObjectError::MappingLimitReached)
    }

        /// Unmap this object and clear the page table entries.
    pub fn unmap(
        &mut self,
        slot: usize,
        aspace: &mut crate::vmm::AddressSpace,
        alloc: &mut dyn PageTableAllocator,
    ) -> Result<(), ObjectError> {
        if self.is_revoked() {
            return Err(ObjectError::Revoked);
        }
        let mapping = self.mappings.get(slot).and_then(|m| *m).ok_or(ObjectError::NotMapped)?;
        aspace.unmap_pages(mapping.vaddr, mapping.size, alloc)
            .map_err(|_| ObjectError::NotMapped)?;
        self.mappings[slot] = None;
        if self.mappings.iter().all(|m| m.is_none()) {
            self.state = MemoryObjectState::Allocated;
        }
        Ok(()
)
    }

    pub fn mapping_count(&self) -> usize {
        self.mappings.iter().filter(|m| m.is_some()).count()
    }

    pub fn all_mappings(&self) -> impl Iterator<Item = &VirtualMapping> {
        self.mappings.iter().filter_map(|m| m.as_ref())
    }

    // ------------------------------------------------------------------
    // Clone & Share
    // ------------------------------------------------------------------

    pub fn clone(&self, new_id: MemoryObjectId, new_cap: MemoryCapability) -> Self {
        MemoryObject {
            id: new_id,
            size: self.size,
            resource_id: self.resource_id,
            state: MemoryObjectState::Allocated,
            phys_addr: self.phys_addr,
            mappings: [None; MAX_MAPPINGS],
            capability: new_cap,
        }
    }

    pub fn share(&mut self, new_cap: MemoryCapability) -> ShareHandle {
        self.state = MemoryObjectState::Shared;
        ShareHandle {
            object_id: self.id,
            capability: new_cap,
        }
    }

    // ------------------------------------------------------------------
    // Revoke
    // ------------------------------------------------------------------

    pub fn revoke(&mut self) {
        self.state = MemoryObjectState::Revoked;
        for slot in &mut self.mappings {
            *slot = None;
        }
    }

    // ------------------------------------------------------------------
    // Query
    // ------------------------------------------------------------------

    pub fn vaddr(&self) -> Option<u64> {
        self.mappings.iter().find_map(|m| m.as_ref().map(|x| x.vaddr))
    }
}

fn rights_to_flags(rights: crate::memory::MemRights) -> MappingFlags {
    let mut f = MappingFlags::from_bits(0);
    if rights.read && rights.write {
        f = MappingFlags::read_write();
    }
    if rights.execute {
        f = f | MappingFlags::executable();
    }
    // user flag: MemRights doesn't have a user bit yet — defer to V3+
    f
}