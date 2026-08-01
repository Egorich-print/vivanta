// ---------------------------------------------------------------------------
// VMM — Virtual Memory Manager (Stage 5)
//
// Architecture-neutral address space abstraction.
// Root page table is an opaque handle (arch-api::mmu::RootPageTable).
// ---------------------------------------------------------------------------

pub mod address_space;
pub mod mapping;
pub mod faults;

pub use address_space::{AddressSpace, AddressSpaceFlags, AddressSpaceId,
                        KERNEL_ADDRESS_SPACE_ID, VmmError,
                        kernel_address_space, kernel_address_space_mut,
                        init_kernel_address_space, register,
                        lookup_root, count};
pub use mapping::{Mapping, MappingSet, VirtRange, MemoryObjectId};
