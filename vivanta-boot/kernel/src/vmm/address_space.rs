use super::mapping::{Backing, Mapping, MappingSet, PhysOwnership, VirtRange};
use super::tables;
use vivanta_arch_api::mmu::{MappingFlags, PageTableAllocator, RootPageTable};
use vivanta_vm::va::{VaAllocator, VaRegion};

pub type AddressSpaceId = u64;

// Storage-budget guard (mission-2 lesson: AddressSpace temporaries live on
// the boot stack in register()/init_kernel_address_space; the stack is
// 64 KiB and kernel_main locals share it). Growth past this bound must be
// a conscious decision, never an accident of struct evolution.
const _: () = assert!(core::mem::size_of::<AddressSpace>() <= 12 * 1024);
const _: () = assert!(core::mem::size_of::<MappingSet>() <= 5 * 1024);

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

        // SAFETY: each Present run is fully covered by live mappings;
        // Lazy/Reserved pieces have no hardware image to clear.
        self.for_present_runs(vaddr, size, |run_start, run_len| unsafe {
            vivanta_arch_api::mmu::mmu_unmap(self.root, run_start, run_len, alloc);
        });

        for (slot, m) in affected.iter().take(n) {
            let old_flags = m.permissions;
            let base = m.virt_range.base;
            let end = m.virt_range.end();
            // Present+Anonymous pieces fully covered by the unmap release
            // their frames (ADR-032 §4): the frame is reachable only through
            // this mapping, so ownership ends here.
            if m.backing == Backing::Present
                && m.phys == PhysOwnership::Anonymous
                && base >= vaddr
                && end <= range_end
            {
                if let Some(backend) = crate::vmm::faults::anonymous_backend() {
                    // SAFETY: backend outlives boot; single-core unmap.
                    unsafe { (*backend).deallocate(m.pa, m.virt_range.size) };
                }
            }
            self.mappings.remove(*slot);
            if base < vaddr {
                let mut piece = *m;
                piece.virt_range = VirtRange::new(base, vaddr - base);
                piece.permissions = old_flags;
                let _ = self.mappings.insert(piece);
            }
            if end > range_end {
                let mut piece = *m;
                piece.virt_range = VirtRange::new(range_end, end - range_end);
                piece.permissions = old_flags;
                let _ = self.mappings.insert(piece);
            }
        }
        self.reclaim_empty_tables(alloc);
        Ok(())
    }

    /// Invoke `f(run_start, run_len)` for every maximal run of *Present*
    /// shadow pieces inside `[vaddr, vaddr+size)`. Lazy/Reserved pieces
    /// have no hardware image and are skipped; adjacent Present pieces are
    /// coalesced so the hardware is touched once per contiguous run.
    fn for_present_runs(&self, vaddr: u64, size: u64, mut f: impl FnMut(u64, u64)) {
        let range_end = vaddr + size;
        let mut pieces: alloc::vec::Vec<(u64, u64)> = self
            .mappings
            .iter()
            .filter(|m| {
                m.backing == Backing::Present
                    && m.virt_range.base < range_end
                    && vaddr < m.virt_range.end()
            })
            .map(|m| {
                (
                    m.virt_range.base.max(vaddr),
                    m.virt_range.end().min(range_end),
                )
            })
            .collect();
        pieces.sort_unstable();
        let mut i = 0;
        while i < pieces.len() {
            let (start, mut end) = pieces[i];
            while i + 1 < pieces.len() && pieces[i + 1].0 == end {
                end = pieces[i + 1].1;
                i += 1;
            }
            f(start, end - start);
            i += 1;
        }
    }

    /// Reserve a specific VA range as LazyAnonymous (used by the ELF
    /// loader for ET_EXEC segments with fixed virtual addresses).
    /// Fails if the range is already occupied or outside the domain.
    pub fn reserve_at(
        &mut self,
        vaddr: u64,
        size: u64,
        permissions: MappingFlags,
        object_id: u64,
    ) -> Result<(), VmmError> {
        if size == 0 || vaddr % 4096 != 0 {
            return Err(VmmError::InvalidRange);
        }
        self.va
            .reserve(vaddr, size)
            .map_err(|_| VmmError::OutOfVa)?;
        let mapping = Mapping::lazy_anonymous(VirtRange::new(vaddr, size), object_id, permissions);
        if self.mappings.insert(mapping).is_none() {
            let _ = self.va.free(vaddr, size);
            return Err(VmmError::MappingTableFull);
        }
        Ok(())
    }

    /// Reserve a lazy anonymous mapping: VA range becomes occupied, no
    /// hardware mapping is created, first access demand-fills one page.
    pub fn reserve_lazy(
        &mut self,
        size: u64,
        permissions: MappingFlags,
        object_id: u64,
        align: u64,
    ) -> Result<u64, VmmError> {
        let vaddr = self.va.alloc(size, align).map_err(|_| VmmError::OutOfVa)?;
        let mapping = Mapping::lazy_anonymous(VirtRange::new(vaddr, size), object_id, permissions);
        if self.mappings.insert(mapping).is_none() {
            let _ = self.va.free(vaddr, size);
            return Err(VmmError::MappingTableFull);
        }
        Ok(vaddr)
    }

    /// Resolve a data abort at `va` by materializing exactly one page of a
    /// LazyAnonymous piece (ADR-032 §2.1). Returns false when the fault is
    /// not resolvable — the caller must treat it as fatal.
    ///
    /// Transaction order (hard rule #6): validate → allocate+zero →
    /// hardware map → shadow transition last.
    pub fn resolve_lazy_fault(
        &mut self,
        va: u64,
        write: bool,
        alloc: &mut dyn PageTableAllocator,
    ) -> bool {
        let Some(m) = self.query(va & !0xFFF) else {
            vivanta_boot_common::println!("  [VMR] reject: no mapping {:#x}", va);
            return false;
        };
        if m.backing != Backing::LazyAnonymous {
            vivanta_boot_common::println!(
                "  [VMR] reject: state={:?} {:#x} piece={:#x}..{:#x} perms={:?}",
                m.backing,
                va,
                m.virt_range.base,
                m.virt_range.end(),
                m.permissions
            );
            return false;
        }
        // Permission gate: write requires RW; reads are granted by every
        // encodable permission combination (all Vivanta flags are readable).
        if write && !m.permissions.is_read_write() {
            vivanta_boot_common::println!(
                "  [VMR] reject: write to RO {:#x} piece={:#x}..{:#x} perms={:?}",
                va,
                m.virt_range.base,
                m.virt_range.end(),
                m.permissions
            );
            return false;
        }
        let piece_base = m.virt_range.base;
        let piece_end = m.virt_range.end();
        // Fill uses CURRENT permissions — post-mprotect state (hard rule #10).
        let perms = m.permissions;
        let object_id = m.object_id;

        let page = va & !0xFFF;

        // Allocate + zero the backing frame.
        let Some(frame) = alloc.try_alloc_page_table_frame() else {
            vivanta_boot_common::println!("  [VM] OOM during demand fill at {:#x}", va);
            return false;
        };
        // SAFETY: frame is a live 4 KiB allocation.
        unsafe { core::ptr::write_bytes(frame as *mut u8, 0, 4096) };

        // SAFETY: page is inside a Lazy piece — no descriptor exists for it.
        unsafe {
            vivanta_arch_api::mmu::mmu_map_object(self.root, page, frame, 4096, perms, alloc);
        }
        vivanta_boot_common::println!(
            "  [VMR] mapped page={:#x} frame={:#x} root={:#x}",
            page,
            frame,
            self.root.0
        );

        // Shadow transition (transactional, value-keyed): split
        // [piece_base..piece_end) into [head Lazy][Present page][tail Lazy].
        let mut pieces: alloc::vec::Vec<Mapping> = alloc::vec::Vec::new();
        if piece_base < page {
            pieces.push(Mapping::lazy_anonymous(
                VirtRange::new(piece_base, page - piece_base),
                object_id,
                perms,
            ));
        }
        pieces.push(Mapping::present(
            VirtRange::new(page, 4096),
            object_id,
            perms,
            frame,
            PhysOwnership::Anonymous,
        ));
        if piece_end > page + 4096 {
            pieces.push(Mapping::lazy_anonymous(
                VirtRange::new(page + 4096, piece_end - page - 4096),
                object_id,
                perms,
            ));
        }
        let affected_value = Mapping::lazy_anonymous(m.virt_range, object_id, perms);
        self.mappings
            .replace_slots(core::slice::from_ref(&affected_value), &pieces)
            .map_err(|_| VmmError::MappingTableFull)
            .expect("lazy split: capacity pre-checked by query");
        true
    }

    /// Materialize a LazyAnonymous page with a PRE-FILLED frame (ELF
    /// loader path). Same transaction as `resolve_lazy_fault` except the
    /// caller supplies the frame contents instead of zero-fill. The
    /// frame's ownership transfers to the mapping (Anonymous).
    pub fn materialize_with(
        &mut self,
        va: u64,
        frame: u64,
        flags: MappingFlags,
        alloc: &mut dyn PageTableAllocator,
    ) -> Result<(), VmmError> {
        let page = va & !0xFFF;
        let Some(m) = self.query(page) else {
            return Err(VmmError::NotMapped);
        };
        if m.backing != Backing::LazyAnonymous {
            return Err(VmmError::InvalidRange);
        }
        let piece_base = m.virt_range.base;
        let piece_end = m.virt_range.end();
        let object_id = m.object_id;

        // SAFETY: page is inside a Lazy piece — no descriptor exists.
        unsafe {
            vivanta_arch_api::mmu::mmu_map_object(self.root, page, frame, 4096, flags, alloc);
        }

        // Shadow split: same as resolve_lazy_fault.
        let mut pieces: alloc::vec::Vec<Mapping> = alloc::vec::Vec::new();
        if piece_base < page {
            pieces.push(Mapping::lazy_anonymous(
                VirtRange::new(piece_base, page - piece_base),
                object_id,
                m.permissions,
            ));
        }
        pieces.push(Mapping::present(
            VirtRange::new(page, 4096),
            object_id,
            flags,
            frame,
            PhysOwnership::Anonymous,
        ));
        if piece_end > page + 4096 {
            pieces.push(Mapping::lazy_anonymous(
                VirtRange::new(page + 4096, piece_end - page - 4096),
                object_id,
                m.permissions,
            ));
        }
        let affected = Mapping::lazy_anonymous(m.virt_range, object_id, m.permissions);
        self.mappings
            .replace_slots(core::slice::from_ref(&affected), &pieces)
            .map_err(|_| VmmError::MappingTableFull)?;
        Ok(())
    }

    /// INV-VM-001 mechanical verifier: for every shadow piece in this
    /// address space, the hardware image must match the logical state
    /// exactly. Present ⇔ valid leaf with matching permission bits;
    /// Lazy/Reserved ⇔ no leaf. Only meaningful for allocator-managed ASes.
    pub fn verify_hardware_consistency(&self) -> Result<(), VmmError> {
        for (_, m) in self.mappings.iter_with_slots() {
            match m.backing {
                Backing::Present => {
                    let mut off = 0u64;
                    while off < m.virt_range.size {
                        // SAFETY: read-only descriptor probe.
                        let desc = unsafe {
                            vivanta_arch_api::mmu::mmu_leaf_descriptor(
                                self.root.0 as u64,
                                m.virt_range.base + off,
                            )
                        };
                        if desc & 1 == 0 {
                            return Err(VmmError::NotMapped); // Present w/o hw
                        }
                        let expected = vivanta_arch_api::mmu::mmu_permission_bits(m.permissions);
                        if desc & expected != expected {
                            return Err(VmmError::InvalidRange); // perm drift
                        }
                        off += 4096;
                    }
                }
                Backing::LazyAnonymous | Backing::Reserved => {
                    let mut off = 0u64;
                    while off < m.virt_range.size {
                        // SAFETY: read-only descriptor probe.
                        let desc = unsafe {
                            vivanta_arch_api::mmu::mmu_leaf_descriptor(
                                self.root.0 as u64,
                                m.virt_range.base + off,
                            )
                        };
                        if desc & 1 != 0 {
                            return Err(VmmError::InvalidRange); // ghost PTE
                        }
                        off += 4096;
                    }
                }
            }
        }
        Ok(())
    }

    /// INV-VM-001 reverse direction: below the allocator's high-water
    /// mark, a descriptor may exist ONLY under a Present piece. Anything
    /// else (freed ranges, Lazy/Reserved pieces, never-touched pages up to
    /// the watermark) must have no leaf — this is the check that catches
    /// stale translations the forward pass cannot see, because it has no
    /// shadow piece to compare against.
    ///
    /// Cost: O(high_water / 4K) walks — boot-audit only, not per-operation.
    pub fn verify_domain_reverse(&self) -> Result<(), VmmError> {
        if self.va.is_disabled() {
            return Ok(());
        }
        let hi = self.va.high_water();
        let mut page = USER_VA_BASE;
        while page < hi {
            let covered_present = self.mappings.iter().any(|m| {
                m.backing == Backing::Present
                    && m.virt_range.base <= page
                    && page < m.virt_range.end()
            });
            // SAFETY: read-only descriptor probe.
            let desc =
                unsafe { vivanta_arch_api::mmu::mmu_leaf_descriptor(self.root.0 as u64, page) };
            if covered_present {
                if desc & 1 == 0 {
                    return Err(VmmError::NotMapped);
                }
            } else if desc & 1 != 0 {
                return Err(VmmError::InvalidRange); // ghost leaf
            }
            page += 4096;
        }
        Ok(())
    }

    /// Unmap every mapping in the allocator-managed domain and release
    /// all anonymous frames. Used at process teardown. Tolerates gaps:
    /// each live piece is removed individually.
    pub fn unmap_all(&mut self, alloc: &mut dyn PageTableAllocator) -> Result<(), VmmError> {
        loop {
            let Some((base, size)) = self
                .mappings
                .iter()
                .next()
                .map(|m| (m.virt_range.base, m.virt_range.size))
            else {
                break;
            };
            // SAFETY: piece exists; per-piece removal is exact.
            unsafe {
                vivanta_arch_api::mmu::mmu_unmap(self.root, base, size, alloc);
            }
            let affected_values: alloc::vec::Vec<Mapping> = self
                .mappings
                .iter()
                .filter(|m| m.virt_range.base == base && m.virt_range.size == size)
                .copied()
                .collect();
            // Release anonymous frames.
            for m in &affected_values {
                if m.backing == Backing::Present && m.phys == PhysOwnership::Anonymous {
                    if let Some(backend) = crate::vmm::faults::anonymous_backend() {
                        // SAFETY: backend outlives boot; single-core.
                        unsafe { (*backend).deallocate(m.pa, m.virt_range.size) };
                    }
                }
            }
            let empty: alloc::vec::Vec<Mapping> = alloc::vec::Vec::new();
            self.mappings
                .replace_slots(&affected_values, &empty)
                .map_err(|_| VmmError::MappingTableFull)?;
            self.reclaim_empty_tables(alloc);
        }
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

        // 3. Program hardware — but only over Present runs. Lazy/Reserved
        // pieces have no descriptors to rewrite (ADR-032 §1); their
        // permission change is metadata-only and takes effect when the
        // page materializes.
        //
        // SAFETY: each run is fully covered by Present mappings.
        self.for_present_runs(vaddr, size, |run_start, run_len| unsafe {
            vivanta_arch_api::mmu::mmu_protect(self.root, run_start, run_len, new_flags, alloc);
        });

        // 4. Commit shadow pieces. Every piece keeps its ORIGINAL
        // backing/pa/phys — only permissions change (ADR-032: a Lazy piece
        // must never be committed as Present).
        for (slot, m) in affected.iter().take(n) {
            let old_flags = m.permissions;
            let base = m.virt_range.base;
            let end = m.virt_range.end();
            self.mappings.remove(*slot);
            if base < vaddr {
                let mut piece = *m;
                piece.virt_range = VirtRange::new(base, vaddr - base);
                piece.permissions = old_flags;
                let _ = self.mappings.insert(piece);
            }
            let cov_start = vaddr.max(base);
            let cov_end = range_end.min(end);
            let mut piece = *m;
            piece.virt_range = VirtRange::new(cov_start, cov_end - cov_start);
            piece.permissions = new_flags;
            let _ = self.mappings.insert(piece);
            if end > range_end {
                let mut piece = *m;
                piece.virt_range = VirtRange::new(range_end, end - range_end);
                piece.permissions = old_flags;
                let _ = self.mappings.insert(piece);
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

/// Find a registered address space by its root table physical address —
/// the fault path identifies the active AS by matching TTBR0_EL1, so no
/// "current AS" global state exists and no stale references are possible.
pub fn find_by_root(root_pa: u64) -> Option<&'static mut AddressSpace> {
    unsafe {
        for i in 0..MAX_ADDRESS_SPACES {
            let ptr = &raw mut ADDRESS_SPACES[i];
            if (*ptr).as_ref().is_some_and(|a| a.root.0 as u64 == root_pa) {
                return (*ptr).as_mut();
            }
        }
    }
    None
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
