/// Process management syscalls.
///
/// These handle process lifecycle: fork, exit, waitpid, kill, getpid, getppid.

use crate::syscall::{ENOMEM, EFAULT, EINVAL, ENOSYS};
use crate::vmm::address_space::find_by_root;
use vivanta_boot_common::println;

/// Exit current process with exit code.
pub fn sys_exit(code: i32) -> ! {
    println!("  syscall: exit({})", code);
    crate::scheduler::thread_exit(code)
}

/// Fork current process: duplicate address space with COW.
/// Returns child PID in parent, 0 in child.
pub fn sys_fork(_as_root: u64) -> u64 {
    println!("  syscall: fork()");
    // TODO(G-M10): Implement actual fork using vmm::address_space::duplicate_as()
    // Requires wiring up PageTableAllocator from MemoryResourceManager
    ENOMEM
}

/// Wait for child process state change.
pub fn sys_waitpid(pid: u64, _status: *mut i32, _options: u64) -> u64 {
    println!("  syscall: waitpid({}, ...)", pid);
    // TODO(G-M10): Implement process waitpid using process_table
    EINVAL // ECHILD - no child processes
}

/// Send signal to process.
pub fn sys_kill(pid: u64, sig: u64) -> u64 {
    println!("  syscall: kill({}, sig={})", pid, sig);
    // TODO(G-M10): Implement signal delivery
    EINVAL // ESRCH - no such process
}

/// Get current process ID.
pub fn sys_getpid() -> u64 {
    let tid = crate::scheduler::current_thread_id();
    println!("  syscall: getpid() -> {}", tid);
    tid
}

/// Get parent process ID.
pub fn sys_getppid() -> u64 {
    let tid = crate::scheduler::current_thread_id();
    println!("  syscall: getppid() for tid={}", tid);
    0 // Parent is stored separately
}

/// Execve: replace current process image with new program.
/// Never returns on success.
pub fn sys_execve(_path: *const u8, _argv: *const *const u8, _envp: *const *const u8) -> u64 {
    println!("  syscall: execve(...)");
    ENOSYS
}

/// Munmap: remove mappings from address space.
pub fn sys_munmap(as_root: u64, addr: u64, len: u64) -> u64 {
    println!("  syscall: munmap(0x{:x}, 0x{:x})", addr, len);
    let Some(_aspace) = find_by_root(as_root) else {
        return EFAULT;
    };
    if len == 0 {
        return EINVAL;
    }
    // TODO(G-M10): Implement munmap using unmap_range() with PageTableAllocator
    // Requires plumbing allocator from MemoryResourceManager
    ENOSYS
}

/// Mprotect: change protection of existing mappings.
pub fn sys_mprotect(as_root: u64, addr: u64, len: u64, prot: u64) -> u64 {
    println!("  syscall: mprotect(0x{:x}, 0x{:x}, 0x{:x})", addr, len, prot);
    let Some(_aspace) = find_by_root(as_root) else {
        return EFAULT;
    };
    if len == 0 {
        return EINVAL;
    }
    let Some(_flags) = crate::syscall::decode_prot(prot) else {
        return EINVAL;
    };
    // TODO(G-M10): Implement mprotect using protect() with PageTableAllocator
    // Requires plumbing allocator from MemoryResourceManager
    ENOSYS
}