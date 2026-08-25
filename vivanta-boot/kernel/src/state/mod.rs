use core::sync::atomic::{AtomicBool, Ordering};

use crate::identity::{BootIdentity, IdentityState, RuntimeIdentity, Uuid};
use crate::memory::MemoryResourceManager;

pub mod hardware;

use self::hardware::HardwareState;

/// Represents the complete state of the Vivanta system.
///
/// All fields are private as per ADR-021. Use getter methods to access state.
pub struct SystemState {
    identity: IdentityState,
    hardware: HardwareState,
    memory_manager: Option<MemoryResourceManager>,
    is_initialized: AtomicBool,
}

impl SystemState {
    /// Creates a new SystemState from BootInfo.
    ///
    /// As per ADR-021, this consumes all needed data from BootInfo.
    /// BootInfo should not be accessed after this call.
    ///
    /// MemoryResourceManager is NOT initialized here — call `init_memory()`
    /// after PmmBitmap is ready.
    pub fn from_boot_info(boot_info: &vivanta_boot_info::BootInfo) -> Self {
        let boot_identity = BootIdentity::cold_start(0);
        let runtime_identity = RuntimeIdentity::from_boot(&boot_identity, 1);

        Self {
            identity: IdentityState::Runtime(runtime_identity),
            hardware: HardwareState::from_boot_info(boot_info),
            memory_manager: None,
            is_initialized: AtomicBool::new(false),
        }
    }

    /// Initialise the Memory Resource Manager with the PMM backend.
    ///
    /// Called after `PmmBitmap` and `PmmBackend` are constructed.
    pub fn init_memory(&mut self, backend: &mut (dyn crate::memory::MemoryBackend + 'static)) {
        let mut mrm = MemoryResourceManager::new();
        unsafe {
            mrm.register(backend);
        }
        self.memory_manager = Some(mrm);
    }

    pub fn initialize(&self) {
        self.is_initialized.store(true, Ordering::SeqCst);
    }

    pub fn is_initialized(&self) -> bool {
        self.is_initialized.load(Ordering::SeqCst)
    }

    pub fn identity(&self) -> &IdentityState {
        &self.identity
    }

    pub fn hardware(&self) -> &HardwareState {
        &self.hardware
    }

    pub fn memory_manager(&self) -> &MemoryResourceManager {
        self.memory_manager.as_ref().expect("MRM not initialized")
    }

    pub fn memory_manager_mut(&mut self) -> &mut MemoryResourceManager {
        self.memory_manager.as_mut().expect("MRM not initialized")
    }

    pub fn runtime_identity(&self) -> Option<&RuntimeIdentity> {
        self.identity.runtime()
    }

    pub fn current_id(&self) -> Uuid {
        self.identity.current_id()
    }

    pub fn generation(&self) -> u64 {
        self.identity.generation()
    }
}
