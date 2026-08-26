// ---------------------------------------------------------------------------
// Fault resolution (ADR-032): the EL1 exception path consults this module
// for the single resolvable fault class. The resolver is a CLIENT of the
// VMM primitives — it never touches descriptors directly (INV-VM-003).
// ---------------------------------------------------------------------------

use crate::memory::AsPageTableAllocator;

/// Allocator plumbing for demand fill. Set once during boot; not a mapping
/// registry — MappingSet remains the only VM state.
static mut VM_ALLOC_CTX: Option<(
    *mut crate::memory::MemoryResourceManager,
    *mut dyn crate::memory::MemoryBackend,
)> = None;

/// Provide the memory context used for demand-fill allocations.
pub fn set_backing_context(
    mrm: *mut crate::memory::MemoryResourceManager,
    backend: *mut dyn crate::memory::MemoryBackend,
) {
    unsafe { VM_ALLOC_CTX = Some((mrm, backend)) };
}

pub(crate) fn backing_context() -> Option<(
    *mut crate::memory::MemoryResourceManager,
    *mut dyn crate::memory::MemoryBackend,
)> {
    unsafe { VM_ALLOC_CTX }
}

/// Backend owning anonymous demand-filled frames (ADR-032 §4). Used by
/// unmap to release them; None before boot establishes the context.
pub(crate) fn anonymous_backend() -> Option<*mut dyn crate::memory::MemoryBackend> {
    backing_context().map(|(_, b)| b)
}

/// Panic handler for unmapped page faults (pre-M6.0 behavior retained for
/// paths that never go through resolution).
pub fn handle_page_fault(virt_addr: u64, write: bool, user: bool, instruction: bool) -> ! {
    panic!(
        "Unhandled page fault: virt=0x{:x} write={} user={} instr={}",
        virt_addr, write, user, instruction
    )
}

/// ADR-032 §2.1: resolve an EL1 data-abort translation fault.
#[unsafe(no_mangle)]
pub extern "Rust" fn vm_try_resolve_data_abort(root_pa: u64, vaddr: u64, write: bool) -> bool {
    vivanta_boot_common::println!(
        "  [VMR] enter root={:#x} va={:#x} w={}",
        root_pa,
        vaddr,
        write as u8
    );
    let Some(aspace) = super::address_space::find_by_root(root_pa) else {
        vivanta_boot_common::println!("  [VMR] no AS for root");
        return false;
    };
    let Some((mrm, backend)) = backing_context() else {
        return false;
    };
    // SAFETY: raw pointers come from the boot-established context; the
    // exception path runs with IRQs masked (single-core).
    let mut alloc = unsafe { AsPageTableAllocator::new(mrm, backend, aspace.id) };
    aspace.resolve_lazy_fault(vaddr, write, &mut alloc)
}

/// ADR-034 §3: resolve a write permission fault as COW break.
#[unsafe(no_mangle)]
pub extern "Rust" fn vm_try_resolve_cow_fault(root_pa: u64, vaddr: u64) -> bool {
    let Some(aspace) = super::address_space::find_by_root(root_pa) else {
        return false;
    };
    let Some((mrm, backend)) = backing_context() else {
        return false;
    };
    // SAFETY: context pointers were established during boot.
    let mut alloc = unsafe { crate::memory::AsPageTableAllocator::new(mrm, backend, aspace.id) };
    aspace.resolve_cow_fault(vaddr, &mut alloc)
}
