// ---------------------------------------------------------------------------
// target-rpi3b-plus — Raspberry Pi 3 B+ (BCM2837B0) kernel binary
// ---------------------------------------------------------------------------

#![no_std]
#![no_main]

use core::panic::PanicInfo;

// ARM64 entry point for GPU firmware (kernel8.img at 0x80000)
core::arch::global_asm!(
    ".section .text._start",
    ".global _start",
    "_start:",
    // ARM64 Image header (64 bytes) — required by Pi GPU firmware
    "b _real_start",
    ".word 0",
    ".quad 0x00000000",          // text_offset = 0 (kernel at load addr)
    ".quad 0",                   // image_size (unknown)
    ".quad 0x0a",               // flags: LE, PIE
    ".quad 0",
    ".quad 0",
    ".quad 0",
    ".word 0x644d5241",          // magic "ARM\64"
    ".word 0",

    "_real_start:",
    "msr daifset, #0xf",
    // Disable MMU (if left enabled by firmware)
    "dsb sy",
    "isb",
    "mrs x5, CurrentEL",
    "and x5, x5, #0xC",
    "cmp x5, #(2 << 2)",
    "b.eq 10f",
    "cmp x5, #(1 << 2)",
    "b.eq 11f",
    "b 7f",
    "10:",
    // EL2: disable MMU at both EL2 and EL1
    "mrs x6, sctlr_el2",
    "bic x6, x6, #1",
    "msr sctlr_el2, x6",
    "dsb sy",
    "isb",
    "tlbi alle2",
    "dsb sy",
    "isb",
    "mrs x6, sctlr_el1",
    "bic x6, x6, #1",
    "msr sctlr_el1, x6",
    "dsb sy",
    "isb",
    "tlbi vmalle1",
    "dsb sy",
    "isb",
    "mov x5, #(0b11 << 20)",
    "msr CPACR_EL1, x5",        // FPEN=3: enable FP at EL1 and EL0
    "msr CPTR_EL2, xzr",        // clear TFP — no FP/SIMD traps to EL2
    // Configure HCR_EL2 for EL1 compatibility, then drop to EL1
    "mov x5, #(1 << 31)",       // RW bit: AArch64 for EL1
    "msr hcr_el2, x5",
    "mov x5, #0x3c5",           // SPSR_EL2: EL1h, DAIF masked
    "msr spsr_el2, x5",
    "adr x5, 12f",              // return address in EL1
    "msr elr_el2, x5",
    "eret",                      // → EL1 at 12f
    "11:",
    // EL1: disable MMU
    "mrs x6, sctlr_el1",
    "bic x6, x6, #1",
    "msr sctlr_el1, x6",
    "dsb sy",
    "isb",
    "tlbi vmalle1",
    "dsb sy",
    "isb",
    "mov x5, #(0b11 << 20)",
    "msr CPACR_EL1, x5",
    "12:",
    // Set up safe exception vector table (all entries: branch to self)
    "adr x6, 8f",
    "msr vbar_el1, x6",
    "isb",
    "b 7f",
    ".balign 2048",
    "8:",
    ".rept 16",
    "b 8b",
    ".endr",
    "7:",
    // Set stack pointer
    "adrp x1, __stack_top",
    "add x1, x1, :lo12:__stack_top",
    "mov sp, x1",
    // Zero BSS
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
    // Save DTB pointer (x0) into BOOT_CONTEXT
    "adrp x1, BOOT_CONTEXT",
    "add x1, x1, :lo12:BOOT_CONTEXT",
    "str x0, [x1]",
    // Call adapter_main (never returns)
    "bl adapter_main",
    "3:",
    "wfi",
    "b 3b",
);

#[no_mangle]
pub unsafe extern "C" fn adapter_main() -> ! {
    // Configure early platform info
    vivanta_boot_common::set_early_platform(
        vivanta_boot_common::EarlyPlatformInfo { uart_base: 0x3F201000 },
    );

    // Initialize PL011 UART at 0x3F201000, 115200 8N1
    let uart = 0x3F201000 as *mut u32;

    // Disable UART, set baud, line control, clear interrupts, re-enable
    uart.add(0x030 / 4).write_volatile(0);                    // UARTCR = 0
    uart.add(0x024 / 4).write_volatile(26);                   // IBRD = 26
    uart.add(0x028 / 4).write_volatile(3);                    // FBRD = 3
    uart.add(0x02C / 4).write_volatile(0x70);                 // LCR_H = 8N1 + FIFO
    uart.add(0x044 / 4).write_volatile(0x7FF);                // ICR = clear all
    uart.add(0x038 / 4).write_volatile(0);                    // IMSC = 0
    uart.add(0x030 / 4).write_volatile(0x301);                // UARTCR = enable TX/RX

    // Wait for TX FIFO ready, then write '.'
    while uart.add(0x018 / 4).read_volatile() & 0x20 != 0 {}  // FR_TXFF
    uart.write_volatile(b'.' as u32);                          // UARTDR = '.'

    // Loop forever
    loop {
        core::hint::spin_loop();
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop { core::hint::spin_loop(); }
}
