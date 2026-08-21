// ---------------------------------------------------------------------------
// Scheduler boundary — vivanta_kernel-provided callbacks called from arch
// ---------------------------------------------------------------------------

/// Opaque handle to the on-stack interrupt frame.
/// Passed through the scheduling path for inspection only —
/// the scheduler no longer copies or owns exception frames.
pub type InterruptFrameHandle = usize;

unsafe extern "Rust" {
    /// Called from the timer IRQ handler in arch.
    /// Signals the scheduler that a tick has occurred.
    pub fn scheduler_tick();

    /// Called after every IRQ, from the arch interrupt dispatcher.
    /// frame: handle to the on-stack exception frame.
    /// The scheduler uses this for inspection; context switching
    /// is handled by vivanta_arch_api::context::context_switch().
    pub fn scheduler_reschedule(frame: InterruptFrameHandle);
}
