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
}