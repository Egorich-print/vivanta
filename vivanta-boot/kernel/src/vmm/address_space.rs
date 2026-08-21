use super::mapping::{Mapping, MappingSet, VirtRange};
use vivanta_arch_api::mmu::{MappingFlags, PageTableAllocator, RootPageTable};

pub type AddressSpaceId = u64;

pub const KERNEL_ADDRESS_SPACE_ID: AddressSpaceId = 0;
const MAX_ADDRESS_SPACES: usize = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AddressSpaceFlags {
    Kernel,
    User,
}

#[derive(Clone, Copy)]
pub struct AddressSpace {
    pub id: AddressSpaceId,
    pub root: RootPageTable,
    pub mappings: MappingSet,
    pub flags: AddressSpaceFlags,
}

impl AddressSpace {
    pub fn new(id: AddressSpaceId, root: RootPageTable, flags: AddressSpaceFlags) -> Self {
        Self {
            id,
            root,
            mappings: MappingSet::new(),
            flags,
        }
    }

    pub fn is_kernel(&self) -> bool {
        self.flags == AddressSpaceFlags::Kernel
    }

    pub fn map_pages(
        &mut self,
        vaddr: u64,
        paddr: u64,
        size: u64,
        flags: MappingFlags,
        alloc: &mut dyn PageTableAllocator,
        object_id: u64,
    ) -> Result<(), VmmError> {
        // Transactional ordering (G2 §3.6): reserve the software slot FIRST.
        // Only after the slot is guaranteed do we program the MMU, so a
        // MappingTableFull failure never leaves an orphaned PTE.
        // (mmu_map_object panics on OOM — boot/runtime fatal — so no rollback
        // is reachable on the failure path after the insert succeeds.)
        let range = VirtRange::new(vaddr, size);
        let mapping = Mapping::new(range, object_id, flags);
        self.mappings
            .insert(mapping)
            .ok_or(VmmError::MappingTableFull)?;
        // SAFETY: caller ensures vaddr range is unmapped.
        unsafe {
            vivanta_arch_api::mmu::mmu_map_object(self.root, vaddr, paddr, size, flags, alloc);
        }
        Ok(())
    }

    pub fn unmap_pages(
        &mut self,
        vaddr: u64,
        size: u64,
        alloc: &mut dyn PageTableAllocator,
    ) -> Result<(), VmmError> {
        // SAFETY: caller ensures vaddr range is mapped
        unsafe {
            vivanta_arch_api::mmu::mmu_unmap(self.root, vaddr, size, alloc);
        }
        for slot in 0..self.mappings.len() {
            if let Some(m) = self.mappings.get(slot) {
                if m.virt_range.base == vaddr && m.virt_range.size == size {
                    self.mappings.remove(slot);
                    break;
                }
            }
        }
        Ok(())
    }

    pub fn query(&self, vaddr: u64) -> Option<&Mapping> {
        self.mappings
            .iter()
            .find(|m| vaddr >= m.virt_range.base && vaddr < m.virt_range.end())
    }

    /// Change permissions on an existing mapping (whole-mapping granularity).
    ///
    /// The range must exactly match one existing mapping of this address
    /// space; otherwise `VmmError::NotMapped` is returned and neither the
    /// hardware nor the software shadow is touched. Partial-range protection
    /// would desynchronise `MappingSet` from the hardware and requires
    /// software mapping splitting — deferred until a VA allocator exists
    /// (post-M5 backlog).
    ///
    /// Transactional ordering mirrors `map_pages`: validate first, then
    /// program the MMU, then commit the software shadow. `mmu_protect`
    /// panics on OOM during a block split (boot/runtime-fatal, same policy
    /// as page-table allocation in `map_pages`), so no rollback path exists
    /// after validation succeeds.
    ///
    /// Transient visibility: single-core, per-descriptor rewrites mean a
    /// concurrent reader observes either the old or the new permission set,
    /// never a torn mapping. That is inherent to permission transitions and
    /// safe for both widening and narrowing.
    pub fn protect(
        &mut self,
        vaddr: u64,
        size: u64,
        new_flags: MappingFlags,
        alloc: &mut dyn PageTableAllocator,
    ) -> Result<(), VmmError> {
        let slot = (0..self.mappings.len())
            .find(|&s| {
                self.mappings
                    .get(s)
                    .is_some_and(|m| m.virt_range.base == vaddr && m.virt_range.size == size)
            })
            .ok_or(VmmError::NotMapped)?;
        // SAFETY: `slot` proves [vaddr, vaddr+size) is an exact mapping in
        // this address space, satisfying mmu_protect's contract.
        unsafe {
            vivanta_arch_api::mmu::mmu_protect(self.root, vaddr, size, new_flags, alloc);
        }
        if let Some(m) = self.mappings.get_mut(slot) {
            m.permissions = new_flags;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VmmError {
    MappingTableFull,
    NotMapped,
}

static mut ADDRESS_SPACES: [Option<AddressSpace>; MAX_ADDRESS_SPACES] =
    [None, None, None, None, None, None, None, None];
static mut NEXT_AS_ID: AddressSpaceId = 1;

pub fn init_kernel_address_space(root: RootPageTable) {
    unsafe {
        ADDRESS_SPACES[0] = Some(AddressSpace::new(0, root, AddressSpaceFlags::Kernel));
    }
}

pub fn register(root: RootPageTable, flags: AddressSpaceFlags) -> AddressSpaceId {
    unsafe {
        let id = NEXT_AS_ID;
        NEXT_AS_ID += 1;
        for i in 0..MAX_ADDRESS_SPACES {
            let ptr = &raw mut ADDRESS_SPACES[i];
            if (*ptr).is_none() {
                *ptr = Some(AddressSpace::new(id, root, flags));
                return id;
            }
        }
    }
    panic!("address space registry full");
}

fn lookup(as_id: AddressSpaceId) -> &'static AddressSpace {
    unsafe {
        for i in 0..MAX_ADDRESS_SPACES {
            let ptr = &raw const ADDRESS_SPACES[i];
            if let Some(ref aspace) = (*ptr).as_ref() {
                if aspace.id == as_id {
                    return aspace;
                }
            }
        }
    }
    panic!("lookup: unknown AddressSpaceId {}", as_id);
}

pub fn lookup_root(as_id: AddressSpaceId) -> RootPageTable {
    lookup(as_id).root
}

pub fn kernel_address_space() -> &'static AddressSpace {
    unsafe {
        ADDRESS_SPACES[0]
            .as_ref()
            .expect("KernelAddressSpace not initialised")
    }
}

pub fn kernel_address_space_mut() -> &'static mut AddressSpace {
    unsafe {
        ADDRESS_SPACES[0]
            .as_mut()
            .expect("KernelAddressSpace not initialised")
    }
}

pub fn count() -> usize {
    unsafe {
        let mut n = 0;
        for i in 0..MAX_ADDRESS_SPACES {
            if ADDRESS_SPACES[i].is_some() {
                n += 1;
            }
        }
        n
    }
}
