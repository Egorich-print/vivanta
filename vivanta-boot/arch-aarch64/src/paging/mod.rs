pub mod descriptor;
pub mod walker;
pub mod mapper;

use descriptor::*;

// ── Memory type ──────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemoryType {
    Normal,
    Device,
}

impl MemoryType {
    fn to_attr_index(self) -> u64 {
        match self {
            MemoryType::Normal => 0,
            MemoryType::Device => 1,
        }
    }
}

// ── Permissions ──────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug)]
pub struct Permissions {
    pub readable: bool,
    pub writable: bool,
    pub executable: bool,
    pub user: bool,
}

impl Permissions {
    pub const fn kernel_rw() -> Self {
        Permissions { readable: true, writable: true, executable: false, user: false }
    }
    pub const fn kernel_rwx() -> Self {
        Permissions { readable: true, writable: true, executable: true, user: false }
    }
    pub const fn kernel_rx() -> Self {
        Permissions { readable: true, writable: false, executable: true, user: false }
    }
    pub const fn user_rw() -> Self {
        Permissions { readable: true, writable: true, executable: false, user: true }
    }
    pub const fn none() -> Self {
        Permissions { readable: false, writable: false, executable: false, user: false }
    }
}

// ── Combined mapping flags ───────────────────────────────────────────────────

#[derive(Clone, Copy, Debug)]
pub struct MappingFlags {
    pub perms: Permissions,
    pub mem_type: MemoryType,
}

impl MappingFlags {
    pub const fn normal(perms: Permissions) -> Self {
        MappingFlags { perms, mem_type: MemoryType::Normal }
    }
    pub const fn device(perms: Permissions) -> Self {
        MappingFlags { perms, mem_type: MemoryType::Device }
    }
    pub const fn identity() -> Self {
        MappingFlags::normal(Permissions::kernel_rwx())
    }

    fn to_descriptor_bits(self, phys: u64, is_block: bool) -> u64 {
        let mut d = DESC_VALID | DESC_AF | DESC_SH_INNER
            | (self.mem_type.to_attr_index() << 2);

        if !is_block {
            d |= DESC_TABLE;
        }

        if self.perms.user {
            d |= 1 << 6;
        } else if !self.perms.writable {
            d |= 2 << 6;
        }

        if !self.perms.executable {
            d |= DESC_PXN | DESC_XN;
        }

        let addr_mask = if is_block { ADDR_MASK_BLOCK } else { ADDR_MASK };
        d | (phys & addr_mask)
    }
}

// ── Mapping descriptor ───────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug)]
pub struct Mapping {
    pub va: u64,
    pub pa: u64,
    pub size: u64,
    pub flags: MappingFlags,
}

impl Mapping {
    pub const fn new(va: u64, pa: u64, size: u64, flags: MappingFlags) -> Self {
        Mapping { va, pa, size, flags }
    }
}

pub use mapper::PageTable;
