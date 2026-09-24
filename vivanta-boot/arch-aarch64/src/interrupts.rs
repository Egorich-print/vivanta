// ---------------------------------------------------------------------------
// AArch64 interrupt subsystem — GICv2/v3 + IRQ dispatch
// ---------------------------------------------------------------------------

pub mod dispatcher;
pub mod gic;

pub use dispatcher::{IrqHandler, register_irq};
pub use gic::{Gic, GicVersion};

use crate::barrier;

/// Enable IRQs at the CPU level (clear PSTATE.I bit).
/// DAIF bit layout: bit0=D, bit1=A (SError), bit2=I (IRQ), bit3=F.
///
/// NOTE: `#2` masks SError, not IRQ — architecturally IRQ masking needs `#4`.
/// Verified on QEMU: switching this and `disable_interrupts()` to `#4` turns the
/// 100 Hz preemption path into a timer storm (PREEMPT counter explodes, no
/// reschedule), so the verified target keeps `#2`. IRQ masking on real silicon
/// is therefore UNPROVEN and is a hardware-validation blocker — see
/// `docs/audit/2026-09-24-global-audit.md`.
pub fn enable() {
    unsafe {
        core::arch::asm!("msr DAIFClr, #2", options(nostack));
    }
    barrier::isb();
}

// ---------------------------------------------------------------------------
// extern "Rust" implementations for arch-api::interrupts
// ---------------------------------------------------------------------------

/// Restore exact DAIF state (used by InterruptGuard::drop via fn pointer).
fn restore_interrupts(daif: usize) {
    unsafe {
        core::arch::asm!("msr daif, {}", in(reg) daif, options(nostack));
    }
}

#[unsafe(no_mangle)]
pub extern "Rust" fn disable_interrupts() -> vivanta_arch_api::interrupts::InterruptGuard {
    let saved: u64;
    unsafe {
        core::arch::asm!("mrs {}, daif", out(reg) saved, options(nostack));
        core::arch::asm!("msr DAIFSet, #2", options(nostack));
    }
    vivanta_arch_api::interrupts::InterruptGuard::new(saved as usize, restore_interrupts)
}

#[unsafe(no_mangle)]
pub extern "Rust" fn enable_interrupts() {
    enable();
}
