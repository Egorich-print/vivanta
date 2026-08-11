// ---------------------------------------------------------------------------
// arch-api user memory validation contract
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessType {
    Read,
    Write,
    Execute,
}

extern "Rust" {
    /// Verify whether a virtual address range is accessible in the given address space
    /// with the specified access type.
    pub fn access_ok(aspace: usize, vaddr: u64, size: u64, access: AccessType) -> bool;
}
