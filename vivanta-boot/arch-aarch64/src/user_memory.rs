// ---------------------------------------------------------------------------
// user_memory.rs — AArch64 user memory validation implementation
//
// access_ok validates a user-supplied range against the ACTIVE page table
// (TTBR0_EL1). The scheduler activates a thread's address space before the
// thread runs, so the active root always corresponds to the current thread.
// ---------------------------------------------------------------------------

use crate::paging::descriptor::*;
use crate::paging::mapper::PageTable;
use vivanta_arch_api::user_memory::AccessType;

/// AP[2:1] encodings (bits 7:6) used by this kernel's mapping code:
///   00 = EL1 RW, 01 = EL0 RW, 10 = EL1 RO, 11 = EL0 RO.
/// Any EL0-accessible page has bit 6 set.
const AP_EL0_ACCESS: u64 = 1 << 6;
const AP_EL0_RO: u64 = 1 << 7;

/// Check whether a leaf descriptor grants `access` from EL0.
fn descriptor_allows(desc: u64, access: AccessType) -> bool {
    if !desc_is_valid(desc) {
        return false;
    }
    // Kernel-only pages (AP bit 6 clear) are never user-accessible.
    if desc & AP_EL0_ACCESS == 0 {
        return false;
    }
    match access {
        AccessType::Read => true,
        AccessType::Write => desc & AP_EL0_RO == 0,
        AccessType::Execute => desc & DESC_XN == 0,
    }
}

fn read_ttbr0() -> u64 {
    let root: u64;
    unsafe {
        core::arch::asm!("mrs {}, ttbr0_el1", out(reg) root, options(nostack));
    }
    root
}

/// Validate that `[vaddr, vaddr + size)` is mapped in the active address
/// space and accessible from EL0 with `access`.
///
/// `aspace` is accepted for API compatibility with the kernel's
/// AddressSpaceId, but the check is performed against the live TTBR0 root,
/// which is always the current thread's address space.
#[unsafe(no_mangle)]
pub extern "Rust" fn access_ok(_aspace: usize, vaddr: u64, size: u64, access: AccessType) -> bool {
    if size == 0 {
        return true;
    }
    let end = match vaddr.checked_add(size) {
        Some(e) => e,
        None => return false,
    };

    let root = read_ttbr0() & ADDR_MASK;
    let pt = PageTable::new(root);

    let mut addr = vaddr;
    while addr < end {
        let page = addr & !0xFFF;
        match pt.leaf_descriptor(page) {
            Some(desc) if descriptor_allows(desc, access) => {}
            _ => return false,
        }
        match page.checked_add(0x1000) {
            Some(next) => addr = next,
            None => return true, // last page fully covered
        }
    }
    true
}

/// Copy `len` bytes from user space into `dst` after validating the whole
/// range. Interrupts are disabled for the duration so a timer IRQ cannot
/// switch address spaces mid-copy (single-core TOCTOU prevention).
#[unsafe(no_mangle)]
pub unsafe extern "Rust" fn copy_from_user(dst: *mut u8, src: u64, len: usize) -> Result<(), ()> {
    if len == 0 {
        return Ok(());
    }
    if dst.is_null() {
        return Err(());
    }
    let _guard = unsafe { vivanta_arch_api::interrupts::disable_interrupts() };
    let root = read_ttbr0() & ADDR_MASK;
    let pt = PageTable::new(root);

    // Validate the full source range page-by-page before copying.
    let end = src.checked_add(len as u64).ok_or(())?;
    let mut addr = src;
    while addr < end {
        let page = addr & !0xFFF;
        if !pt
            .leaf_descriptor(page)
            .map_or(false, |d| descriptor_allows(d, AccessType::Read))
        {
            return Err(());
        }
        match page.checked_add(0x1000) {
            Some(next) => addr = next,
            None => break,
        }
    }

    unsafe {
        core::ptr::copy_nonoverlapping(src as *const u8, dst, len);
    }
    Ok(())
}

/// Copy `len` bytes from `src` into user space at `dst` after validating the
/// whole range. Interrupts disabled for the duration (single-core TOCTOU
/// prevention).
#[unsafe(no_mangle)]
pub unsafe extern "Rust" fn copy_to_user(dst: u64, src: *const u8, len: usize) -> Result<(), ()> {
    if len == 0 {
        return Ok(());
    }
    if src.is_null() {
        return Err(());
    }
    let _guard = unsafe { vivanta_arch_api::interrupts::disable_interrupts() };
    let root = read_ttbr0() & ADDR_MASK;
    let pt = PageTable::new(root);

    let end = dst.checked_add(len as u64).ok_or(())?;
    let mut addr = dst;
    while addr < end {
        let page = addr & !0xFFF;
        if !pt
            .leaf_descriptor(page)
            .map_or(false, |d| descriptor_allows(d, AccessType::Write))
        {
            return Err(());
        }
        match page.checked_add(0x1000) {
            Some(next) => addr = next,
            None => break,
        }
    }

    unsafe {
        core::ptr::copy_nonoverlapping(src, dst as *mut u8, len);
    }
    Ok(())
}
