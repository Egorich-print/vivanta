use vivanta_arch_api::mmu::MappingFlags;

pub type MemoryObjectId = u64;

#[derive(Clone, Copy, Debug)]
pub struct VirtRange {
    pub base: u64,
    pub size: u64,
}

impl VirtRange {
    pub const fn new(base: u64, size: u64) -> Self {
        Self { base, size }
    }

    pub fn end(&self) -> u64 {
        self.base + self.size
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Mapping {
    pub virt_range: VirtRange,
    pub object_id: MemoryObjectId,
    pub permissions: MappingFlags,
}

impl Mapping {
    pub const fn new(
        virt_range: VirtRange,
        object_id: MemoryObjectId,
        permissions: MappingFlags,
    ) -> Self {
        Self {
            virt_range,
            object_id,
            permissions,
        }
    }
}

const MAX_MAPPINGS: usize = 64;

#[derive(Clone, Copy, Debug)]
pub struct MappingSet {
    mappings: [Option<Mapping>; MAX_MAPPINGS],
    count: usize,
}

impl MappingSet {
    pub const fn new() -> Self {
        Self {
            mappings: [None; MAX_MAPPINGS],
            count: 0,
        }
    }

    pub fn insert(&mut self, mapping: Mapping) -> Option<usize> {
        if self.count >= MAX_MAPPINGS {
            return None;
        }
        let slot = self.count;
        self.mappings[slot] = Some(mapping);
        self.count += 1;
        Some(slot)
    }

    pub fn remove(&mut self, slot: usize) {
        if slot < self.count {
            self.mappings[slot] = None;
        }
    }

    pub fn get(&self, slot: usize) -> Option<&Mapping> {
        if slot < self.count {
            self.mappings[slot].as_ref()
        } else {
            None
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &Mapping> {
        self.mappings[..self.count]
            .iter()
            .filter_map(|m| m.as_ref())
    }

    pub fn len(&self) -> usize {
        self.count
    }
}
