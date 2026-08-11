#![no_std]

pub mod interrupts;
pub mod mmap;
pub mod mmio;

pub use interrupts::InterruptControllerInfo;
pub use mmap::{MemoryMap, MemoryRegion, MemoryRegionKind, RegionType};
pub use mmio::{MmioKind, MmioRegion};

#[repr(C)]
pub struct BootInfo {
    pub memory_map: &'static MemoryMap,
    pub mmio_regions: &'static [MmioRegion],
    pub interrupt_controller: Option<InterruptControllerInfo>,
    pub cpu_count: usize,
    pub dtb: Option<usize>,
}
