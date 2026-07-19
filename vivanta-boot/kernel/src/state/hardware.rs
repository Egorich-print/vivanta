use vivanta_boot_info::{BootInfo, MemoryMap, MmioRegion, MmioKind};

const MAX_MMIO: usize = 16;

#[derive(Debug, Clone)]
pub struct HardwareState {
    pub memory_map: MemoryMap,
    pub mmio_regions: [MmioRegion; MAX_MMIO],
    pub mmio_count: usize,
    pub cpu_count: usize,
    pub dtb: Option<usize>,
}

impl HardwareState {
    /// Create a minimal HardwareState without BootInfo (for early bring-up targets).
    pub fn empty() -> Self {
        Self {
            memory_map: MemoryMap::new(),
            mmio_regions: [MmioRegion { base: 0, size: 0, kind: MmioKind::Device }; MAX_MMIO],
            mmio_count: 0,
            cpu_count: 1,
            dtb: None,
        }
    }

    pub fn copy_from(info: &BootInfo) -> Self {
        let mut memory_map = MemoryMap::new();
        for r in info.memory_map.regions() {
            memory_map.push(*r);
        }

        let mut mmio = [MmioRegion { base: 0, size: 0, kind: vivanta_boot_info::MmioKind::Device }; MAX_MMIO];
        let mut mmio_count = 0;
        for r in info.mmio_regions {
            if mmio_count < MAX_MMIO {
                mmio[mmio_count] = *r;
                mmio_count += 1;
            }
        }

        Self {
            memory_map,
            mmio_regions: mmio,
            mmio_count,
            cpu_count: info.cpu_count,
            dtb: info.dtb,
        }
    }

    pub fn mmio_slice(&self) -> &[MmioRegion] {
        &self.mmio_regions[..self.mmio_count]
    }
}
