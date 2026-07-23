#![no_std]
extern crate alloc;

pub mod identity;
pub mod pmm;
pub mod scheduler;
pub mod state;
pub mod vmm;

pub use vivanta_arch_api::pmm::{FrameAllocator, PhysFrame};

use vivanta_boot_common::{println, MemoryRegionKind};
use vivanta_boot_info::BootInfo;
use vivanta_arch_api::mmu::RootPageTable;

extern "C" {
    static __kernel_start: u8;
    static __stack_top: u8;
    static user_code_start: u8;
    static user_code_end: u8;
}

/// Allocator callback for arch boot MMU init.
#[no_mangle]
pub unsafe extern "Rust" fn boot_alloc_frame(ctx: *mut ()) -> u64 {
    let pmm = &mut *(ctx as *mut pmm::PmmBitmap);
    pmm.alloc_frame().map(|f| f.addr).unwrap_or(0)
}

/// The one and only vivanta_kernel entry point.
pub unsafe fn kernel_main(info: &BootInfo) -> ! {
    println!();
    println!("\u{2500}\u{2500}\u{2500}\u{2500} Vivanta Kernel Entry \u{2500}\u{2500}\u{2500}\u{2500}");

    // V1.1: Runtime Identity Bootstrap — construct SystemState from BootInfo
    let system_state = state::SystemState::from_boot_info(info);
    let hardware = system_state.hardware();
    if hardware.dtb_ptr != 0 {
        println!("  DTB at    0x{:x}", hardware.dtb_ptr);
    }
    println!("  {} CPU(s)", hardware.cpu_count);
    // After this point, BootInfo must NOT be referenced for runtime state.
    // All hardware info is accessible through system_state.hardware().

    // ------- CPU early init (exception vectors, FP/SIMD) --------------------
    println!();
    println!("CPU Init:");
    vivanta_arch_api::boot::cpu::early_init();
    println!("  Early init done.");

    // ------- Memory Map -----------------------------------------------------
    println!("Memory Map:");
    for r in hardware.memory_map.regions() {
        let end = r.start + r.size;
        let tag = match r.kind {
            MemoryRegionKind::Usable => "Usable ",
            MemoryRegionKind::Reserved => "Reserve",
            MemoryRegionKind::Mmio => "MMIO  ",
            MemoryRegionKind::BootloaderReclaimable => "Reclai",
            MemoryRegionKind::KernelImage => "Kernel",
            MemoryRegionKind::DeviceMemory => "Device",
            MemoryRegionKind::Framebuffer => "Frameb",
        };
        println!("  0x{:016x} - 0x{:016x}  [{}]", r.start, end - 1, tag);
    }

    // ------- Boot Memory Manager -------------------------------------------
    let region = hardware
        .memory_map
        .regions()
        .iter()
        .find(|r| r.kind == MemoryRegionKind::Usable)
        .expect("no usable memory region");

    let kernel_start = unsafe { &__kernel_start as *const u8 as u64 };
    let kernel_end = unsafe { &__stack_top as *const u8 as u64 };

    let bitmap_base = ((kernel_end + 0xFFF) / 0x1000) * 0x1000;

    let mut boot = pmm::BootMemoryManager::new(region.start, region.size, bitmap_base as *mut u8);
    boot.reserve_kernel(kernel_start, kernel_end);

    if hardware.dtb_ptr != 0 {
        let dtb = hardware.dtb_ptr;
        let dtb_ptr = dtb as *const u8;
        let dtb_total = unsafe { core::ptr::read_volatile(dtb_ptr.add(4) as *const u32) };
        let dtb_size = u32::from_be(dtb_total) as u64;
        boot.reserve_dtb(dtb as u64, dtb_size);
    }

    boot.reserve_bitmap();
    boot.print_stats();

    let mut pmm: pmm::PmmBitmap = boot.finish();

    if let Some(f) = pmm.alloc_frame() {
        println!("  Allocated frame @ 0x{:x}  (ok)", f.addr);
        pmm.free_frame(f);
        println!("  Freed frame              (ok)");
    }

    // ------- VMM: Address Space Construction -------------------------------
    println!();
    println!("Address Space Builder:");
    let alloc_ctx: *mut () = &mut pmm as *mut pmm::PmmBitmap as *mut ();
    let pt = vivanta_arch_api::boot::mmu::mmu_init(alloc_ctx, boot_alloc_frame);

    // Identity-map the usable RAM region
    vivanta_arch_api::boot::mmu::mmu_map_ram(pt, region.start, region.start, region.size);

    // Map MMIO regions (from HardwareState per ADR-021)
    for mmio in hardware.mmio_regions {
        vivanta_arch_api::boot::mmu::mmu_map_range(
            pt, mmio.base, mmio.base, mmio.size, mmio.kind.is_user_accessible(),
        );
    }

    // @@M4@@ EL0 experiment temporarily disabled — see docs/architecture/milestones/M4-execution-foundation.md
    // let user_token = vivanta_arch_api::boot::user::user_bootstrap(pt);
    // if user_token != 0 {
    //     println!("  User token: 0x{:x}", user_token);
    // }

    println!("  L1 table at     0x{:x}", vivanta_arch_api::boot::mmu::mmu_root_addr(pt));
    println!("  RAM ident:      0x{:016x} – 0x{:016x}  ({} MiB)",
        region.start, region.start + region.size - 1, region.size >> 20);
    for mmio in hardware.mmio_regions {
        let kind_str = if mmio.kind.is_user_accessible() { "user" } else { "vivanta_kernel" };
        println!("  MMIO ident:     0x{:x} ({} bytes, {})", mmio.base, mmio.size, kind_str);
    }

    // ------- Wrap in AddressSpace ------------------------------------------
    let root = vivanta_arch_api::mmu::RootPageTable(pt);
    vmm::address_space::init_kernel_address_space(root);

    // Build independent root tables for UserAS1/UserAS2
    let alloc_ctx_root: *mut () = &mut pmm as *mut pmm::PmmBitmap as *mut ();
    let build_root = |label: &str, extra_va: u64, extra_pa: u64| -> RootPageTable {
        let rpt = vivanta_arch_api::boot::mmu::mmu_init(alloc_ctx_root, boot_alloc_frame);
        vivanta_arch_api::boot::mmu::mmu_map_ram(rpt, region.start, region.start, region.size);
        for mmio in hardware.mmio_regions {
            vivanta_arch_api::boot::mmu::mmu_map_range(
                rpt, mmio.base, mmio.base, mmio.size, mmio.kind.is_user_accessible(),
            );
        }
        if extra_pa != 0 {
            vivanta_arch_api::boot::mmu::mmu_map_range(rpt, extra_va, extra_pa, 0x1000, false);
        }
        // M4.5.1: map user code + stack into UserAS1
        if label == "UserAS1" {
            let code_src = &user_code_start as *const u8;
            let code_len = (&user_code_end as *const u8 as usize) - (&user_code_start as *const u8 as usize);
            const CODE_VA: u64 = 0x5E00_0000;
            const STACK_VA: u64 = 0x5E01_0000;
            vivanta_arch_api::boot::mmu::mmu_map_user_pages(rpt, CODE_VA, code_src, code_len, STACK_VA);
            println!("  UserAS1: code=0x{:x}, stack=0x{:x}", CODE_VA, STACK_VA);
        }
        let ra = vivanta_arch_api::boot::mmu::mmu_root_addr(rpt);
        println!("  {} root table at 0x{:x}", label, ra);
        RootPageTable(ra as usize)
    };
    let root1 = build_root("UserAS1", 0, 0);
    let root2 = build_root("UserAS2", 0, 0);

    let user_as1 = vmm::register(root1, vmm::AddressSpaceFlags::User);
    let _user_as2 = vmm::register(root2, vmm::AddressSpaceFlags::User);
    println!("  Address spaces: {} total", vmm::count());

    // ------- Enable MMU ----------------------------------------------------
    println!();
    println!("Enabling MMU...");
    vivanta_arch_api::boot::mmu::mmu_activate(pt);
    println!("MMU enabled successfully.");

    // ------- GIC Discovery & Initialisation --------------------------------
    if hardware.dtb_ptr != 0 {
        println!();
        println!("Interrupt Controller:");
        vivanta_arch_api::boot::irq::irq_init(hardware.dtb_ptr);
        vivanta_arch_api::boot::irq::irq_cpu_enable();

        // @@M4@@ Timer disabled for cooperative-only demo (re-enabled in M4.2)
        // vivanta_arch_api::boot::timer::timer_init();

        // Scheduler
        vivanta_arch_api::boot::sched::sched_init_boot();

        // M4.2.0: timer smoke test — tick counting only
        println!("  Initialising timer...");
        vivanta_arch_api::boot::timer::timer_init();
        println!("  Timer initialised.");

        // @@M4@@ EL0 experiment temporarily disabled
        // println!("  Starting EL0 test...");
        // vivanta_arch_api::boot::user::user_enter(user_token);
    }

    println!();
    println!("Boot complete -- creating user thread");

    // M4.5.1: allocate vivanta_kernel stack for the first user thread
    let stack_base = pmm.alloc_frame().expect("user vivanta_kernel stack frame").addr;
    for _ in 1..4 {
        pmm.alloc_frame().expect("user vivanta_kernel stack frame");
    }
    let kernel_stack_top = (stack_base as usize) + 16384;

    const CODE_VA: u64 = 0x5E00_0000;
    const STACK_VA: u64 = 0x5E01_0000;
    let ut = scheduler::create_user_thread(
        kernel_stack_top,
        (STACK_VA + 4096) as usize,
        CODE_VA as usize,
        user_as1,
    );
    println!("  User thread {}, entry=0x{:x}", ut, CODE_VA);
    println!("Boot thread yielding to user thread");
    println!();

    // yield_now → context_switch → eret_to_user_stub → EL0 → SVC → handler → eret → EL0
    scheduler::yield_now();
    // After yield_now returns, the boot thread has been rescheduled.
    // The user thread is in an infinite loop in EL0 (b . after second SVC).
    println!("Boot thread resumed (user thread still looping in EL0)");
    loop {
        scheduler::yield_now();
    }
}

/// Minimal worker thread that yields forever.
#[allow(dead_code)]
extern "C" fn thread_worker(_arg: usize) {
    loop {
        scheduler::yield_now();
    }
}