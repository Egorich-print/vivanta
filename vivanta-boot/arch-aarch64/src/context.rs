// ---------------------------------------------------------------------------
// AArch64 context implementation — unified context switch
//
// M4.4.5 (ADR-017):
//   - Single context_switch() replaces context_switch_coop/preempt.
//   - ExceptionFrame is never copied between thread stacks.
//   - ExecutionLevel determines SPSR at thread creation.
// ---------------------------------------------------------------------------

use crate::exceptions::ExceptionFrame;
use vivanta_arch_api::context::{ArchContext, ExecutionLevel};

/// Byte size of an ExceptionFrame (34 × 8 bytes = 272).
const FRAME_SIZE: usize = core::mem::size_of::<ExceptionFrame>();

// Compile-time layout checks
const _: () = assert!(core::mem::size_of::<ExceptionFrame>() == 34 * 8);
const _: () = assert!(core::mem::align_of::<ExceptionFrame>() == 8);
const _: () = assert!(core::mem::size_of::<ThreadContext>() == 104);
const _: () = assert!(core::mem::align_of::<ThreadContext>() == 8);

/// Combined block: ThreadContext followed by ExceptionFrame frame area.
/// Layout matches per-stack layout so tc_loc() on either kind returns a
/// valid ThreadContext address.
#[repr(C, align(16))]
struct BootThreadBlock {
    thread_ctx: ThreadContext,
    frame: [u8; FRAME_SIZE],
}

/// Boot thread's context block.
/// `tc_loc(&raw mut BOOT_BLOCK.frame as usize) == &raw mut BOOT_BLOCK.thread_ctx`
static mut BOOT_BLOCK: BootThreadBlock = BootThreadBlock {
    thread_ctx: ThreadContext {
        x19_x30: [0; 12],
        sp: 0,
    },
    frame: [0; FRAME_SIZE],
};

// ---------------------------------------------------------------------------
// Per-thread context layout (on the vivanta_kernel stack):
//   [Synthetic Initial Frame]   ← ArchContext points here (ExceptionFrame layout)
//   [ThreadContext]             ← below, for save/restore
// ---------------------------------------------------------------------------

#[repr(C)]
struct ThreadContext {
    x19_x30: [u64; 12], // x19 through x30
    sp: u64,
}

/// Compute pointer to ThreadContext below a synthetic frame.
/// arch_arch_raw is the raw usize from ArchContext::as_raw().
fn tc_loc(raw: usize) -> *mut ThreadContext {
    (raw - core::mem::size_of::<ThreadContext>()) as *mut ThreadContext
}

extern "C" {
    fn context_switch_asm(current: *mut ThreadContext, next: *const ThreadContext);
}

// ---------------------------------------------------------------------------
// idle_entry — arch-specific WFI loop
// ---------------------------------------------------------------------------

pub fn idle_entry() -> ! {
    loop {
        unsafe {
            core::arch::asm!("wfi", options(nostack));
        }
    }
}

// ---------------------------------------------------------------------------
// context_init — create synthetic initial frame + ThreadContext
// ---------------------------------------------------------------------------

// Reference to the EL1→EL0 trampoline defined in the user module.
extern "C" {
    static eret_to_user_stub: u8;
}

#[no_mangle]
pub unsafe extern "Rust" fn context_init(
    stack_top: usize,
    user_stack_top: usize,
    entry: usize,
    level: ExecutionLevel,
) -> ArchContext {
    let actual_entry = if entry == 0 {
        idle_entry as *const () as usize
    } else {
        entry
    };

    let spsr = match level {
        ExecutionLevel::Kernel => 0x345u64, // EL1h, DAIF masked
        ExecutionLevel::User => 0x000u64,   // EL0t
    };

    // x30 for context_switch_asm ret:
    //   Kernel → entry (thread_trampoline calls the real entry)
    //   User   → eret_to_user_stub (transitions to EL0)
    let entry_x30: u64 = match level {
        ExecutionLevel::Kernel => actual_entry as u64,
        ExecutionLevel::User => &eret_to_user_stub as *const u8 as u64,
    };

    // SP_EL0 for user threads (0 for vivanta_kernel — unused in EL1h)
    let sp_el0 = match level {
        ExecutionLevel::Kernel => 0u64,
        ExecutionLevel::User => user_stack_top as u64,
    };

    let frame_loc = (stack_top - FRAME_SIZE) as *mut ExceptionFrame;
    let x = [0u64; 31];
    frame_loc.write(ExceptionFrame {
        x,
        sp: sp_el0,
        elr: actual_entry as u64,
        spsr,
    });

    let tc = tc_loc(frame_loc as usize);
    core::ptr::write_bytes(tc as *mut u8, 0, core::mem::size_of::<ThreadContext>());
    (*tc).x19_x30[11] = entry_x30; // x30 = trampoline or stub
    (*tc).sp = stack_top as u64; // SP_EL1 = vivanta_kernel stack top

    ArchContext::from_raw(frame_loc as usize)
}

// ---------------------------------------------------------------------------
// context_capture_current — boot thread
// ---------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "Rust" fn context_capture_current() -> ArchContext {
    ArchContext::from_raw(&raw mut BOOT_BLOCK.frame as *mut u8 as usize)
}

// ---------------------------------------------------------------------------
// context_switch — unified context switch
// ---------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "Rust" fn context_switch(old: *mut ArchContext, new: ArchContext) {
    let old_raw = (*old).as_raw();
    let new_raw = new.as_raw();
    context_switch_asm(tc_loc(old_raw), tc_loc(new_raw))
}
