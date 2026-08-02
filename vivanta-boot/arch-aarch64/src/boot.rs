// ---------------------------------------------------------------------------
// Boot-time arch API — #[no_mangle] extern "Rust" implementations
// Called by vivanta_kernel through vivanta_arch_api::boot::*
// ---------------------------------------------------------------------------

use vivanta_arch_api::boot::mmu::{AllocFn, AllocCtx};
use vivanta_arch_api::pmm::PhysFrame;
use vivanta_boot_common::fdt::FdtScanner;

use crate::mmu::{PageTableBuilder, PageFlags};
use crate::interrupts::Gic;

// ---------------------------------------------------------------------------
// Internal allocator adapter: wraps extern "Rust" callback as FrameAllocator
// ---------------------------------------------------------------------------

struct CallbackAllocator {
    alloc_fn: AllocFn,
    alloc_ctx: AllocCtx,
}

impl vivanta_arch_api::pmm::FrameAllocator for CallbackAllocator {
    fn alloc_frame(&mut self) -> Option<PhysFrame> {
        let addr = unsafe { (self.alloc_fn)(self.alloc_ctx) };
        if addr == 0 { None } else { Some(PhysFrame { addr }) }
    }
    fn free_frame(&mut self, _frame: PhysFrame) {}
    fn reserve(&mut self, _start: u64, _size: u64) {}
}

// ---------------------------------------------------------------------------
// Global boot state — stored between vivanta_arch_api calls
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
#[allow(dead_code)]
struct BootGic {
    version: u32,
    dist_base: u64,
    cpu_base: u64,
}

static mut BOOT_PT: Option<PageTableBuilder<CallbackAllocator>> = None;
static mut BOOT_GIC: Option<BootGic> = None;
static mut USER_CODE_PA: u64 = 0;

#[allow(dead_code)]
fn alloc_from_callback() -> u64 {
    let builder = unsafe { BOOT_PT.as_mut().unwrap() };
    builder.alloc_frame().unwrap().addr
}

fn boot_gic() -> BootGic {
    unsafe { BOOT_GIC.as_ref().copied().unwrap() }
}

// ---------------------------------------------------------------------------
// cpu
// ---------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "Rust" fn early_init() {
    crate::exceptions::init();
}

#[no_mangle]
pub unsafe extern "Rust" fn wait_for_interrupt() {
    core::arch::asm!("wfi", options(nostack));
}

// ---------------------------------------------------------------------------
// mmu
// ---------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "Rust" fn mmu_init(alloc_ctx: *mut (), alloc: AllocFn) -> usize {
    let ca = CallbackAllocator { alloc_fn: alloc, alloc_ctx };
    let pt = PageTableBuilder::new(ca);
    let root = pt.root_addr();
    BOOT_PT = Some(pt);
    root as usize
}

#[no_mangle]
pub unsafe extern "Rust" fn mmu_map_range(_pt: usize, vaddr: u64, paddr: u64, size: u64, user: bool) {
    let builder = BOOT_PT.as_mut().unwrap();
    // MMIO is always device memory — not cached, not reordered
    let flags = if user {
        PageFlags::USER_DEVICE
    } else {
        PageFlags::DEVICE
    };
    builder.map(vaddr, paddr, size, flags);
}

#[no_mangle]
pub unsafe extern "Rust" fn mmu_map_ram(_pt: usize, vaddr: u64, paddr: u64, size: u64) {
    let builder = BOOT_PT.as_mut().unwrap();
    builder.map(vaddr, paddr, size, PageFlags::READ_WRITE_EXEC);
}

#[no_mangle]
pub unsafe extern "Rust" fn mmu_activate(_pt: usize) {
    let builder = BOOT_PT.take().unwrap();
    let guard = builder.finish();
    // UART poke before MMU switch
    core::ptr::write_volatile(0x0900_0000 as *mut u32, b'A' as u32);
    guard.activate();
    // UART poke after MMU switch
    core::ptr::write_volatile(0x0900_0000 as *mut u32, b'B' as u32);
}

#[no_mangle]
pub unsafe extern "Rust" fn mmu_root_addr(_pt: usize) -> u64 {
    BOOT_PT.as_ref().unwrap().root_addr()
}

#[no_mangle]
pub unsafe extern "Rust" fn mmu_alloc_frame(_pt: usize) -> u64 {
    let builder = BOOT_PT.as_mut().unwrap();
    builder.alloc_frame().unwrap_or(PhysFrame { addr: 0 }).addr
}

#[no_mangle]
pub unsafe extern "Rust" fn mmu_self_test() {
    crate::paging::self_test::run_smoke_test();
}

// ---------------------------------------------------------------------------
// irq
// ---------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "Rust" fn irq_init(dtb: usize) {
    let dtb_ptr = dtb as *const u8;
    let gic_info = FdtScanner::interrupt_controller(dtb_ptr)
        .expect("no interrupt controller in FDT");

    vivanta_boot_common::println!("  compatible: {}", gic_info.compatible);
    vivanta_boot_common::println!("  distributor: 0x{:x} ({} bytes)", gic_info.distributor.addr, gic_info.distributor.size);
    if let Some(r) = &gic_info.redistributor {
        vivanta_boot_common::println!("  redistributor: 0x{:x} ({} bytes)", r.addr, r.size);
    }

    let version = if gic_info.compatible.contains("gic-v3") { 3 } else { 2 };
    let dist_base = gic_info.distributor.addr;
    let cpu_base = gic_info.redistributor.map_or(0, |r| r.addr);

    let gic = Gic::new(&gic_info);
    gic.init();
    gic.enable_cpu_interface();
    
    BOOT_GIC = Some(BootGic { version, dist_base, cpu_base });
}

#[no_mangle]
pub unsafe extern "Rust" fn irq_cpu_enable() {
    crate::interrupts::enable();
}

// ---------------------------------------------------------------------------
// timer
// ---------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "Rust" fn timer_init() {
    let bg = boot_gic();
    // Init generic timer hardware directly
    let freq: u64;
    core::arch::asm!("mrs {0}, CNTFRQ_EL0", out(reg) freq, options(nostack));
    let tval = (freq / 100) as u32;
    crate::timer::TVAL.store(tval, core::sync::atomic::Ordering::Relaxed);
    core::arch::asm!("msr CNTP_TVAL_EL0, {0}", in(reg) tval as u64, options(nostack));
    core::arch::asm!("msr CNTP_CTL_EL0, {0}", in(reg) 1u64, options(nostack));

    // Register the timer IRQ handler
    crate::interrupts::register_irq(30, crate::timer::timer_handler);

    // Enable IRQ 30 on the GIC distributor
    const GICD_ISENABLER: usize = 0x0100;
    let base = bg.dist_base as *mut u8;
    crate::mmio::mmio_write32(
        base.add(GICD_ISENABLER + (30 / 32) * 4) as *mut u32,
        1 << (30 % 32),
    );
}

// ---------------------------------------------------------------------------
// user
// ---------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "Rust" fn user_bootstrap(_pt: usize) -> usize {
    let mut builder = BOOT_PT.as_mut().unwrap();
    let user = crate::user::UserBootstrap::create(&mut builder);
    // Return user token as the entry address
    user.entry as usize
}

#[no_mangle]
pub unsafe extern "Rust" fn mmu_map_user_pages(
    _pt: usize,
    code_va: u64,
    code_src: *const u8,
    code_len: usize,
    stack_va: u64,
) {
    let builder = BOOT_PT.as_mut().unwrap();
    // Allocate a real physical frame for user code (identity-mapped by early_mmu)
    let code_frame = builder.alloc_frame().expect("mmu_map_user_pages: no frame for code");
    let code_pa = code_frame.addr;
    USER_CODE_PA = code_pa;
    core::ptr::copy_nonoverlapping(code_src, code_pa as *mut u8, code_len);
    if code_len < 4096 {
        core::ptr::write_bytes((code_pa as *mut u8).add(code_len), 0u8, 4096 - code_len);
    }
    builder.map(code_va, code_pa, 4096, PageFlags::USER_READ_WRITE_EXEC);
    // Allocate and map user stack page
    let stack_pa = builder.alloc_frame().expect("mmu_map_user_pages: no frame for stack").addr;
    builder.map(stack_va, stack_pa, 4096, PageFlags::USER_READ_WRITE);
}

#[no_mangle]
pub unsafe extern "Rust" fn flush_user_code_icache() {
    let pa = USER_CODE_PA;
    if pa == 0 { return; }
    let ctr_el0: u64;
    core::arch::asm!("mrs {}, ctr_el0", out(reg) ctr_el0);
    let d_min_line = (ctr_el0 >> 16) & 0xF;
    let line = (4u64) << d_min_line;
    let mask = line - 1;
    let start = pa & !mask;
    let end = pa + 4096;
    let mut addr = start;
    while addr < end {
        core::arch::asm!("dc cvac, {}", in(reg) addr);
        addr += line;
    }
    core::arch::asm!("dsb sy");
    addr = start;
    while addr < end {
        core::arch::asm!("ic ivau, {}", in(reg) addr);
        addr += line;
    }
    core::arch::asm!("dsb sy; isb");
}

