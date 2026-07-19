// ---------------------------------------------------------------------------
// platform-sdm660 — Qualcomm SDM660 platform (MSM UART)
// ---------------------------------------------------------------------------

#![no_std]

use vivanta_boot_common::{Console, set_console, println};

const UART_BASE: *mut u8 = 0x0C17_0000 as *mut u8;

struct Msmuart {
    base: *mut u8,
}

unsafe impl Send for Msmuart {}
unsafe impl Sync for Msmuart {}

static UART: Msmuart = Msmuart::new(UART_BASE);

impl Msmuart {
    const fn new(base: *mut u8) -> Self { Msmuart { base } }

    fn tx_ready(&self) -> bool {
        unsafe { (self.base.add(0x08) as *const u32).read_volatile() & (1 << 2) != 0 }
    }

    fn putchar(&self, c: u8) {
        while !self.tx_ready() { core::hint::spin_loop(); }
        unsafe { (self.base as *mut u32).write_volatile(c as u32); }
    }
}

impl Console for Msmuart {
    fn write_str(&self, s: &str) {
        for &b in s.as_bytes() {
            match b {
                b'\n' => { self.putchar(b'\r'); self.putchar(b'\n'); }
                _ => self.putchar(b),
            }
        }
    }
}

/// Initialise the MSM UART console.
pub fn init_console() {
    set_console(&UART);
}
