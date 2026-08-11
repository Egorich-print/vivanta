use vivanta_boot_common::println;

use crate::paging::descriptor::*;
use crate::paging::mapper::PageTable;
use crate::paging::{MappingFlags, Permissions};

fn read_ttbr0() -> u64 {
    let root: u64;
    unsafe {
        core::arch::asm!("mrs {}, ttbr0_el1", out(reg) root, options(nostack));
    }
    root
}

pub unsafe fn run_smoke_test() {
    let root_pa = read_ttbr0() & ADDR_MASK;
    let pt = PageTable::new(root_pa);

    test_descriptor_constants();
    test_translate_known(&pt);
    test_readback(&pt);

    println!("  MMU smoke test passed");
}

unsafe fn test_translate_known(pt: &PageTable) {
    // Only test addresses that are actually mapped in the page table
    let known_addrs = [0x4000_0000u64, 0x4000_1000, 0x4020_0000, 0x4021_b000];
    for &va in &known_addrs {
        let pa = pt
            .translate(va)
            .unwrap_or_else(|| panic!("translate failed at VA {:#x}", va));
        assert_eq!(pa, va, "identity: VA {:#x} -> PA {:#x} mismatch", va, pa);
    }
}

unsafe fn test_readback(_pt: &PageTable) {
    let test_addr = 0x4000_0000u64;
    let val = core::ptr::read_volatile(test_addr as *const u64);
    core::ptr::write_volatile(test_addr as *mut u64, val);
}

fn test_descriptor_constants() {
    assert!(DESC_VALID == 1 << 0);
    assert!(DESC_TABLE == 1 << 1);
    assert!(DESC_AF == 1 << 10);
    assert!(DESC_SH_INNER == 3 << 8);
    assert!(DESC_PXN == 1 << 53);
    assert!(DESC_XN == 1 << 54);
    assert!(ADDR_MASK == 0x0000_FFFF_FFFF_F000);
    assert!(ADDR_MASK_BLOCK == 0x0000_FFFF_FFE0_0000);

    assert!(DESC_AP_RW_EL1 == 0 << 6);
    assert!(DESC_AP_RO_EL1 == 2 << 6);
    assert!(DESC_AP_RW_EL0 == 1 << 6);

    assert!(DESC_ATTRIDX_NORMAL == 0);
    assert!(DESC_ATTRIDX_DEVICE == 1 << 2);

    let flags = MappingFlags::normal(Permissions::kernel_rwx());
    let desc = flags.to_descriptor_bits(0x4000_0000, true);
    assert!(desc & DESC_VALID != 0, "descriptor: VALID bit missing");
    assert!(desc & DESC_AF != 0, "descriptor: AF bit missing");
    assert!(desc & DESC_PXN == 0, "descriptor: PXN set but executable");
    assert!(desc & DESC_XN == 0, "descriptor: XN set but executable");

    let ro_flags = MappingFlags::normal(Permissions::kernel_rx());
    let ro_desc = ro_flags.to_descriptor_bits(0x4000_0000, true);
    assert!(
        ro_desc & DESC_AP_RO_EL1 == DESC_AP_RO_EL1,
        "descriptor: RO AP bits wrong"
    );
}
