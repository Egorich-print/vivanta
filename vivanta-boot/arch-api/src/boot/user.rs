unsafe extern "Rust" {
    /// Prepare user-space mappings and return an opaque handle.
    /// `pt`: page table handle (from mmu_init).
    pub fn user_bootstrap(pt: usize) -> usize;
}

unsafe extern "Rust" {
    /// (ESR, FAR) of the most recent synchronous EL0 fault that led to task
    /// termination. Diagnostic hook for the boot-time protection audit;
    /// (0, 0) before the first fault.
    pub fn last_el0_fault() -> (u64, u64);
}
