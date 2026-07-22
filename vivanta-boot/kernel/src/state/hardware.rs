// Hardware state management for the Vivanta kernel.
//
// This module provides abstractions for hardware-related state,
// including device tree, memory map, and device registry.

use vivanta_boot_info::{BootInfo, MemoryMap, MemoryRegion, MmioRegion, InterruptControllerInfo};

/// Represents the hardware state of the system.
///
/// Contains all hardware-related information extracted from BootInfo.
/// As per ADR-021, this is the single source of truth for hardware state
/// after SystemState is created.
#[derive(Debug)]
pub struct HardwareState {
    /// Pointer to the device tree blob
    pub dtb_ptr: usize,
    /// Reference to the memory map from BootInfo
    pub memory_map: &'static MemoryMap,
    /// Reference to the MMIO regions from BootInfo
    pub mmio_regions: &'static [MmioRegion],
    /// CPU count
    pub cpu_count: usize,
    /// Interrupt controller information
    pub interrupt_controller: Option<InterruptControllerInfo>,
}

impl HardwareState {
    /// Creates a new HardwareState with default values.
    pub fn new() -> Self {
        // This is unsafe but we need it for the default
        // In practice, this should only be called with valid data
        unsafe {
            Self {
                dtb_ptr: 0,
                memory_map: &*(0 as *const MemoryMap),
                mmio_regions: &*(0 as *const [MmioRegion]),
                cpu_count: 0,
                interrupt_controller: None,
            }
        }
    }
    
    /// Creates a new HardwareState from BootInfo.
    /// 
    /// As per ADR-021, this extracts all hardware-related data from BootInfo.
    /// After this call, BootInfo should not be accessed for hardware information.
    pub fn from_boot_info(boot_info: &BootInfo) -> Self {
        Self {
            dtb_ptr: boot_info.dtb.unwrap_or(0),
            memory_map: boot_info.memory_map,
            mmio_regions: boot_info.mmio_regions,
            cpu_count: boot_info.cpu_count,
            interrupt_controller: boot_info.interrupt_controller,
        }
    }
}
