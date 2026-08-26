// ---------------------------------------------------------------------------
// ELF64 AArch64 loader — pure kernel-side module (M8.2/M8.3/M8.4 contract).
//
// This module is the kernel-side half: it takes a validated [`LoadPlan`]
// (produced by the host-testable `elf` crate) and produces a list of
// concrete VMM operations to apply to the target address space. It never
// touches page tables directly — everything goes through proven primitives
// (reserve_lazy, map_pages, protect, unmap_range). The loader is therefore
// safe, auditable, and reusable for any future image format.

use crate::vmm::{self, AddressSpaceFlags, VmmError};
use crate::elf::{LoadPlan, LoadSegment, ENTRY_ALIGN};
use alloc::vec::Vec;
use vivanta_boot_common::println;

// --- Loader contract -------------------------------------------------------

/// The kernel-side loader consumes a validated plan and applies the
/// operations required to map the image into the target address space.
pub fn load_image(
    plan: LoadPlan,
    aspace: &mut vmm::AddressSpace,
    alloc: &mut dyn vmm::PageTableAllocator,
    object_id: u64,
) -> Result<Addr, VmmError> {
    let mut highest = 0u64;
    for seg in plan.segments {
        // Compute the page-aligned virtual start.
        let va_start = seg.va_start & !(ENTRY_ALIGN - 1);

        // File data (if any) is copied eagerly.
        if seg.has_file_data() {
            let mut kbuf = vec![0u8; seg.filesz as usize];
            // SAFETY: the image is immutable; copy_from_user validates the
            // source range against the caller address space.
            if unsafe {
                vivanta_arch_api::user_memory::copy_from_user(
                    kbuf.as_mut_ptr(),
                    seg.file_off,
                    seg.filesz as usize,
                )
            }.is_err() {
                return Err(VmmError::InvalidRange);
            }
            // Copy the bytes into newly-allocated frames (lazy tail is zeroed
            // by the page allocator).
            for (i, chunk) in kbuf.chunks(4096).enumerate() {
                let va = va_start + (i as u64) * 4096;
                let Some(f) = alloc.try_alloc_page_table_frame() else {
                    return Err(VmmError::OutOfSpace);
                };
                unsafe {
                    core::ptr::copy_nonoverlapping(
                        chunk.as_ptr(),
                        f as *mut u8,
                        chunk.len(),
                    );
                }
                aspace.map_pages(
                    va,
                    f,
                    chunk.len() as u64,
                    seg.flags.into(),
                    alloc,
                    object_id,
                ).map_err(|_| VmmError::InvalidRange)?;
            }
        }

        // Lazy tail (BSS / zero-fill).
        if let Some((lazy_start, lazy_size)) = seg.lazy_tail() {
            for i in 0..(lazy_size / 4096) {
                let va = lazy_start + i * 4096;
                let Some(f) = alloc.try_alloc_page_table_frame() else {
                    return Err(VmmError::OutOfSpace);
                };
                // Frame is zeroed by the page allocator.
                aspace.map_pages(
                    va,
                    f,
                    4096,
                    seg.flags.into(),
                    alloc,
                    object_id,
                ).map_err(|_| VmmError::InvalidRange)?;
            }
        }

        highest = highest.max(seg.va_end);
    }

    Ok(highest)
}
