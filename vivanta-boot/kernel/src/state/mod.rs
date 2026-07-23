// System state management for the Vivanta kernel.
//
// This module provides the core state abstractions for the operating system,
// including identity, hardware, and memory state.
//
// As per ADR-021: System State Encapsulation, all fields are private
// and accessed only through getter methods.

use core::sync::atomic::{AtomicBool, Ordering};

use crate::identity::{BootIdentity, IdentityState, RuntimeIdentity, Uuid};

pub mod hardware;
pub mod identity;

use self::hardware::HardwareState;

/// Represents the complete state of the Vivanta system.
///
/// All fields are private as per ADR-021. Use getter methods to access state.
#[derive(Debug)]
pub struct SystemState {
    identity: IdentityState,
    hardware: HardwareState,
    is_initialized: AtomicBool,
}

impl SystemState {
    /// Creates a new SystemState from BootInfo.
    ///
    /// As per ADR-021, this consumes all needed data from BootInfo.
    /// BootInfo should not be accessed after this call.
    pub fn from_boot_info(boot_info: &vivanta_boot_info::BootInfo) -> Self {
        // Create boot identity from boot info (using 0 as timestamp for now)
        let boot_identity = BootIdentity::cold_start(0);
        
        // Create initial runtime identity from boot identity
        let runtime_identity = RuntimeIdentity::from_boot(&boot_identity, 1);
        
        Self {
            identity: IdentityState::Runtime(runtime_identity),
            hardware: HardwareState::from_boot_info(boot_info),
            is_initialized: AtomicBool::new(false),
        }
    }

    /// Initializes the system state.
    pub fn initialize(&self) {
        self.is_initialized.store(true, Ordering::SeqCst);
    }

    /// Returns true if the system state has been initialized.
    pub fn is_initialized(&self) -> bool {
        self.is_initialized.load(Ordering::SeqCst)
    }
    
    /// Returns a reference to the identity state.
    pub fn identity(&self) -> &IdentityState {
        &self.identity
    }
    
    /// Returns a reference to the hardware state.
    pub fn hardware(&self) -> &HardwareState {
        &self.hardware
    }
    
    /// Returns the current runtime identity if available.
    pub fn runtime_identity(&self) -> Option<&RuntimeIdentity> {
        self.identity.runtime()
    }
    
    /// Returns the current system ID.
    pub fn current_id(&self) -> Uuid {
        self.identity.current_id()
    }
    
    /// Returns the current generation number.
    pub fn generation(&self) -> u64 {
        self.identity.generation()
    }
}
