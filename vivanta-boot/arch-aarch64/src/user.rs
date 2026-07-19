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

core::arch::global_asm!(
    ".global eret_to_user_stub",
    ".balign 16",
    "eret_to_user_stub:",
    // SP_EL1 currently = kernel_stack_top (set by context_switch_asm).
    // Adjust SP to point to the synthetic frame base.
    "sub   sp, sp, #(34 * 8)",
    // Load system registers using x0 as temp, then restore x0 from frame.
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
    "eret",
);

// ---------------------------------------------------------------------------
// Stage 6A — Minimal EL0 bootstrap (AArch64)
// ---------------------------------------------------------------------------

use core::sync::atomic::{AtomicBool, Ordering};
use crate::exceptions::ExceptionFrame;
use crate::mmu::PageFlags;


/// Set to true when the SVC handler is called from EL0.
pub static EL0_SVC_HANDLED: AtomicBool = AtomicBool::new(false);

// User code — placed in .user.text section
// M4.5.1: two SVC calls to prove EL0→EL1→EL0 roundtrip
core::arch::global_asm!(
    ".section .user.text, \"ax\"",
    ".global user_code_start",
    "user_code_start:",
    "mov x0, #42",
    "svc #0",
    "mov x0, #43",
    "svc #0",
    ".global user_code_end",
    "user_code_end:",
    "b .",
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

        pt.map(CODE_VA, code_phys, 4096, PageFlags::USER_READ_WRITE_EXEC);

        let stack_phys = pt.alloc_frame().expect("user stack page").addr;
        vivanta_boot_common::println!("  User stack: PA=0x{:x}, VA=0x{:x}", stack_phys, STACK_VA);
        pt.map(STACK_VA, stack_phys, 4096, PageFlags::USER_READ_WRITE);

        UserBootstrap { code_va: CODE_VA, stack_va: STACK_VA, entry: CODE_VA }
    }

}

// ---------------------------------------------------------------------------
// SVC handler — called from exception vector for lower_aarch64_sync
// ---------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn el0_sync_handler(
    frame: &mut ExceptionFrame,
    _kind: u64,
    esr: u64,
    _far: u64,
) {
    EL0_SVC_HANDLED.store(true, Ordering::Relaxed);
    let _svc_num = esr & 0xFFFF;
    let val = frame.x[0];
    vivanta_boot_common::println!("  SVC from EL0: x0={}", val);
    frame.elr += 4;
}
