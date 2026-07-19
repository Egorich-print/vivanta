pub const MAX_REGIONS: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryRegionKind {
    Usable,
    Reserved,
    KernelImage,
    BootloaderReclaimable,
    DeviceMemory,
    Framebuffer,
    Mmio,
}

#[derive(Debug, Clone, Copy)]
pub struct MemoryRegion {
    pub start: u64,
    pub size: u64,
    pub kind: MemoryRegionKind,
}

impl MemoryRegionKind {
    pub fn is_usable(self) -> bool {
        matches!(self, MemoryRegionKind::Usable)
    }
}

#[derive(Debug, Clone)]
pub struct MemoryMap {
    regions: [MemoryRegion; MAX_REGIONS],
    count: usize,
}

impl MemoryMap {
    pub const fn new() -> Self {
        const EMPTY: MemoryRegion = MemoryRegion {
            start: 0,
            size: 0,
            kind: MemoryRegionKind::Reserved,
        };
        MemoryMap {
            regions: [EMPTY; MAX_REGIONS],
            count: 0,
        }
    }

    pub fn push(&mut self, r: MemoryRegion) {
        if self.count < MAX_REGIONS {
            self.regions[self.count] = r;
            self.count += 1;
        }
    }

    pub fn regions(&self) -> &[MemoryRegion] {
        &self.regions[..self.count]
    }
}

pub type RegionType = MemoryRegionKind;