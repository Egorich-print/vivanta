use crate::va::{PAGE_SIZE, VaAllocator, VaError};

// ---------------------------------------------------------------------------
// Invariant checker: allocated-set tracking independent of the allocator,
// so the tests verify the allocator against a naive model.
// ---------------------------------------------------------------------------

struct Model {
    base: u64,
    end: u64,
    allocated: heapless_set::AllocSet,
}

mod heapless_set {
    /// Fixed-capacity allocated-range set for the test model.
    pub struct AllocSet {
        pub starts: [u64; 4096],
        pub sizes: [u64; 4096],
        pub len: usize,
    }

    impl AllocSet {
        pub fn new() -> Self {
            AllocSet {
                starts: [0; 4096],
                sizes: [0; 4096],
                len: 0,
            }
        }
        #[allow(dead_code)]
        pub fn contains(&self, start: u64, size: u64) -> bool {
            (0..self.len).any(|i| self.starts[i] == start && self.sizes[i] == size)
        }
        pub fn overlaps(&self, start: u64, size: u64) -> bool {
            let end = start + size;
            (0..self.len).any(|i| self.starts[i] < end && start < self.starts[i] + self.sizes[i])
        }
        pub fn insert(&mut self, start: u64, size: u64) {
            assert!(self.len < 4096);
            self.starts[self.len] = start;
            self.sizes[self.len] = size;
            self.len += 1;
        }
        pub fn remove(&mut self, start: u64, size: u64) {
            for i in 0..self.len {
                if self.starts[i] == start && self.sizes[i] == size {
                    self.starts[i] = self.starts[self.len - 1];
                    self.sizes[i] = self.sizes[self.len - 1];
                    self.len -= 1;
                    return;
                }
            }
            panic!("model: range not tracked");
        }
    }
}

impl Model {
    fn new(base: u64, end: u64) -> Self {
        Model {
            base,
            end,
            allocated: heapless_set::AllocSet::new(),
        }
    }

    /// I1+I2: every live allocation is in-domain and disjoint from the others.
    fn check(&self) {
        for i in 0..self.allocated.len {
            let s = self.allocated.starts[i];
            let z = self.allocated.sizes[i];
            assert!(s >= self.base && s + z <= self.end, "I1 violated");
            assert!(s % PAGE_SIZE == 0, "I1 alignment");
            for j in (i + 1)..self.allocated.len {
                let s2 = self.allocated.starts[j];
                let z2 = self.allocated.sizes[j];
                assert!(
                    s + z <= s2 || s2 + z2 <= s,
                    "I2 overlap: {s:#x}+{z} vs {s2:#x}+{z2}"
                );
            }
        }
    }
}

const BASE: u64 = 0x0100_0000;
const END: u64 = 0x4000_0000;

#[test]
fn alloc_basic_and_alignment() {
    let mut va = VaAllocator::try_new(BASE, END).unwrap();
    let a = va.alloc(PAGE_SIZE, PAGE_SIZE).unwrap();
    assert_eq!(a, BASE);
    assert!(!va.is_free(a));
    assert!(va.is_free(a + PAGE_SIZE));

    // Alignment honoured.
    let b = va.alloc(PAGE_SIZE * 3, PAGE_SIZE * 16).unwrap();
    assert_eq!(b % (PAGE_SIZE * 16), 0);

    // Deterministic exhaustion is impossible here, but zero-size is invalid.
    assert_eq!(va.alloc(0, PAGE_SIZE), Err(VaError::InvalidRange));
    // Bad alignment rejected before mutation.
    assert_eq!(va.alloc(PAGE_SIZE, 3), Err(VaError::Unaligned));
    assert_eq!(va.alloc(PAGE_SIZE, 0), Err(VaError::Unaligned));
}

#[test]
fn free_merge_and_double_free() {
    let mut va = VaAllocator::try_new(BASE, END).unwrap();
    let mut ranges = [0u64; 4];
    for r in ranges.iter_mut() {
        *r = va.alloc(PAGE_SIZE, PAGE_SIZE).unwrap();
    }
    // Freeing all four must merge back into one interval (I3).
    for r in &ranges {
        va.free(*r, PAGE_SIZE).unwrap();
    }
    assert_eq!(va.free_range_count(), 1);
    // Double free rejected (I6).
    assert_eq!(va.free(ranges[0], PAGE_SIZE), Err(VaError::DoubleFree));
    // Foreign range rejected.
    assert_eq!(va.free(0x8000_0000, PAGE_SIZE), Err(VaError::ForeignRange));
    assert_eq!(
        va.free(BASE - PAGE_SIZE, PAGE_SIZE),
        Err(VaError::ForeignRange)
    );
    // Unaligned / overflow rejects.
    assert_eq!(va.free(BASE + 1, PAGE_SIZE), Err(VaError::Unaligned));
    assert_eq!(
        va.free(0xFFFF_FFFF_FFFF_F000, PAGE_SIZE * 8),
        Err(VaError::Overflow)
    );
}

#[test]
fn reserve_marks_legacy_regions() {
    let mut va = VaAllocator::try_new(BASE, END).unwrap();
    va.reserve(BASE, PAGE_SIZE * 2).unwrap();
    // Reserved space is not handed out again.
    let next = va.alloc(PAGE_SIZE, PAGE_SIZE).unwrap();
    assert_eq!(next, BASE + PAGE_SIZE * 2);
    // Reserving an already-allocated range fails deterministically.
    assert_eq!(va.reserve(next, PAGE_SIZE), Err(VaError::DoubleFree));
    // Out-of-domain reservation rejected.
    assert_eq!(va.reserve(END, PAGE_SIZE), Err(VaError::ForeignRange));
}

#[test]
fn out_of_space_is_deterministic() {
    let mut va = VaAllocator::try_new(0x1000, 0x1000 + PAGE_SIZE * 4).unwrap();
    assert!(va.alloc(PAGE_SIZE * 4, PAGE_SIZE).is_ok());
    assert_eq!(va.alloc(PAGE_SIZE, PAGE_SIZE), Err(VaError::OutOfSpace));
    // State unchanged by the failed request.
    assert_eq!(va.free_range_count(), 0);
}

#[test]
fn overflow_safe() {
    // Domain near u64::MAX: allocation that would wrap must be rejected.
    let hi_base = (u64::MAX - PAGE_SIZE * 8) & !(PAGE_SIZE - 1);
    let hi_end = u64::MAX - PAGE_SIZE * 4 & !(PAGE_SIZE - 1);
    let mut va = VaAllocator::try_new(hi_base, hi_end).unwrap();
    assert_eq!(
        va.alloc(PAGE_SIZE * 1024, PAGE_SIZE),
        Err(VaError::OutOfSpace)
    );
}

/// Phase-15 deterministic lifecycle stress: thousands of reserve/map-style
/// allocations with interleaved frees, model-checked after every operation.
#[test]
fn repro_drain_fragmentation() {
    // Minimal deterministic reproduction of the drain-count bug.
    let mut va = VaAllocator::try_new(BASE, END).unwrap();
    let mut live: heapless_set::AllocSet = heapless_set::AllocSet::new();
    let mut state: u64 = 0x1234_5678_9ABC_DEF0;
    let mut next = move || {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        state
    };
    for _iter in 0..20_000u32 {
        let pages = 1u64 << (next() % 5);
        match next() % 3 {
            0 | 1 if live.len < 192 => {
                let align = PAGE_SIZE << (next() % 3);
                if let Ok(start) = va.alloc(pages * PAGE_SIZE, align) {
                    live.insert(start, pages * PAGE_SIZE);
                }
            }
            _ => {
                if live.len > 0 {
                    let idx = (next() % live.len as u64) as usize;
                    let s = live.starts[idx];
                    let z = live.sizes[idx];
                    va.free(s, z).unwrap();
                    live.remove(s, z);
                }
            }
        }
    }
    while live.len > 0 {
        let s = live.starts[0];
        let z = live.sizes[0];
        va.free(s, z).unwrap();
        live.remove(s, z);
    }
    let count = va.free_range_count();
    if count != 1 {
        let mut ivals = [(0u64, 0u64); 8];
        for (i, iv) in ivals.iter_mut().enumerate().take(count.min(8)) {
            *iv = crate::va::debug_interval(&va, i).unwrap_or((0, 0));
        }
        panic!("count={count} intervals={ivals:?}");
    }
}

#[test]
fn lifecycle_stress_model_checked() {
    let mut va = VaAllocator::try_new(BASE, END).unwrap();
    let mut model = Model::new(BASE, END);
    // Deterministic LCG so failures reproduce exactly.
    let mut state: u64 = 0x1234_5678_9ABC_DEF0;
    let mut next = move || {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        state
    };

    for _iter in 0..20_000u32 {
        let pages_pow = (next() % 5) as u32; // 1..=16 pages
        let pages = 1u64 << pages_pow;
        match next() % 3 {
            0 | 1 if model.allocated.len < 192 => {
                let align_pow = (next() % 3) as u32; // 4K/16K/64K
                let align = PAGE_SIZE << align_pow;
                if let Ok(start) = va.alloc(pages * PAGE_SIZE, align) {
                    // Model must agree the range was free.
                    assert!(
                        !model.allocated.overlaps(start, pages * PAGE_SIZE),
                        "iter {iter}: allocator handed out overlapping {start:#x}"
                    );
                    model.allocated.insert(start, pages * PAGE_SIZE);
                }
            }
            _ => {
                if model.allocated.len > 0 {
                    let idx = (next() % model.allocated.len as u64) as usize;
                    let s = model.allocated.starts[idx];
                    let z = model.allocated.sizes[idx];
                    va.free(s, z).unwrap_or_else(|e| {
                        panic!("iter {iter}: free of live {s:#x}+{z} failed: {e:?}")
                    });
                    model.allocated.remove(s, z);
                }
            }
        }
        if iter % 512 == 0 {
            model.check();
        }
    }
    model.check();

    // Drain: everything freed back → single merged interval spanning domain.
    while model.allocated.len > 0 {
        let s = model.allocated.starts[0];
        let z = model.allocated.sizes[0];
        va.free(s, z).unwrap();
        model.allocated.remove(s, z);
    }
    assert_eq!(va.free_range_count(), 1);
    let (b, e) = va.domain();
    let only_free = va.is_free(b) && va.is_free(e - PAGE_SIZE);
    assert!(only_free, "domain not fully restored after drain");
}

#[test]
fn high_water_bounds_reverse_scan() {
    let mut va = VaAllocator::try_new(BASE, END).unwrap();
    assert_eq!(va.high_water(), BASE); // untouched domain
    let a = va.alloc(PAGE_SIZE * 2, PAGE_SIZE).unwrap();
    assert_eq!(va.high_water(), a + PAGE_SIZE * 2); // allocation raises it
    va.free(a, PAGE_SIZE * 2).unwrap();
    // Freeing does NOT lower it: stale descriptors may linger there.
    assert_eq!(va.high_water(), a + PAGE_SIZE * 2);
    // A later allocation further out raises again.
    let b = va.alloc(PAGE_SIZE * 8, PAGE_SIZE).unwrap();
    assert_eq!(va.high_water(), b + PAGE_SIZE * 8);
}
