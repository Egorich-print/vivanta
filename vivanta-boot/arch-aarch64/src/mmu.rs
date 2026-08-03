// ---------------------------------------------------------------------------
// AArch64 4-level page table builder
// ---------------------------------------------------------------------------

use vivanta_arch_api::pmm::{FrameAllocator, PhysFrame};
use crate::barrier;
use crate::paging::descriptor::*;
use crate::paging::walker::*;

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
    pub device: bool,
}

impl PageFlags {
    pub const READ_ONLY: Self = Self { writable: false, executable: false, user: false, privileged_executable: true, device: false };
    pub const READ_WRITE: Self = Self { writable: true, executable: false, user: false, privileged_executable: true, device: false };
    pub const READ_WRITE_EXEC: Self = Self { writable: true, executable: true, user: false, privileged_executable: true, device: false };
    pub const USER_READ_WRITE: Self = Self { writable: true, executable: false, user: true, privileged_executable: false, device: false };
    pub const USER_READ_WRITE_EXEC: Self = Self { writable: true, executable: true, user: true, privileged_executable: false, device: false };
    pub const DEVICE: Self = Self { writable: true, executable: false, user: false, privileged_executable: false, device: true };
    pub const USER_DEVICE: Self = Self { writable: true, executable: false, user: true, privileged_executable: false, device: true };
}

fn table_desc(phys: u64) -> u64 {
    DESC_VALID | DESC_TABLE | (phys & ADDR_MASK)
}

fn block_or_page_desc(phys: u64, flags: PageFlags, is_page: bool) -> u64 {
    let attr_idx = if flags.device { DESC_ATTRIDX_DEVICE } else { DESC_ATTRIDX_NORMAL };
    let sh = if flags.device { DESC_SH_NON } else { DESC_SH_INNER };
    let mut d = DESC_VALID | DESC_AF | sh | attr_idx;
    if is_page {
        d |= DESC_TABLE;
    }
    if flags.user {
        d |= 1 << 6;
    } else if !flags.writable {
        d |= 2 << 6;
    }
    if !flags.privileged_executable {
        d |= DESC_PXN;
    }
    if !flags.executable {
        d |= DESC_XN;
    }
    d | (phys & ADDR_MASK)
}

pub struct PageTableBuilder<A: FrameAllocator> {
    alloc: A,
    root: u64,
}

pub struct PageTableGuard {
    pub root: u64,
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
        let addr = table + idx as u64 * 8;
        unsafe { core::ptr::write_volatile(addr as *mut u64, val) }
        // Clean data cache to PoC — ensures walker sees the entry
        unsafe { core::arch::asm!("dc civac, {}", in(reg) addr) }
        barrier::dsb_sy();
    }

    fn table_or_create(&mut self, table: u64, idx: usize) -> u64 {
        let entry = self.read(table, idx);
        if entry & DESC_VALID != 0 {
            if entry & DESC_TABLE == 0 {
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

        // 1. Set memory attributes
        asm!("msr mair_el1, {}", in(reg) 0x44_FF_u64);

        // 2. Set translation control
        let tcr: u64 = (25)
            | (0b01 << 8)
            | (0b01 << 10)
            | (0b11 << 12)
            | (0b00 << 14)
            | (3u64 << 32);
        asm!("msr tcr_el1, {}", in(reg) tcr);

        // 3. Flush TLB (from early identity map)
        asm!("tlbi vmalle1is");
        asm!("dsb sy");
        asm!("isb");

        // 4. Set new page table
        asm!("msr ttbr0_el1, {}", in(reg) self.root);
        asm!("dsb sy");
        asm!("isb");

        // 5. Enable MMU + caches
        let mut sctlr: u64;
        asm!("mrs {}, sctlr_el1", out(reg) sctlr);
        sctlr |= (1 << 0) | (1 << 2) | (1 << 12);
        asm!("msr sctlr_el1, {}", in(reg) sctlr);

        // 6. Sync
        asm!("dsb sy");
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
    // UART poke to verify we're switching address space
    core::ptr::write_volatile(0x0900_0000 as *mut u32, b'S' as u32);
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
    let mut d = DESC_VALID | DESC_TABLE | DESC_AF | DESC_SH_INNER | DESC_ATTRIDX_NORMAL;

    if flags.is_user() {
        d |= 1 << 6;
    } else if !flags.is_read_write() {
        d |= 2 << 6;
    }

    if !flags.is_executable() {
        d |= DESC_PXN | DESC_XN;
    }

    d | (phys & ADDR_MASK)
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

// ---------------------------------------------------------------------------
// Debug: dump page table entries for critical addresses
// ---------------------------------------------------------------------------

/// Walk the page table at `root` for address `va` and print descriptors.
/// Must be called BEFORE mmu_activate (uses early identity map for reads).
pub unsafe fn dump_walk(root: u64, va: u64, label: &str) {
    let l1_idx = ((va >> 30) & 0x1FF) as usize;
    let l2_idx = ((va >> 21) & 0x1FF) as usize;
    let l3_idx = ((va >> 12) & 0x1FF) as usize;
    let page_offset = va & 0xFFF;

    let l1_entry = core::ptr::read_volatile((root + (l1_idx as u64) * 8) as *const u64);
    let l1_valid = l1_entry & DESC_VALID != 0;
    let l1_table = l1_entry & DESC_VALID | DESC_TABLE != 0;
    vivanta_boot_common::println!("  {} VA={:#x}: L1[{}]={:#x} valid={} table={}",
        label, va, l1_idx, l1_entry, l1_valid, l1_entry & DESC_TABLE != 0);

    if !l1_valid || l1_entry & DESC_TABLE == 0 {
        return;
    }
    let l2_table = l1_entry & ADDR_MASK;

    let l2_entry = core::ptr::read_volatile((l2_table + (l2_idx as u64) * 8) as *const u64);
    let l2_valid = l2_entry & DESC_VALID != 0;
    let l2_is_table = l2_entry & DESC_TABLE != 0;
    vivanta_boot_common::println!("    L2[{}]={:#x} valid={} table={}", l2_idx, l2_entry, l2_valid, l2_is_table);

    if !l2_valid { return; }

    if !l2_is_table {
        // L2 block — maps 2 MiB
        let block_pa = l2_entry & ADDR_MASK_BLOCK;
        vivanta_boot_common::println!("    -> BLOCK PA={:#x} (offset={:#x})", block_pa | page_offset, page_offset);
        return;
    }

    let l3_table = l2_entry & ADDR_MASK;
    let l3_entry = core::ptr::read_volatile((l3_table + (l3_idx as u64) * 8) as *const u64);
    let l3_valid = l3_entry & DESC_VALID != 0;
    vivanta_boot_common::println!("    L3[{}]={:#x} valid={}", l3_idx, l3_entry, l3_valid);

    if l3_valid {
        let page_pa = l3_entry & ADDR_MASK;
        vivanta_boot_common::println!("    -> PAGE PA={:#x}", page_pa | page_offset);
    }
}

/// Dump page table for several critical addresses.
#[no_mangle]
pub unsafe extern "Rust" fn dump_critical_tables(root: u64) {
    vivanta_boot_common::println!("=== Page Table Dump (root={:#x}) ===", root);

    // Current PC
    let pc: u64;
    core::arch::asm!("adr {}, .", out(reg) pc);
    dump_walk(root, pc, "PC");

    // Kernel start / end
    extern "C" { static __kernel_start: u8; static __stack_top: u8; }
    let ks = &__kernel_start as *const u8 as u64;
    let ke = &__stack_top as *const u8 as u64;
    dump_walk(root, ks, "kernel_start");
    dump_walk(root, ke, "stack_top");

    // VBAR_EL1
    let vbar: u64;
    core::arch::asm!("mrs {}, vbar_el1", out(reg) vbar);
    dump_walk(root, vbar, "VBAR_EL1");

    // RAM start
    dump_walk(root, 0x4000_0000, "RAM_START");

    // User code VA
    dump_walk(root, 0x5E00_0000, "USER_CODE");

    // UART
    dump_walk(root, 0x0900_0000, "UART");

    // Dump raw L2 block descriptor for kernel text (0x40200000-0x403FFFFF)
    vivanta_boot_common::println!("--- Raw descriptor decode ---");
    let l1_idx = ((0x4020_0000u64 >> 30) & 0x1FF) as usize;
    let l2_idx = ((0x4020_0000u64 >> 21) & 0x1FF) as usize;
    let l1_entry = core::ptr::read_volatile((root + (l1_idx as u64) * 8) as *const u64);
    if l1_entry & DESC_TABLE != 0 {
        let l2_table = l1_entry & ADDR_MASK;
        let l2_entry = core::ptr::read_volatile((l2_table + (l2_idx as u64) * 8) as *const u64);
        vivanta_boot_common::println!("  L2[{}] raw = {:#018x}", l2_idx, l2_entry);
        vivanta_boot_common::println!("    Valid={}", (l2_entry & DESC_VALID) != 0);
        vivanta_boot_common::println!("    Table={}", (l2_entry & DESC_TABLE) != 0);
        vivanta_boot_common::println!("    AF={}", (l2_entry & DESC_AF) != 0);
        vivanta_boot_common::println!("    SH={:#x}", (l2_entry >> 8) & 3);
        vivanta_boot_common::println!("    AP={:#x}", (l2_entry >> 6) & 3);
        vivanta_boot_common::println!("    AttrIdx={:#x}", (l2_entry >> 2) & 7);
        vivanta_boot_common::println!("    PXN={}", (l2_entry >> 53) & 1);
        vivanta_boot_common::println!("    UXN={}", (l2_entry >> 54) & 1);
        vivanta_boot_common::println!("    OutputAddr={:#x}", l2_entry & ADDR_MASK_BLOCK);
    }
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
