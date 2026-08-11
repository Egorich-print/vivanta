#![no_std]

pub mod capability;
pub mod manager;
pub mod object;
pub mod policy;
pub mod resource;

pub use capability::MemRights;
pub use capability::MemoryCapability;
pub use manager::MemoryResourceManager;
pub use object::MemoryObject;
pub use policy::{AllocationRequirements, PlacementPolicy};
pub use resource::MemoryBackend;
