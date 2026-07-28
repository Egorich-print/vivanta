use crate::barrier;

// ── AArch64 VMSAv8-64 descriptor constants (Stage 1, 4 KB granule) ───────────

const DESC_VALID: u64 = 1 << 0;
const DESC_TABLE: u64 = 1 << 1;
const DESC_AF: u64 = 1 << 10;
const DESC_SH_INNER: u64 = 3 << 8;
const DESC_PXN: u64 = 1 << 53;
const DESC_XN: u64 = 1 << 54;

const ADDR_MASK: u64 = 0x0000_FFFF_FFFF_F000;
const ADDR_MASK_BLOCK: u64 = 0x0000_FFFF_FFE0_0000;

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

// ── Page table handle ────────────────────────────────────────────────────────

pub struct PageTable {
    root: u64,
}

impl PageTable {
    pub const fn new(root_pa: u64) -> Self {
        PageTable { root: root_pa }
    }

    pub fn root_addr(&self) -> u64 {
        self.root
    }

    fn descriptor_at(table_pa: u64, index: usize) -> u64 {
        unsafe { core::ptr::read_volatile((table_pa + (index as u64) * 8) as *const u64) }
    }

    fn set_descriptor(table_pa: u64, index: usize, value: u64) {
        unsafe {
            core::ptr::write_volatile((table_pa + (index as u64) * 8) as *mut u64, value);
        }
        barrier::dsb_ishst();
    }

    pub fn apply(&self, mapping: &Mapping) {
        self.map_region(mapping.va, mapping.pa, mapping.size, mapping.flags);
    }

    pub fn map(&self, va: u64, pa: u64, flags: MappingFlags) {
        let l1_idx = ((va >> 30) & 0x1FF) as usize;
        let l2_idx = ((va >> 21) & 0x1FF) as usize;
        let l3_idx = ((va >> 12) & 0x1FF) as usize;

        let l1_entry = Self::descriptor_at(self.root, l1_idx);
        assert!(l1_entry & DESC_VALID != 0, "PageTable::map: L1 entry missing");
        let l2_table = l1_entry & ADDR_MASK;

        let aligned_2mb = (va & 0x1F_FFFF) == 0 && (pa & 0x1F_FFFF) == 0;

        if aligned_2mb {
            Self::set_descriptor(l2_table, l2_idx, flags.to_descriptor_bits(pa, true));
        } else {
            let l2_entry = Self::descriptor_at(l2_table, l2_idx);
            assert!(l2_entry & DESC_VALID != 0, "PageTable::map: L2 entry missing");
            let l3_table = l2_entry & ADDR_MASK;
            Self::set_descriptor(l3_table, l3_idx, flags.to_descriptor_bits(pa, false));
        }
    }

    pub fn unmap(&self, va: u64) {
        let l1_idx = ((va >> 30) & 0x1FF) as usize;
        let l2_idx = ((va >> 21) & 0x1FF) as usize;
        let l3_idx = ((va >> 12) & 0x1FF) as usize;

        let l1_entry = Self::descriptor_at(self.root, l1_idx);
        if l1_entry & DESC_VALID == 0 { return; }
        let l2_table = l1_entry & ADDR_MASK;
        let l2_entry = Self::descriptor_at(l2_table, l2_idx);
        if l2_entry & DESC_VALID == 0 { return; }

        if l2_entry & DESC_TABLE == 0 {
            Self::set_descriptor(l2_table, l2_idx, 0);
        } else {
            let l3_table = l2_entry & ADDR_MASK;
            Self::set_descriptor(l3_table, l3_idx, 0);
        }
    }

    pub fn translate(&self, va: u64) -> Option<u64> {
        let l1_idx = ((va >> 30) & 0x1FF) as usize;
        let l2_idx = ((va >> 21) & 0x1FF) as usize;
        let l3_idx = ((va >> 12) & 0x1FF) as usize;

        let l1_entry = Self::descriptor_at(self.root, l1_idx);
        if l1_entry & DESC_VALID == 0 { return None; }
        let l2_table = l1_entry & ADDR_MASK;
        let l2_entry = Self::descriptor_at(l2_table, l2_idx);
        if l2_entry & DESC_VALID == 0 { return None; }

        if l2_entry & DESC_TABLE == 0 {
            let block_base = l2_entry & ADDR_MASK_BLOCK;
            return Some(block_base | (va & 0x1F_FFFF));
        }

        let l3_table = l2_entry & ADDR_MASK;
        let l3_entry = Self::descriptor_at(l3_table, l3_idx);
        if l3_entry & DESC_VALID == 0 { return None; }
        Some((l3_entry & ADDR_MASK) | (va & 0xFFF))
    }

    pub fn map_region(&self, va_start: u64, pa_start: u64, size: u64, flags: MappingFlags) {
        let mut offset = 0;
        while offset < size {
            let va = va_start + offset;
            let pa = pa_start + offset;

            if (va & 0x1F_FFFF) == 0 && (pa & 0x1F_FFFF) == 0 && (size - offset) >= 0x20_0000 {
                let l1_idx = ((va >> 30) & 0x1FF) as usize;
                let l2_idx = ((va >> 21) & 0x1FF) as usize;

                let l1_entry = Self::descriptor_at(self.root, l1_idx);
                assert!(l1_entry & DESC_VALID != 0, "map_region: L1 missing");
                let l2_table = l1_entry & ADDR_MASK;
                Self::set_descriptor(l2_table, l2_idx, flags.to_descriptor_bits(pa, true));
                offset += 0x20_0000;
            } else {
                let l1_idx = ((va >> 30) & 0x1FF) as usize;
                let l2_idx = ((va >> 21) & 0x1FF) as usize;
                let l3_idx = ((va >> 12) & 0x1FF) as usize;

                let l1_entry = Self::descriptor_at(self.root, l1_idx);
                assert!(l1_entry & DESC_VALID != 0, "map_region: L1 missing");
                let l2_table = l1_entry & ADDR_MASK;
                let l2_entry = Self::descriptor_at(l2_table, l2_idx);
                assert!(l2_entry & DESC_VALID != 0, "map_region: L2 missing");
                let l3_table = l2_entry & ADDR_MASK;
                Self::set_descriptor(l3_table, l3_idx, flags.to_descriptor_bits(pa, false));
                offset += 0x1000;
            }
        }
    }
}
