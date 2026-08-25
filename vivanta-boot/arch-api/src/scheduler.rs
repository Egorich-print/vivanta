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
    ///
    /// Safe to call: implementations only touch kernel-internal
    /// scheduling state and never dereference arch memory.
    pub safe fn scheduler_tick();

    /// Called after every IRQ, from the arch interrupt dispatcher.
    /// frame: handle to the on-stack exception frame (inspected, never
    /// dereferenced by the implementation).
    pub safe fn scheduler_reschedule(frame: InterruptFrameHandle);
}
