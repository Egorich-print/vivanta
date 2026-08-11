// ---------------------------------------------------------------------------
// Kernel Invariants & Verification Infrastructure (M5.0)
// ---------------------------------------------------------------------------

use crate::error::{KernelResult, PmmError};
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
