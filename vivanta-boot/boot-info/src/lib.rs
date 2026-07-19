#![no_std]

pub mod mmap;
pub mod mmio;
pub mod interrupts;

pub use mmap::{MemoryMap, MemoryRegion, RegionType, MemoryRegionKind};
pub use mmio::{MmioRegion, MmioKind};
pub use interrupts::InterruptControllerInfo;

#[repr(C)]
pub struct BootInfo {
    pub memory_map: &'static MemoryMap,
    pub mmio_regions: &'static [MmioRegion],
    pub interrupt_controller: Option<InterruptControllerInfo>,
    pub cpu_count: usize,
    pub dtb: Option<usize>,
}