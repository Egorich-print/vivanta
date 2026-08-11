#![no_std]
#![no_main]

use core::panic::PanicInfo;
use vivanta_arch_aarch64::early_mmu;
use vivanta_boot_common::pl011::Pl011;
use vivanta_boot_common::{println, set_console};
use vivanta_platform_rpi3b::init_uart_gpio;

const PL011_BASE: usize = 0x3F20_1000;

core::arch::global_asm!(
    ".section .text._start",
    ".global _start",
    "_start:",
    "b _real_start",
    ".word 0",
    ".quad 0x00000000",
    ".quad 0",
    ".quad 0x00080000",
    ".quad 0",
    ".quad 0",
    ".quad 0",
    ".word 0x644d5241",
    ".word 0",
    "_real_start:",
    "msr daifset, #0xf",
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
    "1: str x2, [x1], #8",
    "subs x3, x3, #8",
    "b.gt 1b",
    "2:",
    "adrp x1, BOOT_CONTEXT",
    "add x1, x1, :lo12:BOOT_CONTEXT",
    "str x0, [x1]",
    "bl adapter_main",
    "3: wfi",
    "b 3b",
);

static UART: Pl011 = Pl011::new(PL011_BASE);

#[no_mangle]
pub extern "C" fn adapter_main() -> ! {
    init_uart_gpio();
    UART.init(250_000_000, 115_200);
    set_console(&UART);

    println!("=== Vivanta RPi3B+ ===");
    println!("Before MMU");

    early_mmu::enable_identity(4 * 1024 * 1024 * 1024); // 4 GB

    println!("After MMU — identity map active");
    loop {}
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("PANIC: {}", info);
    loop {}
}
