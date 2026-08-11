// MMIO access helpers — AArch64
use core::ptr::{read_volatile, write_volatile};

pub unsafe fn mmio_read8(addr: *const u8) -> u8 {
    read_volatile(addr)
}
pub unsafe fn mmio_read16(addr: *const u16) -> u16 {
    read_volatile(addr)
}
pub unsafe fn mmio_read32(addr: *const u32) -> u32 {
    read_volatile(addr)
}
pub unsafe fn mmio_read64(addr: *const u64) -> u64 {
    read_volatile(addr)
}
pub unsafe fn mmio_write8(addr: *mut u8, val: u8) {
    write_volatile(addr, val)
}
pub unsafe fn mmio_write16(addr: *mut u16, val: u16) {
    write_volatile(addr, val)
}
pub unsafe fn mmio_write32(addr: *mut u32, val: u32) {
    write_volatile(addr, val)
}
pub unsafe fn mmio_write64(addr: *mut u64, val: u64) {
    write_volatile(addr, val)
}
