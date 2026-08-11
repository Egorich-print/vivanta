// ---------------------------------------------------------------------------
// target-test — build-time proof: vivanta_kernel links without real arch crate
// ---------------------------------------------------------------------------

#![no_std]
#![no_main]

use core::panic::PanicInfo;

// Force link arch-test-stub for extern "Rust" symbol resolution
extern crate vivanta_arch_test_stub;

// Minimal entry point — just links everything
core::arch::global_asm!(
    ".section .text._start",
    ".global _start",
    "_start:",
    "b 1f",
    "1:",
    "wfi",
    "b 1b",
);

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        core::hint::spin_loop()
    }
}
