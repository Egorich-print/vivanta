#![no_std]
#![no_main]

// ---------------------------------------------------------------------------
// Vivanta Minimal libc — syscall wrappers for user-space
// ---------------------------------------------------------------------------

/// Syscall numbers (must match kernel/src/syscall.rs)
pub const SYS_READ: u64 = 0;
pub const SYS_WRITE: u64 = 1;
pub const SYS_EXIT: u64 = 2;
pub const SYS_YIELD: u64 = 3;
pub const SYS_MMAP: u64 = 4;

/// Raw syscall invocation.
///
/// Safety: caller must ensure correct syscall number and arguments.
#[inline(always)]
pub unsafe fn syscall0(num: u64) -> u64 {
    let ret: u64;
    core::arch::asm!(
        "svc #0",
        inlateout("x8") num => ret,
        options(nostack)
    );
    ret
}

#[inline(always)]
pub unsafe fn syscall1(num: u64, arg0: u64) -> u64 {
    let ret: u64;
    core::arch::asm!(
        "svc #0",
        inlateout("x8") num => ret,
        in("x0") arg0,
        options(nostack)
    );
    ret
}

#[inline(always)]
pub unsafe fn syscall2(num: u64, arg0: u64, arg1: u64) -> u64 {
    let ret: u64;
    core::arch::asm!(
        "svc #0",
        inlateout("x8") num => ret,
        in("x0") arg0,
        in("x1") arg1,
        options(nostack)
    );
    ret
}

#[inline(always)]
pub unsafe fn syscall3(num: u64, arg0: u64, arg1: u64, arg2: u64) -> u64 {
    let ret: u64;
    core::arch::asm!(
        "svc #0",
        inlateout("x8") num => ret,
        in("x0") arg0,
        in("x1") arg1,
        in("x2") arg2,
        options(nostack)
    );
    ret
}

// ---------------------------------------------------------------------------
// High-level wrappers
// ---------------------------------------------------------------------------

/// Write bytes to file descriptor.
/// Returns bytes written, or -1 on error.
pub fn write(fd: u64, buf: &[u8]) -> i64 {
    unsafe { syscall3(SYS_WRITE, fd, buf.as_ptr() as u64, buf.len() as u64) as i64 }
}

/// Exit current process.
pub fn exit(code: i32) -> ! {
    unsafe {
        syscall1(SYS_EXIT, code as u64);
    }
    loop {
        unsafe { core::arch::asm!("wfi") }
    }
}

/// Yield to scheduler.
pub fn yield_now() {
    unsafe {
        syscall0(SYS_YIELD);
    }
}

/// Read bytes from file descriptor.
/// Returns bytes read, or -1 on error.
pub fn read(fd: u64, buf: &mut [u8]) -> i64 {
    unsafe { syscall3(SYS_READ, fd, buf.as_ptr() as u64, buf.len() as u64) as i64 }
}
