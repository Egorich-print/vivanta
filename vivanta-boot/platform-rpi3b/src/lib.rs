#![no_std]

use core::ptr::{read_volatile, write_volatile};

const GPIO_BASE: usize = 0x3F20_0000;
const GPFSEL1: usize = 0x04;
const GPPUD: usize = 0x94;
const GPPUDCLK0: usize = 0x98;

pub fn init_uart_gpio() {
    unsafe {
        let gpfsel1 = (GPIO_BASE + GPFSEL1) as *mut u32;
        let mut fsel1 = read_volatile(gpfsel1);
        fsel1 &= !((0b111 << 12) | (0b111 << 15));
        fsel1 |= (4 << 12) | (4 << 15);
        write_volatile(gpfsel1, fsel1);

        write_volatile((GPIO_BASE + GPPUD) as *mut u32, 0);
        spin(150);
        write_volatile((GPIO_BASE + GPPUDCLK0) as *mut u32, (1 << 14) | (1 << 15));
        spin(150);
        write_volatile((GPIO_BASE + GPPUDCLK0) as *mut u32, 0);
    }
}

fn spin(cycles: u32) {
    for _ in 0..cycles {
        unsafe {
            core::arch::asm!("nop");
        }
    }
}
