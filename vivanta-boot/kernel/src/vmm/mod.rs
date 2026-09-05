// ---------------------------------------------------------------------------
// VMM — Virtual Memory Manager (Stage 5)
//
// Architecture-neutral address space abstraction.
// Root page table is an opaque handle (arch-api::mmu::RootPageTable).
// ---------------------------------------------------------------------------

pub mod address_space;
pub mod cow_refcount;
pub mod faults;
pub mod mapping;
pub mod tables;

pub use address_space::{
    AddressSpace, AddressSpaceFlags, AddressSpaceId, KERNEL_ADDRESS_SPACE_ID, USER_VA_BASE,
    USER_VA_END, VmmError, address_space_mut_by, count, find_by_root, init_kernel_address_space,
    kernel_address_space, kernel_address_space_mut, lookup_root, peek_next_as_id, register,
    register_child, unregister,
};
pub use faults::make_allocator;
pub use mapping::{Mapping, MappingSet, MemoryObjectId, VirtRange};
