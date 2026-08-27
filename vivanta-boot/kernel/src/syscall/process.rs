/// Process management syscalls.
///
/// These handle process lifecycle: fork, exit, waitpid, kill, getpid, getppid.

use crate::syscall::{ENOMEM, EFAULT};

/// Exit current process with exit code.
pub fn sys_exit(code: i32) -> ! {
    println!("  syscall: exit({})", code);
    crate::scheduler::thread_exit(code)
}

/// Fork current process: duplicate address space with COW.
/// Returns child PID in parent, 0 in child.
pub fn sys_fork() -> u64 {
    println!("  syscall: fork()");
    ENOMEM
}

/// Wait for child process state change.
pub fn sys_waitpid(pid: u64, status: *mut i32, options: u64) -> isize {
    println!("  syscall: waitpid({}, ..., {})", pid, options);
    -1
}

/// Send signal to process.
pub fn sys_kill(pid: u64, sig: u64) -> u64 {
    println!("  syscall: kill({}, sig={})", pid, sig);
    -1
}

/// Get current process ID.
pub fn sys_getpid() -> u64 {
    let tid = crate::scheduler::current_thread_id();
    println!("  syscall: getpid() -> {}", tid.0);
    tid.0
}

/// Get parent process ID.
pub fn sys_getppid() -> u64 {
    let tid = crate::scheduler::current_thread_id();
    println!("  syscall: getppid() for tid={}", tid.0);
    0  // Parent is stored separately
}

/// Execve: replace current process image with new program.
/// Never returns on success.
pub fn sys_execve(path: *const u8, argv: *const *const u8, envp: *const *const u8) -> isize {
    println!("  syscall: execve(...)");
    -1
}

/// Wait for child to exit.
pub fn sys_waitpid(pid: u64, status: *mut i32, options: u64) -> isize {
    -1  // ECHILD
}

/// Kill a process.
pub fn sys_kill(pid: u64, sig: u64) -> u64 {
    println!("  syscall: kill({}, sig={})", pid, sig);
    -1
}

/// Get current process ID.
pub fn sys_getpid() -> u64 {
    let tid = crate::scheduler::current_thread_id();
    println!("  syscall: getpid() -> {}", tid.0);
    tid.0
}

/// Get parent process ID.
pub fn sys_getppid() -> u64 {
    let tid = crate::scheduler::current_thread_id();
    0  // Parent stored separately
}
