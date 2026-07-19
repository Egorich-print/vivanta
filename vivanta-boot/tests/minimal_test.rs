// This is not a real test file. It's a minimal vivanta_kernel for QEMU testing.
// Run with: `cargo run --example minimal_test` (not supported yet).
#![no_std]
#![no_main]

use core::panic::PanicInfo;

core::arch::global_asm!(
    ".section .text._start",
    ".global _start",
    "_start:",
    "ldr x1, =__stack_top",
    "mov sp, x1",
    // Blink the UART
    "ldr x0, =0x09000000",
    "mov w1, #'X'",
    "str w1, [x0]",
    "1:",
    "wfi",
    "b 1b",
);

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
