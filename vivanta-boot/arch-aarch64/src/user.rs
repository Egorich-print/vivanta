// ---------------------------------------------------------------------------
// EL1→EL0 trampoline — ADR-018 User Entry Transition Model
// ---------------------------------------------------------------------------
//
// eret_to_user_stub is the ONLY component allowed to transform a vivanta_kernel
// execution context into an EL0 execution context.
//
// Invariant (ADR-018 §6):
//   x30 for user threads is set to eret_to_user_stub by context_init.
//   context_switch_asm ret's into this stub, which reads the synthetic
//   ExceptionFrame at [SP_EL1 - 272, SP_EL1), loads SP_EL0 / ELR_EL1 /
//   SPSR_EL1, restores x0–x30, and eret's to EL0.
//
// The synthetic frame is created by context_init at thread creation time.
// After eret, SP_EL1 settles at kernel_stack_top - 272.
// ---------------------------------------------------------------------------

#[cfg(target_os = "none")]
core::arch::global_asm!(
    ".global eret_to_user_stub",
    ".balign 16",
    "eret_to_user_stub:",
    // SP_EL1 = kernel_stack_top (set by context_switch_asm)
    // Adjust SP to point to ExceptionFrame base
    "sub   sp, sp, #(34 * 8)",
    // Load system registers from ExceptionFrame
    "ldr   x0, [sp, #(31 * 8)]",
    "msr   sp_el0, x0",
    "ldr   x0, [sp, #(32 * 8)]",
    "msr   elr_el1, x0",
    "ldr   x0, [sp, #(33 * 8)]",
    "msr   spsr_el1, x0",
    // Restore x0–x29
    "ldp   x0, x1,  [sp, #(0 * 8)]",
    "ldp   x2, x3,  [sp, #(2 * 8)]",
    "ldp   x4, x5,  [sp, #(4 * 8)]",
    "ldp   x6, x7,  [sp, #(6 * 8)]",
    "ldp   x8, x9,  [sp, #(8 * 8)]",
    "ldp   x10,x11, [sp, #(10 * 8)]",
    "ldp   x12,x13, [sp, #(12 * 8)]",
    "ldp   x14,x15, [sp, #(14 * 8)]",
    "ldp   x16,x17, [sp, #(16 * 8)]",
    "ldp   x18,x19, [sp, #(18 * 8)]",
    "ldp   x20,x21, [sp, #(20 * 8)]",
    "ldp   x22,x23, [sp, #(22 * 8)]",
    "ldp   x24,x25, [sp, #(24 * 8)]",
    "ldp   x26,x27, [sp, #(26 * 8)]",
    "ldp   x28,x29, [sp, #(28 * 8)]",
    "ldr   x30,     [sp, #(30 * 8)]",
    "dsb   sy",
    "isb",
    "ic    iallu",
    "dsb   sy",
    "isb",
    "eret",
);

// ---------------------------------------------------------------------------
// Stage 6A — Minimal EL0 bootstrap (AArch64)
// ---------------------------------------------------------------------------

use crate::exceptions::ExceptionFrame;
use crate::mmu::PageFlags;

// User code — placed in .user.text section
#[cfg(target_os = "none")]
core::arch::global_asm!(
    ".section .user.text, \"ax\"",
    ".global user_code_start",
    "user_code_start:",
    // write(1, msg, 16)
    "mov  x8, #1",        // SYS_WRITE
    "mov  x0, #1",        // fd = stdout
    "adr  x1, hello_msg", // buf = message
    "mov  x2, #16",       // len
    "svc  #0",
    // exit(0)
    "mov  x8, #2", // SYS_EXIT
    "mov  x0, #0", // code = 0
    "svc  #0",
    "b .", // should not reach here
    // String data embedded in user text section
    ".balign 4",
    "hello_msg:",
    ".ascii \"Hello, Vivanta!\\n\"",
    ".global user_code_end",
    "user_code_end:",
);

extern "C" {
    static user_code_start: u8;
    static user_code_end: u8;
}

fn user_code_size() -> usize {
    unsafe { (&user_code_end as *const u8).offset_from(&user_code_start as *const u8) as usize }
}

fn user_code_src() -> *const u8 {
    unsafe { &user_code_start as *const u8 }
}

// ---------------------------------------------------------------------------
// UserBootstrap
// ---------------------------------------------------------------------------

pub struct UserBootstrap {
    pub code_va: u64,
    pub stack_va: u64,
    pub entry: u64,
}

impl UserBootstrap {
    /// Create user mappings in the page table.
    /// Must be called BEFORE pt.finish().
    pub fn create(
        pt: &mut crate::mmu::PageTableBuilder<impl vivanta_arch_api::pmm::FrameAllocator>,
    ) -> Self {
        const CODE_VA: u64 = 0x5E00_0000;
        const STACK_VA: u64 = 0x5E01_0000;

        let code_phys = pt.alloc_frame().expect("user code page").addr;
        vivanta_boot_common::println!("  User code: PA=0x{:x}, VA=0x{:x}", code_phys, CODE_VA);
        unsafe {
            let dst = code_phys as *mut u8;
            core::ptr::copy_nonoverlapping(user_code_src(), dst, user_code_size());
            if user_code_size() < 4096 {
                core::ptr::write_bytes(dst.add(user_code_size()), 0, 4096 - user_code_size());
            }
        }

        pt.map(CODE_VA, code_phys, 4096, PageFlags::USER_READ_EXEC);

        let stack_phys = pt.alloc_frame().expect("user stack page").addr;
        vivanta_boot_common::println!("  User stack: PA=0x{:x}, VA=0x{:x}", stack_phys, STACK_VA);
        pt.map(STACK_VA, stack_phys, 4096, PageFlags::USER_READ_WRITE);

        UserBootstrap {
            code_va: CODE_VA,
            stack_va: STACK_VA,
            entry: CODE_VA,
        }
    }
}

// ---------------------------------------------------------------------------
// SVC handler — called from exception vector for lower_aarch64_sync
// ---------------------------------------------------------------------------

extern "Rust" {
    fn syscall_dispatch(
        num: u64,
        arg0: u64,
        arg1: u64,
        arg2: u64,
        arg3: u64,
        arg4: u64,
        arg5: u64,
    ) -> u64;
}

#[no_mangle]
pub unsafe extern "C" fn el0_sync_handler(
    frame: &mut ExceptionFrame,
    _kind: u64,
    esr: u64,
    far: u64,
) {
    let ec = (esr >> 26) & 0x3f;
    if ec == 0b010101 {
        // SVC (AArch64) from EL0 — dispatch syscall.
        // ARM: for SVC, ELR_EL1 points to the instruction AFTER the SVC
        // (the SVC is architecturally executed), so we return it unchanged.
        let ret = syscall_dispatch(
            frame.x[8], frame.x[0], frame.x[1], frame.x[2], frame.x[3], frame.x[4], frame.x[5],
        );
        frame.x[0] = ret;
    } else {
        // G3 fault containment: any other synchronous EL0 exception (data
        // abort, undef, alignment, etc.) terminates the current task. We do
        // NOT skip the faulting instruction (`elr += 4`) — that would silently
        // mask faults. The kernel handles the fault as a task-fatal event.
        vivanta_boot_common::println!(
            "  EL0 fault: ESR={:#x} EC={} FAR={:#x} ELR={:#x} — terminating task",
            esr,
            ec,
            far,
            frame.elr
        );
        // Terminate the current task and switch to the next runnable thread.
        // This function does not return (context switch to another thread).
        user_fault_terminate();
    }
}

/// Kernel-provided hook: terminate the task that caused an EL0 fault.
///
/// Implemented by vivanta_kernel as `thread_exit`; never returns.
extern "Rust" {
    fn user_fault_terminate() -> !;
}
