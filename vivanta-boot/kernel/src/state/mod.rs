pub mod identity;
pub mod hardware;

pub use identity::IdentityState;
pub use hardware::HardwareState;
use vivanta_boot_info::BootInfo;

#[derive(Debug, Clone)]
pub struct SystemState {
    pub identity: IdentityState,
    pub hardware: HardwareState,
}

impl SystemState {
    pub fn new(identity: IdentityState, hardware: HardwareState) -> Self {
        Self { identity, hardware }
    }

    pub fn from_boot_info(info: &BootInfo) -> Self {
        Self {
            identity: IdentityState::new_volatile(),
            hardware: HardwareState::copy_from(info),
        }
    }

    pub fn boot(&self) {
        vivanta_boot_common::println!("[V0] Runtime Identity Bootstrap");
    }
}
