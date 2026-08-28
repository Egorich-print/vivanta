// ---------------------------------------------------------------------------
// Syscall boundary — vivanta_kernel-provided dispatch called from arch
//
// Syscall NUMBERS are defined in a single place: kernel/src/syscall.rs
// (SYS_READ..SYS_MMAP). The arch EL0 handler reads the number from x8 and
// passes it here unchanged. Do not redefine syscall numbers in this crate.
// ---------------------------------------------------------------------------

use crate::exception::ExceptionFrame;

unsafe extern "Rust" {
    /// Dispatch a syscall from EL0.
    ///
    /// `num` is the syscall number (x8 in the Linux convention).
    /// `arg0`–`arg5` are the argument registers (x0–x5).
    /// `frame` is a pointer to the saved ExceptionFrame on the kernel stack.
    /// Returns the value to place in x0 on return to EL0.
    pub safe fn syscall_dispatch(
        num: u64,
        arg0: u64,
        arg1: u64,
        arg2: u64,
        arg3: u64,
        arg4: u64,
        arg5: u64,
        frame: *mut ExceptionFrame,
    ) -> u64;
}
