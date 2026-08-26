//! Kernel-side ELF64 AArch64 loader (M8.2).
//!
//! Consumes a validated [`LoadPlan`] from `vivanta-exec` and applies it
//! to a target address space through proven VMM primitives:
//! `reserve_at` (fixed-VA lazy reservation) + `map_pages` (eager file
//! data) + demand-fill (lazy BSS tail). Never writes page tables
//! directly — INV-VM-001 is preserved by construction.

use crate::vmm::{self, VmmError};
use vivanta_arch_api::mmu::{MappingFlags, PageTableAllocator};
use vivanta_exec::elf::{self, LoadPlan, PF_R, PF_W, PF_X};

/// Convert ELF p_flags into MappingFlags. W^X is already rejected by
/// the planner; this conversion always produces a user-accessible
/// mapping (ELF segments are user mappings by definition).
fn flags_from_pflags(p_flags: u32) -> MappingFlags {
    let mut f = MappingFlags::user();
    if p_flags & PF_W != 0 {
        f = f | MappingFlags::read_write();
    }
    if p_flags & PF_X != 0 {
        f = f | MappingFlags::executable();
    }
    // Read is implicit in every Vivanta mapping (decode_prot requires it).
    let _ = p_flags & PF_R;
    f
}

/// Load a validated ELF image into `aspace`. Returns the entry point.
///
/// File data is copied eagerly (identity-mapped kernel image → newly
/// allocated frames). The BSS tail (memsz > filesz) remains
/// LazyAnonymous and demand-fills on first access.
pub fn load_elf(
    image: &[u8],
    aspace: &mut vmm::AddressSpace,
    alloc: &mut dyn PageTableAllocator,
    object_id: u64,
) -> Result<u64, VmmError> {
    let plan: LoadPlan = elf::plan_load(image).map_err(|_| VmmError::InvalidRange)?;

    for s in &plan.segments {
        let size = s.va_end - s.va_start;
        let flags = flags_from_pflags(s.flags);

        // 1. Reserve the entire segment as LazyAnonymous at the fixed VA.
        aspace.reserve_at(s.va_start, size, flags, object_id)?;

        // 2. Eager-copy file data page by page. The image lives in
        // kernel .rodata (identity-mapped); frames are allocated from
        // the same PMM the demand-fill path uses.
        if s.filesz > 0 {
            let mut off = 0u64;
            while off < s.filesz {
                let page_va = (s.va_start + off) & !0xFFF;
                let page_off = (s.va_start + off) & 0xFFF;
                let copy_len = core::cmp::min(4096 - page_off, s.filesz - off);

                let Some(frame) = alloc.try_alloc_page_table_frame() else {
                    return Err(VmmError::OutOfVa);
                };
                // SAFETY: frame is a live 4 KiB allocation; image is
                // identity-mapped kernel .rodata.
                unsafe {
                    core::ptr::write_bytes(frame as *mut u8, 0, 4096);
                    let src = image.as_ptr().add((s.file_off + off) as usize);
                    core::ptr::copy_nonoverlapping(
                        src,
                        (frame + page_off) as *mut u8,
                        copy_len as usize,
                    );
                }
                aspace.materialize_with(page_va, frame, flags, alloc)?;
                off += copy_len;
            }
        }
        // 3. BSS tail stays LazyAnonymous — demand-fill zeros it.
    }

    Ok(plan.entry)
}
