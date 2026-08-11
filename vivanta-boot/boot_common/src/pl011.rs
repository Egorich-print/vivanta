use crate::Console;

pub struct Pl011 {
    base: *mut u32,
}

unsafe impl Send for Pl011 {}
unsafe impl Sync for Pl011 {}

impl Pl011 {
    pub const fn new(base: usize) -> Self {
        Pl011 {
            base: base as *mut u32,
        }
    }

    fn reg(&self, offset: u32) -> *mut u32 {
        unsafe { self.base.add(offset as usize / 4) }
    }

    pub fn init(&self, clock_hz: u32, baud: u32) {
        unsafe {
            self.reg(CR).write_volatile(0);

            let div = 16 * baud;
            let ibrd = clock_hz / div;
            let rem = clock_hz % div;
            let fbrd = (rem * 64 + div / 2) / div;
            self.reg(IBRD).write_volatile(ibrd);
            self.reg(FBRD).write_volatile(fbrd);

            self.reg(LCR_H).write_volatile(0x70);

            self.reg(CR).write_volatile(0x301);
        }
    }

    fn putchar(&self, c: u8) {
        unsafe {
            while (self.reg(FR).read_volatile() & (1 << 5)) != 0 {}
            self.reg(DR).write_volatile(c as u32);
        }
    }
}

impl Console for Pl011 {
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

const DR: u32 = 0x00;
const FR: u32 = 0x18;
const IBRD: u32 = 0x24;
const FBRD: u32 = 0x28;
const LCR_H: u32 = 0x2C;
const CR: u32 = 0x30;
