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
    core::arch::asm!("dsb ish");
    core::arch::asm!("tlbi vmalle1is");
    core::arch::asm!("dsb ish");
    core::arch::asm!("isb");
}
