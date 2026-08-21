// ---------------------------------------------------------------------------
// Interrupt abstraction — RAII guard for disabled interrupts
//
// arch-api defines the guard type and its Drop impl (orphan-rule safe).
// Arch implementations provide disable_interrupts() and enable_interrupts().
// Kernel never uses inline DAIF asm.
// ---------------------------------------------------------------------------

/// RAII guard for disabled interrupts.
///
/// Saves the DAIF state at construction and restores it on drop.
/// Nested guards work correctly because each reads the *current* DAIF.
#[must_use = "InterruptGuard must be held to keep interrupts disabled"]
pub struct InterruptGuard {
    saved_daif: usize,
    restore: fn(usize),
}

impl InterruptGuard {
    #[doc(hidden)]
    pub fn new(saved_daif: usize, restore: fn(usize)) -> Self {
        Self {
            saved_daif,
            restore,
        }
    }
}

impl Drop for InterruptGuard {
    fn drop(&mut self) {
        (self.restore)(self.saved_daif);
    }
}

unsafe extern "Rust" {
    /// Disable interrupts and return a RAII guard.
    /// Interrupts are restored when the guard is dropped.
    pub fn disable_interrupts() -> InterruptGuard;

    /// Unconditionally enable interrupts.
    /// Use with caution — no RAII protection.
    pub fn enable_interrupts();
}
