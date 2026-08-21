use super::mapping::{Mapping, MappingSet, VirtRange};
use super::tables;
use vivanta_arch_api::mmu::{MappingFlags, PageTableAllocator, RootPageTable};
use vivanta_vm::va::{VaAllocator, VaRegion};

pub type AddressSpaceId = u64;

pub const KERNEL_ADDRESS_SPACE_ID: AddressSpaceId = 0;
const MAX_ADDRESS_SPACES: usize = 8;

/// User VA domain for allocator-managed mappings.
///
/// Layout of the 39-bit TTBR0 space (T0SZ=25):
/// ```text
///   0x0000_0000 .. 0x0100_0000   null/guard region — never allocated
///   0x0100_0000 .. 0x4000_0000   user VA allocator domain
///   0x4000_0000 ..               kernel identity RAM / MMIO / boot-era
///                                user images (outside the allocator)
/// ```
pub const USER_VA_BASE: u64 = 0x0100_0000;
pub const USER_VA_END: u64 = 0x4000_0000;

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
    /// Allocator-managed VA domain. Disabled (empty) for the kernel AS,
    /// whose translations are identity-mapped at boot.
    pub va: VaAllocator,
}

impl AddressSpace {
    pub fn new(id: AddressSpaceId, root: RootPageTable, flags: AddressSpaceFlags) -> Self {
        let va = match flags {
            // Null-page guard: base starts above page 0.
            AddressSpaceFlags::User => VaAllocator::try_new(USER_VA_BASE, USER_VA_END),
            AddressSpaceFlags::Kernel => Err(vivanta_vm::VaError::InvalidRange),
        }
        .unwrap_or_else(|_| VaAllocator::disabled());
        Self {
            id,
            root,
            mappings: MappingSet::new(),
            flags,
            va,
        }
    }

    /// Allocate a VA range in this address space and map `paddr..paddr+size`
    /// there. Returns the virtual base. On mapping failure the reserved VA
    /// range is returned to the allocator (no leak).
    #[allow(clippy::too_many_arguments)]
    pub fn map_new_range(
        &mut self,
        paddr: u64,
        size: u64,
        flags: MappingFlags,
        object_id: u64,
        align: u64,
        alloc: &mut dyn PageTableAllocator,
    ) -> Result<u64, VmmError> {
        let vaddr = self.va.alloc(size, align).map_err(|_| VmmError::OutOfVa)?;
        match self.map_pages(vaddr, paddr, size, flags, alloc, object_id) {
            Ok(()) => Ok(vaddr),
            Err(e) => {
                let _ = self.va.free(vaddr, size);
                Err(e)
            }
        }
    }

    /// Unmap a previously allocated range and release its VA reservation.
    pub fn unmap_range(
        &mut self,
        vaddr: u64,
        size: u64,
        alloc: &mut dyn PageTableAllocator,
    ) -> Result<(), VmmError> {
        self.unmap_pages(vaddr, size, alloc)?;
        self.va.free(vaddr, size).map_err(|_| VmmError::NotMapped)?;
        Ok(())
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

    /// Unmap an arbitrary page-aligned sub-range.
    ///
    /// Range semantics mirror [`AddressSpace::protect`]: the range must be
    /// covered by existing mappings; partially overlapping mappings are
    /// truncated in the software shadow (head/tail leftovers keep their
    /// permissions), so `MappingSet` stays an exact image of the hardware.
    /// Transactional ordering identical to `protect`: validate + capacity
    /// pre-check before any mutation, hardware first, shadow commit last.
    pub fn unmap_pages(
        &mut self,
        vaddr: u64,
        size: u64,
        alloc: &mut dyn PageTableAllocator,
    ) -> Result<(), VmmError> {
        let region = VaRegion::new(vaddr, size).map_err(|_| VmmError::InvalidRange)?;
        let range_end = region.end();

        const MAX_AFFECTED: usize = 64;
        let mut affected: [(usize, Mapping); MAX_AFFECTED] = [(
            0,
            Mapping::new(VirtRange::new(0, 0), 0, MappingFlags::from_bits(0)),
        ); MAX_AFFECTED];
        let mut n = 0usize;
        for (slot, m) in self.mappings.iter_with_slots() {
            if m.virt_range.base < range_end && vaddr < m.virt_range.end() {
                assert!(n < MAX_AFFECTED, "unmap: affected overflow");
                affected[n] = (slot, *m);
                n += 1;
            }
        }
        if n == 0 {
            return Err(VmmError::NotMapped);
        }
        affected[..n].sort_by_key(|(_, m)| m.virt_range.base);
        let mut cursor = vaddr;
        for (_, m) in &affected[..n] {
            if m.virt_range.base > cursor {
                return Err(VmmError::NotMapped); // gap
            }
            cursor = cursor.max(m.virt_range.end());
        }
        if cursor < range_end {
            return Err(VmmError::NotMapped); // tail uncovered
        }

        let mut extra_slots = 0usize;
        for (_, m) in &affected[..n] {
            extra_slots += usize::from(m.virt_range.base < vaddr)
                + usize::from(m.virt_range.end() > range_end);
        }
        if self.mappings.len() + extra_slots > MappingSet::capacity() {
            return Err(VmmError::MappingTableFull);
        }

        // SAFETY: coverage proven above — every page in range is mapped.
        unsafe {
            vivanta_arch_api::mmu::mmu_unmap(self.root, vaddr, size, alloc);
        }

        for (slot, m) in affected.iter().take(n) {
            let old_flags = m.permissions;
            let base = m.virt_range.base;
            let end = m.virt_range.end();
            self.mappings.remove(*slot);
            if base < vaddr {
                let _ = self.mappings.insert(Mapping::new(
                    VirtRange::new(base, vaddr - base),
                    m.object_id,
                    old_flags,
                ));
            }
            if end > range_end {
                let _ = self.mappings.insert(Mapping::new(
                    VirtRange::new(range_end, end - range_end),
                    m.object_id,
                    old_flags,
                ));
            }
        }
        self.reclaim_empty_tables(alloc);
        Ok(())
    }

    /// Reclaim page-table frames of this address space that are provably
    /// unreachable (ADR-031): a frame leaves the hierarchy only when
    ///
    /// 1. it contains zero valid descriptors (hardware truth, not the
    ///    software shadow — split-inherited block pages make shadow-empty
    ///    tables non-empty),
    /// 2. its parent descriptor is cleared afterwards, and
    /// 3. every leaf translation under it was already invalidated by the
    ///    per-page TLBI performed at unmap time; the cleared parent entry
    ///    adds no translation of its own.
    ///
    /// The check-and-unlink runs under the IRQ guard: a preempting context
    /// mapping into this address space between the emptiness proof and the
    /// unlink would resurrect a reachable table as reclaimable (single-core
    /// TOCTOU rule). Loops to fixpoint because emptying an L3 may empty its
    /// L2. Root frames are boot-allocated, unknown to the registry, and
    /// therefore never reclaimed.
    fn reclaim_empty_tables(&mut self, alloc: &mut dyn PageTableAllocator) {
        let _irq = crate::interrupts_guard();
        loop {
            let Some(entry) = tables::find_empty(self.id) else {
                break;
            };
            // SAFETY: single-core, IRQs disabled; ownership moves out of the
            // registry before the frame is handed back to the backend.
            // SAFETY: single-core, IRQs disabled by the guard above.
            let e = unsafe { tables::take(entry.frame, self.id) }
                .expect("registry entry vanished under IRQ guard");
            // SAFETY: registry-verified table frame and index.
            unsafe {
                vivanta_arch_api::mmu::mmu_clear_table_entry(e.parent_table, e.parent_index);
                alloc.reclaim_page_table_frame(e.frame);
            }
        }
    }

    pub fn query(&self, vaddr: u64) -> Option<&Mapping> {
        self.mappings
            .iter()
            .find(|m| vaddr >= m.virt_range.base && vaddr < m.virt_range.end())
    }

    /// Change permissions on an arbitrary page-aligned sub-range.
    ///
    /// The range must be covered by existing mappings; mappings partially
    /// overlapping the range are split in the software shadow so that
    /// `MappingSet` stays an exact image of the hardware. Transactional
    /// ordering:
    ///
    /// 1. validate alignment / overflow / coverage,
    /// 2. pre-compute all shadow pieces and verify slot capacity — every
    ///    failure path returns before any state changes,
    /// 3. program the hardware (`mmu_protect`; panics only on OOM during a
    ///    block split — boot/runtime-fatal, same policy as `map_pages`),
    /// 4. commit the shadow pieces.
    ///
    /// Transient visibility: single-core, per-descriptor rewrites mean a
    /// concurrent reader observes either the old or the new permission set,
    /// never a torn mapping.
    pub fn protect(
        &mut self,
        vaddr: u64,
        size: u64,
        new_flags: MappingFlags,
        alloc: &mut dyn PageTableAllocator,
    ) -> Result<(), VmmError> {
        let region = VaRegion::new(vaddr, size).map_err(|_| VmmError::InvalidRange)?;
        let range_end = region.end();

        // 1-2. Gather overlapping mappings sorted by base and verify full
        // coverage with no gaps.
        const MAX_AFFECTED: usize = 64;
        let mut affected: [(usize, Mapping); MAX_AFFECTED] = [(
            0,
            Mapping::new(VirtRange::new(0, 0), 0, MappingFlags::from_bits(0)),
        ); MAX_AFFECTED];
        let mut n = 0usize;
        for (slot, m) in self.mappings.iter_with_slots() {
            if m.virt_range.base < range_end && vaddr < m.virt_range.end() {
                affected[n] = (slot, *m);
                n += 1;
                assert!(n <= affected.len(), "protect: affected overflow");
            }
        }
        if n == 0 {
            return Err(VmmError::NotMapped);
        }
        affected[..n].sort_by_key(|(_, m)| m.virt_range.base);
        let mut cursor = vaddr;
        for (_, m) in &affected[..n] {
            if m.virt_range.base > cursor {
                return Err(VmmError::NotMapped); // gap in coverage
            }
            cursor = cursor.max(m.virt_range.end());
        }
        if cursor < range_end {
            return Err(VmmError::NotMapped); // tail not covered
        }

        // Pre-compute pieces; count worst-case slot need before mutating.
        let mut extra_slots = 0usize;
        for (_, m) in &affected[..n] {
            let head = m.virt_range.base < vaddr;
            let tail = m.virt_range.end() > range_end;
            extra_slots += usize::from(head) + usize::from(tail);
        }
        if self.mappings.len() + extra_slots > MappingSet::capacity() {
            return Err(VmmError::MappingTableFull);
        }

        // 3. Program hardware for the whole requested range at once.
        //
        // SAFETY: coverage was proven above — every page in [vaddr, end)
        // is mapped in this address space.
        unsafe {
            vivanta_arch_api::mmu::mmu_protect(self.root, vaddr, size, new_flags, alloc);
        }

        // 4. Commit shadow pieces.
        for (slot, m) in affected.iter().take(n) {
            let old_flags = m.permissions;
            let base = m.virt_range.base;
            let end = m.virt_range.end();
            self.mappings.remove(*slot);
            // Head piece keeps old permissions.
            if base < vaddr {
                let _ = self.mappings.insert(Mapping::new(
                    VirtRange::new(base, vaddr - base),
                    m.object_id,
                    old_flags,
                ));
            }
            // Covered piece gets the new permissions.
            let cov_start = vaddr.max(base);
            let cov_end = range_end.min(end);
            let _ = self.mappings.insert(Mapping::new(
                VirtRange::new(cov_start, cov_end - cov_start),
                m.object_id,
                new_flags,
            ));
            // Tail piece keeps old permissions.
            if end > range_end {
                let _ = self.mappings.insert(Mapping::new(
                    VirtRange::new(range_end, end - range_end),
                    m.object_id,
                    old_flags,
                ));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VmmError {
    MappingTableFull,
    NotMapped,
    OutOfVa,
    InvalidRange,
    AddressSpaceBusy,
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

/// Remove an address space from the registry.
///
/// The space must have no live mappings. All page-table frames recorded in
/// the ownership registry for this AS are reclaimed bottom-up (they are all
/// provably empty once no mappings remain); root frames are boot-allocated,
/// untracked, and leak by design. Boot-era address spaces (whose tables were
/// allocated before the registry existed) simply drop out of the registry —
/// their frames stay leaked, as before.
pub fn unregister(as_id: AddressSpaceId) -> Result<(), VmmError> {
    unsafe {
        for i in 0..MAX_ADDRESS_SPACES {
            let ptr = &raw mut ADDRESS_SPACES[i];
            if let Some(ref aspace) = (*ptr).as_ref() {
                if aspace.id == as_id {
                    if aspace.mappings.len() != 0 {
                        return Err(VmmError::AddressSpaceBusy);
                    }
                    // Reclaim every tracked table of this AS.
                    while let Some(entry) = tables::find_empty(as_id) {
                        // SAFETY: single-core boot context; no concurrent
                        // mapping into a dying address space.
                        let e = tables::take(entry.frame, as_id).expect("registry entry vanished");
                        vivanta_arch_api::mmu::mmu_clear_table_entry(
                            e.parent_table,
                            e.parent_index,
                        );
                        (*e.backend).deallocate(e.frame, 4096);
                    }
                    *ptr = None;
                    return Ok(());
                }
            }
        }
    }
    Err(VmmError::NotMapped)
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

/// Mutable lookup by address-space id.
///
/// # Safety
/// Single-core: the caller must guarantee no aliasing `&mut` is live
/// (the boot monitor uses this under masked IRQs).
pub unsafe fn address_space_mut_by(as_id: AddressSpaceId) -> &'static mut AddressSpace {
    unsafe {
        for i in 0..MAX_ADDRESS_SPACES {
            let ptr = &raw mut ADDRESS_SPACES[i];
            if (*ptr).as_ref().is_some_and(|a| a.id == as_id) {
                return (*ptr).as_mut().expect("checked some");
            }
        }
    }
    panic!("address_space_mut_by: unknown id {}", as_id);
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
