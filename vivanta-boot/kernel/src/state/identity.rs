// Identity state management for the Vivanta kernel.
//
// This module provides the core identity abstractions for the operating system.
// 
// NOTE: This module is deprecated in favor of the new identity module
// (`kernel/src/identity/mod.rs`) as per ADR-024: Identity Model Separation.
// The old types are kept for backward compatibility during V1.1 migration.
//
// New code should use:
// - `crate::identity::BootIdentity` for boot-time identity
// - `crate::identity::RuntimeIdentity` for runtime identity
// - `crate::identity::PersistentIdentity` for persistent identity
// - `crate::identity::IdentityState` for the state enum

use core::sync::atomic::{AtomicU64, Ordering};

static BOOT_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Core identity structure containing unique identifiers for the system.
/// 
/// # Deprecated
/// Use `crate::identity::RuntimeIdentity` instead. This type will be removed in V1.2.
#[deprecated(note = "Use crate::identity::RuntimeIdentity instead. See ADR-024.")]
#[derive(Debug, Clone, Copy)]
pub struct RuntimeIdentity {
    pub public_key: [u8; 32],
    pub boot_id: u64,
}

impl RuntimeIdentity {
    pub fn generate() -> Self {
        Self {
            public_key: [0u8; 32],
            boot_id: BOOT_COUNTER.fetch_add(1, Ordering::Relaxed),
        }
    }
}

/// Represents the identity state of the system.
/// 
/// # Deprecated
/// Use `crate::identity::IdentityState` instead. This type will be removed in V1.2.
#[deprecated(note = "Use crate::identity::IdentityState instead. See ADR-024.")]
#[derive(Debug, Clone, Copy)]
pub enum IdentityState {
    Volatile(RuntimeIdentity),
    Persistent(RuntimeIdentity),
}
