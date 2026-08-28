// ---------------------------------------------------------------------------
// Architecture context — opaque token for per-thread CPU context
//
// M4.4.5: Unified context switch model (ADR-017).
// ArchContext is an opaque newtype over usize.
// Kernel never inspects the inner value.
// ---------------------------------------------------------------------------

use crate::exception::ExceptionFrame;

/// Opaque token representing a thread's saved CPU context.
///
/// Created by `context_init` or `context_capture_current`,
/// consumed by `context_switch`.
///
/// Kernel must NOT inspect the inner value.
/// Only arch implementation crates may extract the raw handle.
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct ArchContext(usize);

impl ArchContext {
    #[doc(hidden)]
    pub fn from_raw(val: usize) -> Self {
        Self(val)
    }

    #[doc(hidden)]
    pub fn as_raw(&self) -> usize {
        self.0
    }
}

/// Execution privilege level for a thread.
///
/// Determines the SPSR value at thread creation and the target
/// exception level on `eret`.
///
/// This is NOT a scheduler classification or lifecycle state.
/// It describes privilege, not scheduling class.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExecutionLevel {
    /// EL1h (vivanta_kernel mode) — SPSR = 0x345
    Kernel,
    /// EL0t (user mode) — SPSR = 0x000
    User,
}

unsafe extern "Rust" {
    /// Initialise a thread's vivanta_kernel stack with the given entry point
    /// and execution level.
    ///
    /// `stack_top` — top of the vivanta_kernel stack (SP_EL1 value after restore).
    /// `stack_bottom` — lowest address of the vivanta_kernel stack region. The
    ///   per-thread ThreadContext is placed here, at the bottom, so the growing
    ///   stack (from `stack_top` downward) and exception frames pushed on it can
    ///   never overwrite the saved context (INV-002 fix).
    /// `user_stack_top` — top of the user stack (SP_EL0 value), 0 for vivanta_kernel threads.
    /// `entry` — entry point address (thread_trampoline for vivanta_kernel, user code for user).
    /// `level` — ExecutionLevel::Kernel or ExecutionLevel::User.
    ///
    /// Returns an opaque ArchContext token stored in the Thread struct.
    pub fn context_init(
        stack_top: usize,
        stack_bottom: usize,
        user_stack_top: usize,
        entry: usize,
        level: ExecutionLevel,
    ) -> ArchContext;

    /// Capture the current thread's context at boot time.
    /// Returns an ArchContext for the already-running boot thread.
    pub fn context_capture_current() -> ArchContext;

    /// Context switch — save callee-saved registers + SP to *old,
    /// restore from *new. Used by yield_now() and (in future) timer reschedule.
    pub fn context_switch(old: *mut ArchContext, new: ArchContext);

    /// Create a forked child context.
    ///
    /// Copies the parent's ThreadContext (at `parent_stack_bottom`) and
    /// ExceptionFrame (at `parent_frame`) to the child's kernel stack.
    /// Modifies the child's ExceptionFrame.x[0] = 0 (return value for child).
    ///
    /// `child_stack_top` — top of child's kernel stack (stack_base + KERNEL_STACK_SIZE)
    /// `child_stack_bottom` — bottom of child's kernel stack (stack_base)
    /// `parent_stack_bottom` — bottom of parent's kernel stack (where ThreadContext lives)
    /// `parent_frame` — pointer to parent's ExceptionFrame (saved by SVC entry)
    ///
    /// Returns ArchContext for the child thread.
    pub fn context_fork(
        child_stack_top: usize,
        child_stack_bottom: usize,
        parent_stack_bottom: usize,
        parent_frame: *const ExceptionFrame,
    ) -> ArchContext;
}
