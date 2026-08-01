// ---------------------------------------------------------------------------
// vivanta_kernel scheduler — Thread lifecycle, RunQueue, scheduling policy
// ---------------------------------------------------------------------------

pub mod task;
pub mod task_manager;
pub mod thread;
pub mod runqueue;

use thread::{Thread, ThreadState, ThreadEntry, ThreadId, Priority};
use runqueue::{RunQueue, RunQueueError};
use vivanta_arch_api::pmm::FrameAllocator;
use crate::vmm::AddressSpaceId;
use alloc::vec::Vec;

const KERNEL_STACK_SIZE: usize = 16384;

static mut RUNQUEUE: Option<RunQueue> = None;
static mut BOOT_THREAD_ID: ThreadId = 0;
static mut IDLE_THREAD_ID: ThreadId = 0;

use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
static NEED_RESCHEDULE: AtomicBool = AtomicBool::new(false);
static ATOMIC_CURRENT: AtomicUsize = AtomicUsize::new(0);

// ---------------------------------------------------------------------------
// RunQueue accessor helpers (unsafe because static mut)
// ---------------------------------------------------------------------------

fn rq() -> &'static mut RunQueue {
    unsafe { 
        if RUNQUEUE.is_none() {
            RUNQUEUE = Some(RunQueue::new());
        }
        RUNQUEUE.as_mut().unwrap()
    }
}

// ---------------------------------------------------------------------------
// Scheduler API
// ---------------------------------------------------------------------------

pub fn thread_set_state(id: ThreadId, new_state: ThreadState) {
    rq().set_state(id, new_state)
        .expect("thread_set_state failed");
}

pub fn register(thread: Thread) {
    rq().insert(thread).expect("runqueue full");
}

pub fn create_user_thread(
    kernel_stack_top: usize,
    user_stack_top: usize,
    entry: usize,
    address_space: AddressSpaceId,
    priority: Priority,
) -> ThreadId {
    let ctx = unsafe {
        vivanta_arch_api::context::context_init(
            kernel_stack_top,
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
    let stack_base = alloc.alloc_frame().expect("stack frame 0").addr;
    for _ in 1..4 {
        alloc.alloc_frame().expect("stack frame");
    }
    let stack_top = (stack_base as usize) + KERNEL_STACK_SIZE;
    let ctx = unsafe {
        vivanta_arch_api::context::context_init(
            stack_top,
            0,  // user_stack_top — vivanta_kernel threads don't use EL0
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
    
    let current_id = unsafe { 
        let idx = ATOMIC_CURRENT.load(Ordering::Relaxed);
        rq().iter().nth(idx).map(|t| t.id).unwrap_or(0)
    };
    
    let next_id = match rq().find_next_ready(current_id, true) {
        Some(id) => id,
        None => return, // No other thread to run
    };
    
    if current_id == next_id {
        return;
    }
    
    // Set current thread to Ready
    thread_set_state(current_id, ThreadState::Ready);
    
    // Update current index
    let next_idx = rq().iter().position(|t| t.id == next_id).unwrap_or(0);
    ATOMIC_CURRENT.store(next_idx, Ordering::Relaxed);
    
    // Activate address space if different
    let next = rq().get(next_id).unwrap();
    let current = rq().get(current_id).unwrap();
    
    if next.address_space != current.address_space {
        if cfg!(feature = "trace-address-space") {
            vivanta_boot_common::println!("  [AS switch] thread {} → {}", current_id, next_id);
        }
        let root = crate::vmm::address_space::lookup_root(next.address_space);
        unsafe { vivanta_arch_api::mmu::activate_address_space(root); }
    }
    
    // Perform context switch
    unsafe {
        vivanta_arch_api::context::context_switch(
            &mut rq().get_mut(current_id).unwrap().context,
            rq().get(next_id).unwrap().context,
        );
    }
    
    // After context switch, we are now running as 'next'
    let now_idx = ATOMIC_CURRENT.load(Ordering::Relaxed);
    if let Some(t) = rq().iter().nth(now_idx) {
        if t.id != unsafe { IDLE_THREAD_ID } {
            thread_set_state(t.id, ThreadState::Running);
        }
    }
}

pub fn schedule_tick() {
    NEED_RESCHEDULE.store(true, Ordering::Relaxed);
}

pub fn maybe_reschedule(_frame: usize) {
    // M4.4.5 (ADR-017): _frame belongs to the interrupted execution context.
    // Scheduler never copies or owns this frame.
    // Context switch changes SP_EL1 via context_switch() — the same
    // mechanism used by yield_now(). The frame parameter provides access
    // to exception state for inspection only, not for copying.

    if !NEED_RESCHEDULE.load(Ordering::Relaxed) { return; }
    NEED_RESCHEDULE.store(false, Ordering::Relaxed);

    yield_now();
}

// ---------------------------------------------------------------------------
// Thread lifecycle
// ---------------------------------------------------------------------------

extern "C" fn thread_trampoline(_arg: usize) {
    let current_id = unsafe { 
        let idx = ATOMIC_CURRENT.load(Ordering::Relaxed);
        rq().iter().nth(idx).map(|t| t.id).unwrap_or(0)
    };
    let entry = rq().get(current_id)
        .and_then(|t| t.entry)
        .expect("trampoline: no entry");
    thread_set_state(current_id, ThreadState::Running);
    entry(0);
    thread_exit();
}

pub fn thread_exit() -> ! {
    let _guard = unsafe { vivanta_arch_api::interrupts::disable_interrupts() };
    cleanup();
    
    let current_id = unsafe { 
        let idx = ATOMIC_CURRENT.load(Ordering::Relaxed);
        rq().iter().nth(idx).map(|t| t.id).unwrap_or(0)
    };
    let current_as = rq().get(current_id).unwrap().address_space;
    
    thread_set_state(current_id, ThreadState::Terminated);
    
    // Find next thread to run
    let next_id = match rq().find_next_ready(current_id, false) {
        Some(id) => id,
        None => panic!("thread_exit: no thread to run"),
    };
    
    // Update current index
    let next_idx = rq().iter().position(|t| t.id == next_id).unwrap_or(0);
    ATOMIC_CURRENT.store(next_idx, Ordering::Relaxed);
    
    // Activate address space if different
    let next = rq().get(next_id).unwrap();
    if next.address_space != current_as {
        if cfg!(feature = "trace-address-space") {
            vivanta_boot_common::println!("  [AS switch] thread {} → {}", current_id, next_id);
        }
        let root = crate::vmm::address_space::lookup_root(next.address_space);
        unsafe { vivanta_arch_api::mmu::activate_address_space(root); }
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
    let boot_id = unsafe { BOOT_THREAD_ID };
    let idle_id = unsafe { IDLE_THREAD_ID };
    
    let to_remove: Vec<ThreadId> = rq().iter()
        .filter(|t| t.state == ThreadState::Terminated && t.id != boot_id && t.id != idle_id)
        .map(|t| t.id)
        .collect();
    
    for id in to_remove {
        rq().remove(id);
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
        }).expect("Failed to insert boot thread");
        
        BOOT_THREAD_ID = boot_id;
        ATOMIC_CURRENT.store(0, Ordering::Relaxed);
        
        // Create idle thread
        let idle_id = rq().alloc_id();
        let idle_top = (&mut IDLE_STACK[0] as *mut u8 as usize) + KERNEL_STACK_SIZE;
        let idle_ctx = vivanta_arch_api::context::context_init(
            idle_top,
            0,  // user_stack_top — idle thread never enters EL0
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
        }).expect("Failed to insert idle thread");
        
        IDLE_THREAD_ID = idle_id;
    }
}

// ---------------------------------------------------------------------------
// External entry points
// ---------------------------------------------------------------------------

/// Called from the arch timer handler via extern "Rust"
#[no_mangle]
pub unsafe extern "Rust" fn scheduler_tick() {
    schedule_tick();
}

/// Called from the arch IRQ dispatcher via extern "Rust"
#[no_mangle]
pub unsafe extern "Rust" fn scheduler_reschedule(frame: usize) {
    maybe_reschedule(frame);
}

/// Called from kernel_main via vivanta_arch_api::boot::sched::sched_init_boot()
#[no_mangle]
pub unsafe extern "Rust" fn sched_init_boot() {
    init_boot();
}
