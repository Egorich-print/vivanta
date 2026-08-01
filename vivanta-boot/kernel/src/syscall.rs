use vivanta_boot_common::println;
use crate::syscall;
use vivanta_arch_api::context::ArchContext;

pub const SYS_YIELD: u64 = 0;

#[no_mangle]
pub unsafe extern "Rust" fn syscall_dispatch(
    num: u64,
    arg0: u64,
    arg1: u64,
    arg2: u64,
    arg3: u64,
    arg4: u64,
    arg5: u64,
) -> u64 {
    match num {
        0 => { // SYS_READ
            // TODO: implement proper dispatch
            0
        }
        1 => { // SYS_WRITE
            // TODO: implement proper dispatch
            arg2 // return count as bytes written
        }
        2 => { // SYS_EXIT
            // TODO: implement proper exit
            loop { core::arch::asm!("wfi") }
        }
        3 => { // SYS_YIELD
            crate::scheduler::yield_now();
            0
        }
        4 => { // SYS_MMAP
            -12i64 as u64 // -ENOMEM
        }
        _ => {
            println!("  syscall: unknown num={}", num);
            -38i64 as u64 // -ENOSYS
        }
    }
}
