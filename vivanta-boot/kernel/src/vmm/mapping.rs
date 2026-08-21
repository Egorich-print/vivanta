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
        // Reuse the first hole left by remove(); only grow `count` when no
        // hole exists. This keeps capacity usable across insert/remove churn.
        for (i, slot) in self.mappings.iter_mut().enumerate() {
            if slot.is_none() {
                *slot = Some(mapping);
                if i >= self.count {
                    self.count = i + 1;
                }
                return Some(i);
            }
        }
        None
    }

    pub fn remove(&mut self, slot: usize) {
        if slot < self.count {
            self.mappings[slot] = None;
            // Shrink count while the tail is empty so `len()` reflects live
            // mappings and a fresh insert can reuse the freed slot.
            while self.count > 0 && self.mappings[self.count - 1].is_none() {
                self.count -= 1;
            }
        }
    }

    pub fn get(&self, slot: usize) -> Option<&Mapping> {
        if slot < self.count {
            self.mappings[slot].as_ref()
        } else {
            None
        }
    }

    pub fn get_mut(&mut self, slot: usize) -> Option<&mut Mapping> {
        if slot < self.count {
            self.mappings[slot].as_mut()
        } else {
            None
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &Mapping> {
        self.mappings[..self.count]
            .iter()
            .filter_map(|m| m.as_ref())
    }

    /// Iterate live mappings together with their slot indices.
    pub fn iter_with_slots(&self) -> impl Iterator<Item = (usize, &Mapping)> {
        self.mappings[..self.count]
            .iter()
            .enumerate()
            .filter_map(|(i, m)| m.as_ref().map(|m| (i, m)))
    }

    pub fn len(&self) -> usize {
        self.count
    }

    pub const fn capacity() -> usize {
        MAX_MAPPINGS
    }
}
