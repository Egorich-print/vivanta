// ---------------------------------------------------------------------------
// Physical Memory Manager — bitmap frame allocator
// ---------------------------------------------------------------------------

use vivanta_boot_common::println;
use vivanta_arch_api::pmm::{FrameAllocator, PhysFrame};

pub const FRAME_SIZE: u64 = 4096;

/// Physical memory frame bitmap allocator.
pub struct PmmBitmap {
    bitmap: *mut u8,
    total_frames: usize,
    region_start: u64,
}

unsafe impl Send for PmmBitmap {}
unsafe impl Sync for PmmBitmap {}

impl FrameAllocator for PmmBitmap {
    fn alloc_frame(&mut self) -> Option<PhysFrame> {
        for i in 0..self.total_frames {
            if !self.test(i) {
                self.set(i, true);
                return Some(PhysFrame {
                    addr: self.region_start + i as u64 * FRAME_SIZE,
                });
            }
        }
        None
    }

    fn free_frame(&mut self, frame: PhysFrame) {
        let offset = frame.addr.wrapping_sub(self.region_start);
        let idx = (offset / FRAME_SIZE) as usize;
        if idx < self.total_frames {
            self.set(idx, false);
        }
    }

    fn reserve(&mut self, start: u64, size: u64) {
        let end = start.saturating_add(size);
        let region_end = self.region_start + self.total_frames as u64 * FRAME_SIZE;
        let lo = start.max(self.region_start);
        let hi = end.min(region_end);
        if lo >= hi { return; }
        let first = ((lo - self.region_start) / FRAME_SIZE) as usize;
        let last = ((hi - 1 - self.region_start) / FRAME_SIZE) as usize;
        for i in first..=last.min(self.total_frames - 1) {
            self.set(i, true);
        }
    }
}

impl PmmBitmap {
    pub unsafe fn init(bitmap_start: *mut u8, region_start: u64, region_size: u64) -> Self {
        let total_frames = (region_size / FRAME_SIZE) as usize;
        let byte_len = Self::bitmap_size(region_size);
        core::ptr::write_bytes(bitmap_start, 0, byte_len);
        PmmBitmap { bitmap: bitmap_start, total_frames, region_start }
    }

    pub const fn bitmap_size(region_size: u64) -> usize {
        let frames = region_size / FRAME_SIZE;
        (frames as usize + 7) / 8
    }

    pub fn free_count(&self) -> usize {
        let mut count = 0;
        for i in 0..self.total_frames {
            if !self.test(i) { count += 1; }
        }
        count
    }

    pub fn total_frames(&self) -> usize {
        self.total_frames
    }

    fn test(&self, idx: usize) -> bool {
        let byte = unsafe { *self.bitmap.add(idx / 8) };
        (byte >> (idx % 8)) & 1 != 0
    }

    fn set(&mut self, idx: usize, allocated: bool) {
        let p = unsafe { &mut *self.bitmap.add(idx / 8) };
        if allocated {
            *p |= 1 << (idx % 8);
        } else {
            *p &= !(1 << (idx % 8));
        }
    }
}

// ---------------------------------------------------------------------------
// BootMemoryManager
// ---------------------------------------------------------------------------

pub struct BootMemoryManager {
    pmm: PmmBitmap,
    region_start: u64,
    region_size: u64,
    bitmap_base: u64,
    bitmap_bytes: usize,
    kernel_start: u64,
    kernel_end: u64,
    dtb_addr: u64,
    dtb_size: u64,
}

impl BootMemoryManager {
    pub unsafe fn new(region_start: u64, region_size: u64, bitmap_base: *mut u8) -> Self {
        let bitmap_bytes = PmmBitmap::bitmap_size(region_size);
        BootMemoryManager {
            pmm: unsafe { PmmBitmap::init(bitmap_base, region_start, region_size) },
            region_start,
            region_size,
            bitmap_base: bitmap_base as u64,
            bitmap_bytes,
            kernel_start: 0,
            kernel_end: 0,
            dtb_addr: 0,
            dtb_size: 0,
        }
    }

    pub fn reserve_kernel(&mut self, start: u64, end: u64) {
        self.kernel_start = start;
        self.kernel_end = end;
        self.pmm.reserve(start, end - start);
    }

    pub fn reserve_dtb(&mut self, addr: u64, size: u64) {
        self.dtb_addr = addr;
        self.dtb_size = size;
        self.pmm.reserve(addr, size);
    }

    pub fn reserve_bitmap(&mut self) {
        let pages = (self.bitmap_bytes as u64 + 0xFFF) / 0x1000;
        self.pmm.reserve(self.bitmap_base, pages * 0x1000);
    }

    pub fn finish(self) -> PmmBitmap {
        self.pmm
    }

    pub fn print_stats(&self) {
        let total = self.pmm.total_frames();
        let used = total - self.pmm.free_count();
        println!();
        println!("Physical Memory Manager:");
        println!("  Region    0x{:016x} – 0x{:016x}  ({} MiB)",
            self.region_start, self.region_start + self.region_size - 1, self.region_size >> 20);
        println!("  Bitmap    {} bytes at 0x{:x}", self.bitmap_bytes, self.bitmap_base);
        println!("  Reserved  {} / {} frames  (vivanta_kernel + DTB + bitmap)", used, total);
        println!("  Free      {} / {} frames", total - used, total);
    }
}
