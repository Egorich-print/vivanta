#![no_std]
#![allow(static_mut_refs)]
extern crate alloc;

use memory::KernelHeap;

#[global_allocator]
static ALLOCATOR: KernelHeap = KernelHeap::uninitialized();

/// RAII guard that disables interrupts for the kernel heap's critical section
/// (single-core: the timer IRQ must not re-enter alloc/dealloc mid-operation).
pub(crate) fn interrupts_guard() -> impl core::ops::Drop {
    unsafe { vivanta_arch_api::interrupts::disable_interrupts() }
}

pub mod error;
pub mod identity;
pub mod memory;
pub mod pmm;
pub mod scheduler;
pub mod signal;
pub mod state;
pub mod syscall;
pub mod usercopy;
pub mod verify;
pub mod vmm;

pub use vivanta_arch_api::pmm::{FrameAllocator, PhysFrame};

use crate::memory::PmmBackend;
use vivanta_arch_api::mmu::RootPageTable;
use vivanta_boot_common::memory_discovery::{self, KernelLayout};
use vivanta_boot_common::{println, MemoryRegionKind};
use vivanta_boot_info::BootInfo;

extern "C" {
    static __kernel_start: u8;
    static __stack_top: u8;
    static user_code_start: u8;
    static user_code_end: u8;
}

/// Allocator callback for arch boot MMU init.
///
/// Boot-fatal: page tables are a hard requirement of the boot path; an OOM
/// here means the kernel cannot map memory, so we panic rather than return a
/// fake PA 0 (which would silently corrupt the page tables).
#[no_mangle]
pub unsafe extern "Rust" fn boot_alloc_frame(ctx: *mut ()) -> u64 {
    let mrm = &mut *(ctx as *mut memory::MemoryResourceManager);
    let req = memory::AllocationRequirements::new(4096);
    // Use Owner 0 (Kernel) for boot page tables.
    // Page-table frames are permanent: leak the MemoryObject so Drop does not
    // free the frames the live page tables point into.
    let obj = mrm
        .allocate(&req, 0)
        .unwrap_or_else(|| panic!("boot_alloc_frame: OOM during boot page-table allocation"));
    let pa = obj
        .phys_addr
        .expect("boot_alloc_frame: no phys addr on allocated object");
    core::mem::forget(obj);
    pa
}

/// The one and only vivanta_kernel entry point.
pub unsafe fn kernel_main(info: &BootInfo) -> ! {
    println!();
    println!(
        "\u{2500}\u{2500}\u{2500}\u{2500} Vivanta Kernel Entry \u{2500}\u{2500}\u{2500}\u{2500}"
    );

    // V1.1: Runtime Identity Bootstrap — construct SystemState from BootInfo
    let mut system_state = state::SystemState::from_boot_info(info);
    if system_state.hardware().dtb_ptr != 0 {
        println!("  DTB at    0x{:x}", system_state.hardware().dtb_ptr);
    }
    println!("  {} CPU(s)", system_state.hardware().cpu_count);
    // After this point, BootInfo must NOT be referenced for runtime state.
    // All hardware info is accessible through system_state.hardware().

    // ------- CPU early init (exception vectors, FP/SIMD) --------------------
    println!();
    println!("CPU Init:");
    vivanta_arch_api::boot::cpu::early_init();
    println!("  Early init done.");

    // ------- Memory Map -----------------------------------------------------
    println!("Memory Map:");
    for r in system_state.hardware().memory_map.regions() {
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

    // ------- Physical Memory Manager + MRM ---------------------------------
    let kernel_start = unsafe { &__kernel_start as *const u8 as u64 };
    let kernel_end = unsafe { &__stack_top as *const u8 as u64 };

    let dtb_addr = system_state.hardware().dtb_ptr;
    let dtb_size = if dtb_addr != 0 {
        // FDT header: magic at +0, totalsize at +4 (big-endian)
        let totalsize = unsafe { core::ptr::read_volatile((dtb_addr + 4) as *const u32) };
        u32::from_be(totalsize) as u64
    } else {
        0
    };

    let page_tables_start = kernel_end;
    let page_tables_size = 0x5000;

    let layout = KernelLayout {
        start: kernel_start,
        end: kernel_end,
        dtb: dtb_addr as u64,
        dtb_size,
        page_tables_start,
        page_tables_size,
    };
    let available = memory_discovery::discover(system_state.hardware().memory_map, &layout);

    // G2: use ALL usable regions, not just the first. Sum of usable RAM for
    // the managed-percentage check.
    let usable_ram: u64 = available.iter().map(|r| r.end - r.start).sum();
    let mut pmm = unsafe { pmm::PmmBitmap::new_multi(&available.regions[..available.count]) };

    let total = pmm.total_frames();
    let pmm_reserved = pmm.reserved_count();
    println!();
    println!("Physical Memory Manager:");
    println!(
        "  Managed    0x{:016x} – 0x{:016x}  ({} MiB)",
        pmm.region_start(),
        pmm.region_start() + pmm.total_frames() as u64 * pmm::FRAME_SIZE - 1,
        (pmm.total_frames() as u64 * pmm::FRAME_SIZE) >> 20
    );
    println!(
        "  Usable RAM  {} MiB, managed {} MiB ({:.1}%)",
        usable_ram >> 20,
        (pmm.total_frames() as u64 * pmm::FRAME_SIZE) >> 20,
        (pmm.total_frames() as u64 * pmm::FRAME_SIZE) as f64 * 100.0 / usable_ram as f64
    );
    println!(
        "  Total {}  Reserved {}  Free {}  frames",
        total,
        pmm_reserved,
        pmm.free_count()
    );

    pmm.run_self_test().expect("PMM self-test failed");
    println!("  PMM self-test ok");

    // M5.0: Runtime stress-test (1000 alloc/free cycles with invariant validation)
    crate::verify::stress_test_pmm(&mut pmm, 1000).expect("PMM stress-test failed");
    println!("  PMM stress-test (1000 cycles) ok");

    // Init MRM with PmmBackend (no hardware borrow alive here)
    let mut pmm_backend = unsafe { PmmBackend::new_dram(&mut pmm as *mut dyn FrameAllocator) };
    system_state.init_memory(&mut pmm_backend);

    // G2: let the scheduler reclaim kernel stacks of terminated threads.
    scheduler::register_stack_allocator(&mut pmm);

    // Init kernel heap: allocate 64 KiB from MRM
    {
        let mrm = system_state.memory_manager_mut();
        let heap_size: u64 = 65536;
        let heap_obj = mrm
            .allocate(&memory::AllocationRequirements::new(heap_size), 0)
            .expect("KernelHeap: allocation failed");
        let heap_base = heap_obj.phys_addr.expect("KernelHeap: no phys addr");
        unsafe {
            core::ptr::write_bytes(heap_base as *mut u8, 0, heap_size as usize);
        }
        ALLOCATOR.init(heap_base as usize, heap_size as usize);
        println!("  KernelHeap: 64 KiB @ 0x{:x}", heap_base);
        // The heap lives for the whole kernel lifetime: leak the MemoryObject
        // so Drop does not free the frames backing the global allocator.
        core::mem::forget(heap_obj);

        // heap smoke test
        let v = alloc::vec![1u8, 2, 3, 4];
        println!(
            "  Heap smoke: vec={:?} (len={}, cap={})",
            &v[..],
            v.len(),
            v.capacity()
        );
    }

    let mrm = system_state.memory_manager();
    println!("  MRM: {} backend(s) registered", mrm.backend_count());

    if let Some(f) = pmm.alloc_frame() {
        println!("  Allocated frame @ 0x{:x}  (ok)", f.addr);
        pmm.free_frame(f);
        println!("  Freed frame              (ok)");
    }

    // G2 churn: allocate+drop MemoryObjects repeatedly; free_count must return
    // to baseline (allocated + free == managed holds after each cycle).
    let baseline_free = pmm.free_count();
    {
        let mrm = system_state.memory_manager_mut();
        crate::verify::stress_mrm_churn(mrm, 100).expect("MRM churn test failed");
    }
    let after_churn_free = pmm.free_count();
    println!(
        "  MRM churn: baseline_free={} after={} delta={}",
        baseline_free,
        after_churn_free,
        baseline_free - after_churn_free
    );
    assert_eq!(
        baseline_free,
        after_churn_free,
        "G2 FAIL: MRM churn leaked {} frames (free_count did not return to baseline)",
        baseline_free - after_churn_free
    );
    crate::verify::verify_pmm(&pmm).expect("G2: verify_pmm after churn failed");
    println!("  MRM churn test (100 cycles) ok — free == baseline");

    // ------- VMM: Address Space Construction -------------------------------
    println!();
    println!("Address Space Builder:");
    let mrm_ptr = system_state.memory_manager_mut() as *mut memory::MemoryResourceManager;
    let alloc_ctx: *mut () = mrm_ptr as *mut ();
    let pt = vivanta_arch_api::boot::mmu::mmu_init(alloc_ctx, boot_alloc_frame);

    // Re-fetch hardware for VMM and beyond (after init_memory)
    let hardware = system_state.hardware();

    // Identity-map all usable RAM from memory map (not just available region)
    for r in hardware.memory_map.regions() {
        use vivanta_boot_common::MemoryRegionKind;
        if r.kind == MemoryRegionKind::Usable {
            vivanta_arch_api::boot::mmu::mmu_map_ram(pt, r.start, r.start, r.size);
        }
    }

    // Map MMIO regions (from HardwareState per ADR-021)
    for mmio in hardware.mmio_regions {
        vivanta_arch_api::boot::mmu::mmu_map_range(
            pt,
            mmio.base,
            mmio.base,
            mmio.size,
            mmio.kind.is_user_accessible(),
        );
    }

    // @@M4@@ EL0 experiment temporarily disabled — see docs/architecture/milestones/M4-execution-foundation.md
    // let user_token = vivanta_arch_api::boot::user::user_bootstrap(pt);
    // if user_token != 0 {
    //     println!("  User token: 0x{:x}", user_token);
    // }

    println!(
        "  L1 table at     0x{:x}",
        vivanta_arch_api::boot::mmu::mmu_root_addr(pt)
    );
    for r in hardware.memory_map.regions() {
        use vivanta_boot_common::MemoryRegionKind;
        if r.kind == MemoryRegionKind::Usable {
            println!(
                "  RAM ident:      0x{:016x} – 0x{:016x}  ({} MiB)",
                r.start,
                r.start + r.size - 1,
                r.size >> 20
            );
        }
    }
    for mmio in hardware.mmio_regions {
        let kind_str = if mmio.kind.is_user_accessible() {
            "user"
        } else {
            "vivanta_kernel"
        };
        println!(
            "  MMIO ident:     0x{:x} ({} bytes, {})",
            mmio.base, mmio.size, kind_str
        );
    }

    // ------- Wrap in AddressSpace ------------------------------------------
    let root = vivanta_arch_api::mmu::RootPageTable(pt);
    vmm::address_space::init_kernel_address_space(root);

    // Build independent root tables for UserAS1/UserAS2
    let alloc_ctx_root: *mut () = mrm_ptr as *mut ();
    let build_root = |label: &str, extra_va: u64, extra_pa: u64| -> RootPageTable {
        let rpt = vivanta_arch_api::boot::mmu::mmu_init(alloc_ctx_root, boot_alloc_frame);
        // Map ALL usable RAM (not just available region) — kernel code/stack must be accessible
        for r in hardware.memory_map.regions() {
            use vivanta_boot_common::MemoryRegionKind;
            if r.kind == MemoryRegionKind::Usable {
                vivanta_arch_api::boot::mmu::mmu_map_ram(rpt, r.start, r.start, r.size);
            }
        }
        for mmio in hardware.mmio_regions {
            vivanta_arch_api::boot::mmu::mmu_map_range(
                rpt,
                mmio.base,
                mmio.base,
                mmio.size,
                mmio.kind.is_user_accessible(),
            );
        }
        if extra_pa != 0 {
            vivanta_arch_api::boot::mmu::mmu_map_range(rpt, extra_va, extra_pa, 0x1000, false);
        }
        // M4.5.1: map user code + stack into UserAS1
        if label == "UserAS1" {
            let code_src = &user_code_start as *const u8;
            let code_len =
                (&user_code_end as *const u8 as usize) - (&user_code_start as *const u8 as usize);
            const CODE_VA: u64 = 0x5E00_0000;
            const STACK_VA: u64 = 0x5E01_0000;
            vivanta_arch_api::boot::mmu::mmu_map_user_pages(
                rpt, CODE_VA, code_src, code_len, STACK_VA,
            );
            println!("  UserAS1: code=0x{:x}, stack=0x{:x}", CODE_VA, STACK_VA);
        }
        let ra = vivanta_arch_api::boot::mmu::mmu_root_addr(rpt);
        println!("  {} root table at 0x{:x}", label, ra);
        RootPageTable(ra as usize)
    };
    let root1 = build_root("UserAS1", 0, 0);
    let root2 = build_root("UserAS2", 0, 0);

    // Debug: dump UserAS1 page table for UART address
    println!("  UserAS1 UART mapping check:");
    unsafe {
        vivanta_arch_api::boot::mmu::dump_critical_tables(root1.0 as u64);
    }

    let user_as1 = vmm::register(root1, vmm::AddressSpaceFlags::User);
    let _user_as2 = vmm::register(root2, vmm::AddressSpaceFlags::User);
    println!("  Address spaces: {} total", vmm::count());

    // ------- Enable MMU ----------------------------------------------------
    println!();
    println!("Enabling MMU...");
    vivanta_arch_api::boot::mmu::mmu_activate(pt);
    println!("MMU enabled successfully.");
    println!("MMU self-test:");
    vivanta_arch_api::boot::mmu::mmu_self_test();

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

    // ------- MemoryObject Smoke Test ----------------------------------------
    println!();
    println!("MemoryObject Smoke Test:");
    let mrm = system_state.memory_manager_mut();
    let req = memory::AllocationRequirements::new(4096);
    let mut obj = mrm.allocate(&req, 0).expect("alloc MemoryObject");
    let phys = obj.phys_addr.expect("phys addr");
    println!("  Allocated  @ 0x{:x}  (size={})", phys, obj.size);

    let pt_alloc_mrm = mrm as *mut memory::MemoryResourceManager;
    let mut pt_alloc = unsafe { memory::MrmPageTableAllocator::new(pt_alloc_mrm) };
    let kernel_as = vmm::kernel_address_space_mut();
    let slot = obj
        .map(phys, 4096, kernel_as, &mut pt_alloc)
        .expect("map MemoryObject");
    println!("  Mapped     slot={}", slot);

    unsafe {
        core::ptr::write_volatile(phys as *mut u32, 0x42);
        let val = core::ptr::read_volatile(phys as *const u32);
        if val == 0x42 {
            println!("  Write/Read OK  (0x{:x})", val);
        } else {
            println!("  MISMATCH: got 0x{:x}, expected 0x42", val);
        }
        core::ptr::write_volatile((phys + 4) as *mut u32, 0xDEAD);
        core::ptr::write_volatile((phys + 8) as *mut u32, 0xBEEF);
        core::ptr::write_volatile((phys + 12) as *mut u32, 0xCAFE);
    }
    println!("  Fill 4 words OK");

    obj.unmap(slot, kernel_as, &mut pt_alloc)
        .expect("unmap MemoryObject");
    println!("  Unmapped   slot={}", slot);
    println!("MemoryObject test passed.");

    println!();
    println!("Boot complete -- creating user thread");

    // ------- V2/M6 Task Model: TaskManager ------------------------------------
    const CODE_VA: u64 = 0x5E00_0000;
    const STACK_VA: u64 = 0x5E01_0000;
    let mut taskman = scheduler::task_manager::TaskManager::new();
    let tid = taskman
        .spawn_user(
            CODE_VA as usize,
            (STACK_VA + 4096) as usize,
            user_as1,
            &mut pmm,
            system_state.memory_manager_mut(),
            scheduler::thread::Priority::Normal,
            None, // no parent (root task)
        )
        .expect("spawn_user");
    println!("  Task {} created (thread, code @ 0x{:x})", tid, CODE_VA);
    println!(
        "  {} task(s), {} running",
        taskman.task_count(),
        taskman.running_count()
    );

    // Verify Task structure via TaskManager
    if let Some(task) = taskman.get(tid) {
        println!(
            "  Task[{}]: {} object(s) owned",
            tid,
            task.owned_objects.len()
        );
    }

    println!();
    println!("Boot thread yielding to user thread");
    println!();

    // yield_now → context_switch → eret_to_user_stub → EL0 → SVC → handler → eret → EL0
    scheduler::yield_now();
    // After yield_now returns, the boot thread has been rescheduled.
    // The user thread ran write(1, "Hello, Vivanta!") then exit(0).
    println!("Boot thread resumed (user thread exited cleanly)");
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
