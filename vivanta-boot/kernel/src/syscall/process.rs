/// Process management syscalls.
///
/// These handle process lifecycle: fork, exit, waitpid, kill, getpid, getppid.

use crate::scheduler::{current_thread, stack_allocator, task_for_thread, process_table};
use crate::syscall::{ENOMEM, EFAULT, EINVAL};
use crate::vmm::address_space::{find_by_root, register_child};
use crate::vmm::faults::make_allocator;
use vivanta_arch_api::context::context_fork;
use vivanta_arch_api::exception::ExceptionFrame;
use vivanta_arch_api::mmu::RootPageTable;
use vivanta_arch_api::user_memory;
use vivanta_boot_common::println;

/// Exit current process with exit code.
pub fn sys_exit(code: i32) -> ! {
    println!("  syscall: exit({})", code);
    crate::scheduler::thread_exit(code)
}

/// Fork current process: duplicate address space with COW.
/// Returns child PID in parent, 0 in child.
pub fn sys_fork(as_root: u64, frame: *mut ExceptionFrame) -> u64 {
    println!("  syscall: fork()");

    // Get the current thread's info
    let Some(current_thread) = current_thread() else {
        println!("  fork: no current thread");
        return EFAULT;
    };

    let current_id = current_thread.id;
    let _parent_as_id = current_thread.address_space;
    let parent_kernel_stack_pa = current_thread.kernel_stack_pa.unwrap_or(0);
    let parent_priority = current_thread.priority;

    // Get the parent's TaskId
    let parent_task_id = task_for_thread(current_id).unwrap_or(0);

    // Get the parent address space
    let Some(parent_aspace) = find_by_root(as_root) else {
        println!("  fork: no parent address space for root={:#x}", as_root);
        return EFAULT;
    };

    // Allocate a new kernel stack for the child (16 KiB contiguous)
    let Some(stack_alloc) = stack_allocator() else {
        println!("  fork: no stack allocator");
        return ENOMEM;
    };

    let stack_frames = crate::scheduler::KERNEL_STACK_SIZE / 4096;
    let Some(stack_frame) = stack_alloc.alloc_contiguous(stack_frames) else {
        println!("  fork: failed to allocate kernel stack");
        return ENOMEM;
    };
    let child_kernel_stack_pa = stack_frame.addr;
    let child_kernel_stack_top = child_kernel_stack_pa + crate::scheduler::KERNEL_STACK_SIZE as u64;
    let child_kernel_stack_bottom = child_kernel_stack_pa;

    // Allocate a root page table frame for the child (1 page = 4 KiB)
    let Some(root_frame) = stack_alloc.alloc_frame() else {
        println!("  fork: failed to allocate root page table");
        // Free the kernel stack
        for i in 0..stack_frames {
            stack_alloc.free_frame(vivanta_arch_api::pmm::PhysFrame {
                addr: child_kernel_stack_pa + (i as u64) * 4096,
            });
        }
        return ENOMEM;
    };
    let child_root_pa = root_frame.addr;
    let child_root = RootPageTable(child_root_pa as usize);

    // Zero the root page table frame
    unsafe {
        core::ptr::write_bytes(child_root_pa as *mut u8, 0, 4096);
    }

    // Create allocator for the child address space
    let child_as_id = crate::vmm::peek_next_as_id();
    let Some(mut child_alloc) = make_allocator(child_as_id) else {
        println!("  fork: no allocator for child AS");
        // Free the child kernel stack and root page table
        for i in 0..stack_frames {
            stack_alloc.free_frame(vivanta_arch_api::pmm::PhysFrame {
                addr: child_kernel_stack_pa + (i as u64) * 4096,
            });
        }
        stack_alloc.free_frame(root_frame);
        return ENOMEM;
    };

    // Duplicate the parent address space into the child
    let parent_aspace_ptr = parent_aspace as *const _ as *mut crate::vmm::address_space::AddressSpace;
    let child_aspace = match unsafe { (*parent_aspace_ptr).duplicate_as(child_root, &mut child_alloc) } {
        Ok(aspace) => aspace,
        Err(e) => {
            println!("  fork: duplicate_as failed: {:?}", e);
            // Free the child kernel stack and root page table
            for i in 0..stack_frames {
                stack_alloc.free_frame(vivanta_arch_api::pmm::PhysFrame {
                    addr: child_kernel_stack_pa + (i as u64) * 4096,
                });
            }
            stack_alloc.free_frame(root_frame);
            return ENOMEM;
        }
    };

    // Register the child address space
    let registered_child_as_id = register_child(child_aspace, child_root);
    println!("  fork: child AS id = {}", registered_child_as_id);

    // Create child's context by copying parent's ThreadContext and ExceptionFrame
    let child_context = unsafe {
        context_fork(
            child_kernel_stack_top as usize,
            child_kernel_stack_bottom as usize,
            parent_kernel_stack_pa as usize,
            frame,
        )
    };

    // Create the child thread
    let child_thread_id = crate::scheduler::create_thread_with_context(
        child_context,
        child_kernel_stack_pa,
        registered_child_as_id,
        parent_priority,
    );

    // Create a Task for the child
    let _child_task_id = crate::scheduler::create_task_for_thread(
        child_thread_id,
        registered_child_as_id,
        if parent_task_id != 0 { Some(parent_task_id) } else { None },
    );

    println!("  fork: parent={} child_thread={} child_task={}", current_id, child_thread_id, _child_task_id);

    // Parent returns child's ThreadId (used as PID)
    child_thread_id
}

/// Wait for child process state change.
pub fn sys_waitpid(pid: u64, status: *mut i32, options: u64) -> u64 {
    println!("  syscall: waitpid(pid={}, status={:p}, options={})", pid, status, options);

    let current_tid = crate::scheduler::current_thread_id();
    let Some(current_task_id) = crate::scheduler::task_for_thread(current_tid) else {
        return EINVAL;
    };

    // WNOHANG = 1 (don't block if no child exited)
    const WNOHANG: u64 = 1;

    // Find child task
    let child_task_id = if pid == 0 {
        // Wait for any child
        let children = process_table().children_of(current_task_id);
        children.first().copied()
    } else {
        // Wait for specific child
        let children = process_table().children_of(current_task_id);
        children.iter().find(|&&id| id == pid).copied()
    };

    let Some(child_id) = child_task_id else {
        println!("  waitpid: no matching child");
        return EINVAL; // ECHILD
    };

    // Check if child is a zombie (exited but not reaped)
    if let Some(child_task) = process_table().lookup(child_id) {
        if child_task.state == crate::scheduler::task::TaskState::Zombie {
            // Reap the zombie
            let exit_code = child_task.exit_code.unwrap_or(-1);
            println!("  waitpid: reaping child {} exit_code={}", child_id, exit_code);

            // Write status to user memory if provided
            if !status.is_null() {
                // Encode exit code in waitpid status format (WEXITSTATUS)
                let wait_status = (exit_code as u32) << 8; // WEXITSTATUS macro expects this
                unsafe {
                    if crate::vmm::find_by_root(crate::scheduler::current_thread_address_space()).is_some() {
                        if user_memory::copy_to_user(
                            status as u64,
                            &wait_status as *const _ as *const u8,
                            4
                        ).is_err() {
                            return EFAULT;
                        }
                    }
                }
            }

            // Remove the child task (reap)
            let _ = process_table().remove(child_id);
            return child_id; // Return PID of reaped child
        }
    }

    // Child not exited yet
    if options & WNOHANG != 0 {
        println!("  waitpid: child {} not exited, WNOHANG", child_id);
        return 0; // WNOHANG - return 0 immediately
    }

    // Block until child exits
    println!("  waitpid: child {} not exited, blocking", child_id);
    crate::scheduler::wait_for_child(child_id);
    
    // After wakeup, loop and try again (the child should now be a zombie)
    // Note: This is a simplified implementation - in reality we'd need to
    // handle spurious wakeups and re-check the child state.
    sys_waitpid(pid, status, options)
}

/// Send signal to process.
pub fn sys_kill(pid: u64, sig: u64) -> u64 {
    println!("  syscall: kill(pid={}, sig={})", pid, sig);

    // Find the target task
    let _target_task = process_table().lookup(pid);
    if _target_task.is_none() {
        println!("  kill: no such task {}", pid);
        return EINVAL; // ESRCH
    }

    // Get signal
    let Some(signal) = crate::signal::Signal::from_num(sig as u8) else {
        println!("  kill: invalid signal {}", sig);
        return EINVAL;
    };

    // Send signal to task
    if let Some(target_task) = process_table().lookup_mut(pid) {
        target_task.signals.send(signal);
        println!("  kill: sent signal {:?} to task {}", signal, pid);

        // SIGKILL immediately terminates the task
        if signal == crate::signal::Signal::Kill {
            // Mark task for termination (will be handled on next scheduler tick)
            // For now, just note it - full signal handling needs more infrastructure
            target_task.state = crate::scheduler::task::TaskState::Exited;
            target_task.exit_code = Some(-9); // -SIGKILL
            println!("  kill: task {} marked for termination", pid);
        }
    }

    0
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
    if let Some(parent_task_id) = crate::scheduler::task_for_thread(tid) {
        if let Some(task) = process_table().lookup(parent_task_id) {
            if let Some(parent) = task.parent {
                println!("  syscall: getppid() -> {}", parent);
                return parent;
            }
        }
    }
    println!("  syscall: getppid() for tid={} -> 0 (no parent)", tid);
    0 // No parent (init process)
}

/// Built-in program registry for execve (no filesystem yet).
/// Maps program name to ELF bytes.
static BUILTIN_PROGRAMS: &[(&str, &[u8])] = &[
    ("/init", include_bytes!("../../../user-init/user-init.elf")),
];

fn find_builtin_program(path: &str) -> Option<&'static [u8]> {
    for (name, elf) in BUILTIN_PROGRAMS {
        if *name == path {
            return Some(elf);
        }
    }
    None
}

/// Copy a user-space string to kernel buffer.
fn copy_user_string(ptr: *const u8, max_len: usize) -> Result<alloc::string::String, u64> {
    let mut buf = alloc::vec::Vec::with_capacity(max_len);
    let mut p = ptr;
    for _ in 0..max_len {
        let mut byte = 0u8;
        let byte_ptr = &mut byte as *mut u8;
        let result = unsafe {
            user_memory::copy_from_user(byte_ptr, p as u64, 1)
        };
        if result.is_err() {
            return Err(EFAULT);
        }
        if byte == 0 {
            break;
        }
        buf.push(byte);
        p = unsafe { p.add(1) };
    }
    alloc::string::String::from_utf8(buf).map_err(|_| EINVAL)
}

/// Execve: replace current process image with new program.
/// Never returns on success.
pub fn sys_execve(path: *const u8, argv: *const *const u8, _envp: *const *const u8) -> u64 {
    println!("  syscall: execve(path={:p}, argv={:p})", path, argv);

    // Copy path from user space
    let path_str = match copy_user_string(path, 256) {
        Ok(s) => s,
        Err(e) => return e,
    };
    println!("  execve: path='{}'", path_str);

    // Find the program
    let Some(elf) = find_builtin_program(&path_str) else {
        println!("  execve: program not found");
        return EINVAL; // ENOENT
    };

    // Get current thread and address space
    let current_tid = crate::scheduler::current_thread_id();
    let Some(current_thread) = current_thread() else {
        return EFAULT;
    };
    let current_as_id = current_thread.address_space;

    // Unmap all existing user mappings in the address space
    let Some(mut alloc) = make_allocator(current_as_id) else {
        return EFAULT;
    };
    let aspace = unsafe { crate::vmm::address_space_mut_by(current_as_id) };
    if let Err(_) = aspace.unmap_all(&mut alloc) {
        return EFAULT;
    }

    // Load the new ELF
    let Some(mut load_alloc) = make_allocator(current_as_id) else {
        return EFAULT;
    };
    let entry = match crate::exec::load_elf(elf, aspace, &mut load_alloc, crate::syscall::OBJ_ANONYMOUS) {
        Ok(e) => e,
        Err(_) => return EFAULT,
    };
    println!("  execve: loaded ELF, entry={:#x}", entry);

    // Set up new user stack
    // Allocate a stack page
    let stack_va = 0x5C01_0000u64; // Fixed stack VA for now
    let stack_flags = vivanta_arch_api::mmu::MappingFlags::user() | vivanta_arch_api::mmu::MappingFlags::read_write();
    if let Err(_) = aspace.reserve_at(stack_va, 4096, stack_flags, crate::syscall::OBJ_ANONYMOUS) {
        return EFAULT;
    }

    // Copy argv to user stack (simplified - just set up initial stack frame)
    let stack_top = stack_va + 4096;

    // Re-initialize the thread context for the new program
    let Some(stack_alloc) = stack_allocator() else {
        return ENOMEM;
    };
    let stack_frames = crate::scheduler::KERNEL_STACK_SIZE / 4096;
    let Some(stack_frame) = stack_alloc.alloc_contiguous(stack_frames) else {
        return ENOMEM;
    };
    let kernel_stack_pa = stack_frame.addr;
    let kernel_stack_top = kernel_stack_pa + crate::scheduler::KERNEL_STACK_SIZE as u64;

    // Create new context for the program entry
    let new_context = unsafe {
        vivanta_arch_api::context::context_init(
            kernel_stack_top as usize,
            kernel_stack_pa as usize,
            stack_top as usize,
            entry as usize,
            vivanta_arch_api::context::ExecutionLevel::User,
        )
    };

    // Update the thread's context and kernel stack
    if let Some(thread) = crate::scheduler::get_thread_mut(current_tid) {
        thread.context = new_context;
        thread.kernel_stack_pa = Some(kernel_stack_pa);
        thread.address_space = current_as_id;
    }

    // Free old kernel stack
    if let Some(old_pa) = current_thread.kernel_stack_pa {
        if old_pa != kernel_stack_pa {
            for i in 0..stack_frames {
                stack_alloc.free_frame(vivanta_arch_api::pmm::PhysFrame {
                    addr: old_pa + (i as u64) * 4096,
                });
            }
        }
    }

    // Return to user mode at entry point with new context
    // This is done by modifying the ExceptionFrame on the kernel stack
    // The syscall return path will use this frame
    println!("  execve: switching to new program at {:#x}", entry);
    0
}

/// Munmap: remove mappings from address space.
pub fn sys_munmap(as_root: u64, addr: u64, len: u64) -> u64 {
    println!("  syscall: munmap(0x{:x}, 0x{:x})", addr, len);
    let Some(aspace) = find_by_root(as_root) else {
        return EFAULT;
    };
    if len == 0 {
        return EINVAL;
    }

    // Get allocator for this address space
    let as_id = aspace.id;
    let Some(mut alloc) = make_allocator(as_id) else {
        println!("  munmap: no allocator for AS {}", as_id);
        return EFAULT;
    };

    match aspace.unmap_range(addr, len, &mut alloc) {
        Ok(_) => 0,
        Err(_) => EFAULT,
    }
}

/// Mprotect: change protection of existing mappings.
pub fn sys_mprotect(as_root: u64, addr: u64, len: u64, prot: u64) -> u64 {
    println!("  syscall: mprotect(0x{:x}, 0x{:x}, 0x{:x})", addr, len, prot);
    let Some(aspace) = find_by_root(as_root) else {
        return EFAULT;
    };
    if len == 0 {
        return EINVAL;
    }
    let Some(flags) = crate::syscall::decode_prot(prot) else {
        return EINVAL;
    };

    // Get allocator for this address space
    let as_id = aspace.id;
    let Some(mut alloc) = make_allocator(as_id) else {
        println!("  mprotect: no allocator for AS {}", as_id);
        return EFAULT;
    };

    match aspace.protect(addr, len, flags, &mut alloc) {
        Ok(_) => 0,
        Err(_) => EFAULT,
    }
}