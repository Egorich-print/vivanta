// ---------------------------------------------------------------------------
// AArch64 4-level page table builder
// ---------------------------------------------------------------------------

use vivanta_arch_api::pmm::{FrameAllocator, PhysFrame};

const ENTRY_VALID: u64 = 1 << 0;
const ENTRY_TABLE: u64 = 1 << 1;
const ENTRY_AF: u64 = 1 << 10;
const ENTRY_PXN: u64 = 1 << 53;
const ENTRY_XN: u64 = 1 << 54;
const ADDR_MASK: u64 = 0x0000_FFFF_FFFF_F000;

const ATTR_NORMAL: u64 = 0;
const SH_INNER: u64 = 3 << 8;

/// Page attribute flags — describes intended access semantics.
///
/// The architecture backend translates these into ISA-specific descriptor bits.
/// See ADR-019 §1 for the encoding table.
#[derive(Clone, Copy, Debug)]
pub struct PageFlags {
    pub writable: bool,
    pub executable: bool,
    pub user: bool,
    pub privileged_executable: bool,
}

impl PageFlags {
    pub const READ_ONLY: Self = Self { writable: false, executable: false, user: false, privileged_executable: true };
    pub const READ_WRITE: Self = Self { writable: true, executable: false, user: false, privileged_executable: true };
    pub const READ_WRITE_EXEC: Self = Self { writable: true, executable: true, user: false, privileged_executable: true };
    pub const USER_READ_WRITE: Self = Self { writable: true, executable: false, user: true, privileged_executable: false };
    pub const USER_READ_WRITE_EXEC: Self = Self { writable: true, executable: true, user: true, privileged_executable: false };
}

fn table_desc(phys: u64) -> u64 {
    ENTRY_VALID | ENTRY_TABLE | (phys & ADDR_MASK)
}

fn block_or_page_desc(phys: u64, flags: PageFlags, is_page: bool) -> u64 {
    let mut d = ENTRY_VALID | ENTRY_AF | SH_INNER | (ATTR_NORMAL << 2);
    if is_page {
        d |= ENTRY_TABLE;
    }
    if flags.user {
        d |= 1 << 6;
    } else if !flags.writable {
        d |= 2 << 6;
    }
    if !flags.privileged_executable {
        d |= ENTRY_PXN;
    }
    if !flags.executable {
        d |= ENTRY_XN;
    }
    d | (phys & ADDR_MASK)
}

pub struct PageTableBuilder<A: FrameAllocator> {
    alloc: A,
    root: u64,
}

pub struct PageTableGuard {
    root: u64,
}

impl<A: FrameAllocator> PageTableBuilder<A> {
    pub fn new(mut alloc: A) -> Self {
        let l1 = alloc.alloc_frame().expect("cannot alloc L1 table").addr;
        unsafe { core::ptr::write_bytes(l1 as *mut u8, 0, 4096) }
        PageTableBuilder { alloc, root: l1 }
    }

    pub fn root_addr(&self) -> u64 {
        self.root
    }

    /// Allocate a frame through the builder's internal allocator.
    pub fn alloc_frame(&mut self) -> Option<PhysFrame> {
        self.alloc.alloc_frame()
    }

    pub fn map(&mut self, virt: u64, phys: u64, size: u64, flags: PageFlags) {
        let mut offset = 0u64;
        while offset < size {
            let va = virt + offset;
            let pa = phys + offset;

            if va & 0x1F_FFFF == 0 && pa & 0x1F_FFFF == 0 && (size - offset) >= 0x20_0000 {
                let l1 = ((va >> 30) & 0x1FF) as usize;
                let l2 = ((va >> 21) & 0x1FF) as usize;
                let l2t = self.table_or_create(self.root, l1);
                self.write(l2t, l2, block_or_page_desc(pa, flags, false));
                offset += 0x20_0000;
            } else {
                let l1 = ((va >> 30) & 0x1FF) as usize;
                let l2 = ((va >> 21) & 0x1FF) as usize;
                let l3 = ((va >> 12) & 0x1FF) as usize;
                let l2t = self.table_or_create(self.root, l1);
                let l3t = self.table_or_create(l2t, l2);
                self.write(l3t, l3, block_or_page_desc(pa, flags, true));
                offset += 0x1000;
            }
        }
        tlbi_range(virt, size);
    }

    pub fn finish(self) -> PageTableGuard {
        PageTableGuard { root: self.root }
    }

    fn read(&self, table: u64, idx: usize) -> u64 {
        unsafe { core::ptr::read_volatile((table + idx as u64 * 8) as *const u64) }
    }

    fn write(&self, table: u64, idx: usize, val: u64) {
        unsafe { core::ptr::write_volatile((table + idx as u64 * 8) as *mut u64, val) }
    }

    fn table_or_create(&mut self, table: u64, idx: usize) -> u64 {
        let entry = self.read(table, idx);
        if entry & ENTRY_VALID != 0 {
            if entry & ENTRY_TABLE == 0 {
                let l3_frame = self.alloc.alloc_frame().expect("cannot alloc L3 frame").addr;
                unsafe { split_l2_block(table, idx, entry, l3_frame); }
                return l3_frame;
            }
            return entry & ADDR_MASK;
        }
        let frame = self.alloc.alloc_frame().expect("cannot alloc page-table frame").addr;
        unsafe { core::ptr::write_bytes(frame as *mut u8, 0, 4096) }
        self.write(table, idx, table_desc(frame));
        frame
    }
}

impl PageTableGuard {
    pub unsafe fn activate(&self) {
        use core::arch::asm;

        asm!("msr mair_el1, {}", in(reg) 0x44_FF_u64);

        let tcr: u64 = (25)
            | (0b01 << 8)
            | (0b01 << 10)
            | (0b11 << 12)
            | (0b00 << 14)
            | (3u64 << 32);
        asm!("msr tcr_el1, {}", in(reg) tcr);
        asm!("msr ttbr0_el1, {}", in(reg) self.root);

        asm!("dsb ish");
        asm!("isb");

        let sctlr: u64 = (1 << 0) | (1 << 2) | (1 << 12);
        asm!("msr sctlr_el1, {}", in(reg) sctlr);
        asm!("isb");
    }

    pub fn root_addr(&self) -> u64 {
        self.root
    }
}

/// Runtime address‑space activation: write TTBR0_EL1 + TLBI.
/// Called by the scheduler when switching between threads with different
/// address spaces.
#[no_mangle]
pub unsafe extern "Rust" fn activate_address_space(root: vivanta_arch_api::mmu::RootPageTable) {
    let ttbr = root.0 as u64;
    core::arch::asm!("msr TTBR0_EL1, {}", in(reg) ttbr);
    tlbi_all_sync();
    core::arch::asm!("ic ialluis");
    core::arch::asm!("dsb sy; isb");
}

// ---------------------------------------------------------------------------
// Runtime page table modification — mmu_map_object / mmu_unmap
// ---------------------------------------------------------------------------

use vivanta_arch_api::mmu::{MappingFlags, PageTableAllocator, RootPageTable};

fn flags_to_desc_bits(flags: MappingFlags, phys: u64) -> u64 {
    let mut d = ENTRY_VALID | ENTRY_TABLE | ENTRY_AF | SH_INNER | (ATTR_NORMAL << 2);

    if flags.is_user() {
        d |= 1 << 6;
    } else if !flags.is_read_write() {
        d |= 2 << 6;
    }

    if !flags.is_executable() {
        d |= ENTRY_PXN | ENTRY_XN;
    }

    d | (phys & ADDR_MASK)
}

fn read_desc(addr: u64) -> u64 {
    unsafe { core::ptr::read_volatile(addr as *const u64) }
}

fn write_desc(addr: u64, val: u64) {
    unsafe { core::ptr::write_volatile(addr as *mut u64, val) }
}

enum WalkResult {
    ExistingL3(u64, usize),
    NeedsSplit {
        l2_table_addr: u64,
        l2_index: usize,
        block_desc: u64,
    },
}

fn walk_to_l3(pt_root: u64, vaddr: u64) -> WalkResult {
    let l1_idx = ((vaddr >> 30) & 0x1FF) as usize;
    let l2_idx = ((vaddr >> 21) & 0x1FF) as usize;
    let l3_idx = ((vaddr >> 12) & 0x1FF) as usize;

    let l1_entry = read_desc(pt_root + (l1_idx as u64) * 8);
    if l1_entry & ENTRY_VALID == 0 {
        panic!("mmu_map_object: L1 entry missing at idx {}", l1_idx);
    }
    if l1_entry & ENTRY_TABLE == 0 {
        panic!("mmu_map_object: L1 entry is not a table at idx {}", l1_idx);
    }
    let l2_table = l1_entry & ADDR_MASK;

    let l2_entry = read_desc(l2_table + (l2_idx as u64) * 8);
    if l2_entry & ENTRY_VALID == 0 {
        panic!("mmu_map_object: L2 entry missing at idx {}", l2_idx);
    }
    if l2_entry & ENTRY_TABLE == 0 {
        return WalkResult::NeedsSplit {
            l2_table_addr: l2_table,
            l2_index: l2_idx,
            block_desc: l2_entry,
        };
    }
    let l3_table = l2_entry & ADDR_MASK;
    WalkResult::ExistingL3(l3_table, l3_idx)
}

unsafe fn split_l2_block(l2_table_addr: u64, l2_index: usize, block_desc: u64, l3_frame_paddr: u64) {
    let block_base = block_desc & ADDR_MASK;
    let attrs = block_desc & !(ADDR_MASK | 0x3);

    let l3_addr = l3_frame_paddr;

    core::ptr::write_bytes(l3_addr as *mut u8, 0, 4096);

    for i in 0..512 {
        let page_paddr = block_base + (i as u64 * 0x1000);
        let desc = ENTRY_VALID | ENTRY_TABLE | ENTRY_AF | attrs | page_paddr;
        write_desc(l3_addr + (i as u64 * 8), desc);
    }

    barrier_write();
    write_desc(l2_table_addr + (l2_index as u64) * 8, l3_addr | ENTRY_VALID | ENTRY_TABLE);
}

fn barrier_write() {
    unsafe { core::arch::asm!("dsb ishst", options(nomem, nostack)); }
}

fn barrier_full() {
    unsafe { core::arch::asm!("dsb ish", options(nomem, nostack)); }
}

fn barrier_insn() {
    unsafe { core::arch::asm!("isb", options(nomem, nostack)); }
}

fn tlbi_page(va: u64) {
    unsafe { core::arch::asm!("tlbi vaae1is, {}", in(reg) va, options(nostack)); }
}

fn tlbi_all() {
    unsafe { core::arch::asm!("tlbi vmalle1is", options(nomem, nostack)); }
}

fn tlbi_range(vaddr: u64, size: u64) {
    barrier_write();
    for offset in (0..size).step_by(0x1000) {
        tlbi_page(vaddr + offset);
    }
    barrier_full();
    barrier_insn();
}

fn tlbi_all_sync() {
    barrier_full();
    tlbi_all();
    barrier_full();
    barrier_insn();
}

#[no_mangle]
pub unsafe extern "Rust" fn mmu_map_object(
    pt: RootPageTable,
    vaddr: u64,
    paddr: u64,
    size: u64,
    flags: MappingFlags,
    alloc: &mut dyn PageTableAllocator,
) {
    let root = pt.0 as u64;
    let mut offset = 0u64;
    while offset < size {
        let va = vaddr + offset;
        loop {
            match walk_to_l3(root, va) {
                WalkResult::ExistingL3(l3_table, l3_idx) => {
                    let desc = flags_to_desc_bits(flags, paddr + offset);
                    write_desc(l3_table + (l3_idx as u64) * 8, desc);
                    break;
                }
                WalkResult::NeedsSplit { l2_table_addr, l2_index, block_desc } => {
                    let frame_paddr = alloc.alloc_page_table_frame();
                    split_l2_block(l2_table_addr, l2_index, block_desc, frame_paddr);
                }
            }
        }
        offset += 0x1000;
    }
    tlbi_range(vaddr, size);
}

#[no_mangle]
pub unsafe extern "Rust" fn mmu_unmap(
    pt: RootPageTable,
    vaddr: u64,
    size: u64,
    alloc: &mut dyn PageTableAllocator,
) {
    let root = pt.0 as u64;
    let mut offset = 0u64;
    while offset < size {
        let va = vaddr + offset;
        loop {
            match walk_to_l3(root, va) {
                WalkResult::ExistingL3(l3_table, l3_idx) => {
                    write_desc(l3_table + (l3_idx as u64) * 8, 0);
                    break;
                }
                WalkResult::NeedsSplit { l2_table_addr, l2_index, block_desc } => {
                    let frame_paddr = alloc.alloc_page_table_frame();
                    split_l2_block(l2_table_addr, l2_index, block_desc, frame_paddr);
                }
            }
        }
        offset += 0x1000;
    }
    tlbi_range(vaddr, size);
}

/// Flush D-cache (to PoC) and invalidate I-cache (to PoU) for a virtual address range.
///
/// Must be called after writing code that will be fetched as instructions,
/// e.g. loading user code into a data page.  `va` is the *virtual* address
/// of the range (or identity-mapped physical address if MMU is off).
pub fn flush_icache_range(va: u64, size: u64) {
    let ctr_el0: u64;
    unsafe { core::arch::asm!("mrs {}, ctr_el0", out(reg) ctr_el0) }
    let d_min_line = (ctr_el0 >> 16) & 0xF;
    let line = (4u64) << d_min_line;
    let mask = line - 1;
    let start = va & !mask;
    let end = va + size;
    let mut addr = start;
    unsafe {
        while addr < end {
            core::arch::asm!("dc cvac, {}", in(reg) addr);
            addr += line;
        }
        core::arch::asm!("dsb sy");
        addr = start;
        while addr < end {
            core::arch::asm!("ic ivau, {}", in(reg) addr);
            addr += line;
        }
        core::arch::asm!("dsb sy; isb");
    }
}
