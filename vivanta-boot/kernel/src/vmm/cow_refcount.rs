//! Frame-level refcount registry for CoWShared physical frames (ADR-034).
//!
//! When a `Present` piece is duplicated via `fork()`, both parent and child
//! shadow entries become `CoWShared { refcount: N }` and point to the SAME
//! physical frame. The shadow's per-piece refcount is convenient but
//! insufficient as the single authority — when the LAST owning shadow piece
//! is removed (unmap, AS teardown, COW break), nobody decremented the count
//! and the frame leaks forever.
//!
//! This module provides a global, frame-keyed refcount registry that
//! outlives any individual address space. Every time a `CoWShared` shadow
//! piece is created or destroyed, the registry is updated. When the count
//! reaches zero, the frame is released to a kernel-registered deallocator
//! (see [`set_free_fn`]).
//!
//! Registry exhaustion degrades to the "leak model" (deterministic, safe) —
//! the frame stays mapped but its refcount is no longer tracked here.
//!
//! Safety: the kernel is single-core and runs with IRQs masked during
//! exception path resolution, so the `static mut` array is safe to access
//! from any VMM primitive without explicit locking. The same invariants
//! apply to `tables::TABLE_REGISTRY` (ADR-031).

pub const MAX_COW_FRAMES: usize = 256;

/// Frame deallocator. Called when a CoWShared frame's refcount reaches 0.
/// The kernel registers this at boot once the memory backend is known.
type FreeFn = fn(u64);

static mut FREE_FN: Option<FreeFn> = None;

/// Register the frame deallocator used when a CoWShared frame's refcount
/// hits zero. Idempotent; last call wins.
pub fn set_free_fn(f: FreeFn) {
    unsafe { FREE_FN = Some(f) };
}

#[derive(Clone, Copy)]
struct FrameRefcount {
    /// 0 == slot is free (sentinel).
    pa: u64,
    /// Number of `CoWShared` shadow pieces currently pointing at `pa`.
    /// Only meaningful when `pa != 0`.
    refcount: u32,
}

const FREE_FRAME: FrameRefcount = FrameRefcount { pa: 0, refcount: 0 };

static mut COW_REGISTRY: [FrameRefcount; MAX_COW_FRAMES] = [FREE_FRAME; MAX_COW_FRAMES];
static COW_COUNT: core::sync::atomic::AtomicUsize = core::sync::atomic::AtomicUsize::new(0);

/// Increment the refcount for `pa` by `delta`, inserting a new entry if
/// none exists. The caller must guarantee that `delta` is the new owner's
/// join count (1 for nested fork, 2 for a fresh fork of a Present piece).
///
/// Returns `Some(new_count)` on success. Returns `None` if the registry is
/// full and the frame is not yet tracked — in that case the frame stays
/// mapped but its refcount is no longer tracked (deterministic leak).
pub fn inc(pa: u64, delta: u32) -> Option<u32> {
    unsafe {
        for slot in COW_REGISTRY.iter_mut() {
            if slot.pa == pa {
                slot.refcount = slot.refcount.saturating_add(delta);
                return Some(slot.refcount);
            }
        }
        // Not found — insert a new entry.
        if COW_COUNT.load(core::sync::atomic::Ordering::Relaxed) >= MAX_COW_FRAMES {
            return None;
        }
        for slot in COW_REGISTRY.iter_mut() {
            if slot.pa == 0 {
                slot.pa = pa;
                slot.refcount = delta;
                COW_COUNT.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
                return Some(delta);
            }
        }
        None
    }
}

/// Decrement the refcount for `pa`. When the count reaches zero, the
/// frame is released via the registered deallocator and the slot is freed.
///
/// Returns `Some(remaining)` after a successful decrement, `Some(0)`
/// (and frees the frame) when this was the last owner. Returns `None`
/// if the frame was not tracked (caller bug — leak silently).
pub fn dec(pa: u64) -> Option<u32> {
    unsafe {
        for slot in COW_REGISTRY.iter_mut() {
            if slot.pa == pa {
                if slot.refcount <= 1 {
                    slot.pa = 0;
                    slot.refcount = 0;
                    COW_COUNT.fetch_sub(1, core::sync::atomic::Ordering::Relaxed);
                    if let Some(f) = FREE_FN {
                        f(pa);
                    }
                    return Some(0);
                }
                slot.refcount -= 1;
                return Some(slot.refcount);
            }
        }
        None
    }
}

/// Look up the current refcount for `pa`. Returns 0 if not tracked.
pub fn count_of(pa: u64) -> u32 {
    unsafe {
        for slot in COW_REGISTRY.iter_mut() {
            if slot.pa == pa {
                return slot.refcount;
            }
        }
    }
    0
}

/// Total number of tracked CoW frames (for boot-time diagnostics).
pub fn total() -> usize {
    COW_COUNT.load(core::sync::atomic::Ordering::Relaxed)
}
