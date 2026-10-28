// ---------------------------------------------------------------------------
// AArch64 interrupt subsystem — GICv2/v3 + IRQ dispatch
// ---------------------------------------------------------------------------

pub mod dispatcher;
pub mod gic;

pub use dispatcher::{IrqHandler, register_irq};
pub use gic::{Gic, GicVersion};

use crate::barrier;

/// Enable IRQs at the CPU level (clear PSTATE.I bit).
///
/// The `DAIFSet`/`DAIFClr` `imm4` field order is bit3=D (Debug),
/// bit2=A (SError), bit1=I (IRQ), bit0=F (FIQ). PSTATE.I is bit 7, mirrored as
/// bit 1 of the `imm4` field. So `msr DAIFSet, #2` sets the IRQ mask (I) and
/// `msr DAIFClr, #2` clears it: IRQ masking here is correct. `#4` would set the
/// SError mask (A) instead. Verified empirically on QEMU by reading DAIF after
/// each instruction; see docs/audit/2026-09-24-global-audit.md and
/// docs/planning/mission-2026-10/40-silicon-blockers.md.
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
