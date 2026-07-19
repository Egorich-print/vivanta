#![no_std]

pub mod object;
pub mod resource;
pub mod manager;
pub mod capability;
pub mod policy;

pub use object::MemoryObject;
pub use resource::MemoryBackend;
pub use manager::MemoryResourceManager;
pub use capability::MemoryCapability;
pub use capability::MemRights;
pub use policy::{PlacementPolicy, AllocationRequirements};
