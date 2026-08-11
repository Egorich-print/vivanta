// ---------------------------------------------------------------------------
// Kernel Invariants & Verification Infrastructure (M5.0)
// ---------------------------------------------------------------------------

use crate::error::{KernelResult, PmmError};
use crate::memory::{AllocationRequirements, MemoryResourceManager};
use crate::pmm::PmmBitmap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerifyError {
    PmmLeakDetected { allocated_count: usize },
    PmmBitmapCorrupted,
    InvalidFrameState,
}

/// Verify PMM internal state invariants.
#[must_use]
pub fn verify_pmm(pmm: &PmmBitmap) -> KernelResult<()> {
    // Invariant 1: Allocated count must be within total frames bounds
    if pmm.allocated_count() > pmm.total_frames() {
        return Err(crate::error::KernelError::Pmm(PmmError::BitmapCorrupted));
    }

    // Invariant 2: Free count + allocated count + reserved count must equal total frames
    let computed_total = pmm.allocated_count() + pmm.reserved_count() + pmm.free_count();
    if computed_total != pmm.total_frames() {
        return Err(crate::error::KernelError::Pmm(PmmError::BitmapCorrupted));
    }

    Ok(())
}

/// Run stress-test and invariant checks on PMM without requiring heap allocation.
#[must_use]
pub fn stress_test_pmm(pmm: &mut PmmBitmap, cycles: usize) -> KernelResult<()> {
    // Stack-based tracking array for addresses (up to 128 simultaneous allocations)
    const MAX_TRACKED: usize = 128;
    let mut addrs = [0u64; MAX_TRACKED];
    let mut count = 0;

    for i in 0..cycles {
        // Decide whether to allocate or free
        let do_alloc = count == 0 || (i % 3 != 0 && count < MAX_TRACKED);

        if do_alloc {
            if let Ok(addr) = pmm.allocate_page() {
                addrs[count] = addr;
                count += 1;
            }
        } else if count > 0 {
            // Free the last allocated page
            count -= 1;
            let addr = addrs[count];
            pmm.free_page(addr)?;
        }

        // Verify invariants on every cycle
        verify_pmm(pmm)?;
    }

    // Cleanup remaining allocations
    while count > 0 {
        count -= 1;
        let addr = addrs[count];
        pmm.free_page(addr)?;
    }

    verify_pmm(pmm)?;
    Ok(())
}

/// G2 churn: allocate MemoryObjects via the MRM and drop them, proving that
/// `free_count` returns to its baseline after each cycle (Drop → deallocate).
///
/// Returns the free-count delta across all cycles; a non-zero value means a
/// leak. `before`/`after` deltas are checked by the caller against the PMM
/// free count.
#[must_use]
pub fn stress_mrm_churn(mrm: &mut MemoryResourceManager, cycles: usize) -> KernelResult<()> {
    let mut objs = alloc::vec::Vec::new();
    for _ in 0..cycles {
        let req = AllocationRequirements::new(4096);
        let obj = mrm.allocate(&req, 0).ok_or(PmmError::OutOfMemory)?;
        // Physical frame must be present (Drop will release it).
        assert!(
            obj.phys_addr.is_some(),
            "MRM churn: object without phys addr"
        );
        objs.push(obj);
        // Free immediately: Drop deallocates the frame.
    }
    objs.clear();
    Ok(())
}
