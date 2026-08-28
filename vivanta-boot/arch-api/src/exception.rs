// ---------------------------------------------------------------------------
// Exception frame — architecture-specific saved CPU state
//
// This is the AArch64 exception frame layout. Other architectures
// should define their own equivalent if needed.
// ---------------------------------------------------------------------------

/// Saved CPU state at exception entry.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ExceptionFrame {
    pub x: [u64; 31],
    pub sp: u64,
    pub elr: u64,
    pub spsr: u64,
}

impl ExceptionFrame {
    /// Size of the exception frame in bytes.
    pub const SIZE: usize = core::mem::size_of::<ExceptionFrame>();
}

const _: () = assert!(ExceptionFrame::SIZE == 34 * 8);