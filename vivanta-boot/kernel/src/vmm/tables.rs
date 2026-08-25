//! Page-table frame ownership registry (ADR-031).
//!
//! Every runtime-allocated page-table frame is recorded here at install
//! time together with its parent descriptor location and the memory
//! backend that owns the physical storage. A frame may leave the registry
//! only through [`reclaim`], which the caller may invoke after proving:
//!
//! 1. the table is **empty** (`mmu_table_valid_leaves == 0`, hardware truth),
//! 2. its parent descriptor has been cleared (`mmu_clear_table_entry`),
//! 3. all translations under it were invalidated (per-page TLBI at unmap).
//!
//! Frames whose install was never recorded (boot-era tables) are unknown
//! to this registry and therefore can never be reclaimed — the intentional
//! leak remains their lifetime model. Registry exhaustion also degrades to
//! the leak model (deterministic, safe).

use crate::memory::resource::MemoryBackend;

#[derive(Clone, Copy)]
pub struct TableEntry {
    /// Physical address of the table frame.
    pub frame: u64,
    /// Owning address space id.
    pub as_id: u64,
    /// 2 = L2 table (blocks / L3 pointers), 3 = L3 table (4 KiB pages),
    /// in this kernel's root=L1 naming.
    pub level: u8,
    /// Parent table physical address.
    pub parent_table: u64,
    /// Index of this table's descriptor in the parent.
    pub parent_index: usize,
    /// Backend owning the physical storage (for deallocation on reclaim).
    pub backend: *mut dyn MemoryBackend,
}

pub const MAX_TABLES: usize = 256;

static mut TABLE_REGISTRY: [Option<TableEntry>; MAX_TABLES] = [None; MAX_TABLES];
/// Live-entry counter. Relaxed atomic: mutated only under the IRQ-guarded
/// registry operations, read freely for stats — removes static-mut reads.
static TABLE_COUNT: core::sync::atomic::AtomicUsize = core::sync::atomic::AtomicUsize::new(0);

/// Record a freshly installed table frame. Returns false when the registry
/// is full — the frame stays installed and reachable but is never reclaimed
/// (safe leak).
pub fn record(entry: TableEntry) -> bool {
    unsafe {
        if TABLE_COUNT.load(core::sync::atomic::Ordering::Relaxed) >= MAX_TABLES {
            return false;
        }
        for slot in TABLE_REGISTRY.iter_mut() {
            if slot.is_none() {
                *slot = Some(entry);
                TABLE_COUNT.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
                return true;
            }
        }
        false
    }
}

/// Find the registry index of a frame owned by `as_id`.
fn find_index(frame: u64, as_id: u64) -> Option<usize> {
    unsafe {
        (0..MAX_TABLES)
            .find(|&i| TABLE_REGISTRY[i].is_some_and(|e| e.frame == frame && e.as_id == as_id))
    }
}

/// Take the entry out of the registry (ownership transferred back to caller).
///
/// # Safety
/// Single-core: caller must hold off preempting contexts (IRQ guard) so the
/// check-out/check-in sequence cannot race.
pub unsafe fn take(frame: u64, as_id: u64) -> Option<TableEntry> {
    let idx = find_index(frame, as_id)?;
    let e = unsafe { TABLE_REGISTRY[idx] };
    unsafe {
        TABLE_REGISTRY[idx] = None;
        TABLE_COUNT.fetch_sub(1, core::sync::atomic::Ordering::Relaxed);
    }
    e
}

/// Find a tracked table of `as_id` whose hardware content is fully invalid
/// (zero valid descriptors). Emptiness is read from the table itself — the
/// software shadow cannot prove emptiness because split-inherited block
/// pages are invisible to it.
pub fn find_empty(as_id: u64) -> Option<TableEntry> {
    unsafe {
        (0..MAX_TABLES).find_map(|i| {
            TABLE_REGISTRY[i].filter(|e| {
                e.as_id == as_id && vivanta_arch_api::mmu::mmu_table_valid_leaves(e.frame) == 0
            })
        })
    }
}

/// Read-only lookup.
pub fn lookup(frame: u64, as_id: u64) -> Option<TableEntry> {
    unsafe { find_index(frame, as_id).and_then(|i| TABLE_REGISTRY[i]) }
}

/// Number of tracked frames for an address space.
pub fn count_for_as(as_id: u64) -> usize {
    unsafe {
        (0..MAX_TABLES)
            .filter(|&i| TABLE_REGISTRY[i].is_some_and(|e| e.as_id == as_id))
            .count()
    }
}

pub fn total() -> usize {
    TABLE_COUNT.load(core::sync::atomic::Ordering::Relaxed)
}
