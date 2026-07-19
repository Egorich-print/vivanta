use vivanta_boot_common::Console;

const UARTDR: u32 = 0x000;
const UARTFR: u32 = 0x018;
const UARTIBRD: u32 = 0x024;
const UARTFBRD: u32 = 0x028;
const UARTLCR_H: u32 = 0x02C;
const UARTCR: u32 = 0x030;
const UARTIMSC: u32 = 0x038;
const UARTICR: u32 = 0x044;

const FR_TXFF: u32 = 0x020;

const LCR_H_FEN: u32 = 0x10;
const LCR_H_WLEN_8: u32 = 0x60;

const CR_UARTEN: u32 = 0x001;
const CR_TXE: u32 = 0x100;
const CR_RXE: u32 = 0x200;

fn reg(base: *mut u32, offset: u32) -> *mut u32 {
    unsafe { base.add(offset as usize / 4) }
}

unsafe fn write_reg(base: *mut u32, offset: u32, value: u32) {
    core::ptr::write_volatile(reg(base, offset), value);
}

unsafe fn read_reg(base: *mut u32, offset: u32) -> u32 {
    core::ptr::read_volatile(reg(base, offset))
}

pub struct Pl011Uart {
    base: *mut u32,
}

impl Pl011Uart {
    pub const fn new(base_addr: usize) -> Self {
        Pl011Uart {
            base: base_addr as *mut u32,
        }
    }

    pub unsafe fn init(&self) {
        write_reg(self.base, UARTCR, 0);
        write_reg(self.base, UARTIBRD, 13);
        write_reg(self.base, UARTFBRD, 1);
        write_reg(self.base, UARTLCR_H, LCR_H_FEN | LCR_H_WLEN_8);
        write_reg(self.base, UARTIMSC, 0);
        write_reg(self.base, UARTICR, 0x7FF);
        write_reg(self.base, UARTCR, CR_UARTEN | CR_TXE | CR_RXE);
    }

    fn tx_byte(&self, byte: u8) {
        unsafe {
            while read_reg(self.base, UARTFR) & FR_TXFF != 0 {}
            write_reg(self.base, UARTDR, byte as u32);
        }
    }

    fn tx_str(&self, s: &str) {
        for &b in s.as_bytes() {
            match b {
                b'\n' => {
                    self.tx_byte(b'\r');
                    self.tx_byte(b'\n');
                }
                c => self.tx_byte(c),
            }
        }
    }
}

unsafe impl Sync for Pl011Uart {}

impl Console for Pl011Uart {
    fn write_str(&self, s: &str) {
        self.tx_str(s);
    }
}


