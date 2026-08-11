// ---------------------------------------------------------------------------
// VMM — Virtual Memory Manager (Stage 5)
//
// Architecture-neutral address space abstraction.
// Root page table is an opaque handle (arch-api::mmu::RootPageTable).
// ---------------------------------------------------------------------------

pub mod address_space;
pub mod faults;
pub mod mapping;

pub use address_space::{
    count, init_kernel_address_space, kernel_address_space, kernel_address_space_mut, lookup_root,
    register, AddressSpace, AddressSpaceFlags, AddressSpaceId, VmmError, KERNEL_ADDRESS_SPACE_ID,
};
pub use mapping::{Mapping, MappingSet, MemoryObjectId, VirtRange};
