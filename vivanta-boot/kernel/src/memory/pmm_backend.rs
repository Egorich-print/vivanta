use crate::memory::resource::{AllocError, MemoryBackend, MemoryProperties, PhysAddr};
use vivanta_arch_api::pmm::FrameAllocator;

/// A MemoryBackend that wraps the kernel's PmmBitmap frame allocator.
///
/// Stores the frame allocator as a raw pointer (no borrowing).
/// This makes PmmBackend `'static`, which allows storing it in
/// MemoryResourceManager's `*mut dyn MemoryBackend` array.
pub struct PmmBackend {
    pmm: *mut dyn FrameAllocator,
    properties: MemoryProperties,
}

impl PmmBackend {
    pub unsafe fn new(pmm: *mut dyn FrameAllocator, properties: MemoryProperties) -> Self {
        PmmBackend { pmm, properties }
    }

    pub unsafe fn new_dram(pmm: *mut dyn FrameAllocator) -> Self {
        unsafe { Self::new(pmm, MemoryProperties::dram_4gb()) }
    }

    fn pmm(&mut self) -> &mut dyn FrameAllocator {
        unsafe { &mut *self.pmm }
    }
}

impl MemoryBackend for PmmBackend {
    fn allocate(&mut self, size: u64, align: u64) -> Result<PhysAddr, AllocError> {
        if align > 4096 {
            return Err(AllocError::AlignmentNotSupported);
        }
        let frames_needed = ((size + 4095) / 4096) as usize;
        let pmm = self.pmm();
        // G2 contiguity contract: multi-frame allocations use the explicit
        // contiguous API. PMM's alloc_contiguous returns an atomic run of
        // contiguous free frames (no partial leak on failure).
        let first = pmm
            .alloc_contiguous(frames_needed)
            .ok_or(AllocError::OutOfCapacity)?;
        Ok(first.addr)
    }

    fn deallocate(&mut self, addr: PhysAddr, size: u64) {
        let frames = ((size + 4095) / 4096) as usize;
        let pmm = self.pmm();
        for i in 0..frames {
            pmm.free_frame(vivanta_arch_api::pmm::PhysFrame {
                addr: addr + (i as u64) * 4096,
            });
        }
    }

    fn properties(&self) -> MemoryProperties {
        self.properties
    }

    fn name(&self) -> &'static str {
        "DRAM (PMM)"
    }
}
