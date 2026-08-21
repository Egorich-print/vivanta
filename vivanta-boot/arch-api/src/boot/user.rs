unsafe extern "Rust" {
    /// Prepare user-space mappings and return an opaque handle.
    /// `pt`: page table handle (from mmu_init).
    pub fn user_bootstrap(pt: usize) -> usize;
}
