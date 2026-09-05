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

/// Backing lifecycle of a mapping piece (ADR-032 §1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Backing {
    /// Hardware mapping exists for the entire piece.
    Present,
    /// Hardware mapping exists (readable, write-suppressed); a WRITE
    /// permission fault triggers copy-on-write resolution (ADR-034).
    CoW,
    /// No hardware mapping; first access demand-fills exactly one page
    /// from anonymous memory (PMM frame, zeroed).
    LazyAnonymous,
    /// Reservation only; no automatic fill; access faults fatally.
    Reserved,
}

/// Physical-frame ownership of a Present piece (ADR-032 §4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PhysOwnership {
    /// PA provided by the caller / MemoryObject — VMM never frees it.
    External,
    /// PA allocated by the VM layer for this piece; released to the PMM
    /// when the piece is unmapped. Cannot be aliased (PA never published).
    Anonymous,
    /// PA shared by exactly `refcount` mappings via COW (ADR-034).
    /// The last owner to unmap receives the physical frame. The shadow
    /// refcount is the single authority for "how many owners remain".
    CoWShared { refcount: u32 },
}

#[derive(Clone, Copy, Debug)]
pub struct Mapping {
    pub virt_range: VirtRange,
    pub object_id: MemoryObjectId,
    pub permissions: MappingFlags,
    pub backing: Backing,
    /// Physical base for Present pieces; 0 otherwise.
    pub pa: u64,
    pub phys: PhysOwnership,
}

impl Mapping {
    /// Fully materialized, externally-owned mapping (classic path).
    pub const fn new(
        virt_range: VirtRange,
        object_id: MemoryObjectId,
        permissions: MappingFlags,
    ) -> Self {
        Self {
            virt_range,
            object_id,
            permissions,
            backing: Backing::Present,
            pa: 0,
            phys: PhysOwnership::External,
        }
    }

    /// Present piece with explicit physical base and ownership.
    pub const fn present(
        virt_range: VirtRange,
        object_id: MemoryObjectId,
        permissions: MappingFlags,
        pa: u64,
        phys: PhysOwnership,
    ) -> Self {
        Self {
            virt_range,
            object_id,
            permissions,
            backing: Backing::Present,
            pa,
            phys,
        }
    }

    /// Present piece shared via copy-on-write.
    pub const fn cow_shared(
        virt_range: VirtRange,
        object_id: MemoryObjectId,
        permissions: MappingFlags,
        pa: u64,
        refcount: u32,
    ) -> Self {
        Self {
            virt_range,
            object_id,
            permissions,
            backing: Backing::CoW,
            pa,
            phys: PhysOwnership::CoWShared { refcount },
        }
    }

    /// Lazy anonymous reservation (demand-filled on first access).
    pub const fn lazy_anonymous(
        virt_range: VirtRange,
        object_id: MemoryObjectId,
        permissions: MappingFlags,
    ) -> Self {
        Self {
            virt_range,
            object_id,
            permissions,
            backing: Backing::LazyAnonymous,
            pa: 0,
            phys: PhysOwnership::Anonymous,
        }
    }

    /// Pure reservation; accesses fault fatally.
    pub const fn reserved(
        virt_range: VirtRange,
        object_id: MemoryObjectId,
        permissions: MappingFlags,
    ) -> Self {
        Self {
            virt_range,
            object_id,
            permissions,
            backing: Backing::Reserved,
            pa: 0,
            phys: PhysOwnership::External,
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
            // Drop refcount if the removed mapping owned a CoWShared frame.
            if let Some(m) = self.mappings[slot] {
                release_cow_frame(m);
            }
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

    /// Debug accessor for boot-time diagnostics.
    pub fn mappings_debug(&self) -> impl Iterator<Item = &Mapping> {
        self.iter()
    }

    /// Remove a mapping by its virtual base address.
    pub fn remove_by_base(&mut self, base: u64) -> Option<Mapping> {
        let slot = (0..self.count).find(|&i| {
            self.mappings[i]
                .as_ref()
                .is_some_and(|m| m.virt_range.base == base)
        })?;
        let removed = self.mappings[slot];
        self.remove(slot);
        removed
    }

    /// Transactionally replace every mapping whose slot is in
    /// `affected_slots` with `pieces` (ADR-032 INV-VM-001).
    ///
    /// Index-independent: affected mappings are identified by VALUE, the
    /// surviving set is rebuilt from scratch, and capacity is verified
    /// before any mutation. Slot-index arithmetic after concurrent
    /// insert/remove cycles was observed to corrupt neighbouring pieces
    /// (wrong permissions at wrong addresses).
    pub fn replace_slots(
        &mut self,
        affected_values: &[Mapping],
        pieces: &[Mapping],
    ) -> Result<(), ()> {
        let survivors = self.count_minus_affected(affected_values);
        if survivors + pieces.len() > MAX_MAPPINGS {
            return Err(());
        }
        // Drop CoWShared refcounts for the about-to-be-replaced mappings
        // before the slots are overwritten — the registry is the single
        // authority for "how many owners remain" and must see the
        // decrement regardless of the new pieces' backing.
        for a in affected_values {
            release_cow_frame(*a);
        }
        let mut next = Self::new();
        for (_, m) in self.iter_with_slots() {
            if !affected_values.iter().any(|a| {
                a.virt_range.base == m.virt_range.base && a.virt_range.size == m.virt_range.size
            }) {
                next.insert(*m);
            }
        }
        for p in pieces {
            if next.insert(*p).is_none() {
                return Err(()); // cannot happen after the capacity check
            }
        }
        *self = next;
        Ok(())
    }

    fn count_minus_affected(&self, affected: &[Mapping]) -> usize {
        self.iter()
            .filter(|m| {
                !affected.iter().any(|a| {
                    a.virt_range.base == m.virt_range.base && a.virt_range.size == m.virt_range.size
                })
            })
            .count()
    }
}

/// Release one CoWShared frame reference. Called when a shadow piece
/// that owns a frame is removed (MappingSet::remove, replace_slots).
/// The registry counts owners across all address spaces — when the
/// count hits zero the physical frame returns to the boot-registered
/// deallocator.
fn release_cow_frame(m: Mapping) {
    if let PhysOwnership::CoWShared { .. } = m.phys {
        let _ = crate::vmm::cow_refcount::dec(m.pa);
    }
}
