//! Persistent Identity
//!
//! Identity that persists across reboots.
//! Stored in persistent storage (disk/flash) and loaded at boot.

use alloc::vec::Vec;

use super::Uuid;

/// Identity that persists across reboots
///
/// This identity is stored in persistent storage and represents
/// the long-term identity of the system. It contains metadata
/// about the system's lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistentIdentity {
    /// Unique identifier for this system
    /// This ID remains constant across reboots
    pub id: Uuid,
    /// Version of the persistent identity format
    pub version: u64,
    /// Timestamp when this identity was first created
    pub created_at: u64,
    /// Timestamp when this identity was last updated
    pub last_updated: u64,
    /// Signature of the hardware this identity is associated with
    /// Used to detect hardware changes
    pub hardware_signature: Vec<u8>,
}

impl PersistentIdentity {
    /// Creates a new PersistentIdentity
    pub fn new(
        id: Uuid,
        version: u64,
        created_at: u64,
        last_updated: u64,
        hardware_signature: Vec<u8>,
    ) -> Self {
        Self {
            id,
            version,
            created_at,
            last_updated,
            hardware_signature,
        }
    }

    /// Creates a new PersistentIdentity with current timestamps
    pub fn new_now(id: Uuid, version: u64, hardware_signature: Vec<u8>, timestamp: u64) -> Self {
        Self {
            id,
            version,
            created_at: timestamp,
            last_updated: timestamp,
            hardware_signature,
        }
    }

    /// Updates the last_updated timestamp
    pub fn touch(&mut self, timestamp: u64) {
        self.last_updated = timestamp;
    }

    /// Updates the hardware signature
    pub fn update_hardware_signature(&mut self, signature: Vec<u8>, timestamp: u64) {
        self.hardware_signature = signature;
        self.last_updated = timestamp;
    }

    /// Returns the age of this identity (last_updated - created_at)
    pub fn age(&self) -> u64 {
        self.last_updated.saturating_sub(self.created_at)
    }
}

impl Default for PersistentIdentity {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            version: 1,
            created_at: 0,
            last_updated: 0,
            hardware_signature: Vec::new(),
        }
    }
}
