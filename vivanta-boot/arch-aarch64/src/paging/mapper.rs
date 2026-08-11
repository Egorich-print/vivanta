use crate::barrier;
use crate::paging::descriptor::*;
use crate::paging::walker;
use crate::paging::{Mapping, MappingFlags};

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
        walker::read_desc(table_pa + (index as u64) * 8)
    }

    fn set_descriptor(table_pa: u64, index: usize, value: u64) {
        walker::write_desc(table_pa + (index as u64) * 8, value);
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
        assert!(
            l1_entry & DESC_VALID != 0,
            "PageTable::map: L1 entry missing"
        );
        let l2_table = l1_entry & ADDR_MASK;

        let aligned_2mb = (va & 0x1F_FFFF) == 0 && (pa & 0x1F_FFFF) == 0;

        if aligned_2mb {
            Self::set_descriptor(l2_table, l2_idx, flags.to_descriptor_bits(pa, true));
        } else {
            let l2_entry = Self::descriptor_at(l2_table, l2_idx);
            assert!(
                l2_entry & DESC_VALID != 0,
                "PageTable::map: L2 entry missing"
            );
            let l3_table = l2_entry & ADDR_MASK;
            Self::set_descriptor(l3_table, l3_idx, flags.to_descriptor_bits(pa, false));
        }
    }

    pub fn unmap(&self, va: u64) {
        let l1_idx = ((va >> 30) & 0x1FF) as usize;
        let l2_idx = ((va >> 21) & 0x1FF) as usize;
        let l3_idx = ((va >> 12) & 0x1FF) as usize;

        let l1_entry = Self::descriptor_at(self.root, l1_idx);
        if l1_entry & DESC_VALID == 0 {
            return;
        }
        let l2_table = l1_entry & ADDR_MASK;
        let l2_entry = Self::descriptor_at(l2_table, l2_idx);
        if l2_entry & DESC_VALID == 0 {
            return;
        }

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
        if l1_entry & DESC_VALID == 0 {
            return None;
        }
        let l2_table = l1_entry & ADDR_MASK;
        let l2_entry = Self::descriptor_at(l2_table, l2_idx);
        if l2_entry & DESC_VALID == 0 {
            return None;
        }

        if l2_entry & DESC_TABLE == 0 {
            let block_base = l2_entry & ADDR_MASK_BLOCK;
            return Some(block_base | (va & 0x1F_FFFF));
        }

        let l3_table = l2_entry & ADDR_MASK;
        let l3_entry = Self::descriptor_at(l3_table, l3_idx);
        if l3_entry & DESC_VALID == 0 {
            return None;
        }
        Some((l3_entry & ADDR_MASK) | (va & 0xFFF))
    }

    /// Return the raw leaf descriptor covering `va` (L2 block or L3 page),
    /// without translating to a physical address.
    ///
    /// Used by `access_ok` to validate user access permissions against the
    /// live page table. Never allocates and never modifies tables.
    pub fn leaf_descriptor(&self, va: u64) -> Option<u64> {
        let l1_idx = ((va >> 30) & 0x1FF) as usize;
        let l2_idx = ((va >> 21) & 0x1FF) as usize;
        let l3_idx = ((va >> 12) & 0x1FF) as usize;

        let l1_entry = Self::descriptor_at(self.root, l1_idx);
        if l1_entry & DESC_VALID == 0 {
            return None;
        }
        if l1_entry & DESC_TABLE == 0 {
            return None;
        }
        let l2_table = l1_entry & ADDR_MASK;
        let l2_entry = Self::descriptor_at(l2_table, l2_idx);
        if l2_entry & DESC_VALID == 0 {
            return None;
        }

        if l2_entry & DESC_TABLE == 0 {
            return Some(l2_entry);
        }

        let l3_table = l2_entry & ADDR_MASK;
        let l3_entry = Self::descriptor_at(l3_table, l3_idx);
        if l3_entry & DESC_VALID == 0 {
            return None;
        }
        Some(l3_entry)
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
