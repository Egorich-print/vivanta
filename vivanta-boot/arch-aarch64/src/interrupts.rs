// ---------------------------------------------------------------------------
// AArch64 interrupt subsystem — GICv2/v3 + IRQ dispatch
// ---------------------------------------------------------------------------

pub mod gic;
pub mod dispatcher;

pub use dispatcher::{register_irq, IrqHandler};
pub use gic::{Gic, GicVersion};

use crate::barrier;

/// Enable IRQs at the CPU level (clear PSTATE.I bit).
/// QEMU 11.0.2 quirk: DAIFClr immediate encoding is shifted left by 6,
/// so #2 clears bit 7 (I) rather than #4 per ARM spec.
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
    unsafe { core::arch::asm!("msr daif, {}", in(reg) daif, options(nostack)); }
}

#[no_mangle]
pub extern "Rust" fn disable_interrupts() -> vivanta_arch_api::interrupts::InterruptGuard {
    let saved: u64;
    unsafe {
        core::arch::asm!("mrs {}, daif", out(reg) saved, options(nostack));
        core::arch::asm!("msr DAIFSet, #2", options(nostack));
    }
    vivanta_arch_api::interrupts::InterruptGuard::new(saved as usize, restore_interrupts)
}

#[no_mangle]
pub extern "Rust" fn enable_interrupts() {
    enable();
}
