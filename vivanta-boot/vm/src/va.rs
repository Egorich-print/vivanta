//! Virtual address allocator.
//!
//! Invariants (checked by the host test-suite, thousands of lifecycle ops):
//! - I1  Every allocated range lies inside `[base, end)` and is page-aligned.
//! - I2  Allocated ranges never overlap each other.
//! - I3  Free ranges are disjoint and sorted; adjacent free ranges are merged,
//!       so the free list is a canonical interval representation.
//! - I4  `alloc` is first-fit by address with explicit alignment; failure is
//!       deterministic (`VaError::OutOfSpace`) and mutates nothing.
//! - I5  All arithmetic is overflow-checked; `base`/`end`/`size`/`align`
//!       violations are rejected before any state change.
//! - I6  `free` of a range that overlaps any *free* range is rejected
//!       (`DoubleFree` / `ForeignRange`), so double-free and foreign-range
//!       frees cannot corrupt the interval structure.
//! - I7  The allocator never hands out addresses across its domain boundary;
//!       user/kernel separation is enforced by constructing separate
//!       allocators with disjoint domains.

/// Page granularity for all Vivanta VA allocations.
pub const PAGE_SIZE: u64 = 4096;

/// Maximum number of free intervals tracked. Exhaustion is a deterministic
/// error, never silent corruption: at kernel scale (a handful of mappings)
/// this bound is far beyond reach, and overflow of the free list leaves
/// allocated state untouched.
pub const MAX_FREE_RANGES: usize = 256;

/// A half-open virtual range `[start, start + size)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VaRegion {
    pub start: u64,
    pub size: u64,
}

impl VaRegion {
    /// Checked constructor: rejects empty ranges, unaligned starts and
    /// end-of-address-space overflow.
    pub fn new(start: u64, size: u64) -> Result<Self, VaError> {
        if size == 0 {
            return Err(VaError::InvalidRange);
        }
        if start % PAGE_SIZE != 0 || size % PAGE_SIZE != 0 {
            return Err(VaError::Unaligned);
        }
        let end = start.checked_add(size).ok_or(VaError::Overflow)?;
        if end > u64::MAX - PAGE_SIZE {
            return Err(VaError::Overflow);
        }
        Ok(VaRegion { start, size })
    }

    pub fn end(&self) -> u64 {
        self.start + self.size
    }

    fn overlaps(&self, other_start: u64, other_end: u64) -> bool {
        self.start < other_end && other_start < self.end()
    }
}

/// Deterministic allocator errors. No error mutates allocator state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VaError {
    InvalidRange,
    Unaligned,
    Overflow,
    OutOfSpace,
    FreeListFull,
    DoubleFree,
    ForeignRange,
}

/// First-fit virtual address allocator over a fixed domain.
///
/// The free list is the single source of truth: a range is "allocated" iff
/// it lies inside the domain and intersects no free interval. This makes
/// overlap impossible by construction (I2) and keeps every operation O(n)
/// with n = `MAX_FREE_RANGES`.
#[derive(Debug, Clone, Copy)]
pub struct VaAllocator {
    base: u64,
    end: u64,
    free: [Option<VaRegion>; MAX_FREE_RANGES],
    used: usize,
    /// Historical maximum end of any allocated/reserved range.
    mapped_water: u64,
}

impl VaAllocator {
    /// Create an allocator for `[base, end)` with the whole domain free.
    /// Both bounds must be page-aligned and `end > base`.
    pub fn try_new(base: u64, end: u64) -> Result<Self, VaError> {
        if base % PAGE_SIZE != 0 || end % PAGE_SIZE != 0 || end <= base {
            return Err(VaError::InvalidRange);
        }
        let mut free = [None; MAX_FREE_RANGES];
        free[0] = Some(VaRegion {
            start: base,
            size: end - base,
        });
        Ok(VaAllocator {
            base,
            end,
            free,
            used: 1,
            mapped_water: base,
        })
    }

    /// An allocator with an empty domain: every request fails
    /// deterministically. Used for address spaces (the kernel AS) whose
    /// translations are managed outside the allocator.
    pub fn disabled() -> Self {
        VaAllocator {
            base: 0,
            end: 0,
            free: [None; MAX_FREE_RANGES],
            used: 0,
            mapped_water: 0,
        }
    }

    pub fn is_disabled(&self) -> bool {
        self.base == self.end
    }

    pub fn domain(&self) -> (u64, u64) {
        (self.base, self.end)
    }

    /// Mark `[start, start+size)` as allocated without handing it out —
    /// used to reserve legacy/boot regions inside the domain.
    pub fn reserve(&mut self, start: u64, size: u64) -> Result<(), VaError> {
        let region = VaRegion::new(start, size)?;
        if region.start < self.base || region.end() > self.end {
            return Err(VaError::ForeignRange);
        }
        // Carve out of the free list.
        let mut carved = 0usize;
        let mut i = 0;
        while i < self.used {
            let r = self.free[i].expect("free slot invariant");
            if region.overlaps(r.start, r.end()) {
                // Split r into the parts outside `region`.
                let head = region.start > r.start;
                let tail = region.end() < r.end();
                match (head, tail) {
                    (true, true) => {
                        if self.used >= MAX_FREE_RANGES {
                            return Err(VaError::FreeListFull);
                        }
                        let lo = VaRegion {
                            start: r.start,
                            size: region.start - r.start,
                        };
                        let hi = VaRegion {
                            start: region.end(),
                            size: r.end() - region.end(),
                        };
                        self.free[i] = Some(lo);
                        self.insert_at(self.used, hi);
                        self.used += 1;
                        carved += 1;
                        i += 1;
                    }
                    (true, false) => {
                        self.free[i] = Some(VaRegion {
                            start: r.start,
                            size: region.start - r.start,
                        });
                        carved += 1;
                    }
                    (false, true) => {
                        self.free[i] = Some(VaRegion {
                            start: region.end(),
                            size: r.end() - region.end(),
                        });
                        carved += 1;
                    }
                    (false, false) => {
                        self.remove_at(i);
                        carved += 1;
                        continue;
                    }
                }
            }
            i += 1;
        }
        if carved == 0 {
            // Nothing free covered any part: either already fully allocated
            // or outside the domain — both make the reservation invalid.
            return Err(VaError::DoubleFree);
        }
        self.normalize();
        self.raise_water(region.end());
        Ok(())
    }

    /// Allocate `size` bytes with `align` (power of two, multiple of page).
    /// First-fit by address. On failure nothing changes.
    pub fn alloc(&mut self, size: u64, align: u64) -> Result<u64, VaError> {
        if size == 0 {
            return Err(VaError::InvalidRange);
        }
        if align == 0 || !align.is_power_of_two() || align % PAGE_SIZE != 0 {
            return Err(VaError::Unaligned);
        }
        let rounded = size.div_ceil(PAGE_SIZE) * PAGE_SIZE;
        for i in 0..self.used {
            let r = self.free[i].expect("free slot invariant");
            let aligned_start = match round_up(r.start, align) {
                Some(a) => a,
                // Rounding past the address space: this range cannot serve
                // the request; keep scanning.
                None => continue,
            };
            let candidate_end = match aligned_start.checked_add(rounded) {
                Some(e) => e,
                // Would wrap past u64: cannot fit here — deterministic
                // OutOfSpace if nothing else fits, never a spurious error.
                None => continue,
            };
            if aligned_start < r.start || candidate_end > r.end() {
                continue;
            }
            // Carve the allocation out of this free range.
            let head = aligned_start > r.start;
            let tail = candidate_end < r.end();
            match (head, tail) {
                (true, true) => {
                    if self.used >= MAX_FREE_RANGES {
                        continue; // try next range rather than fail outright
                    }
                    let lo = VaRegion {
                        start: r.start,
                        size: aligned_start - r.start,
                    };
                    let hi = VaRegion {
                        start: candidate_end,
                        size: r.end() - candidate_end,
                    };
                    self.free[i] = Some(lo);
                    self.insert_at(self.used, hi);
                    self.used += 1;
                }
                (true, false) => {
                    self.free[i] = Some(VaRegion {
                        start: r.start,
                        size: aligned_start - r.start,
                    });
                }
                (false, true) => {
                    self.free[i] = Some(VaRegion {
                        start: candidate_end,
                        size: r.end() - candidate_end,
                    });
                }
                (false, false) => {
                    self.remove_at(i);
                }
            }
            self.normalize();
            self.raise_water(candidate_end);
            return Ok(aligned_start);
        }
        Err(VaError::OutOfSpace)
    }

    /// Return `[start, start+size)` to the free list. Rejects double frees
    /// and ranges that were never allocated (overlap an existing free range)
    /// or lie outside the domain.
    pub fn free(&mut self, start: u64, size: u64) -> Result<(), VaError> {
        let region = VaRegion::new(start, size)?;
        if region.start < self.base || region.end() > self.end {
            return Err(VaError::ForeignRange);
        }
        // I6: must not intersect any existing free range.
        for i in 0..self.used {
            let r = self.free[i].expect("free slot invariant");
            if region.overlaps(r.start, r.end()) {
                return Err(VaError::DoubleFree);
            }
        }
        if self.used >= MAX_FREE_RANGES {
            return Err(VaError::FreeListFull);
        }
        self.insert_at(self.used, region);
        self.used += 1;
        self.normalize();
        self.raise_water(region.end());
        Ok(())
    }

    /// True when `addr` lies in a free interval (i.e. NOT allocated).
    pub fn is_free(&self, addr: u64) -> bool {
        if addr < self.base || addr >= self.end {
            return false;
        }
        (0..self.used).any(|i| {
            let r = self.free[i].expect("free slot invariant");
            addr >= r.start && addr < r.end()
        })
    }

    pub fn free_range_count(&self) -> usize {
        self.used
    }

    /// Historical high-water mark of allocation: descriptors can only
    /// exist below this address, because leaves are installed exclusively
    /// for ranges handed out by `alloc`/`reserve`. Bounds the reverse
    /// hardware scan (INV-VM-001 reverse direction).
    pub fn high_water(&self) -> u64 {
        self.mapped_water
    }

    fn raise_water(&mut self, end: u64) {
        if end > self.mapped_water {
            self.mapped_water = end;
        }
    }

    // -- internals --------------------------------------------------------

    fn insert_at(&mut self, idx: usize, r: VaRegion) {
        self.free[idx] = Some(r);
    }

    fn remove_at(&mut self, idx: usize) {
        // Compact: move last live entry into the hole.
        let last = self.used - 1;
        self.free[idx] = self.free[last];
        self.free[last] = None;
        self.used = last;
    }

    /// Sort free ranges by start and merge adjacent ones, restoring I3.
    fn normalize(&mut self) {
        // Insertion sort (n ≤ 128).
        for i in 1..self.used {
            let key = self.free[i];
            let mut j = i;
            while j > 0 {
                let prev = self.free[j - 1].expect("live slot");
                let cur = key.expect("live slot");
                if prev.start <= cur.start {
                    break;
                }
                self.free[j] = self.free[j - 1];
                j -= 1;
            }
            self.free[j] = key;
        }
        // Merge adjacent runs via write-index compaction. A swap-remove
        // here would drag an arbitrary tail element into the scan window
        // and silently skip merges (observed as fragmented drains).
        if self.used == 0 {
            return;
        }
        let mut w = 0usize; // index of the current output run
        for r in 1..self.used {
            let cur = self.free[r].expect("live slot");
            let top = self.free[w].expect("live slot");
            if top.end() == cur.start {
                self.free[w] = Some(VaRegion {
                    start: top.start,
                    size: top.size + cur.size,
                });
            } else {
                w += 1;
                self.free[w] = Some(cur);
            }
        }
        for k in (w + 1)..MAX_FREE_RANGES {
            self.free[k] = None;
        }
        self.used = w + 1;
    }
}

fn round_up(v: u64, align: u64) -> Option<u64> {
    debug_assert!(align.is_power_of_two());
    let mask = align - 1;
    // Already-aligned values must round up to themselves without going
    // through checked_add — near-u64::MAX addresses would otherwise
    // report a spurious overflow.
    if v & mask == 0 {
        return Some(v);
    }
    v.checked_add(mask).map(|x| x & !mask)
}

/// Test-only introspection: the i-th live free interval.
#[doc(hidden)]
pub fn debug_interval(va: &VaAllocator, i: usize) -> Option<(u64, u64)> {
    va.free.get(i).copied().flatten().map(|r| (r.start, r.size))
}
