use crate::error::{PmmError, PmmResult};
use vivanta_arch_api::pmm::{FrameAllocator, PhysFrame};
use vivanta_boot_common::memory_discovery::AvailableRegion;

pub const FRAME_SIZE: u64 = 4096;

pub struct PmmBitmap {
    bitmap: *mut u8,
    total_frames: usize,
    region_start: u64,
    allocated_count: usize,
    reserved_count: usize,
}

unsafe impl Send for PmmBitmap {}
unsafe impl Sync for PmmBitmap {}

impl FrameAllocator for PmmBitmap {
    fn alloc_frame(&mut self) -> Option<PhysFrame> {
        for i in 0..self.total_frames {
            if !self.test(i) {
                self.set(i, true);
                self.allocated_count += 1;
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
        if idx < self.total_frames && self.test(idx) {
            self.set(idx, false);
            self.allocated_count = self.allocated_count.saturating_sub(1);
        }
    }

    fn reserve(&mut self, start: u64, size: u64) {
        let end = start.saturating_add(size);
        let region_end = self.region_start + self.total_frames as u64 * FRAME_SIZE;
        let lo = start.max(self.region_start);
        let hi = end.min(region_end);
        if lo >= hi {
            return;
        }
        let first = ((lo - self.region_start) / FRAME_SIZE) as usize;
        let last = ((hi - 1 - self.region_start) / FRAME_SIZE) as usize;
        for i in first..=last.min(self.total_frames - 1) {
            if !self.test(i) {
                self.set(i, true);
                self.reserved_count += 1;
            }
        }
    }
}

impl PmmBitmap {
    pub unsafe fn new(region: &AvailableRegion) -> Self {
        assert!(
            region.start % FRAME_SIZE == 0,
            "PMM: region start ({:#x}) must be page-aligned",
            region.start
        );
        let bitmap_start = region.start as *mut u8;
        let region_size = region.end - region.start;
        let mut pmm = Self::init(bitmap_start, region.start, region_size);
        let bitmap_bytes = Self::bitmap_size(region_size);
        let bitmap_pages = (bitmap_bytes as u64 + 0xFFF) / 0x1000;
        pmm.reserve(region.start, bitmap_pages * 0x1000);
        pmm
    }

    pub unsafe fn init(bitmap_start: *mut u8, region_start: u64, region_size: u64) -> Self {
        let total_frames = (region_size / FRAME_SIZE) as usize;
        let byte_len = Self::bitmap_size(region_size);
        core::ptr::write_bytes(bitmap_start, 0, byte_len);
        PmmBitmap {
            bitmap: bitmap_start,
            total_frames,
            region_start,
            allocated_count: 0,
            reserved_count: 0,
        }
    }

    pub const fn bitmap_size(region_size: u64) -> usize {
        let frames = region_size / FRAME_SIZE;
        (frames as usize + 7) / 8
    }

    pub fn total_frames(&self) -> usize {
        self.total_frames
    }

    pub fn free_count(&self) -> usize {
        self.total_frames - self.allocated_count - self.reserved_count
    }

    pub fn allocated_count(&self) -> usize {
        self.allocated_count
    }

    pub fn reserved_count(&self) -> usize {
        self.reserved_count
    }

    pub fn region_start(&self) -> u64 {
        self.region_start
    }

    #[must_use]
    pub fn allocate_page(&mut self) -> PmmResult<u64> {
        self.alloc_frame()
            .map(|f| f.addr)
            .ok_or(PmmError::OutOfMemory)
    }

    pub fn free_page(&mut self, addr: u64) -> PmmResult<()> {
        if addr < self.region_start
            || addr >= self.region_start + self.total_frames as u64 * FRAME_SIZE
        {
            return Err(PmmError::InvalidAddress);
        }
        self.free_frame(PhysFrame { addr });
        Ok(())
    }

    pub fn reserve_page(&mut self, addr: u64) {
        self.reserve(addr, FRAME_SIZE);
    }

    pub fn reserve_range(&mut self, start: u64, end: u64) {
        if end > start {
            self.reserve(start, end - start);
        }
    }

    pub fn is_allocated(&self, addr: u64) -> bool {
        let offset = addr.wrapping_sub(self.region_start);
        let idx = (offset / FRAME_SIZE) as usize;
        idx < self.total_frames && self.test(idx)
    }

    fn test(&self, idx: usize) -> bool {
        let byte = unsafe { *self.bitmap.add(idx / 8) };
        (byte >> (idx % 8)) & 1 != 0
    }

    fn set(&mut self, idx: usize, value: bool) {
        let p = unsafe { &mut *self.bitmap.add(idx / 8) };
        if value {
            *p |= 1 << (idx % 8);
        } else {
            *p &= !(1 << (idx % 8));
        }
    }

    pub fn run_self_test(&mut self) -> PmmResult<()> {
        let pa = self.allocate_page()?;
        let pb = self.allocate_page()?;

        assert_ne!(pa, pb, "PMM: consecutive pages have same address");
        assert!(self.is_allocated(pa), "PMM: page not marked allocated");
        assert!(self.is_allocated(pb), "PMM: page not marked allocated");

        self.free_page(pa)?;
        assert!(!self.is_allocated(pa), "PMM: page not freed");

        let re_alloc = self.allocate_page()?;
        assert_eq!(re_alloc, pa, "PMM: freed page not reused first");

        self.free_page(re_alloc)?;
        self.free_page(pb)?;

        assert_eq!(
            self.allocated_count, 0,
            "PMM: allocated leak after self-test"
        );
        Ok(())
    }
}
