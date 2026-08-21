// ---------------------------------------------------------------------------
// target-lavender — Lavender (SDM660 / Redmi Note 7) vivanta_kernel binary
// ---------------------------------------------------------------------------

#![no_std]
#![no_main]

use core::panic::PanicInfo;
use vivanta_boot_common::println;

// ARM64 entry with Image header
core::arch::global_asm!(
    ".section .text._start, \"ax\"",
    ".global _start",
    "_start:",
    "b _real_start",
    ".word 0",
    ".quad 0x80000",
    ".quad 0x400000",
    ".quad 0x0a",
    ".quad 0",
    ".quad 0",
    ".quad 0",
    ".byte 0x41, 0x52, 0x4d, 0x64",
    ".word 0",
    ".fill 0x80000 - (.-_start), 1, 0",
    ".global _real_start",
    "_real_start:",
    "msr daifset, #0xf",
    "mrs x5, CurrentEL",
    "and x5, x5, #0xC",
    "lsr x5, x5, #2",
    "cmp x5, #2",
    "b.eq 5f",
    "cmp x5, #1",
    "b.eq 6f",
    "b 7f",
    "5:",
    "mov x5, #(0b11 << 20)",
    "msr CPTR_EL2, x5",
    "b 7f",
    "6:",
    "mov x5, #(0b11 << 20)",
    "msr CPACR_EL1, x5",
    "7:",
    "adrp x1, __stack_top",
    "add x1, x1, :lo12:__stack_top",
    "mov sp, x1",
    "adrp x1, __bss_start",
    "add x1, x1, :lo12:__bss_start",
    "adrp x2, __bss_end",
    "add x2, x2, :lo12:__bss_end",
    "sub x3, x2, x1",
    "cbz x3, 2f",
    "mov x2, xzr",
    "1:",
    "str x2, [x1], #8",
    "subs x3, x3, #8",
    "b.gt 1b",
    "2:",
    "bl adapter_main",
    "3:",
    "wfi",
    "b 3b",
);

#[unsafe(no_mangle)]
pub unsafe extern "C" fn adapter_main() -> ! {
    vivanta_platform_sdm660::init_console();

    println!();
    println!("\u{2500}\u{2500}\u{2500}\u{2500} Vivanta Boot v0.1 \u{2500}\u{2500}\u{2500}\u{2500}");
    println!("  Arch: ARM64");
    println!("  SoC:  SDM660");
    println!("  UART: 0x{0:x}", 0x0C17_0000usize);
    println!();

    loop {
        core::hint::spin_loop();
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        core::hint::spin_loop();
    }
}
