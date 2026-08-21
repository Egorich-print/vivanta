/// Kernel-provided callback: allocates one 4 KiB frame.
/// Returns the physical address, or 0 on failure.
pub type AllocFn = unsafe extern "Rust" fn(ctx: *mut ()) -> u64;

/// Opaque context passed through to the alloc callback.
pub type AllocCtx = *mut ();

unsafe extern "Rust" {
    /// Initialise page tables. Returns a handle (root table physical address).
    pub fn mmu_init(alloc_ctx: AllocCtx, alloc: AllocFn) -> usize;

    /// Map a device MMIO range.
    /// `user`: if true, the region is user-accessible (EL0).
    pub fn mmu_map_range(pt: usize, vaddr: u64, paddr: u64, size: u64, user: bool);

    /// Map RAM (identity-mapped, executable).
    pub fn mmu_map_ram(pt: usize, vaddr: u64, paddr: u64, size: u64);

    /// Activate the page table (switch MMU on).
    pub fn mmu_activate(pt: usize);

    /// Return the root-level physical address of the page table.
    pub fn mmu_root_addr(pt: usize) -> u64;

    /// Allocate one frame through the page table's allocator context.
    pub fn mmu_alloc_frame(pt: usize) -> u64;

    /// Map user code and stack pages in a boot-time page table.
    /// Copies user code from the vivanta_kernel's .user.text section.
    pub fn mmu_map_user_pages(
        pt: usize,
        code_va: u64,
        code_src: *const u8,
        code_len: usize,
        stack_va: u64,
    );

    /// Flush D-cache (to PoC) and invalidate I-cache (to PoU) for the user code page.
    /// Must be called with MMU enabled (after mmu_activate).
    pub fn flush_user_code_icache();

    /// Run MMU smoke tests on the currently active page table.
    /// Panics on failure.
    pub fn mmu_self_test();

    /// Verify the W^X permission encoding of a user address space (G3).
    ///
    /// Walks the page table rooted at `root_pa` and asserts that the leaf
    /// descriptor of `code_va` is EL0 read-only + executable (AP=11, XN=0,
    /// PXN=1) and the leaf of `stack_va` is EL0 read-write, non-executable
    /// (AP=01, XN=1). Prints machine-checkable `[WX]` lines and panics on
    /// any violation. Reads only physical table memory — safe before or
    /// after MMU activation.
    pub fn wx_verify_user_as(root_pa: u64, code_va: u64, stack_va: u64);

    /// Debug: dump page table entries for critical addresses.
    /// Must be called BEFORE mmu_activate.
    pub fn dump_critical_tables(root: u64);

    /// Debug: walk `root` for `va` and print the descriptor chain.
    pub unsafe fn dump_walk(root: u64, va: u64, label: &str);
}
