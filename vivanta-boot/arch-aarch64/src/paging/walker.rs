use crate::paging::descriptor::*;

pub fn read_desc(addr: u64) -> u64 {
    unsafe { core::ptr::read_volatile(addr as *const u64) }
}

/// Write a descriptor entry.
///
/// The store must be cleaned to PoC: ARM table walkers do not snoop the
/// data cache, so a descriptor that lives only in a dirty cache line is
/// invisible to subsequent translations. `PageTableBuilder::write` has
/// always done this; the runtime paths (map/unmap/protect/demand-fill)
/// go through this function, so the clean lives here — one choke point.
pub fn write_desc(addr: u64, val: u64) {
    unsafe {
        core::ptr::write_volatile(addr as *mut u64, val);
        // ARM table walkers do not snoop the data cache — a descriptor that
        // lives only in a dirty cache line is invisible to translations. Clean
        // to PoC so runtime map/unmap/protect/demand-fill entries are coherent.
        core::arch::asm!("dc civac, {}", in(reg) addr, options(nostack, preserves_flags));
        core::arch::asm!("dsb ish", options(nomem, nostack, preserves_flags));
    }
}

// ── Walk result ──────────────────────────────────────────────────────────────

pub enum WalkResult {
    ExistingL3(u64, usize),
    NeedsSplit {
        l2_table_addr: u64,
        l2_index: usize,
        block_desc: u64,
    },
    /// L1 entry invalid or not a table: the caller may allocate and
    /// install an L2 table there (map path only).
    MissingL2 {
        l1_table: u64,
        l1_index: usize,
    },
    /// L2 entry invalid: the caller may allocate and install an L3 table
    /// there (map path only).
    MissingL3 {
        l2_table: u64,
        l2_index: usize,
    },
}

/// Non-fatal walk used by the map path: reports what is missing instead of
/// panicking, so an allocator-backed mapper can create intermediate tables.
pub fn walk_to_l3(pt_root: u64, vaddr: u64) -> WalkResult {
    let l1_idx = ((vaddr >> 30) & 0x1FF) as usize;
    let l2_idx = ((vaddr >> 21) & 0x1FF) as usize;
    let l3_idx = ((vaddr >> 12) & 0x1FF) as usize;

    let l1_entry = read_desc(pt_root + (l1_idx as u64) * 8);
    if !desc_is_valid(l1_entry) || !desc_is_table(l1_entry) {
        return WalkResult::MissingL2 {
            l1_table: pt_root,
            l1_index: l1_idx,
        };
    }
    let l2_table = l1_entry & ADDR_MASK;

    let l2_entry = read_desc(l2_table + (l2_idx as u64) * 8);
    if !desc_is_valid(l2_entry) {
        return WalkResult::MissingL3 {
            l2_table,
            l2_index: l2_idx,
        };
    }
    if desc_is_block(l2_entry) {
        return WalkResult::NeedsSplit {
            l2_table_addr: l2_table,
            l2_index: l2_idx,
            block_desc: l2_entry,
        };
    }
    let l3_table = l2_entry & ADDR_MASK;
    WalkResult::ExistingL3(l3_table, l3_idx)
}

/// Install a freshly zeroed child table descriptor into `parent[index]`.
/// Mechanism primitive: no allocation, barriers included.
///
/// # Safety
/// - `frame_paddr` must be a valid, zeroed 4 KiB table frame.
/// - `parent`/`index` must designate a live table slot.
pub unsafe fn install_child_table(parent: u64, index: usize, frame_paddr: u64) {
    write_desc(
        parent + (index as u64) * 8,
        DESC_VALID | DESC_TABLE | (frame_paddr & ADDR_MASK),
    );
}

// ── Block split ──────────────────────────────────────────────────────────────

/// Split a 2 MiB block descriptor into a full L3 table of 4 KiB page entries.
///
/// SAFETY:
/// - Only performs descriptor transformation. Never allocates memory.
/// - Caller provides a valid, zeroed L3 table frame at `l3_frame_paddr`.
pub unsafe fn split_l2_block(
    l2_table_addr: u64,
    l2_index: usize,
    block_desc: u64,
    l3_frame_paddr: u64,
) {
    let block_base = block_desc & ADDR_MASK;
    let attrs = block_desc & !(ADDR_MASK | 0x3);

    let l3_addr = l3_frame_paddr;

    for i in 0..512 {
        let page_paddr = block_base + (i as u64 * 0x1000);
        let desc = DESC_VALID | DESC_TABLE | DESC_AF | attrs | page_paddr;
        write_desc(l3_addr + (i as u64 * 8), desc);
    }

    barrier_write();
    write_desc(
        l2_table_addr + (l2_index as u64) * 8,
        l3_addr | DESC_VALID | DESC_TABLE, // L2 table descriptor (see table_desc note)
    );
}

// ── Permission rewrite (pure mechanism) ─────────────────────────────────────

/// Rewrite the permission bits of a leaf descriptor (L2 block or L3 page).
///
/// Preserves validity, type, output address, AF, shareability and ATTRIDX;
/// replaces AP[2:1] (via `ap_bits`) and XN/PXN. Pure bit transformation —
/// never reads or writes memory, never allocates (ADR-030 §4).
///
/// Note: `executable` grants EL0 execution (XN=0); PXN is always set for
/// user-executable pages so EL1 can never fetch from them. Kernel mappings
/// keep PXN clear only when `executable` is requested for EL1 — the runtime
/// mmu layer never maps EL1-executable user pages, so a single XN/PXN pair
/// covers both cases: `!executable → XN|PXN`, `executable → XN cleared,
/// PXN set iff user`.
pub fn leaf_with_permissions(desc: u64, user: bool, writable: bool, executable: bool) -> u64 {
    let mut d = desc & !(DESC_AP_MASK | DESC_PXN | DESC_XN);
    d |= ap_bits(user, writable);
    if !executable {
        d |= DESC_PXN | DESC_XN;
    } else if user {
        d |= DESC_PXN;
    }
    d
}

// ── Barriers ─────────────────────────────────────────────────────────────────

pub fn barrier_write() {
    unsafe {
        core::arch::asm!("dsb ishst", options(nomem, nostack));
    }
}

pub fn barrier_full() {
    unsafe {
        core::arch::asm!("dsb ish", options(nomem, nostack));
    }
}

pub fn barrier_insn() {
    unsafe {
        core::arch::asm!("isb", options(nomem, nostack));
    }
}

// ── TLBI ─────────────────────────────────────────────────────────────────────

pub fn tlbi_page(va: u64) {
    unsafe {
        core::arch::asm!("tlbi vaae1is, {}", in(reg) va, options(nostack));
    }
}

pub fn tlbi_all() {
    unsafe {
        core::arch::asm!("tlbi vmalle1is", options(nomem, nostack));
    }
}

/// Invalidate translations after runtime descriptor changes.
///
/// M6.0 note: this issues a FULL `tlbi vmalle1is` rather than per-VA
/// `tlbi vaae1is`. Vivanta is single-core with no ASIDs, mapping
/// operations are rare, and a full flush is bulletproof against both
/// QEMU's per-VA invalidation quirks (observed: stale entries surviving
/// `tlbi vaae1is` across descriptor re-writes at recycled VAs) and any
/// hardware erratum. Per-VA invalidation returns when ASIDs arrive
/// (post-M6 backlog) and must be re-validated on hardware.
pub fn tlbi_range(vaddr: u64, size: u64) {
    let _ = (vaddr, size); // kept for call-site compatibility
    barrier_write();
    tlbi_all();
    barrier_full();
    barrier_insn();
}

pub fn tlbi_all_sync() {
    barrier_full();
    tlbi_all();
    barrier_full();
    barrier_insn();
}
