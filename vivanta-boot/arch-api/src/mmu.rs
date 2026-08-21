// ---------------------------------------------------------------------------
// Root page table handle + page mapping flags
// ---------------------------------------------------------------------------

/// Opaque handle to an architecture-specific root page table.
///
/// The vivanta_kernel never inspects the value; each architecture defines how it
/// maps to hardware (AArch64 → TTBR0_EL1, x86_64 → CR3, RISC‑V → SATP).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RootPageTable(pub usize);

/// Opaque page-mapping flags. Bits are defined by each architecture
/// implementation. Kernel constructs flags via named methods only.
#[derive(Clone, Copy, Debug)]
pub struct MappingFlags {
    bits: u64,
}

impl MappingFlags {
    pub const fn from_bits(bits: u64) -> Self {
        Self { bits }
    }

    pub const fn read_write() -> Self {
        Self { bits: 0b001 }
    }

    pub const fn executable() -> Self {
        Self { bits: 0b010 }
    }

    pub const fn user() -> Self {
        Self { bits: 0b100 }
    }

    pub fn is_read_write(&self) -> bool {
        self.bits & 0b001 != 0
    }
    pub fn is_executable(&self) -> bool {
        self.bits & 0b010 != 0
    }
    pub fn is_user(&self) -> bool {
        self.bits & 0b100 != 0
    }
}

impl core::ops::BitOr for MappingFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self {
            bits: self.bits | rhs.bits,
        }
    }
}

/// Allocator for intermediate page table frames.
///
/// Separates the MMU page-table walker from the memory source.
/// Implementations can wrap PMM, MemoryObject, a bootstrap allocator, etc.
///
/// Ownership protocol (ADR-031): the architecture layer allocates a frame
/// and installs it into a parent table; ownership is transferred to the
/// caller's registry via [`PageTableAllocator::table_installed`]. A frame
/// may be returned to the allocator only through
/// [`PageTableAllocator::reclaim_page_table_frame`] after the caller has
/// proven it unreachable (parent entry cleared + TLB invalidated).
pub trait PageTableAllocator {
    fn alloc_page_table_frame(&mut self) -> u64;

    /// Notification: the arch layer installed `frame` as a child table at
    /// `index` of `parent_table`. `level` is 2 for L2 tables (holding 2 MiB
    /// blocks / L3 pointers) and 3 for L3 tables (holding 4 KiB pages),
    /// using this kernel's root=L1 naming.
    /// Default: no-op (boot-era allocators keep the intentional-leak model).
    fn table_installed(&mut self, _frame: u64, _parent_table: u64, _index: usize, _level: u8) {}

    /// Return a previously-installed, provably-unreachable table frame to
    /// the underlying memory source. Only called by the reclamation path.
    /// Default: no-op (frame leaks — safe fallback).
    fn reclaim_page_table_frame(&mut self, _frame: u64) {}
}

unsafe extern "Rust" {
    /// Activate an address space by writing its root page table into the
    /// hardware register (TTBR0_EL1 on AArch64, CR3 on x86_64, SATP on RISC‑V).
    ///
    /// # Safety
    ///
    /// - `root` must point to a valid, fully‑constructed page table.
    /// - The caller must ensure no conflicting translation is live.
    pub fn activate_address_space(root: RootPageTable);

    /// Map a physical region at a virtual address in an existing address space.
    ///
    /// Maps `size` bytes starting at `paddr` to `vaddr` with the given flags.
    /// When an L2 block descriptor is encountered, `alloc` is called to obtain
    /// a frame for the new L3 page table.
    ///
    /// # Safety
    ///
    /// - `pt` must be a valid root page table.
    /// - The virtual range must not already be mapped.
    /// - 4KiB-aligned regions, single 4KiB page at a time.
    pub fn mmu_map_object(
        pt: RootPageTable,
        vaddr: u64,
        paddr: u64,
        size: u64,
        flags: MappingFlags,
        alloc: &mut dyn PageTableAllocator,
    );

    /// Unmap a virtual region from an existing address space.
    ///
    /// Clears the leaf descriptors and invalidates TLB entries.
    ///
    /// # Safety
    ///
    /// - `pt` must be a valid root page table.
    /// - The virtual range must be currently mapped.
    pub fn mmu_unmap(pt: RootPageTable, vaddr: u64, size: u64, alloc: &mut dyn PageTableAllocator);

    /// Change the permissions of an already-mapped virtual range.
    ///
    /// Rewrites the permission bits (access permission, execute-never) of
    /// every leaf descriptor covering `[vaddr, vaddr + size)` to `flags`,
    /// preserving physical addressing, memory type, shareability and the
    /// access flag. Where a 2 MiB block descriptor covers only part of the
    /// requested change, it is split into 4 KiB pages first (`alloc` supplies
    /// the L3 frame). TLB entries for the range are invalidated before return.
    ///
    /// The range must be page-aligned and cover whole pages. Physical frames
    /// are never allocated for data — only for split page tables.
    ///
    /// # Safety
    ///
    /// - `pt` must be a valid root page table.
    /// - Every page in the range must currently be mapped.
    pub fn mmu_protect(
        pt: RootPageTable,
        vaddr: u64,
        size: u64,
        flags: MappingFlags,
        alloc: &mut dyn PageTableAllocator,
    );

    /// Count valid leaf descriptors in a table frame (mechanism primitive).
    ///
    /// Reads the 512 entries of the table at physical address `table_pa`
    /// and returns how many are valid. Used by the kernel's reclamation
    /// policy to prove a table is empty before unlinking it. Pure read.
    pub fn mmu_table_valid_leaves(table_pa: u64) -> u32;

    /// Read the raw leaf descriptor covering `va` in the table rooted at
    /// `root_pa` (mechanism primitive). Returns 0 when no valid leaf exists.
    /// Used by boot-time audits to assert descriptor attributes (AF, AP, XN)
    /// that QEMU does not enforce.
    pub fn mmu_leaf_descriptor(root_pa: u64, va: u64) -> u64;

    /// Clear one descriptor entry in a table frame (mechanism primitive).
    ///
    /// Writes an invalid (0) descriptor at `table_pa[index]` with the
    /// required barrier. Used to unlink an empty child table from its
    /// parent. The caller must have invalidated any translations that
    /// depended on the subtree being unlinked.
    ///
    /// # Safety
    ///
    /// - `table_pa` must be a live table frame recorded in the caller's
    ///   ownership registry.
    /// - `index` must be < 512.
    pub unsafe fn mmu_clear_table_entry(table_pa: u64, index: usize);
}
