#![no_std]

// ---------------------------------------------------------------------------
// Arch test stub — provides all #[no_mangle] extern "Rust" symbols
// that vivanta_kernel expects from arch-api declarations. Used for build-time
// validation that vivanta_kernel does not depend on any real architecture.
// ---------------------------------------------------------------------------

// cpu
#[unsafe(no_mangle)]
pub unsafe extern "Rust" fn early_init() {}
#[unsafe(no_mangle)]
pub unsafe extern "Rust" fn wait_for_interrupt() {
    loop {
        core::hint::spin_loop()
    }
}

// mmu
#[unsafe(no_mangle)]
pub unsafe extern "Rust" fn mmu_init(
    _alloc_ctx: *mut (),
    _alloc: unsafe extern "Rust" fn(*mut ()) -> u64,
) -> usize {
    0
}
#[unsafe(no_mangle)]
pub unsafe extern "Rust" fn mmu_map_range(
    _pt: usize,
    _vaddr: u64,
    _paddr: u64,
    _size: u64,
    _user: bool,
) {
}
#[unsafe(no_mangle)]
pub unsafe extern "Rust" fn mmu_map_ram(_pt: usize, _vaddr: u64, _paddr: u64, _size: u64) {}
#[unsafe(no_mangle)]
pub unsafe extern "Rust" fn mmu_activate(_pt: usize) {}
#[unsafe(no_mangle)]
pub unsafe extern "Rust" fn mmu_root_addr(_pt: usize) -> u64 {
    0
}
#[unsafe(no_mangle)]
pub unsafe extern "Rust" fn mmu_alloc_frame(_pt: usize) -> u64 {
    0
}

// irq
#[unsafe(no_mangle)]
pub unsafe extern "Rust" fn irq_init(_dtb: usize) {}
#[unsafe(no_mangle)]
pub unsafe extern "Rust" fn irq_cpu_enable() {}

// interrupts (needed by boot_common console lock G4)
use vivanta_arch_api::interrupts::InterruptGuard;
#[unsafe(no_mangle)]
pub extern "Rust" fn disable_interrupts() -> InterruptGuard {
    fn restore(_daif: usize) {}
    InterruptGuard::new(0, restore)
}
#[unsafe(no_mangle)]
pub extern "Rust" fn enable_interrupts() {}

// timer
#[unsafe(no_mangle)]
pub unsafe extern "Rust" fn timer_init() {}

// context
use vivanta_arch_api::context::{ArchContext, ExecutionLevel};
#[unsafe(no_mangle)]
pub unsafe extern "Rust" fn context_init(
    _stack_top: usize,
    _stack_bottom: usize,
    _user_stack_top: usize,
    _entry: usize,
    _level: ExecutionLevel,
) -> ArchContext {
    ArchContext::from_raw(0)
}
#[unsafe(no_mangle)]
pub unsafe extern "Rust" fn context_capture_current() -> ArchContext {
    ArchContext::from_raw(0)
}
#[unsafe(no_mangle)]
pub unsafe extern "Rust" fn context_switch(_old: *mut ArchContext, _new: ArchContext) {}

// user (only needed if user_bootstrap is in the boot module)
#[unsafe(no_mangle)]
pub unsafe extern "Rust" fn user_bootstrap(_pt: usize) -> usize {
    0
}

#[unsafe(no_mangle)]
pub extern "Rust" fn last_el0_fault() -> (u64, u64) {
    (0, 0)
}

#[unsafe(no_mangle)]
pub unsafe extern "Rust" fn mmu_map_user_pages(
    _pt: usize,
    _code_va: u64,
    _code_src: *const u8,
    _code_len: usize,
    _stack_va: u64,
) {
}

#[unsafe(no_mangle)]
pub unsafe extern "Rust" fn flush_user_code_icache() {}

// sched (boot-time init — called from kernel_main)
#[unsafe(no_mangle)]
pub unsafe extern "Rust" fn sched_init_boot() {}

// runtime mmu
use vivanta_arch_api::mmu::{MappingFlags, PageTableAllocator, RootPageTable};

#[unsafe(no_mangle)]
pub unsafe extern "Rust" fn activate_address_space(_root: RootPageTable) {}

#[unsafe(no_mangle)]
pub unsafe extern "Rust" fn mmu_map_object(
    _pt: RootPageTable,
    _vaddr: u64,
    _paddr: u64,
    _size: u64,
    _flags: MappingFlags,
    _alloc: &mut dyn PageTableAllocator,
) {
}

#[unsafe(no_mangle)]
pub unsafe extern "Rust" fn mmu_unmap(
    _pt: RootPageTable,
    _vaddr: u64,
    _size: u64,
    _alloc: &mut dyn PageTableAllocator,
) {
}

#[unsafe(no_mangle)]
pub extern "Rust" fn mmu_table_valid_leaves(_table_pa: u64) -> u32 {
    0
}

#[unsafe(no_mangle)]
pub extern "Rust" fn mmu_permission_bits(_flags: MappingFlags) -> u64 {
    0
}

#[unsafe(no_mangle)]
pub extern "Rust" fn mmu_read_table_entry(_table_pa: u64, _index: usize) -> u64 {
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "Rust" fn mmu_write_table_entry(_table_pa: u64, _index: usize, _value: u64) {}

#[unsafe(no_mangle)]
pub extern "Rust" fn mmu_leaf_descriptor(_root_pa: u64, _va: u64) -> u64 {
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "Rust" fn mmu_clear_table_entry(_table_pa: u64, _index: usize) {}

#[unsafe(no_mangle)]
pub unsafe extern "Rust" fn mmu_protect(
    _pt: RootPageTable,
    _vaddr: u64,
    _size: u64,
    _flags: MappingFlags,
    _alloc: &mut dyn PageTableAllocator,
) {
}

#[unsafe(no_mangle)]
pub unsafe extern "Rust" fn mmu_self_test() {}

#[unsafe(no_mangle)]
pub unsafe extern "Rust" fn wx_verify_user_as(_root_pa: u64, _code_va: u64, _stack_va: u64) {}

#[unsafe(no_mangle)]
pub unsafe extern "Rust" fn dump_walk(_root: u64, _va: u64, _label: &str) {}
