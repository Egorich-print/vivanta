#![no_std]
#![no_main]

use core::panic::PanicInfo;

use vivanta_boot_common::{println, set_console, with_console, Console};

const UART_BASE: *mut u8 = 0x0C17_0000 as *mut u8;

struct Msmuart {
    base: *mut u8,
}

unsafe impl Send for Msmuart {}
unsafe impl Sync for Msmuart {}

static UART: Msmuart = Msmuart::new(UART_BASE);

impl Msmuart {
    const fn new(base: *mut u8) -> Self {
        Msmuart { base }
    }

    fn tx_ready(&self) -> bool {
        unsafe { (self.base.add(0x08) as *const u32).read_volatile() & (1 << 2) != 0 }
    }

    fn putchar(&self, c: u8) {
        while !self.tx_ready() {
            core::hint::spin_loop();
        }
        unsafe {
            (self.base as *mut u32).write_volatile(c as u32);
        }
    }
}

impl Console for Msmuart {
    fn write_str(&self, s: &str) {
        for &b in s.as_bytes() {
            match b {
                b'\n' => {
                    self.putchar(b'\r');
                    self.putchar(b'\n');
                }
                _ => self.putchar(b),
            }
        }
    }
}

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

#[no_mangle]
pub unsafe extern "C" fn adapter_main() -> ! {
    set_console(&UART);

    println!();
    println!("\u{2500}\u{2500}\u{2500}\u{2500} Theseus Boot v0.1 \u{2500}\u{2500}\u{2500}\u{2500}");
    println!("  Arch: ARM64");
    println!("  SoC:  SDM660");
    println!("  UART: 0x{:08x}", UART_BASE as usize);
    println!();

    loop {
        core::hint::spin_loop();
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    with_console(|c| c.write_str("\n!!! FIRST LIGHT PANIC\n"));
    loop {
        core::hint::spin_loop();
    }
}
