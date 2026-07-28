#![no_std]

// ---------------------------------------------------------------------------
// Arch test stub — provides all #[no_mangle] extern "Rust" symbols
// that vivanta_kernel expects from arch-api declarations. Used for build-time
// validation that vivanta_kernel does not depend on any real architecture.
// ---------------------------------------------------------------------------

// cpu
#[no_mangle]
pub unsafe extern "Rust" fn early_init() {}
#[no_mangle]
pub unsafe extern "Rust" fn wait_for_interrupt() {
    loop { core::hint::spin_loop() }
}

// mmu
#[no_mangle]
pub unsafe extern "Rust" fn mmu_init(_alloc_ctx: *mut (), _alloc: unsafe extern "Rust" fn(*mut ()) -> u64) -> usize { 0 }
#[no_mangle]
pub unsafe extern "Rust" fn mmu_map_range(_pt: usize, _vaddr: u64, _paddr: u64, _size: u64, _user: bool) {}
#[no_mangle]
pub unsafe extern "Rust" fn mmu_map_ram(_pt: usize, _vaddr: u64, _paddr: u64, _size: u64) {}
#[no_mangle]
pub unsafe extern "Rust" fn mmu_activate(_pt: usize) {}
#[no_mangle]
pub unsafe extern "Rust" fn mmu_root_addr(_pt: usize) -> u64 { 0 }
#[no_mangle]
pub unsafe extern "Rust" fn mmu_alloc_frame(_pt: usize) -> u64 { 0 }

// irq
#[no_mangle]
pub unsafe extern "Rust" fn irq_init(_dtb: usize) {}
#[no_mangle]
pub unsafe extern "Rust" fn irq_cpu_enable() {}

// timer
#[no_mangle]
pub unsafe extern "Rust" fn timer_init() {}

// context
use vivanta_arch_api::context::{ArchContext, ExecutionLevel};
#[no_mangle]
pub unsafe extern "Rust" fn context_init(_stack_top: usize, _user_stack_top: usize, _entry: usize, _level: ExecutionLevel) -> ArchContext {
    ArchContext::from_raw(0)
}
#[no_mangle]
pub unsafe extern "Rust" fn context_capture_current() -> ArchContext {
    ArchContext::from_raw(0)
}
#[no_mangle]
pub unsafe extern "Rust" fn context_switch(_old: *mut ArchContext, _new: ArchContext) {}

// user (only needed if user_bootstrap is in the boot module)
#[no_mangle]
pub unsafe extern "Rust" fn user_bootstrap(_pt: usize) -> usize { 0 }

#[no_mangle]
pub unsafe extern "Rust" fn mmu_map_user_pages(_pt: usize, _code_va: u64, _code_src: *const u8, _code_len: usize, _stack_va: u64) {}

#[no_mangle]
pub unsafe extern "Rust" fn flush_user_code_icache() {}

// sched (boot-time init — called from kernel_main)
#[no_mangle]
pub unsafe extern "Rust" fn sched_init_boot() {}

// runtime mmu
use vivanta_arch_api::mmu::{MappingFlags, PageTableAllocator, RootPageTable};

#[no_mangle]
pub unsafe extern "Rust" fn activate_address_space(_root: RootPageTable) {}

#[no_mangle]
pub unsafe extern "Rust" fn mmu_map_object(_pt: RootPageTable, _vaddr: u64, _paddr: u64, _size: u64, _flags: MappingFlags, _alloc: &mut dyn PageTableAllocator) {}

#[no_mangle]
pub unsafe extern "Rust" fn mmu_unmap(_pt: RootPageTable, _vaddr: u64, _size: u64, _alloc: &mut dyn PageTableAllocator) {}