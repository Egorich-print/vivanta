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

    // Create a Task for the child — PID is TaskId, not ThreadId (ADR-035)
    let child_task_id = crate::scheduler::create_task_for_thread(
        child_thread_id,
        registered_child_as_id,
        if parent_task_id != 0 { Some(parent_task_id) } else { None },
    );

    println!("  fork: parent_tid={} parent_task={} child_thread={} child_task={}", current_id, parent_task_id, child_thread_id, child_task_id);

    // Inherit signal dispositions and blocked mask (POSIX: child inherits
    // handlers/blocked, but pending is cleared). Keeps fork/waitpid/kill flow.
    if parent_task_id != 0 {
        let (blocked, handlers) = if let Some(p) = process_table().lookup(parent_task_id) {
            (p.signals.blocked, p.signals.handlers)
        } else {
            (0, [crate::signal::SigAction { handler: 0, mask: 0, flags: 0 }; crate::signal::MAX_SIG])
        };
        if let Some(c) = process_table().lookup_mut(child_task_id) {
            c.signals.blocked = blocked;
            c.signals.handlers = handlers;
            c.signals.pending = None;
        }
    }

    // Parent returns child's TaskId (PID) — TaskId is the process identity
    child_task_id
}

/// Wait for child process state change.
pub fn sys_waitpid(pid: u64, status: *mut i32, options: u64) -> u64 {
    println!("  syscall: waitpid(pid={}, status={:p}, options={})", pid, status, options);

    let current_tid = crate::scheduler::current_thread_id();
    let Some(current_task_id) = crate::scheduler::task_for_thread(current_tid) else {
        return EINVAL;
    };

    const WNOHANG: u64 = 1;

    loop {
        // 1. Check for already-zombie child to reap immediately.
        // For pid==0 (or pid==u64::MAX for -1) — any child in same pgrp, we treat as any.
        let reap_id = if pid == 0 || pid == u64::MAX {
            let children = process_table().children_of(current_task_id);
            children.into_iter().find(|&cid| {
                process_table().lookup(cid).is_some_and(|t| t.state == crate::scheduler::task::TaskState::Zombie)
            })
        } else {
            // Specific pid
            let children = process_table().children_of(current_task_id);
            if children.contains(&pid) {
                if process_table().lookup(pid).is_some_and(|t| t.state == crate::scheduler::task::TaskState::Zombie) {
                    Some(pid)
                } else {
                    // Child exists but not yet zombie
                    None
                }
            } else {
                // No such child
                println!("  waitpid: no matching child {}", pid);
                return EINVAL; // ECHILD
            }
        };

        if let Some(child_id) = reap_id {
            // Safe to reap: we re-lookup to get exit_code (avoid TOCTOU with lock, single-core so okay)
            let exit_code = process_table().lookup(child_id).and_then(|t| t.exit_code).unwrap_or(-1);
            println!("  waitpid: reaping child {} exit_code={}", child_id, exit_code);
            if !status.is_null() {
                let wait_status = (exit_code as u32) << 8;
                // SAFETY: copy_to_user validates user range against current AS
                unsafe {
                    if user_memory::copy_to_user(status as u64, &wait_status as *const _ as *const u8, 4).is_err() {
                        return EFAULT;
                    }
                }
            }
            // Also clean parent's children vec to avoid tombstone leakage
            if let Some(parent_task) = process_table().lookup_mut(current_task_id) {
                parent_task.children.retain(|&c| c != child_id);
            }
            let _ = process_table().remove(child_id);
            return child_id;
        }

        // 2. No zombie found — check if any child exists at all (for ECHILD)
        let children = process_table().children_of(current_task_id);
        let has_children = if pid == 0 || pid == u64::MAX {
            !children.is_empty()
        } else {
            children.contains(&pid)
        };
        if !has_children {
            println!("  waitpid: no matching child (ECHILD)");
            return EINVAL;
        }

        // 3. Not yet exited
        if options & WNOHANG != 0 {
            println!("  waitpid: no zombie, WNOHANG -> 0");
            return 0;
        }

        // 4. Block: put current thread to Blocked and yield.
        // For any-child wait we block on 0 (wake on any child), else specific.
        let wait_id = if pid == 0 || pid == u64::MAX { 0 } else { pid };
        println!("  waitpid: blocking for child {} (any={})", wait_id, wait_id==0);
        crate::scheduler::wait_for_child(wait_id);
        // SAFETY: wait_for_child is IRQ-guarded and sets Blocked; yield switches out.
        crate::scheduler::yield_now();
        // Woken — loop to re-check zombie (handles spurious wakeup)
    }
}

/// Send signal to process.
pub fn sys_kill(pid: u64, sig: u64) -> u64 {
    println!("  syscall: kill(pid={}, sig={})", pid, sig);

    // Find the target task — filter tombstones (Exited not killable)
    let is_live = process_table().lookup(pid).is_some_and(|t| t.state != crate::scheduler::task::TaskState::Exited);
    if !is_live {
        println!("  kill: no such task {}", pid);
        return EINVAL; // ESRCH
    }

    let Some(signal) = crate::signal::Signal::from_num(sig as u8) else {
        println!("  kill: invalid signal {}", sig);
        return EINVAL;
    };

    // SIGKILL is unblockable — clear blocked mask before send is handled in SignalState
    // For now just send and handle termination.
    if signal == crate::signal::Signal::Kill {
        // Need task's parent for wake and threads to terminate
        let (parent, threads) = {
            let t = process_table().lookup(pid).unwrap();
            (t.parent, t.threads.clone())
        };
        // Mark task Zombie (canonical, not Exited) so waitpid can reap
        if let Some(target_task) = process_table().lookup_mut(pid) {
            target_task.exit(-9); // Zombie with -SIGKILL
            target_task.signals.send(signal);
            println!("  kill: task {} -> Zombie (-SIGKILL)", pid);
        }
        // Terminate each thread of the target (except we don't self-terminate via this path if pid == current task)
        let current_tid = crate::scheduler::current_thread_id();
        let current_task = crate::scheduler::task_for_thread(current_tid);
        let is_self_kill = current_task == Some(pid);
        for tid in threads {
            if tid == current_tid && is_self_kill {
                // Self-kill will be handled by caller returning; thread_exit will run on next scheduling
                // Instead of immediate termination, just mark; the syscall return will still happen.
                // If we are self-killing, we should exit now. But kill is not supposed to be noreturn.
                // POSIX kill(self) just queues signal; the signal is delivered on return to user.
                // So we don't call thread_exit here; the pending signal will be checked on next entry.
                // Wake parent waiters immediately though.
                continue;
            }
            // SAFETY: thread_set_state is used under IRQ guard inside, but we are in syscall context.
            crate::scheduler::thread_set_state(tid, crate::scheduler::thread::ThreadState::Terminated);
        }
        // Wake parent and clean children vec
        if let Some(parent_id) = parent {
            if let Some(parent_task) = process_table().lookup_mut(parent_id) {
                parent_task.signals.send(crate::signal::Signal::Chld);
            }
            crate::scheduler::wake_waiters(pid);
        } else {
            // No parent — still wake any waiter for this specific pid
            crate::scheduler::wake_waiters(pid);
        }
        // Also wake any waiter for any child (0) — handle by also waking 0 queue? wake_waiters already handles 0==any in its logic when waking child.
        // But waiters for any child (wait_id 0) are woken when we call wake_waiters(pid) because condition is waited_for==0 || waited_for==pid.
        // So single call is enough.
        return 0;
    }

    // Non-KILL signals — just queue
    if let Some(target_task) = process_table().lookup_mut(pid) {
        target_task.signals.send(signal);
        println!("  kill: sent signal {:?} to task {}", signal, pid);
    }
    // If target is blocked in waitpid, wake it if signal is not ignored? For now wake if blocked.
    // A blocked waiter will re-check and return EINTR? Simplified: wake.
    crate::scheduler::wake_waiters(pid);
    0
}

/// Sigaction: install/inspect signal disposition for the current task.
///
/// ABI: rt_sigaction(sig, act_ptr, oldact_ptr) -> 0 or -errno
/// - sig in 1..31 (MAX_SIG=32). SIGKILL (9) cannot be caught/ignored -> EINVAL.
/// - act_ptr / oldact_ptr are user VAs to `SigAction` (handler:u64,mask:u64,flags:u32).
///   Null means absent (query-only or no old copy).
/// Minimal: stores handler VA in Task's SignalState.handlers[sig].
pub fn sys_sigaction(sig: u64, act: u64, oldact: u64) -> u64 {
    use crate::signal::{MAX_SIG, SIG_DFL, SIGKILL, SigAction};
    println!("  syscall: rt_sigaction(sig={}, act={:#x}, oldact={:#x})", sig, act, oldact);

    if sig == 0 || sig >= MAX_SIG as u64 {
        println!("  rt_sigaction: invalid sig {}", sig);
        return EINVAL;
    }
    // SIGKILL is uncapturable/unignorable per POSIX — any attempt to set
    // a non-default disposition must fail. Query-only (act==0) is allowed.
    if sig == SIGKILL as u64 && act != 0 {
        // SAFETY: copy_from_user validates range before read; we peek handler
        // to decide if caller tries to change disposition.
        let mut tmp = SigAction { handler: SIG_DFL, mask: 0, flags: 0 };
        // Try to read the user act; if fault, return EFAULT.
        // If handler != DFL, reject.
        let res = unsafe {
            vivanta_arch_api::user_memory::copy_from_user(
                &mut tmp as *mut _ as *mut u8,
                act,
                core::mem::size_of::<SigAction>(),
            )
        };
        if res.is_err() {
            return EFAULT;
        }
        if tmp.handler != SIG_DFL {
            println!("  rt_sigaction: cannot handle SIGKILL");
            return EINVAL;
        }
        // DFL for SIGKILL is the only valid value — allow but it's already DFL.
        // Fall through to normal install path (no-op).
    }

    let current_tid = crate::scheduler::current_thread_id();
    let Some(current_task_id) = crate::scheduler::task_for_thread(current_tid) else {
        return EINVAL;
    };

    // Snapshot old action for copy-out (before any mutation).
    if oldact != 0 {
        let cur = {
            let Some(t) = process_table().lookup(current_task_id) else {
                return EINVAL;
            };
            t.signals.handlers[sig as usize]
        };
        // SAFETY: copy_to_user validates user range against current AS first.
        let res = unsafe {
            vivanta_arch_api::user_memory::copy_to_user(
                oldact,
                &cur as *const _ as *const u8,
                core::mem::size_of::<SigAction>(),
            )
        };
        if res.is_err() {
            return EFAULT;
        }
    }

    if act != 0 {
        let mut new_act = SigAction { handler: SIG_DFL, mask: 0, flags: 0 };
        // SAFETY: copy_from_user validates the source range against the active
        // address space (TTBR0 at entry) before any kernel deref.
        let res = unsafe {
            vivanta_arch_api::user_memory::copy_from_user(
                &mut new_act as *mut _ as *mut u8,
                act,
                core::mem::size_of::<SigAction>(),
            )
        };
        if res.is_err() {
            return EFAULT;
        }
        // Re-validate after copy: SIGKILL already handled; general range check
        // for handler values: allow DFL, IGN, or any user VA (>1). No further
        // VA validation now (lazy — fault will be delivered).
        if sig == SIGKILL as u64 && new_act.handler != SIG_DFL {
            return EINVAL;
        }
        // Install
        if let Some(t) = process_table().lookup_mut(current_task_id) {
            t.signals.handlers[sig as usize] = new_act;
            println!("  rt_sigaction: task {} sig {} -> handler={:#x} mask={:#x} flags={:#x}",
                current_task_id, sig, new_act.handler, new_act.mask, new_act.flags);
        } else {
            return EINVAL;
        }
    }
    0
}

/// Sigreturn stub: placeholder for user handler return trampoline.
///
/// Real rt_sigreturn would restore the saved ExceptionFrame / blocked mask
/// from the signal frame pushed by the delivery path. For now it simply
/// returns 0 so the build and syscall dispatch are wired.
pub fn sys_sigreturn() -> u64 {
    println!("  syscall: rt_sigreturn() -> stub 0");
    0
}

/// Get current process ID — returns TaskId (PID), not ThreadId.
pub fn sys_getpid() -> u64 {
    let tid = crate::scheduler::current_thread_id();
    let pid = crate::scheduler::task_for_thread(tid).unwrap_or(tid);
    println!("  syscall: getpid() tid={} -> pid={}", tid, pid);
    pid
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
/// Overwrites the live SVC ExceptionFrame at *frame and returns 0;
/// the vectors.rs eret epilogue will restore ELR/SP/SPSR from that frame.
pub fn sys_execve(frame: *mut ExceptionFrame, path: *const u8, _argv: *const *const u8, _envp: *const *const u8) -> u64 {
    // SAFETY: frame is the live ExceptionFrame pushed by save_and_eret_sync at SP_EL1,
    // still on the current kernel stack. Caller (el0_sync_handler) guarantees validity.
    if frame.is_null() {
        return EFAULT;
    }
    println!("  syscall: execve(path={:p}) frame={:p}", path, frame);

    let path_str = match copy_user_string(path, 256) {
        Ok(s) => s,
        Err(e) => return e,
    };
    println!("  execve: path='{}'", path_str);

    let Some(elf) = find_builtin_program(&path_str) else {
        println!("  execve: program not found");
        return EINVAL; // ENOENT
    };

    let Some(current_thread) = current_thread() else {
        return EFAULT;
    };
    let current_as_id = current_thread.address_space;

    // Unmap all existing user mappings
    let Some(mut alloc) = make_allocator(current_as_id) else {
        return EFAULT;
    };
    let aspace = unsafe { crate::vmm::address_space_mut_by(current_as_id) };
    if aspace.unmap_all(&mut alloc).is_err() {
        return EFAULT;
    }

    // Load new ELF into same AS
    let Some(mut load_alloc) = make_allocator(current_as_id) else {
        return EFAULT;
    };
    let entry = match crate::exec::load_elf(elf, aspace, &mut load_alloc, crate::syscall::OBJ_ANONYMOUS) {
        Ok(e) => e,
        Err(e) => {
            println!("  execve: load_elf failed {:?}", e);
            return EFAULT;
        }
    };
    println!("  execve: loaded ELF entry={:#x}", entry);

    let stack_va = 0x5C01_0000u64;
    let stack_flags = vivanta_arch_api::mmu::MappingFlags::user() | vivanta_arch_api::mmu::MappingFlags::read_write();
    if aspace.reserve_at(stack_va, 4096, stack_flags, crate::syscall::OBJ_ANONYMOUS).is_err() {
        return EFAULT;
    }
    let stack_top = stack_va + 4096;

    // Overwrite the live frame — this is what eret will use.
    // Keep kernel stack; only user state changes.
    // SAFETY: frame is valid for write, single-core, no aliasing.
    unsafe {
        (*frame).elr = entry;
        (*frame).sp = stack_top;
        (*frame).spsr = 0x000; // EL0t
        (*frame).x = [0u64; 31];
        // x0 would be argc if we set up argv; for now 0
    }

    // Clear pending signals for new image (POSIX exec clears handlers)
    let current_tid = crate::scheduler::current_thread_id();
    if let Some(task_id) = crate::scheduler::task_for_thread(current_tid) {
        if let Some(task) = process_table().lookup_mut(task_id) {
            task.signals = crate::signal::SignalState::new();
        }
    }

    println!("  execve: switching to entry={:#x} sp={:#x}", entry, stack_top);
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