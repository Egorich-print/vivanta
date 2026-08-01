pub mod capability;
pub mod kernel_heap;
pub mod manager;
pub mod object;
pub mod pmm_backend;
pub mod policy;
pub mod resource;

pub use capability::{MemRights, MemoryCapability, OwnerId};
pub use kernel_heap::KernelHeap;
pub use manager::MemoryResourceManager;
pub use object::{MemoryObject, MemoryObjectId, MemoryObjectState, ObjectError, MrmPageTableAllocator, ShareHandle, VirtualMapping};
pub use pmm_backend::PmmBackend;
pub use policy::{AllocationRequirements, PlacementPolicy};
pub use resource::{AllocError, MemoryBackend, MemoryProperties, PhysAddr, ResourceId};