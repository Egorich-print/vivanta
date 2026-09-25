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
/// IMPORTANT: `#2` masks SError, NOT IRQ — architecturally IRQ masking needs
/// `#4`, so every `InterruptGuard` here only serialises SError. Switching to
/// `#4` was measured and does NOT boot: with real IRQ masking the GIC stops
/// delivering the timer after ~3 ticks (measured: PSTATE.I clear, DAIF low
/// nibble 0, TICK_COUNT frozen), so the 100 Hz preemption path dies and a
/// preemption worker spins forever. That GIC delivery defect is the real
/// blocker for correct IRQ masking; see docs/audit/2026-09-24-global-audit.md.
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
