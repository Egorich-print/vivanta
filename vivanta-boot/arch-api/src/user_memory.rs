// ---------------------------------------------------------------------------
// arch-api user memory validation contract
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessType {
    Read,
    Write,
    Execute,
}

unsafe extern "Rust" {
    /// Verify whether a virtual address range is accessible in the given address space
    /// with the specified access type.
    pub fn access_ok(aspace: usize, vaddr: u64, size: u64, access: AccessType) -> bool;

    /// Copy `len` bytes from user space into `dst`.
    ///
    /// The full range is validated via `access_ok(Read)` first; the copy runs
    /// with interrupts disabled so no scheduler/address-space switch can race
    /// the access (single-core TOCTOU prevention).
    ///
    /// # Safety
    /// `dst` must point to a valid kernel buffer of at least `len` bytes.
    ///
    /// Returns `Ok(())` on success, `Err(())` if the source range is invalid.
    pub unsafe fn copy_from_user(dst: *mut u8, src: u64, len: usize) -> Result<(), ()>;

    /// Copy `len` bytes from `src` into user space at `dst`.
    ///
    /// The full range is validated via `access_ok(Write)` first; the copy runs
    /// with interrupts disabled (single-core TOCTOU prevention).
    ///
    /// # Safety
    /// `src` must point to a valid kernel buffer of at least `len` bytes.
    ///
    /// Returns `Ok(())` on success, `Err(())` if the destination range is
    /// invalid.
    pub unsafe fn copy_to_user(dst: u64, src: *const u8, len: usize) -> Result<(), ()>;
}
