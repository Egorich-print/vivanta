use vivanta_boot_common::println;

pub const SYS_YIELD: u64 = 0;

#[no_mangle]
pub unsafe extern "Rust" fn syscall_dispatch(
    num: u64,
    _arg0: u64,
    _arg1: u64,
    _arg2: u64,
    _arg3: u64,
    _arg4: u64,
    _arg5: u64,
) -> u64 {
    match num {
        SYS_YIELD => {
            crate::scheduler::yield_now();
            0
        }
        _ => {
            println!("  syscall: unknown num={}", num);
            0
        }
    }
}
