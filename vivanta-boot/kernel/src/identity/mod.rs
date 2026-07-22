//! Identity module for Vivanta kernel
//!
//! This module provides the identity model for the operating system,
//! separating concerns between boot-time, runtime, and persistent identity.
//!
//! # Architecture
//!
//! The identity model follows ADR-024: Identity Model Separation:
//! - `BootIdentity`: Identity established during boot (from BootInfo)
//! - `RuntimeIdentity`: Identity active during kernel execution
//! - `PersistentIdentity`: Identity that survives across reboots

pub mod boot;
pub mod persistent;
pub mod runtime;
pub mod uuid;

pub use boot::BootIdentity;
pub use persistent::PersistentIdentity;
pub use runtime::RuntimeIdentity;
pub use uuid::Uuid;

/// The current state of the system's identity
#[derive(Debug, Clone, PartialEq)]
pub enum IdentityState {
    /// System is booting, only BootIdentity is available
    Booting(BootIdentity),
    /// System is running with RuntimeIdentity
    Runtime(RuntimeIdentity),
    /// System has loaded PersistentIdentity and has active RuntimeIdentity
    Persistent(PersistentIdentity, RuntimeIdentity),
}

impl IdentityState {
    /// Returns the boot ID if available
    pub fn boot_id(&self) -> Option<Uuid> {
        match self {
            IdentityState::Booting(boot) => Some(boot.boot_id),
            IdentityState::Runtime(runtime) => Some(runtime.boot_id),
            IdentityState::Persistent(_, runtime) => Some(runtime.boot_id),
        }
    }

    /// Returns the runtime identity if available
    pub fn runtime(&self) -> Option<&RuntimeIdentity> {
        match self {
            IdentityState::Booting(_) => None,
            IdentityState::Runtime(r) => Some(r),
            IdentityState::Persistent(_, r) => Some(r),
        }
    }

    /// Returns the persistent identity if available
    pub fn persistent(&self) -> Option<&PersistentIdentity> {
        match self {
            IdentityState::Booting(_) => None,
            IdentityState::Runtime(_) => None,
            IdentityState::Persistent(p, _) => Some(p),
        }
    }

    /// Returns the current system ID
    pub fn current_id(&self) -> Uuid {
        match self {
            IdentityState::Booting(boot) => boot.boot_id,
            IdentityState::Runtime(runtime) => runtime.id,
            IdentityState::Persistent(persistent, _) => persistent.id,
        }
    }

    /// Returns the current generation number
    pub fn generation(&self) -> u64 {
        match self {
            IdentityState::Booting(_) => 0,
            IdentityState::Runtime(runtime) => runtime.generation,
            IdentityState::Persistent(_, runtime) => runtime.generation,
        }
    }
}

/// Source of the boot
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootSource {
    /// Booted from firmware/bootloader
    Firmware,
    /// Booted from cold start (power on)
    ColdStart,
    /// Booted from warm reset
    WarmReset,
    /// Booted from hibernation
    Hibernation,
}

impl Default for BootSource {
    fn default() -> Self {
        Self::ColdStart
    }
}
