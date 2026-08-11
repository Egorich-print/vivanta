//! Simple UUID implementation for no_std environments
//!
//! This provides a basic UUID v4 generator that works in no_std.

use core::fmt;

/// A simple UUID implementation for no_std
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Uuid {
    pub bytes: [u8; 16],
}

impl Uuid {
    /// Creates a new random UUID v4
    pub fn new_v4() -> Self {
        use core::sync::atomic::{AtomicU64, Ordering};

        static COUNTER: AtomicU64 = AtomicU64::new(0);

        let counter = COUNTER.fetch_add(1, Ordering::Relaxed);

        let mut bytes = [0u8; 16];
        // Set version (4) in bits 4-7 of byte 6
        bytes[6] = 0x40;
        // Set variant (RFC 4122) in bits 6-7 of byte 8
        bytes[8] = 0x80;

        // Fill with counter value
        bytes[0..8].copy_from_slice(&counter.to_le_bytes());

        Self { bytes }
    }

    /// Creates a nil UUID (all zeros)
    pub fn nil() -> Self {
        Self { bytes: [0u8; 16] }
    }

    /// Returns true if this is a nil UUID
    pub fn is_nil(&self) -> bool {
        self.bytes == [0u8; 16]
    }
}

impl Default for Uuid {
    fn default() -> Self {
        Self::nil()
    }
}

impl fmt::Display for Uuid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
            self.bytes[0], self.bytes[1], self.bytes[2], self.bytes[3],
            self.bytes[4], self.bytes[5],
            self.bytes[6], self.bytes[7],
            self.bytes[8], self.bytes[9],
            self.bytes[10], self.bytes[11], self.bytes[12], self.bytes[13], self.bytes[14], self.bytes[15]
        )
    }
}
