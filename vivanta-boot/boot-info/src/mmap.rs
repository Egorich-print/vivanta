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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_and_iterate() {
        let mut map = MemoryMap::new();
        map.push(MemoryRegion {
            start: 0x4000_0000,
            size: 0x1000,
            kind: MemoryRegionKind::Usable,
        });
        map.push(MemoryRegion {
            start: 0x9000_0000,
            size: 0x1000,
            kind: MemoryRegionKind::Mmio,
        });
        assert_eq!(map.regions().len(), 2);
        assert!(map.regions()[0].kind.is_usable());
        assert_eq!(map.regions()[0].start, 0x4000_0000);
        assert_eq!(map.regions()[1].kind, MemoryRegionKind::Mmio);
    }

    #[test]
    fn capacity_limit_respected() {
        let mut map = MemoryMap::new();
        for i in 0..(MAX_REGIONS + 4) {
            map.push(MemoryRegion {
                start: i as u64,
                size: 1,
                kind: MemoryRegionKind::Reserved,
            });
        }
        assert_eq!(map.regions().len(), MAX_REGIONS);
    }

    #[test]
    fn new_is_empty() {
        let map = MemoryMap::new();
        assert_eq!(map.regions().len(), 0);
    }
}
