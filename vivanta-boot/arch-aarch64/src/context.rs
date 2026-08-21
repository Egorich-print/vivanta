// ---------------------------------------------------------------------------
// AArch64 context implementation — unified context switch
//
// M4.4.5 (ADR-017):
//   - Single context_switch() replaces context_switch_coop/preempt.
//   - ExceptionFrame is never copied between thread stacks.
//   - ExecutionLevel determines SPSR at thread creation.
//
// INV-002 fix: the per-thread ThreadContext lives at the BOTTOM of the kernel
// stack (stack_bottom). The stack grows down from stack_top, so the saved
// context can never be clobbered by stack usage or by exception frames pushed
// onto a shallow stack. The synthetic ExceptionFrame (used only by
// eret_to_user_stub on first entry to EL0) stays at stack_top - FRAME_SIZE.
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

#[repr(C)]
struct ThreadContext {
    x19_x30: [u64; 12], // x19 through x30
    sp: u64,
}

/// Boot thread's context block (static, not on any stack).
/// ArchContext points directly at `thread_ctx`.
#[repr(C, align(16))]
struct BootThreadBlock {
    thread_ctx: ThreadContext,
    frame: [u8; FRAME_SIZE],
}

static mut BOOT_BLOCK: BootThreadBlock = BootThreadBlock {
    thread_ctx: ThreadContext {
        x19_x30: [0; 12],
        sp: 0,
    },
    frame: [0; FRAME_SIZE],
};

unsafe extern "C" {
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
// context_init — create ThreadContext (bottom) + synthetic frame (top)
// ---------------------------------------------------------------------------

// Reference to the EL1→EL0 trampoline defined in the user module.
unsafe extern "C" {
    static eret_to_user_stub: u8;
}

#[unsafe(no_mangle)]
pub unsafe extern "Rust" fn context_init(
    stack_top: usize,
    stack_bottom: usize,
    user_stack_top: usize,
    entry: usize,
    level: ExecutionLevel,
) -> ArchContext {
    unsafe {
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
            ExecutionLevel::User => &raw const eret_to_user_stub as *const u8 as u64,
        };

        // SP_EL0 for user threads (0 for vivanta_kernel — unused in EL1h)
        let sp_el0 = match level {
            ExecutionLevel::Kernel => 0u64,
            ExecutionLevel::User => user_stack_top as u64,
        };

        // ThreadContext lives at the BOTTOM of the kernel stack region.
        let tc = stack_bottom as *mut ThreadContext;
        core::ptr::write_bytes(tc as *mut u8, 0, core::mem::size_of::<ThreadContext>());
        (*tc).x19_x30[11] = entry_x30; // x30 = trampoline or stub
        (*tc).sp = stack_top as u64; // SP_EL1 = vivanta_kernel stack top

        // Synthetic initial frame: needed only for user threads (eret_to_user_stub
        // reads it at [SP_EL1 - FRAME_SIZE, SP_EL1) on first entry to EL0).
        if level == ExecutionLevel::User {
            let frame_loc = (stack_top - FRAME_SIZE) as *mut ExceptionFrame;
            let x = [0u64; 31];
            frame_loc.write(ExceptionFrame {
                x,
                sp: sp_el0,
                elr: actual_entry as u64,
                spsr,
            });
        }

        ArchContext::from_raw(tc as usize)
    }
}

// ---------------------------------------------------------------------------
// context_capture_current — boot thread
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "Rust" fn context_capture_current() -> ArchContext {
    unsafe { ArchContext::from_raw(&raw mut BOOT_BLOCK.thread_ctx as *mut ThreadContext as usize) }
}

// ---------------------------------------------------------------------------
// context_switch — unified context switch
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "Rust" fn context_switch(old: *mut ArchContext, new: ArchContext) {
    unsafe {
        let old_tc = (*old).as_raw() as *mut ThreadContext;
        let new_tc = new.as_raw() as *const ThreadContext;
        context_switch_asm(old_tc, new_tc)
    }
}
