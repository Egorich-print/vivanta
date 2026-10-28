// ---------------------------------------------------------------------------
// Scheduler failpoint harness — mission 2026-10, wave 1.1
// ---------------------------------------------------------------------------
//
// Purpose: make the open scheduler race reproduce *deterministically* so that
// a fix can be validated. Per the mission charter (R2) NO causative fix may be
// landed before a failpoint actually fires — the previous attempt at this
// (branch `diag/exit-code-race`) failed precisely because it printed inside the
// racing window and changed the timing it was trying to observe.
//
// Two hard rules for everything in this file:
//
//   1. NEVER print from inside an armed window. Observability is BSS counters
//      only. A `println!` inside the window is a heisenbug generator: UART
//      output takes long enough to change the interleaving, which is exactly
//      how the earlier attempt hid the defect instead of reproducing it.
//
//   2. The whole module is behind the `sched-failpoint` cargo feature. With
//      the feature off, every statement below is compiled out, so the release
//      image carries no counters, no branches and no added instructions.
//      That is gate G6 ("CLEAN stays green") — provable by disassembly, not
//      by argument.
//
// The whole module is behind the `sched-failpoint` cargo feature. With the
// feature off, every statement below is compiled out, so the release image
// carries no counters, no branches and no added instructions.

use core::sync::atomic::{AtomicU64, Ordering};

use super::SwitchOrigin;

/// Failpoints armed by default.
///
/// The control build (`sched-failpoint` without `sched-failpoint-fire`)
/// arms nothing: the harness observes but never injects, which is what
/// proves the harness code itself does not perturb the kernel.
#[cfg(feature = "sched-failpoint-fire")]
const DEFAULT_ARMED: u64 = FP_F3_RESUME_NESTED;
#[cfg(not(feature = "sched-failpoint-fire"))]
const DEFAULT_ARMED: u64 = 0;

/// Mask of armed failpoints. Bit flags below.
///
/// The harness script can arm or disarm individual bits through
/// `arm`/`disarm`.
pub static ARMED: AtomicU64 = AtomicU64::new(DEFAULT_ARMED);

/// F3 — nested scheduler entry inside the resume window.
///
/// Armed at the point where the resuming thread re-enables interrupts. At
/// that moment the CPU is still inside the exception-handler call chain that
/// led to the switch, so the victim's `ExceptionFrame` (which lives at
/// `[SP_EL1 - 272]`) is still live and still holds the victim's EL0 state.
/// Forcing a scheduler entry here reproduces the "second scheduler entry
/// while an exception frame is live" condition that the open race depends on.
pub const FP_F3_RESUME_NESTED: u64 = 1 << 0;

/// F1 — tick between `CURRENT_THREAD.store` and `context_switch`.
///
/// Armed inside the publish→switch window. With interrupts masked this is
/// expected to be inert; if it is NOT inert, the masking invariant is broken
/// and must be reopened rather than assumed.
pub const FP_F1_STORE_TO_SWITCH: u64 = 1 << 1;

/// F2 — reschedule request consumed without producing a switch.
///
/// Counts `maybe_reschedule` clearing `NEED_RESCHEDULE` and then finding no
/// eligible thread. The flag is lost, so the pending reschedule is deferred
/// to the next tick — jitter today, lost wakeup if the tick source stalls.
pub const FP_F2_CONSUMED_NO_SWITCH: u64 = 1 << 2;

// --- counters -------------------------------------------------------------

/// Number of times the resume path re-enabled interrupts.
pub static RESUME_UNMASKS: AtomicU64 = AtomicU64::new(0);
/// Number of nested scheduler entries forced by F3.
pub static RESUME_NESTED_ENTRIES: AtomicU64 = AtomicU64::new(0);
/// Number of forced ticks injected by F1.
pub static STORE_SWITCH_TICKS: AtomicU64 = AtomicU64::new(0);
/// Number of reschedule requests consumed without a switch (F2 observation).
pub static CONSUMED_NO_SWITCH: AtomicU64 = AtomicU64::new(0);
/// Context switches performed, by origin.
pub static SWITCH_TOTAL: AtomicU64 = AtomicU64::new(0);
/// Switches where the interrupted context was EL0 (user).
pub static SWITCH_FROM_EL0: AtomicU64 = AtomicU64::new(0);
/// Switches triggered from the timer reschedule path.
pub static SWITCH_TIMER: AtomicU64 = AtomicU64::new(0);
/// Timer switches that preempted an EL0 thread.
///
/// This is the interesting population: an EL0 thread preempted by the timer
/// has its EL0 state parked in the exception frame that is still live while
/// the chain unwinds. Measured at ~1 per boot, which matches the 1-in-2..5
/// CI failure rate of the open race.
pub static SWITCH_TIMER_FROM_EL0: AtomicU64 = AtomicU64::new(0);
/// For the last EL0-preempting timer switch: distance between the exception
/// frame base and the kernel SP at switch time.
///
/// Non-zero is normal — it is the depth of the exception-handler call chain,
/// which unwinds on resume and restores SP to the frame base. It is recorded
/// to prove the chain-unwind invariant rather than to assert a defect.
pub static EL0_SP_DELTA: AtomicU64 = AtomicU64::new(0);

/// Is `bit` armed?
#[inline]
pub fn is_armed(bit: u64) -> bool {
    ARMED.load(Ordering::Relaxed) & bit != 0
}

/// Arm a failpoint. Only used by the test harness.
pub fn arm(bit: u64) {
    ARMED.fetch_or(bit, Ordering::Relaxed);
}

/// Disarm a failpoint.
pub fn disarm(bit: u64) {
    ARMED.fetch_and(!bit, Ordering::Relaxed);
}

/// Read `SPSR_EL1` and report whether the interrupted context was EL0.
///
/// Bit 0 of SPSR_EL1 is `M`: 0 means EL0t (user), 1 means EL1h (kernel).
/// On AArch64 the DAIF field of the `DAIFSet`/`DAIFClr` immediates is
/// bit3=D, bit2=A, bit1=I, bit0=F — so `msr DAIFSet, #2` masks IRQ, not
/// SError. (Verified against the Arm Architecture Reference Manual and by
/// direct QEMU read-back; the opposite claim used to live in this tree.)
#[inline]
pub fn interrupted_from_el0() -> bool {
    let spsr: u64;
    unsafe {
        core::arch::asm!(
            "mrs {}, spsr_el1",
            out(reg) spsr,
            options(nomem, nostack, preserves_flags)
        );
    }
    spsr & 1 == 0
}

/// Read the current kernel SP (SP_EL1).
#[inline]
pub fn kernel_sp() -> u64 {
    let sp: u64;
    unsafe {
        core::arch::asm!(
            "mov {}, sp",
            out(reg) sp,
            options(nomem, nostack, preserves_flags)
        );
    }
    sp
}

/// Record one context switch.
///
/// `frame` is the exception-frame base when the switch happened inside an
/// exception handler, and 0 otherwise.
pub fn record_switch(origin: SwitchOrigin, frame: usize) {
    SWITCH_TOTAL.fetch_add(1, Ordering::Relaxed);
    let from_el0 = interrupted_from_el0();
    if from_el0 {
        SWITCH_FROM_EL0.fetch_add(1, Ordering::Relaxed);
    }
    if origin == SwitchOrigin::Timer {
        SWITCH_TIMER.fetch_add(1, Ordering::Relaxed);
        if from_el0 {
            SWITCH_TIMER_FROM_EL0.fetch_add(1, Ordering::Relaxed);
            if frame != 0 {
                EL0_SP_DELTA.store((frame as u64).wrapping_sub(kernel_sp()), Ordering::Relaxed);
            }
        }
    }
}

/// Print a one-shot summary of the harness counters.
///
/// Must only be called from a point that is NOT inside an armed window —
/// boot has finished by then, so the timing of the race is no longer
/// perturbed.
pub fn report() {
    vivanta_boot_common::println!(
        "  [FP] switches={} from_el0={} timer={} timer_el0={} el0_sp_delta={}",
        SWITCH_TOTAL.load(Ordering::Relaxed),
        SWITCH_FROM_EL0.load(Ordering::Relaxed),
        SWITCH_TIMER.load(Ordering::Relaxed),
        SWITCH_TIMER_FROM_EL0.load(Ordering::Relaxed),
        EL0_SP_DELTA.load(Ordering::Relaxed),
    );
    vivanta_boot_common::println!(
        "  [FP] resume_unmasks={} nested_entries={} store_switch_ticks={} consumed_no_switch={}",
        RESUME_UNMASKS.load(Ordering::Relaxed),
        RESUME_NESTED_ENTRIES.load(Ordering::Relaxed),
        STORE_SWITCH_TICKS.load(Ordering::Relaxed),
        CONSUMED_NO_SWITCH.load(Ordering::Relaxed),
    );
}
