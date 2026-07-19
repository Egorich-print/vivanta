use crate::Console;

pub struct Ns16550 {
    base: *mut u8,
    reg_shift: u8,
}

unsafe impl Send for Ns16550 {}
unsafe impl Sync for Ns16550 {}

impl Ns16550 {
    pub const fn new(base: *mut u8, reg_shift: u8) -> Self {
        Ns16550 { base, reg_shift }
    }

    fn reg32(&self, reg: u8) -> *mut u32 {
        unsafe { (self.base.add((reg as usize) << self.reg_shift as usize)) as *mut u32 }
    }

    fn thr(&self) -> *mut u32 {
        self.reg32(0)
    }

    fn lsr(&self) -> *mut u32 {
        self.reg32(5)
    }

    fn tx_ready(&self) -> bool {
        unsafe { self.lsr().read_volatile() & (1 << 5) != 0 }
    }

    fn putchar(&self, c: u8) {
        while !self.tx_ready() {
            core::hint::spin_loop();
        }
        unsafe {
            self.thr().write_volatile(c as u32);
        }
    }
}

impl Console for Ns16550 {
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
