// Hardware state management for the Vivanta kernel.
//
// This module provides abstractions for hardware-related state,
// including device tree, memory map, and device registry.

use vivanta_boot_info::BootInfo;

/// Represents the hardware state of the system.
#[derive(Debug)]
pub struct HardwareState {
    pub dtb_ptr: usize,
}

impl HardwareState {
    /// Creates a new HardwareState with default values.
    pub fn new() -> Self {
        Self { dtb_ptr: 0 }
    }
    
    /// Creates a new HardwareState from BootInfo.
    pub fn from_boot_info(boot_info: &BootInfo) -> Self {
        Self {
            dtb_ptr: boot_info.dtb.unwrap_or(0),
        }
    }
}
