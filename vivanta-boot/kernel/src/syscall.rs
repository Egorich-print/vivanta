use vivanta_boot_common::println;

/// Max bytes accepted by the `write` syscall per call (kernel stack buffer).
const WRITE_BUF_SIZE: usize = 256;

pub const SYS_READ: u64 = 0;
pub const SYS_WRITE: u64 = 1;
pub const SYS_EXIT: u64 = 2;
pub const SYS_YIELD: u64 = 3;
pub const SYS_MMAP: u64 = 4;

#[unsafe(no_mangle)]
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
            let buf = arg1 as u64;
            let count = arg2 as usize;
            if fd == 1 || fd == 2 {
                // G3: bulk-validate + copy the whole range into a kernel
                // buffer before touching any byte. copy_from_user fails the
                // entire call (-EFAULT) if ANY page is invalid.
                if count > WRITE_BUF_SIZE {
                    return -22i64 as u64; // -EINVAL
                }
                let mut kbuf = [0u8; WRITE_BUF_SIZE];
                // SAFETY: kbuf is a kernel stack buffer; copy_from_user
                // validates the source range first.
                if unsafe {
                    vivanta_arch_api::user_memory::copy_from_user(kbuf.as_mut_ptr(), buf, count)
                }
                .is_err()
                {
                    println!("  syscall: write -> -EFAULT (invalid user range)");
                    return -14i64 as u64; // -EFAULT
                }
                // Direct PL011 UART write.
                for &byte in kbuf[..count].iter() {
                    let uart = 0x0900_0000 as *mut u32;
                    unsafe {
                        while core::ptr::read_volatile(uart.add(0x18 / 4)) & (1 << 5) != 0 {}
                        core::ptr::write_volatile(uart, byte as u32);
                    }
                }
                count as u64
            } else {
                -1i64 as u64
            }
        }
        SYS_EXIT => {
            println!("  syscall: exit({})", arg0);
            crate::scheduler::thread_exit(arg0 as i32);
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
