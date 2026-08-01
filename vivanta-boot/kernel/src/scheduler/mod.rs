// ---------------------------------------------------------------------------
// vivanta_kernel scheduler — Thread lifecycle, RunQueue, scheduling policy
// ---------------------------------------------------------------------------

pub mod task;
pub mod task_manager;
pub mod thread;
pub mod runqueue;

use thread::{Thread, ThreadState, ThreadEntry};
use vivanta_arch_api::pmm::FrameAllocator;
use crate::vmm::AddressSpaceId;

const MAX_THREADS: usize = 8;
const IDLE_SLOT: usize = MAX_THREADS - 1;
const KERNEL_STACK_SIZE: usize = 16384;

static mut NEXT_ID: u64 = 1;
static mut RUNQUEUE: [Option<Thread>; MAX_THREADS] = [
    None, None, None, None, None, None, None, None,
];

use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
static NEED_RESCHEDULE: AtomicBool = AtomicBool::new(false);
static ATOMIC_CURRENT: AtomicUsize = AtomicUsize::new(0);

fn find_next_ready(from: usize) -> usize {
    let n = MAX_THREADS;
    for i in 1..n {
        let idx = (from + i) % n;
        if idx == IDLE_SLOT { continue; }
        let slot = unsafe { &raw const RUNQUEUE[idx] };
        if let Some(ref t) = unsafe { (*slot).as_ref() } {
            if t.state == ThreadState::Ready { return idx; }
        }
    }
    let idle_slot = unsafe { &raw const RUNQUEUE[IDLE_SLOT] };
    if let Some(ref t) = unsafe { (*idle_slot).as_ref() } {
        if t.state == ThreadState::Ready { return IDLE_SLOT; }
    }
    panic!("find_next_ready: no Ready thread");
}

fn runqueue_ref(idx: usize) -> &'static Thread {
    let ptr = unsafe { &raw const RUNQUEUE[idx] };
    unsafe { (*ptr).as_ref().expect("no thread at index") }
}

fn runqueue_mut(idx: usize) -> &'static mut Thread {
    let ptr = unsafe { &raw mut RUNQUEUE[idx] };
    unsafe { (*ptr).as_mut().expect("no thread at index") }
}

pub fn register(thread: Thread) {
    unsafe {
        for i in 0..MAX_THREADS {
            let ptr = &raw mut RUNQUEUE[i];
            if (*ptr).is_none() {
                *ptr = Some(thread);
                return;
            }
        }
    }
    panic!("runqueue full");
}

pub fn create_user_thread(
    kernel_stack_top: usize,
    user_stack_top: usize,
    entry: usize,
    address_space: AddressSpaceId,
) -> u64 {
    unsafe {
        let ctx = vivanta_arch_api::context::context_init(
            kernel_stack_top,
            user_stack_top,
            entry,
            vivanta_arch_api::context::ExecutionLevel::User,
        );
        let id = NEXT_ID;
        NEXT_ID += 1;
        let thread = Thread {
            id,
            state: ThreadState::Ready,
            context: ctx,
            entry: None,
            address_space,
            level: vivanta_arch_api::context::ExecutionLevel::User,
        };
        register(thread);
        id
    }
}

pub fn create_kernel_thread(
    entry: ThreadEntry,
    _arg: usize,
    alloc: &mut impl FrameAllocator,
    address_space: AddressSpaceId,
) -> u64 {
    unsafe {
        let stack_base = alloc.alloc_frame().expect("stack frame 0").addr;
        for _ in 1..4 {
            alloc.alloc_frame().expect("stack frame");
        }
        let stack_top = (stack_base as usize) + KERNEL_STACK_SIZE;
        let ctx = vivanta_arch_api::context::context_init(
            stack_top,
            0,  // user_stack_top — vivanta_kernel threads don't use EL0
            thread_trampoline as *const () as usize,
            vivanta_arch_api::context::ExecutionLevel::Kernel,
        );
        let id = NEXT_ID;
        NEXT_ID += 1;
        let thread = Thread {
            id,
            state: ThreadState::Ready,
            context: ctx,
            entry: Some(entry),
            address_space,
            level: vivanta_arch_api::context::ExecutionLevel::Kernel,
        };
        register(thread);
        id
    }
}

pub fn yield_now() {
    let _guard = unsafe { vivanta_arch_api::interrupts::disable_interrupts() };
    unsafe {
        let cur = ATOMIC_CURRENT.load(Ordering::Relaxed);
        let nxt = find_next_ready(cur);
        if nxt == cur { return; }

        let current = runqueue_mut(cur);
        let next = runqueue_ref(nxt);

        current.state = ThreadState::Ready;
        ATOMIC_CURRENT.store(nxt, Ordering::Relaxed);

        // Activate the next thread's address space if different
        if next.address_space != current.address_space {
            if cfg!(feature = "trace-address-space") {
                vivanta_boot_common::println!("  [AS switch] thread {} → {}", current.id, next.id);
            }
            let root = crate::vmm::address_space::lookup_root(next.address_space);
            vivanta_arch_api::mmu::activate_address_space(root);
        }

        vivanta_arch_api::context::context_switch(
            &mut current.context,
            next.context,
        );

        let now = ATOMIC_CURRENT.load(Ordering::Relaxed);
        if now != IDLE_SLOT {
            runqueue_mut(now).state = ThreadState::Running;
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

    let cur = ATOMIC_CURRENT.load(Ordering::Relaxed);
    let nxt = find_next_ready(cur);
    if nxt == cur { return; }

    let current = runqueue_mut(cur);
    let next = runqueue_ref(nxt);

    current.state = ThreadState::Ready;
    ATOMIC_CURRENT.store(nxt, Ordering::Relaxed);

    if next.address_space != current.address_space {
        let root = crate::vmm::address_space::lookup_root(next.address_space);
        unsafe { vivanta_arch_api::mmu::activate_address_space(root); }
    }

    unsafe {
        vivanta_arch_api::context::context_switch(
            &mut runqueue_mut(cur).context,
            runqueue_ref(nxt).context,
        );
    }
}

pub fn cleanup() {
    unsafe {
        for i in 0..MAX_THREADS {
            if i == 0 || i == IDLE_SLOT { continue; }
            let ptr = &raw mut RUNQUEUE[i];
            if let Some(ref t) = *ptr {
                if t.state == ThreadState::Terminated {
                    *ptr = None;
                }
            }
        }
    }
}

pub fn thread_exit() -> ! {
    let _guard = unsafe { vivanta_arch_api::interrupts::disable_interrupts() };
    cleanup();
    let cur = ATOMIC_CURRENT.load(Ordering::Relaxed);
    let cur_as = runqueue_mut(cur).address_space;
    runqueue_mut(cur).state = ThreadState::Terminated;

    let nxt = find_next_ready(cur);

    let next = runqueue_ref(nxt);
    let next_ctx = next.context;
    ATOMIC_CURRENT.store(nxt, Ordering::Relaxed);

    // Activate the next thread's address space if different
    if next.address_space != cur_as {
        if cfg!(feature = "trace-address-space") {
            vivanta_boot_common::println!("  [AS switch] thread {} → {}", cur, nxt);
        }
        let root = crate::vmm::address_space::lookup_root(next.address_space);
        unsafe { vivanta_arch_api::mmu::activate_address_space(root); }
    }

    unsafe {
        vivanta_arch_api::context::context_switch(
            &mut runqueue_mut(cur).context,
            next_ctx,
        );
    }
    unreachable!()
}

extern "C" fn thread_trampoline(_arg: usize) {
    let cur = ATOMIC_CURRENT.load(Ordering::Relaxed);
    let entry = {
        let t = runqueue_ref(cur);
        t.entry.expect("trampoline: no entry")
    };
    runqueue_mut(cur).state = ThreadState::Running;
    entry(0);
    thread_exit();
}

static mut IDLE_STACK: [u8; KERNEL_STACK_SIZE] = [0; KERNEL_STACK_SIZE];

pub fn init_boot() {
    let kernel_as = crate::vmm::KERNEL_ADDRESS_SPACE_ID;
    unsafe {
        let boot_ctx = vivanta_arch_api::context::context_capture_current();
        
        let ptr = &raw mut RUNQUEUE[0];
        *ptr = Some(Thread {
            id: 0,
            state: ThreadState::Running,
            context: boot_ctx,
            entry: None,
            address_space: kernel_as,
            level: vivanta_arch_api::context::ExecutionLevel::Kernel,
        });

        let idle_ptr = &raw mut RUNQUEUE[IDLE_SLOT];
        let idle_top = (&mut IDLE_STACK[0] as *mut u8 as usize) + KERNEL_STACK_SIZE;
        let idle_ctx = vivanta_arch_api::context::context_init(
            idle_top,
            0,  // user_stack_top — idle thread never enters EL0
            0,
            vivanta_arch_api::context::ExecutionLevel::Kernel,
        );
        *idle_ptr = Some(Thread {
            id: IDLE_SLOT as u64,
            state: ThreadState::Ready,
            context: idle_ctx,
            entry: None,
            address_space: kernel_as,
            level: vivanta_arch_api::context::ExecutionLevel::Kernel,
        });
    }
}

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
