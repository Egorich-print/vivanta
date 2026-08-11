// ---------------------------------------------------------------------------
// Physical frame allocator contract — vivanta_kernel ↔ arch boundary
// ---------------------------------------------------------------------------

/// A physical memory frame.
#[derive(Debug, Clone, Copy)]
pub struct PhysFrame {
    pub addr: u64,
}

/// Trait for raw physical frame allocation.
/// Used by vivanta_kernel's PmmBitmap and arch's CallbackAllocator.
/// Not a HAL trait — it's a concrete utility boundary.
pub trait FrameAllocator {
    fn alloc_frame(&mut self) -> Option<PhysFrame>;
    fn free_frame(&mut self, frame: PhysFrame);
    fn reserve(&mut self, start: u64, size: u64);

    /// Allocate `n` physically CONTIGUOUS frames. Returns the first frame
    /// address on success, `None` on failure (nothing is leaked).
    ///
    /// Default: for `n == 1` delegates to `alloc_frame`; for `n > 1` returns
    /// `None` unless the implementor provides real contiguity. Callers that
    /// need a contiguous range MUST use this method, never `alloc_frame` × n.
    fn alloc_contiguous(&mut self, n: usize) -> Option<PhysFrame> {
        if n == 1 {
            self.alloc_frame()
        } else {
            None
        }
    }
}
