//! User-VM fault resolution hook (ADR-032).
//!
//! The architecture exception path classifies faults and consults this
//! hook for the single resolvable class (EL1 data-abort translation
//! fault). The kernel implements it on top of the VMM primitives; the
//! arch layer never touches mapping state itself.

unsafe extern "Rust" {
    /// Attempt to resolve an EL1 data-abort *translation* fault at
    /// `vaddr` in the address space whose root is `root_pa`, for a read
    /// (`write = false`) or write access.
    ///
    /// Returns `true` when the fault was resolved and the faulting
    /// instruction may retry (ELR untouched). Returns `false` when the
    /// fault must be treated as fatal (no mapping, permission mismatch,
    /// Reserved piece, OOM).
    pub safe fn vm_try_resolve_data_abort(root_pa: u64, vaddr: u64, write: bool) -> bool;
}
