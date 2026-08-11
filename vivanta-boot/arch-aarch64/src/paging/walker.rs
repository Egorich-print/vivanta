use crate::paging::descriptor::*;

pub fn read_desc(addr: u64) -> u64 {
    unsafe { core::ptr::read_volatile(addr as *const u64) }
}

pub fn write_desc(addr: u64, val: u64) {
    unsafe { core::ptr::write_volatile(addr as *mut u64, val) }
}

// ── Walk result ──────────────────────────────────────────────────────────────

pub enum WalkResult {
    ExistingL3(u64, usize),
    NeedsSplit {
        l2_table_addr: u64,
        l2_index: usize,
        block_desc: u64,
    },
}

pub fn walk_to_l3(pt_root: u64, vaddr: u64) -> WalkResult {
    let l1_idx = ((vaddr >> 30) & 0x1FF) as usize;
    let l2_idx = ((vaddr >> 21) & 0x1FF) as usize;
    let l3_idx = ((vaddr >> 12) & 0x1FF) as usize;

    let l1_entry = read_desc(pt_root + (l1_idx as u64) * 8);
    if !desc_is_valid(l1_entry) {
        panic!("walk_to_l3: L1 entry missing at idx {}", l1_idx);
    }
    if !desc_is_table(l1_entry) {
        panic!("walk_to_l3: L1 entry is not a table at idx {}", l1_idx);
    }
    let l2_table = l1_entry & ADDR_MASK;

    let l2_entry = read_desc(l2_table + (l2_idx as u64) * 8);
    if !desc_is_valid(l2_entry) {
        panic!("walk_to_l3: L2 entry missing at idx {}", l2_idx);
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

pub fn tlbi_range(vaddr: u64, size: u64) {
    barrier_write();
    for offset in (0..size).step_by(0x1000) {
        tlbi_page(vaddr + offset);
    }
    barrier_full();
    barrier_insn();
}

pub fn tlbi_all_sync() {
    barrier_full();
    tlbi_all();
    barrier_full();
    barrier_insn();
}
