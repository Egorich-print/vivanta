//! Boot Identity
//!
//! Identity established during the boot process.
//! This identity is transient and only valid during initialization.

use super::{BootSource, Uuid};

/// Identity established during boot process
///
/// Created from BootInfo and used only during initialization.
/// This identity is ephemeral and does not persist beyond boot.
#[derive(Debug, Clone, PartialEq)]
pub struct BootIdentity {
    /// Unique identifier for this boot session
    pub boot_id: Uuid,
    /// Timestamp when boot started (monotonic counter or timestamp)
    pub boot_timestamp: u64,
    /// Source of the boot
    pub source: BootSource,
}

impl BootIdentity {
    /// Creates a new BootIdentity
    pub fn new(boot_id: Uuid, boot_timestamp: u64, source: BootSource) -> Self {
        Self {
            boot_id,
            boot_timestamp,
            source,
        }
    }

    /// Creates a BootIdentity from a boot timestamp and source
    pub fn with_timestamp(timestamp: u64, source: BootSource) -> Self {
        Self {
            boot_id: Uuid::new_v4(),
            boot_timestamp: timestamp,
            source,
        }
    }

    /// Creates a BootIdentity for cold start
    pub fn cold_start(timestamp: u64) -> Self {
        Self::with_timestamp(timestamp, BootSource::ColdStart)
    }

    /// Creates a BootIdentity for warm reset
    pub fn warm_reset(timestamp: u64) -> Self {
        Self::with_timestamp(timestamp, BootSource::WarmReset)
    }
}

impl Default for BootIdentity {
    fn default() -> Self {
        Self::cold_start(0)
    }
}
