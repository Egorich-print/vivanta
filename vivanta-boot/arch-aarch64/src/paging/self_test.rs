use vivanta_boot_common::println;

use crate::paging::descriptor::*;
use crate::paging::mapper::PageTable;
use crate::paging::walker::leaf_with_permissions;
use crate::paging::{MappingFlags, Permissions};

fn read_ttbr0() -> u64 {
    let root: u64;
    unsafe {
        core::arch::asm!("mrs {}, ttbr0_el1", out(reg) root, options(nostack));
    }
    root
}

pub unsafe fn run_smoke_test() {
    unsafe {
        let root_pa = read_ttbr0() & ADDR_MASK;
        let pt = PageTable::new(root_pa);

        test_descriptor_constants();
        test_wx_encoding();
        test_translate_known(&pt);
        test_readback(&pt);

        println!("  MMU smoke test passed");
    }
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
    unsafe {
        let test_addr = 0x4000_0000u64;
        let val = core::ptr::read_volatile(test_addr as *const u64);
        core::ptr::write_volatile(test_addr as *mut u64, val);
    }
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

/// W^X encoding regression matrix (G3).
///
/// Every descriptor builder in the crate must route AP bits through
/// `ap_bits()`. A user read-only page (code) encodes AP=11 — EL0 *read-only*
/// — never AP=01 (EL0 read-write). Regression guard for
/// docs/investigations/WX-user-code-ap-encoding.md.
pub(crate) fn test_wx_encoding() {
    use crate::mmu::{PageFlags, block_or_page_desc, flags_to_desc_bits};
    use vivanta_arch_api::mmu::MappingFlags as ApiFlags;

    // ap_bits truth table.
    assert_eq!(ap_bits(false, false), DESC_AP_RO_EL1); // kernel RO → 0b10
    assert_eq!(ap_bits(false, true), DESC_AP_RW_EL1); // kernel RW → 0b00
    assert_eq!(ap_bits(true, false), DESC_AP_RO_EL0); // user   RO → 0b11
    assert_eq!(ap_bits(true, true), DESC_AP_RW_EL0); // user   RW → 0b01

    // PageFlags path (boot builder: boot user image).
    let code = block_or_page_desc(0x5000_0000, PageFlags::USER_READ_EXEC, true);
    assert_eq!(
        code & DESC_AP_MASK,
        DESC_AP_RO_EL0,
        "W^X FAIL: user code page is not EL0 read-only"
    );
    assert_eq!(code & DESC_XN, 0, "W^X FAIL: user code page is XN");
    assert_ne!(code & DESC_PXN, 0, "W^X FAIL: user code EL1-executable");

    let stack = block_or_page_desc(0x5000_1000, PageFlags::USER_READ_WRITE, true);
    assert_eq!(stack & DESC_AP_MASK, DESC_AP_RW_EL0);
    assert_ne!(stack & DESC_XN, 0, "W^X FAIL: user stack executable");

    let ktext = block_or_page_desc(0x4000_0000, PageFlags::READ_ONLY, true);
    assert_eq!(ktext & DESC_AP_MASK, DESC_AP_RO_EL1);

    // arch-api MappingFlags path (runtime mmu_map_object / mmu_protect).
    let uro = flags_to_desc_bits(ApiFlags::executable() | ApiFlags::user(), 0x5000_0000);
    assert_eq!(
        uro & DESC_AP_MASK,
        DESC_AP_RO_EL0,
        "W^X FAIL: api user+RO+X not EL0 read-only"
    );

    // paging Permissions path (early identity map / PageTable handle).
    let pro = MappingFlags::normal(Permissions {
        readable: true,
        writable: false,
        executable: true,
        user: true,
    })
    .to_descriptor_bits(0x5000_0000, true);
    assert_eq!(
        pro & DESC_AP_MASK,
        DESC_AP_RO_EL0,
        "W^X FAIL: paging user RX not EL0 read-only"
    );

    // leaf_with_permissions: pure rewrite preserves address/type/AF/SH/ATTR.
    let base = DESC_VALID
        | DESC_AF
        | DESC_SH_INNER
        | DESC_ATTRIDX_NORMAL
        | (0x5000_0000 & ADDR_MASK)
        | DESC_AP_RW_EL0
        | DESC_XN;
    let flipped = leaf_with_permissions(base, true, false, true);
    assert_eq!(flipped & DESC_AP_MASK, DESC_AP_RO_EL0);
    assert_eq!(flipped & DESC_XN, 0);
    assert_ne!(flipped & DESC_PXN, 0);
    assert_eq!(flipped & ADDR_MASK, base & ADDR_MASK, "rewrite moved PA");
    assert_eq!(
        flipped & (DESC_VALID | DESC_AF | DESC_SH_INNER | DESC_ATTRIDX_NORMAL),
        base & (DESC_VALID | DESC_AF | DESC_SH_INNER | DESC_ATTRIDX_NORMAL)
    );
}

/// Boot-time W^X verification of a user address space (G3 evidence).
///
/// Reads back the live leaf descriptors for `code_va` and `stack_va` from
/// the table hierarchy at `root_pa` and asserts:
///   code  → valid, AP=11 (EL0 RO), XN=0, PXN=1   (RX from EL0)
///   stack → valid, AP=01 (EL0 RW), XN=1          (RW, non-exec)
/// Prints machine-checkable `[WX]` lines; panics on any violation.
pub fn wx_verify_user_as(root_pa: u64, code_va: u64, stack_va: u64) {
    let pt = PageTable::new(root_pa & ADDR_MASK);

    let code_desc = pt
        .leaf_descriptor(code_va)
        .unwrap_or_else(|| panic!("[WX] FAIL: no leaf descriptor for code VA {:#x}", code_va));
    let stack_desc = pt
        .leaf_descriptor(stack_va)
        .unwrap_or_else(|| panic!("[WX] FAIL: no leaf descriptor for stack VA {:#x}", stack_va));

    let code_ap = code_desc & DESC_AP_MASK;
    let code_ro = code_ap == DESC_AP_RO_EL0;
    let code_x = code_desc & DESC_XN == 0;
    let code_pxn = code_desc & DESC_PXN != 0;
    println!(
        "  [WX] root={:#x} code_va={:#x} desc={:#x} AP={:#b} XN={} PXN={}",
        root_pa,
        code_va,
        code_desc,
        code_ap >> 6,
        !code_x as u8,
        code_pxn as u8
    );
    assert!(
        code_ro,
        "[WX] FAIL: user code is NOT EL0 read-only (AP={:#b})",
        code_ap >> 6
    );
    assert!(code_x, "[WX] FAIL: user code is execute-never");
    assert!(
        code_pxn,
        "[WX] FAIL: user code lacks PXN (EL1 can fetch it)"
    );

    let stack_ap = stack_desc & DESC_AP_MASK;
    let stack_rw = stack_ap == DESC_AP_RW_EL0;
    let stack_xn = stack_desc & DESC_XN != 0;
    println!(
        "  [WX] root={:#x} stack_va={:#x} desc={:#x} AP={:#b} XN={}",
        root_pa,
        stack_va,
        stack_desc,
        stack_ap >> 6,
        stack_xn as u8
    );
    assert!(stack_rw, "[WX] FAIL: user stack is not EL0 writable");
    assert!(
        stack_xn,
        "[WX] FAIL: user stack is executable (W^X violation)"
    );

    println!("  [WX] user AS W^X verification PASS");
}
