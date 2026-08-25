// ---------------------------------------------------------------------------
// AArch64 Generic Timer — Non-secure Physical Timer (CNTP)
// ---------------------------------------------------------------------------

use crate::barrier;
use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};

const TICK_HZ: u64 = 100;

static TICK_COUNT: AtomicU64 = AtomicU64::new(0);
pub(crate) static TVAL: AtomicU32 = AtomicU32::new(0);

/// ARM Generic Timer frequency in Hz.
pub fn frequency() -> u64 {
    let freq: u64;
    unsafe {
        core::arch::asm!("mrs {0}, CNTFRQ_EL0", out(reg) freq, options(nostack));
    }
    freq
}

/// Current physical count value.
pub fn counter() -> u64 {
    let val: u64;
    unsafe {
        core::arch::asm!("mrs {0}, CNTPCT_EL0", out(reg) val, options(nostack));
        barrier::isb();
    }
    val
}

/// Program the timer to fire after `tval` ticks.
pub fn set_tval(tval: u32) {
    unsafe {
        core::arch::asm!("msr CNTP_TVAL_EL0, {0}", in(reg) tval as u64, options(nostack));
        barrier::isb();
    }
}

/// Remaining ticks before the timer fires.
pub fn tval() -> u32 {
    let val: u64;
    unsafe {
        core::arch::asm!("mrs {0}, CNTP_TVAL_EL0", out(reg) val, options(nostack));
    }
    val as u32
}

/// Enable the timer (CNTP_CTL_EL0.ENABLE = 1, IMASK = 0).
pub fn enable() {
    unsafe {
        core::arch::asm!("msr CNTP_CTL_EL0, {0}", in(reg) 1u64, options(nostack));
        barrier::isb();
    }
}

/// Disable the timer.
pub fn disable() {
    unsafe {
        core::arch::asm!("msr CNTP_CTL_EL0, {0}", in(reg) 0u64, options(nostack));
        barrier::isb();
    }
}

/// Read CNTP_CTL_EL0.
pub fn ctl() -> u64 {
    let val: u64;
    unsafe {
        core::arch::asm!("mrs {0}, CNTP_CTL_EL0", out(reg) val, options(nostack));
    }
    val
}

/// Global tick count.
#[unsafe(no_mangle)]
pub extern "Rust" fn ticks() -> u64 {
    TICK_COUNT.load(Ordering::Relaxed)
}

// ---------------------------------------------------------------------------
// IRQ handler + scheduler hook
// ---------------------------------------------------------------------------

/// Timer IRQ handler. Registered as IRQ 30.
pub fn timer_handler(_irq: u32) {
    TICK_COUNT.fetch_add(1, Ordering::Relaxed);
    set_tval(TVAL.load(Ordering::Relaxed));
    vivanta_arch_api::scheduler::scheduler_tick()
}
/// Register the timer IRQ handler and start periodic ticks.
pub unsafe fn init(gic: &crate::interrupts::Gic) {
    unsafe {
        let freq = frequency();
        TVAL.store((freq / TICK_HZ) as u32, Ordering::Relaxed);
        init_timer_only();
        gic.enable_irq(30);
    }
}

/// Timer init without GIC dependency.
/// Only sets up the timer hardware and registers the IRQ handler.
/// Caller must enable IRQ 30 on the interrupt controller separately.
pub unsafe fn init_timer_only() {
    unsafe {
        let tval = TVAL.load(Ordering::Relaxed);
        set_tval(tval);
        enable();
        crate::interrupts::register_irq(30, timer_handler);
    }
}
