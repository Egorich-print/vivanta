// ---------------------------------------------------------------------------
// Syscall boundary — vivanta_kernel-provided dispatch called from arch
// ---------------------------------------------------------------------------

/// Syscall numbers (shared between kernel and arch).
pub const SYS_YIELD: u64 = 0;

extern "Rust" {
    /// Dispatch a syscall from EL0.
    ///
    /// `num` is the syscall number (x8 in the Linux convention).
    /// `arg0`–`arg5` are the argument registers (x0–x5).
    /// Returns the value to place in x0 on return to EL0.
    pub fn syscall_dispatch(num: u64, arg0: u64, arg1: u64, arg2: u64, arg3: u64, arg4: u64, arg5: u64) -> u64;
}
