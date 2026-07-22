//! Runtime Identity
//!
//! Identity active during kernel runtime execution.
//! This is the primary identity used by the system during operation.

use super::{BootIdentity, Uuid};

/// Identity active during kernel runtime
///
/// Contains the current operational identity of the system.
/// This identity is created when transitioning from boot to runtime.
#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeIdentity {
    /// Unique identifier for this runtime session
    pub id: Uuid,
    /// Generation number, increments on each runtime initialization
    pub generation: u64,
    /// Links to the BootIdentity that created this runtime
    pub boot_id: Uuid,
}

impl RuntimeIdentity {
    /// Creates a new RuntimeIdentity from a BootIdentity
    pub fn from_boot(boot: &BootIdentity, generation: u64) -> Self {
        Self {
            id: Uuid::new_v4(),
            generation,
            boot_id: boot.boot_id,
        }
    }

    /// Creates a new RuntimeIdentity with a specific ID
    pub fn with_id(id: Uuid, generation: u64, boot_id: Uuid) -> Self {
        Self {
            id,
            generation,
            boot_id,
        }
    }

    /// Increments the generation and returns a new RuntimeIdentity
    pub fn next_generation(&self) -> Self {
        Self {
            id: Uuid::new_v4(),
            generation: self.generation + 1,
            boot_id: self.boot_id,
        }
    }
}

impl Default for RuntimeIdentity {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            generation: 1,
            boot_id: Uuid::nil(),
        }
    }
}
