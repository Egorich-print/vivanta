use vivanta_arch_api::mmu::RootPageTable;
use super::mapping::MappingSet;

pub type AddressSpaceId = u64;

pub const KERNEL_ADDRESS_SPACE_ID: AddressSpaceId = 0;
const MAX_ADDRESS_SPACES: usize = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AddressSpaceFlags {
    Kernel,
    User,
}

#[derive(Clone, Copy)]
pub struct AddressSpace {
    pub id: AddressSpaceId,
    pub root: RootPageTable,
    pub mappings: MappingSet,
    pub flags: AddressSpaceFlags,
}

impl AddressSpace {
    pub fn new(id: AddressSpaceId, root: RootPageTable, flags: AddressSpaceFlags) -> Self {
        Self {
            id,
            root,
            mappings: MappingSet::new(),
            flags,
        }
    }

    pub fn is_kernel(&self) -> bool {
        self.flags == AddressSpaceFlags::Kernel
    }
}

static mut ADDRESS_SPACES: [Option<AddressSpace>; MAX_ADDRESS_SPACES] = [
    None, None, None, None, None, None, None, None,
];
static mut NEXT_AS_ID: AddressSpaceId = 1;

pub fn init_kernel_address_space(root: RootPageTable) {
    unsafe {
        ADDRESS_SPACES[0] = Some(AddressSpace::new(0, root, AddressSpaceFlags::Kernel));
    }
}

pub fn register(root: RootPageTable, flags: AddressSpaceFlags) -> AddressSpaceId {
    unsafe {
        let id = NEXT_AS_ID;
        NEXT_AS_ID += 1;
        for i in 0..MAX_ADDRESS_SPACES {
            let ptr = &raw mut ADDRESS_SPACES[i];
            if (*ptr).is_none() {
                *ptr = Some(AddressSpace::new(id, root, flags));
                return id;
            }
        }
    }
    panic!("address space registry full");
}

fn lookup(as_id: AddressSpaceId) -> &'static AddressSpace {
    unsafe {
        for i in 0..MAX_ADDRESS_SPACES {
            let ptr = &raw const ADDRESS_SPACES[i];
            if let Some(ref aspace) = (*ptr).as_ref() {
                if aspace.id == as_id {
                    return aspace;
                }
            }
        }
    }
    panic!("lookup: unknown AddressSpaceId {}", as_id);
}

pub fn lookup_root(as_id: AddressSpaceId) -> RootPageTable {
    lookup(as_id).root
}

pub fn kernel_address_space() -> &'static AddressSpace {
    unsafe {
        ADDRESS_SPACES[0].as_ref().expect("KernelAddressSpace not initialised")
    }
}

pub fn count() -> usize {
    unsafe {
        let mut n = 0;
        for i in 0..MAX_ADDRESS_SPACES {
            if ADDRESS_SPACES[i].is_some() {
                n += 1;
            }
        }
        n
    }
}
