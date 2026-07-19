use core::sync::atomic::{AtomicU64, Ordering};

static BOOT_COUNTER: AtomicU64 = AtomicU64::new(0);

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

#[derive(Debug, Clone, Copy)]
pub enum IdentityState {
    Volatile(RuntimeIdentity),
    Persistent(RuntimeIdentity),
}

impl IdentityState {
    pub fn new_volatile() -> Self {
        Self::Volatile(RuntimeIdentity::generate())
    }

    pub fn is_volatile(&self) -> bool {
        matches!(self, IdentityState::Volatile(_))
    }

    pub fn boot_id(&self) -> u64 {
        match self {
            IdentityState::Volatile(id) | IdentityState::Persistent(id) => id.boot_id,
        }
    }
}
