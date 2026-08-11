use crate::barrier;
use crate::paging::descriptor::*;
use crate::paging::{MappingFlags, PageTable, Permissions};

const L1_ENTRIES: usize = 512;
const L2_TABLE_COUNT: usize = 4;

#[repr(align(4096))]
struct L1Table([u64; L1_ENTRIES]);

#[repr(align(4096))]
struct L2Table([u64; L2_ENTRIES]);
const L2_ENTRIES: usize = 512;

static mut L1: L1Table = L1Table([0; L1_ENTRIES]);
static mut L2: [L2Table; L2_TABLE_COUNT] = [
    L2Table([0; L2_ENTRIES]),
    L2Table([0; L2_ENTRIES]),
    L2Table([0; L2_ENTRIES]),
    L2Table([0; L2_ENTRIES]),
];

fn phys_addr<T>(ptr: *const T) -> u64 {
    ptr as u64
}

fn l1_table_desc(l2_phys: u64) -> u64 {
    DESC_TABLE | (l2_phys & ADDR_MASK)
}

pub fn build_identity(size: u64) -> u64 {
    assert!(
        size <= 4 * 1024 * 1024 * 1024,
        "build_identity: size > 4 GB"
    );
    assert!(
        size % 0x20_0000 == 0,
        "build_identity: size must be 2 MB-aligned"
    );

    let l1_phys = unsafe { phys_addr(&L1) };

    for l1_idx in 0..L2_TABLE_COUNT {
        let base = (l1_idx as u64) * 0x4000_0000;
        if base >= size {
            break;
        }
        let l2 = unsafe { &L2[l1_idx] };
        let l2_phys = phys_addr(l2);
        unsafe {
            L1.0[l1_idx] = l1_table_desc(l2_phys);
        }
    }

    barrier::dsb_ish();

    let pt = PageTable::new(l1_phys);
    pt.map_region(0, 0, size, MappingFlags::normal(Permissions::kernel_rwx()));

    l1_phys
}

const MAIR_ATTR0_NORMAL: u64 = 0xFF;
const MAIR_ATTR1_DEVICE: u64 = 0x00;

pub fn configure_mair() {
    let mair = (MAIR_ATTR1_DEVICE << 8) | MAIR_ATTR0_NORMAL;
    unsafe {
        core::arch::asm!("msr mair_el1, {0:x}", in(reg) mair, options(nostack));
    }
}

pub fn configure_tcr() {
    const T0SZ: u64 = 32;
    const TG0_4KB: u64 = 0;
    const SH0_INNER: u64 = 3;
    const ORGN_WBWA: u64 = 1;
    const IRGN_WBWA: u64 = 1;

    let tcr: u64 =
        T0SZ | (TG0_4KB << 14) | (SH0_INNER << 12) | (ORGN_WBWA << 10) | (IRGN_WBWA << 8);
    unsafe {
        core::arch::asm!("msr tcr_el1, {}", in(reg) tcr, options(nostack));
    }
}

pub fn enable_mmu() {
    let mut sctlr: u64;
    unsafe {
        core::arch::asm!("mrs {}, sctlr_el1", out(reg) sctlr, options(nostack));
        sctlr |= 1;
        core::arch::asm!("msr sctlr_el1, {}", in(reg) sctlr, options(nostack));
        core::arch::asm!("isb", options(nostack));
    }
}

pub fn enable_identity(size: u64) {
    configure_mair();

    let ttbr0 = build_identity(size);

    configure_tcr();

    unsafe {
        core::arch::asm!("msr ttbr0_el1, {}", in(reg) ttbr0, options(nostack));
    }

    barrier::dsb_ish();
    barrier::isb();

    enable_mmu();

    unsafe {
        core::arch::asm!("tlbi vmalle1is", options(nostack));
        core::arch::asm!("dsb ish", options(nostack));
        core::arch::asm!("isb", options(nostack));
    }
}
