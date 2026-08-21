// ---------------------------------------------------------------------------
// VMM — Virtual Memory Manager (Stage 5)
//
// Architecture-neutral address space abstraction.
// Root page table is an opaque handle (arch-api::mmu::RootPageTable).
// ---------------------------------------------------------------------------

pub mod address_space;
pub mod faults;
pub mod mapping;
pub mod tables;

pub use address_space::{
    AddressSpace, AddressSpaceFlags, AddressSpaceId, KERNEL_ADDRESS_SPACE_ID, USER_VA_BASE,
    USER_VA_END, VmmError, address_space_mut_by, count, init_kernel_address_space,
    kernel_address_space, kernel_address_space_mut, lookup_root, register, unregister,
};
pub use mapping::{Mapping, MappingSet, MemoryObjectId, VirtRange};
