// ---------------------------------------------------------------------------
// MemoryCapability — capability-based access to Memory Objects
// ---------------------------------------------------------------------------

use crate::object::MemoryObjectId;

/// Identifier for a capability instance.
pub type CapabilityId = u64;

/// Identifier for an owner (process / job / driver host).
pub type OwnerId = u64;

/// Access rights associated with a capability.
#[derive(Debug, Clone, Copy)]
pub struct MemRights {
    pub read: bool,
    pub write: bool,
    pub execute: bool,
    pub map: bool,
    pub share: bool,
}

impl MemRights {
    pub const READ: Self = MemRights {
        read: true,
        write: false,
        execute: false,
        map: false,
        share: false,
    };
    pub const WRITE: Self = MemRights {
        read: true,
        write: true,
        execute: false,
        map: false,
        share: false,
    };
    pub const EXEC: Self = MemRights {
        read: true,
        write: false,
        execute: true,
        map: false,
        share: false,
    };
    pub const FULL: Self = MemRights {
        read: true,
        write: true,
        execute: true,
        map: true,
        share: true,
    };
}

/// A capability granting access to a MemoryObject.
///
/// Enforcement is deferred until userspace/IPC exist.
/// Currently `check()` always returns true.
#[derive(Debug, Clone, Copy)]
pub struct MemoryCapability {
    pub id: CapabilityId,
    pub object: MemoryObjectId,
    pub rights: MemRights,
    pub owner: OwnerId,
}

impl MemoryCapability {
    pub fn new(
        id: CapabilityId,
        object: MemoryObjectId,
        rights: MemRights,
        owner: OwnerId,
    ) -> Self {
        MemoryCapability {
            id,
            object,
            rights,
            owner,
        }
    }

    /// Check whether this capability grants the requested rights.
    /// Currently a placeholder — always returns true.
    pub fn check(&self, _required: MemRights) -> bool {
        true
    }
}
