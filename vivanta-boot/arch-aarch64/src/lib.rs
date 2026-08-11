// ---------------------------------------------------------------------------
// arch-aarch64 — AArch64 architecture implementation
// ---------------------------------------------------------------------------

#![no_std]
#![allow(static_mut_refs)]

extern crate vivanta_boot_common;

pub mod barrier;
pub mod boot;
pub mod context;
pub mod early_mmu; // used by target-rpi3b-plus (standalone diagnostic, not kernel path)
pub mod exceptions;
pub mod interrupts;
pub mod mmio;
pub mod mmu;
pub mod paging;
pub mod thread;
pub mod timer;
pub mod vectors;
// pub mod sync;  // removed — unused IrqGuard, vivanta_kernel has its own in scheduler/mod.rs
pub mod user;
pub mod user_memory;

/// Initialize architecture: set exception vectors.
pub fn init() {
    exceptions::init();
}

/// Check if the MMU is enabled (reads SCTLR_EL1.M bit).
pub fn is_mmu_enabled() -> bool {
    let sctlr: u64;
    unsafe {
        core::arch::asm!("mrs {}, sctlr_el1", out(reg) sctlr, options(nostack));
    }
    (sctlr & 1) != 0
}
