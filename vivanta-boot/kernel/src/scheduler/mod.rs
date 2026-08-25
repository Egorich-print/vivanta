// ---------------------------------------------------------------------------
// vivanta_kernel scheduler — Thread lifecycle, RunQueue, scheduling policy
// ---------------------------------------------------------------------------

pub mod process_table;
pub mod runqueue;
pub mod task;
pub mod task_manager;
pub mod thread;

use crate::vmm::AddressSpaceId;
use alloc::vec::Vec;
use process_table::ProcessTable;
use runqueue::RunQueue;
use task::{TaskId, TaskState};
use thread::{Priority, Thread, ThreadEntry, ThreadId, ThreadState};
use vivanta_arch_api::pmm::FrameAllocator;

pub const KERNEL_STACK_SIZE: usize = 16384;

static mut RUNQUEUE: Option<RunQueue> = None;
static mut PROCESS_TABLE: Option<ProcessTable> = None;
/// Boot/idle thread identities. Set once during `init_boot`, read-only
/// afterwards — Relaxed atomics remove the static-mut hazard entirely.
static BOOT_THREAD_ID: AtomicU64 = AtomicU64::new(0);
static IDLE_THREAD_ID: AtomicU64 = AtomicU64::new(0);
/// Frame allocator used to reclaim kernel stacks of terminated threads.
/// Set once during boot (kernel_main) after the PMM is initialised.
static mut STACK_ALLOCATOR: Option<*mut dyn FrameAllocator> = None;

/// Register the allocator used to free kernel stacks on thread exit (G2).
/// The PMM outlives all threads; the pointer stays valid for the kernel's
/// whole lifetime (same aliasing contract as PmmBackend).
pub fn register_stack_allocator(alloc: &mut (dyn FrameAllocator + 'static)) {
    let ptr: *mut dyn FrameAllocator = alloc;
    unsafe {
        STACK_ALLOCATOR = Some(ptr);
    }
}

use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};
static NEED_RESCHEDULE: AtomicBool = AtomicBool::new(false);
/// The ThreadId of the currently running thread (NOT a runqueue index).
/// Index-based `current` aliased stale threads after slot reuse; ThreadId is
/// immutable and never reused, so churn (spawn A, B, kill A, spawn C) cannot
/// corrupt it (G4 §8).
static CURRENT_THREAD: AtomicU64 = AtomicU64::new(0);

/// Read the current ThreadId.
pub fn current_thread_id() -> ThreadId {
    CURRENT_THREAD.load(Ordering::Relaxed)
}

/// G4 invariant check: how many threads are in the Running state right now.
/// The scheduler model requires exactly one Running thread on single-core.
pub fn running_thread_count() -> usize {
    rq().iter()
        .filter(|t| t.state == ThreadState::Running)
        .count()
}

// ---------------------------------------------------------------------------
// RunQueue accessor helpers (unsafe because static mut)
// ---------------------------------------------------------------------------

pub fn thread_set_state(id: ThreadId, new_state: ThreadState) {
    rq().set_state(id, new_state)
        .expect("thread_set_state failed");
}

fn rq() -> &'static mut RunQueue {
    unsafe {
        if RUNQUEUE.is_none() {
            RUNQUEUE = Some(RunQueue::new());
        }
        RUNQUEUE.as_mut().unwrap()
    }
}

fn pt() -> &'static mut ProcessTable {
    unsafe {
        if PROCESS_TABLE.is_none() {
            PROCESS_TABLE = Some(ProcessTable::new());
        }
        PROCESS_TABLE.as_mut().unwrap()
    }
}

// ---------------------------------------------------------------------------
// Scheduler API
// ---------------------------------------------------------------------------

/// Get the `AddressSpaceId` of the currently running thread.
pub fn current_thread_address_space() -> AddressSpaceId {
    rq().get(current_thread_id())
        .map(|t| t.address_space)
        .unwrap_or(0)
}

/// Put current thread to sleep for `ticks` timer ticks.
pub fn sleep(ticks: u64) {
    let _guard = unsafe { vivanta_arch_api::interrupts::disable_interrupts() };

    let current_id = current_thread_id();

    // Get current tick count from timer
    let current_tick = unsafe { vivanta_arch_api::boot::timer::ticks() };

    // Set sleep_until on thread
    if let Some(t) = rq().get_mut(current_id) {
        t.sleep_until = Some(current_tick + ticks);
        t.state = ThreadState::Sleeping;
    }

    // Yield to another thread
    yield_now();
}

/// Wake up a sleeping thread.
pub fn wake(id: ThreadId) {
    let _guard = unsafe { vivanta_arch_api::interrupts::disable_interrupts() };

    if let Some(t) = rq().get_mut(id) {
        if t.state == ThreadState::Sleeping {
            t.sleep_until = None;
            t.state = ThreadState::Ready;
        }
    }
}

/// Check for sleeping threads that should be woken up.
/// Called from scheduler_tick().
pub fn check_sleeping_threads() {
    let current_tick = unsafe { vivanta_arch_api::boot::timer::ticks() };

    let to_wake: Vec<ThreadId> = rq()
        .iter()
        .filter(|t| t.state == ThreadState::Sleeping)
        .filter(|t| t.sleep_until.map_or(false, |until| current_tick >= until))
        .map(|t| t.id)
        .collect();

    for id in to_wake {
        wake(id);
    }
}

pub fn register(thread: Thread) {
    rq().insert(thread).expect("runqueue full");
}

pub fn create_user_thread(
    kernel_stack_top: usize,
    kernel_stack_bottom: usize,
    user_stack_top: usize,
    entry: usize,
    address_space: AddressSpaceId,
    priority: Priority,
) -> ThreadId {
    let ctx = unsafe {
        vivanta_arch_api::context::context_init(
            kernel_stack_top,
            kernel_stack_bottom,
            user_stack_top,
            entry,
            vivanta_arch_api::context::ExecutionLevel::User,
        )
    };
    let id = rq().alloc_id();
    let thread = Thread {
        id,
        state: ThreadState::Created,
        priority,
        context: ctx,
        entry: None,
        address_space,
        level: vivanta_arch_api::context::ExecutionLevel::User,
        sleep_until: None,
        kernel_stack_pa: Some((kernel_stack_top - KERNEL_STACK_SIZE) as u64),
    };
    register(thread);
    thread_set_state(id, ThreadState::Ready);
    id
}

pub fn create_kernel_thread(
    entry: ThreadEntry,
    _arg: usize,
    alloc: &mut impl FrameAllocator,
    address_space: AddressSpaceId,
    priority: Priority,
) -> ThreadId {
    // Kernel stack: KERNEL_STACK_SIZE (16 KiB) must be physically contiguous
    // so SP_EL1 is a valid single stack. Use the explicit contiguous contract.
    let stack_base = alloc
        .alloc_contiguous(KERNEL_STACK_SIZE / 4096)
        .expect("kernel stack contiguous alloc failed")
        .addr;
    let stack_top = (stack_base as usize) + KERNEL_STACK_SIZE;
    let stack_bottom = stack_base as usize;
    let ctx = unsafe {
        vivanta_arch_api::context::context_init(
            stack_top,
            stack_bottom,
            0, // user_stack_top — vivanta_kernel threads don't use EL0
            thread_trampoline as *const () as usize,
            vivanta_arch_api::context::ExecutionLevel::Kernel,
        )
    };
    let id = rq().alloc_id();
    let thread = Thread {
        id,
        state: ThreadState::Created,
        priority,
        context: ctx,
        entry: Some(entry),
        address_space,
        level: vivanta_arch_api::context::ExecutionLevel::Kernel,
        sleep_until: None,
        kernel_stack_pa: Some(stack_base),
    };
    register(thread);
    thread_set_state(id, ThreadState::Ready);
    id
}

// ---------------------------------------------------------------------------
// Context switch
// ---------------------------------------------------------------------------

pub fn yield_now() {
    let _guard = unsafe { vivanta_arch_api::interrupts::disable_interrupts() };

    let current_id = current_thread_id();
    let current_as = rq().get(current_id).map(|t| t.address_space).unwrap_or(0);

    let next_id = match rq().find_next_ready(current_id, true) {
        Some(id) => id,
        None => {
            return;
        }
    };

    if current_id == next_id {
        return;
    }

    // G4 running invariant: exactly one thread Running at any instant.
    // Transition order:
    //   current: Running -> Ready
    //   next:    Ready  -> Running   (BEFORE the context switch, so an EL0
    //            thread entered via eret_to_user_stub is already Running;
    //            the post-switch bookkeeping can no longer be skipped)
    thread_set_state(current_id, ThreadState::Ready);
    thread_set_state(next_id, ThreadState::Running);

    // Store the ThreadId, not a slot index (G4 §8).
    CURRENT_THREAD.store(next_id, Ordering::Relaxed);

    // Activate address space if different
    let next_as = rq().get(next_id).unwrap().address_space;

    if next_as != current_as {
        if cfg!(feature = "trace-address-space") {
            vivanta_boot_common::println!("  [AS switch] thread {} → {}", current_id, next_id);
        }
        let root = crate::vmm::address_space::lookup_root(next_as);
        unsafe {
            vivanta_arch_api::mmu::activate_address_space(root);
        }
    }

    // Perform context switch
    unsafe {
        vivanta_arch_api::context::context_switch(
            &mut rq().get_mut(current_id).unwrap().context,
            rq().get(next_id).unwrap().context,
        );
    }
    // After context switch we are running as `next`, which was already set
    // to Running before the switch.
}

pub fn schedule_tick() {
    check_sleeping_threads();
    NEED_RESCHEDULE.store(true, Ordering::Relaxed);
}

pub fn maybe_reschedule(_frame: usize) {
    if !NEED_RESCHEDULE.load(Ordering::Relaxed) {
        return;
    }
    NEED_RESCHEDULE.store(false, Ordering::Relaxed);
    yield_now();
}

// ---------------------------------------------------------------------------
// Thread lifecycle
// ---------------------------------------------------------------------------

/// Find the Task that owns the given thread, if any.
///
/// M6: threads are owned by Tasks (`Task.threads`); this is how the scheduler
/// links a terminating thread back to its process container.
pub fn task_for_thread(tid: ThreadId) -> Option<TaskId> {
    pt().iter()
        .find(|t| t.threads.contains(&tid))
        .map(|t| t.task_id)
}

extern "C" fn thread_trampoline(_arg: usize) {
    // First entry of a kernel thread: the previous thread's InterruptGuard is
    // still "held" across the context switch, so IRQs are disabled here.
    // Enable them so timer preemption can reschedule this thread (G4).
    unsafe {
        vivanta_arch_api::interrupts::enable_interrupts();
    }
    let current_id = current_thread_id();
    // M6: when a task's first thread starts executing, mark the task Running.
    if let Some(tid) = task_for_thread(current_id) {
        if let Some(task) = pt().lookup_mut(tid) {
            if task.state == TaskState::Created {
                task.state = TaskState::Running;
            }
        }
    }
    let entry = rq()
        .get(current_id)
        .and_then(|t| t.entry)
        .expect("trampoline: no entry");
    thread_set_state(current_id, ThreadState::Running);
    entry(0);
    thread_exit(0); // entry returned => normal completion
}

/// Called from the arch EL0 fault handler (G3 fault containment): terminate
/// the current task without returning to EL0. A fault is an abnormal exit
/// (convention: negative code).
#[unsafe(no_mangle)]
pub extern "Rust" fn user_fault_terminate() -> ! {
    thread_exit(-1)
}

/// Terminate the current thread and mark its owning Task as exited.
///
/// M6 (G6-A/G6-B): the process container learns the thread's exit code so
/// `Task::exit()` / `TaskState::Exited` become real, and the parent/monitor
/// can collect it on reap.
pub fn thread_exit(exit_code: i32) -> ! {
    let _guard = unsafe { vivanta_arch_api::interrupts::disable_interrupts() };

    // Snapshot the current ThreadId FIRST (ThreadId is stable across slot
    // reuse; never read it after cleanup may have removed/reused slots).
    let current_id = current_thread_id();
    let current_as = rq().get(current_id).unwrap().address_space;

    // M6 (G6-A/G6-B): notify the owning Task of this thread's exit code.
    // Only the LAST thread of a task transitions it to Exited; intermediate
    // threads just end (state stays Running until the last one).
    if let Some(tid) = task_for_thread(current_id) {
        let remaining = pt()
            .lookup(tid)
            .map(|t| {
                t.threads
                    .iter()
                    .filter(|th| **th != current_id && rq().get(**th).is_some())
                    .count()
            })
            .unwrap_or(0);
        if remaining == 0 {
            if let Some(task) = pt().lookup_mut(tid) {
                if task.state != TaskState::Exited {
                    task.exit(exit_code);
                    vivanta_boot_common::println!(
                        "  [task] Task {} -> Exited code={}",
                        tid,
                        exit_code
                    );
                }
            }
        }
    }

    // Remove previously terminated threads (never the current one — it is
    // still present; we mark it Terminated after cleanup).
    cleanup();

    thread_set_state(current_id, ThreadState::Terminated);

    // Find next thread to run
    let next_id = match rq().find_next_ready(current_id, false) {
        Some(id) => id,
        None => panic!("thread_exit: no thread to run"),
    };

    // G4 running invariant: next becomes Running BEFORE the switch so the
    // resumed thread (including an EL0 thread) is Running while executing.
    if next_id != current_id {
        thread_set_state(next_id, ThreadState::Running);
    }

    // Store the ThreadId, not a slot index.
    CURRENT_THREAD.store(next_id, Ordering::Relaxed);

    // Activate address space if different
    let next = rq().get(next_id).unwrap();
    if next.address_space != current_as {
        if cfg!(feature = "trace-address-space") {
            vivanta_boot_common::println!("  [AS switch] thread {} → {}", current_id, next_id);
        }
        let root = crate::vmm::address_space::lookup_root(next.address_space);
        unsafe {
            vivanta_arch_api::mmu::activate_address_space(root);
        }
    }

    // Perform context switch
    unsafe {
        vivanta_arch_api::context::context_switch(
            &mut rq().get_mut(current_id).unwrap().context,
            rq().get(next_id).unwrap().context,
        );
    }
    unreachable!()
}

fn cleanup() {
    // Remove terminated threads (except boot and idle)
    let boot_id = BOOT_THREAD_ID.load(Ordering::Relaxed);
    let idle_id = IDLE_THREAD_ID.load(Ordering::Relaxed);

    let to_remove: Vec<ThreadId> = rq()
        .iter()
        .filter(|t| t.state == ThreadState::Terminated && t.id != boot_id && t.id != idle_id)
        .map(|t| t.id)
        .collect();

    for id in to_remove {
        let removed = rq().remove(id);
        if let Some(t) = removed {
            // G2 reclamation: return the contiguous kernel stack frames to the
            // physical allocator. Boot/idle threads use static stacks (None).
            if let Some(pa) = t.kernel_stack_pa {
                unsafe {
                    if let Some(alloc) = STACK_ALLOCATOR {
                        let frames = KERNEL_STACK_SIZE / 4096;
                        for i in 0..frames {
                            (*alloc).free_frame(vivanta_arch_api::pmm::PhysFrame {
                                addr: pa + (i as u64) * 4096,
                            });
                        }
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Initialization
// ---------------------------------------------------------------------------

static mut IDLE_STACK: [u8; KERNEL_STACK_SIZE] = [0; KERNEL_STACK_SIZE];

pub fn init_boot() {
    let kernel_as = crate::vmm::KERNEL_ADDRESS_SPACE_ID;
    unsafe {
        let boot_ctx = vivanta_arch_api::context::context_capture_current();
        let boot_id = rq().alloc_id();

        rq().insert(Thread {
            id: boot_id,
            state: ThreadState::Running,
            priority: Priority::Normal,
            context: boot_ctx,
            entry: None,
            address_space: kernel_as,
            level: vivanta_arch_api::context::ExecutionLevel::Kernel,
            sleep_until: None,
            kernel_stack_pa: None, // static boot stack, never freed
        })
        .expect("Failed to insert boot thread");

        BOOT_THREAD_ID.store(boot_id, Ordering::Relaxed);
        CURRENT_THREAD.store(boot_id, Ordering::Relaxed);

        // Create idle thread
        let idle_id = rq().alloc_id();
        let idle_bottom = &mut IDLE_STACK[0] as *mut u8 as usize;
        let idle_top = idle_bottom + KERNEL_STACK_SIZE;
        let idle_ctx = vivanta_arch_api::context::context_init(
            idle_top,
            idle_bottom,
            0, // user_stack_top — idle thread never enters EL0
            0,
            vivanta_arch_api::context::ExecutionLevel::Kernel,
        );

        rq().insert(Thread {
            id: idle_id,
            state: ThreadState::Ready,
            priority: Priority::Idle,
            context: idle_ctx,
            entry: None,
            address_space: kernel_as,
            level: vivanta_arch_api::context::ExecutionLevel::Kernel,
            sleep_until: None,
            kernel_stack_pa: None, // static idle stack, never freed
        })
        .expect("Failed to insert idle thread");

        IDLE_THREAD_ID.store(idle_id, Ordering::Relaxed);
    }
}

// ---------------------------------------------------------------------------
// External entry points
// ---------------------------------------------------------------------------

/// Called from the arch timer handler via the arch-api boundary.
/// Body is fully safe: it only touches kernel-internal scheduling state.
#[unsafe(no_mangle)]
pub extern "Rust" fn scheduler_tick() {
    schedule_tick();
}

/// Called from the arch IRQ dispatcher via the arch-api boundary.
/// The raw frame handle is inspected, never dereferenced here.
#[unsafe(no_mangle)]
pub extern "Rust" fn scheduler_reschedule(frame: usize) {
    maybe_reschedule(frame);
}

/// Called from kernel_main via vivanta_arch_api::boot::sched::sched_init_boot().
#[unsafe(no_mangle)]
pub extern "Rust" fn sched_init_boot() {
    init_boot();
}
