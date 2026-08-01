#![no_std]
#![no_main]

extern crate vivanta_user_libc;

use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        unsafe { core::arch::asm!("wfi") }
    }
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    let msg = b"Hello, Vivanta!\n";
    vivanta_user_libc::write(1, msg);
    vivanta_user_libc::exit(0);
}
