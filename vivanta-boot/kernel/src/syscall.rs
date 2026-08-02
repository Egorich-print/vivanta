use vivanta_boot_common::println;

pub const SYS_READ: u64 = 0;
pub const SYS_WRITE: u64 = 1;
pub const SYS_EXIT: u64 = 2;
pub const SYS_YIELD: u64 = 3;
pub const SYS_MMAP: u64 = 4;

#[no_mangle]
pub unsafe extern "Rust" fn syscall_dispatch(
    num: u64,
    arg0: u64,
    arg1: u64,
    arg2: u64,
    _arg3: u64,
    _arg4: u64,
    _arg5: u64,
) -> u64 {
    match num {
        SYS_READ => {
            // Stub: return 0
            0
        }
        SYS_WRITE => {
            // arg0 = fd, arg1 = buf_ptr, arg2 = count
            let fd = arg0;
            let buf = arg1 as *const u8;
            let count = arg2 as usize;
            if fd == 1 || fd == 2 {
                // stdout/stderr: write to UART
                for i in 0..count {
                    let byte = *buf.add(i);
                    // Direct PL011 UART write
                    let uart = 0x0900_0000 as *mut u32;
                    while core::ptr::read_volatile(uart.add(0x18 / 4)) & (1 << 5) != 0 {}
                    core::ptr::write_volatile(uart, byte as u32);
                }
                count as u64
            } else {
                -1i64 as u64
            }
        }
        SYS_EXIT => {
            println!("  syscall: exit({})", arg0);
            crate::scheduler::thread_exit();
        }
        SYS_YIELD => {
            crate::scheduler::yield_now();
            0
        }
        SYS_MMAP => {
            -12i64 as u64 // -ENOMEM
        }
        _ => {
            println!("  syscall: unknown num={}", num);
            -38i64 as u64 // -ENOSYS
        }
    }
}
