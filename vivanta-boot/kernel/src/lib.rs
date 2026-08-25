#![no_std]
#![allow(static_mut_refs)]
extern crate alloc;

use core::sync::atomic::{AtomicU64, Ordering};

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
use vivanta_boot_common::{MemoryRegionKind, println};
use vivanta_boot_info::BootInfo;

unsafe extern "C" {
    static __kernel_start: u8;
    static __stack_top: u8;
    static user_code_start: u8;
    static user_code_end: u8;
    static fault_code_start: u8;
    static fault_code_end: u8;
    static exec_nx_code_start: u8;
    static exec_nx_code_end: u8;
    static kread_code_start: u8;
    static kread_code_end: u8;
    static unmapped_code_start: u8;
    static unmapped_code_end: u8;
}

/// Allocator callback for arch boot MMU init.
///
/// Boot-fatal: page tables are a hard requirement of the boot path; an OOM
/// here means the kernel cannot map memory, so we panic rather than return a
/// fake PA 0 (which would silently corrupt the page tables).
#[unsafe(no_mangle)]
pub unsafe extern "Rust" fn boot_alloc_frame(ctx: *mut ()) -> u64 {
    unsafe {
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
}

/// The one and only vivanta_kernel entry point.
pub unsafe fn kernel_main(info: &BootInfo) -> ! {
    unsafe {
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
        vivanta_boot_common::set_console_irq_guard(|| {
            vivanta_arch_api::interrupts::disable_interrupts()
        });
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
        let kernel_start = &raw const __kernel_start as u64;
        let kernel_end = &raw const __stack_top as u64;

        let dtb_addr = system_state.hardware().dtb_ptr;
        let dtb_size = if dtb_addr != 0 {
            // FDT header: magic at +0, totalsize at +4 (big-endian)
            let totalsize = core::ptr::read_volatile((dtb_addr + 4) as *const u32);
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
        let mut pmm = pmm::PmmBitmap::new_multi(&available.regions[..available.count]);

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
        let mut pmm_backend = PmmBackend::new_dram(&mut pmm as *mut dyn FrameAllocator);
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
            core::ptr::write_bytes(heap_base as *mut u8, 0, heap_size as usize);
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

        // Re-fetch hardware for VMM and beyond (after init_memory).
        // The &'static fields are copied out so the root-builder closures
        // hold raw copies instead of keeping the SystemState borrow alive
        // across the memory_manager_mut() calls below.
        let (memory_map, mmio_regions) = {
            let hw = system_state.hardware();
            (hw.memory_map, hw.mmio_regions)
        };

        // Identity-map all usable RAM from memory map (not just available region)
        for r in memory_map.regions() {
            use vivanta_boot_common::MemoryRegionKind;
            if r.kind == MemoryRegionKind::Usable {
                vivanta_arch_api::boot::mmu::mmu_map_ram(pt, r.start, r.start, r.size);
            }
        }

        // Map MMIO regions (from HardwareState per ADR-021)
        for mmio in mmio_regions {
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
        for r in memory_map.regions() {
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
        for mmio in mmio_regions {
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
        let build_root = |label: &str,
                          extra_va: u64,
                          extra_pa: u64,
                          user_pages: Option<(*const u8, usize, u64, u64)>|
         -> RootPageTable {
            let rpt = vivanta_arch_api::boot::mmu::mmu_init(alloc_ctx_root, boot_alloc_frame);
            // Map ALL usable RAM (not just available region) — kernel code/stack must be accessible
            for r in memory_map.regions() {
                use vivanta_boot_common::MemoryRegionKind;
                if r.kind == MemoryRegionKind::Usable {
                    vivanta_arch_api::boot::mmu::mmu_map_ram(rpt, r.start, r.start, r.size);
                }
            }
            for mmio in mmio_regions {
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
            // Map user code + stack when provided (demo, fault task and
            // Phase-10 protection-fault scenarios all use this path).
            if let Some((code_src, code_len, code_va, stack_va)) = user_pages {
                vivanta_arch_api::boot::mmu::mmu_map_user_pages(
                    rpt, code_va, code_src, code_len, stack_va,
                );
                println!("  {}: code=0x{:x}, stack=0x{:x}", label, code_va, stack_va);
            }
            let ra = vivanta_arch_api::boot::mmu::mmu_root_addr(rpt);
            println!("  {} root table at 0x{:x}", label, ra);
            RootPageTable(ra as usize)
        };
        let demo_user_pages = {
            let code_src = &raw const user_code_start;
            let code_len =
                (&raw const user_code_end as usize) - (&raw const user_code_start as usize);
            Some((code_src, code_len, 0x5E00_0000u64, 0x5E01_0000u64))
        };
        let fault_user_pages = {
            let code_src = &raw const fault_code_start;
            let code_len =
                (&raw const fault_code_end as usize) - (&raw const fault_code_start as usize);
            Some((code_src, code_len, 0x5F00_0000u64, 0x5F01_0000u64))
        };
        let root1 = build_root("UserAS1", 0, 0, demo_user_pages);
        let root2 = build_root("UserAS2", 0, 0, fault_user_pages);

        // G3 W^X verification: read back the live leaf descriptors of both user
        // address spaces and assert user code is EL0 read-only+executable and
        // user stacks are EL0 read-write, non-executable.
        println!("  W^X permission verification:");
        const WX_CODE_VA: u64 = 0x5E00_0000;
        const WX_STACK_VA: u64 = 0x5E01_0000;
        vivanta_arch_api::boot::mmu::wx_verify_user_as(root1.0 as u64, WX_CODE_VA, WX_STACK_VA);
        const WX_FAULT_CODE_VA: u64 = 0x5F00_0000;
        const WX_FAULT_STACK_VA: u64 = 0x5F01_0000;
        vivanta_arch_api::boot::mmu::wx_verify_user_as(
            root2.0 as u64,
            WX_FAULT_CODE_VA,
            WX_FAULT_STACK_VA,
        );

        // Debug: dump UserAS1 page table for UART address
        println!("  UserAS1 UART mapping check:");
        vivanta_arch_api::boot::mmu::dump_critical_tables(root1.0 as u64);

        let user_as1 = vmm::register(root1, vmm::AddressSpaceFlags::User);
        let user_as2 = vmm::register(root2, vmm::AddressSpaceFlags::User);
        println!("  Address spaces: {} total", vmm::count());

        // ------- Enable MMU ----------------------------------------------------
        println!();
        println!("Enabling MMU...");
        vivanta_arch_api::boot::mmu::mmu_activate(pt);
        println!("MMU enabled successfully.");
        println!("MMU self-test:");
        vivanta_arch_api::boot::mmu::mmu_self_test();

        // ------- GIC Discovery & Initialisation --------------------------------
        if dtb_addr != 0 {
            println!();
            println!("Interrupt Controller:");
            vivanta_arch_api::boot::irq::irq_init(dtb_addr);
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
        let mut pt_alloc = memory::MrmPageTableAllocator::new(pt_alloc_mrm);
        let kernel_as = vmm::kernel_address_space_mut();
        let slot = obj
            .map(phys, 4096, kernel_as, &mut pt_alloc)
            .expect("map MemoryObject");
        println!("  Mapped     slot={}", slot);

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
        println!("  Fill 4 words OK");

        obj.unmap(slot, kernel_as, &mut pt_alloc)
            .expect("unmap MemoryObject");
        println!("  Unmapped   slot={}", slot);
        println!("MemoryObject test passed.");

        // ------------------------------------------------------------------
        // Phase-7/8 protection audit: permission transitions + TLB coherence
        // on the ACTIVE kernel address space. The page is first written via
        // its identity VA (establishing a live RW translation), then cycled
        // RW -> RO -> RW through AddressSpace::protect. The final write is
        // the discriminating assertion: a stale RO TLB entry from the RO
        // phase would permission-fault here and panic the kernel.
        // ------------------------------------------------------------------
        println!("Protect/TLBI transition test:");
        let probe = pmm.alloc_frame().expect("protect-test frame");
        let kva: u64 = probe.addr;
        core::ptr::write_volatile(kva as *mut u64, 0xA11CE);
        {
            let mrm = system_state.memory_manager_mut();
            let mut pt_alloc = memory::MrmPageTableAllocator::new(mrm as *mut _);
            let kas = vmm::kernel_address_space_mut();
            use vivanta_arch_api::mmu::MappingFlags as ApiMFlags;
            // Track the page in the shadow; splits the surrounding 2 MiB
            // identity block into pages (map_pages split path).
            kas.map_pages(
                kva,
                kva,
                4096,
                ApiMFlags::read_write(),
                &mut pt_alloc,
                0xF00D,
            )
            .expect("map protect-test page");
            // RW -> RO: mapping must stay readable.
            kas.protect(kva, 4096, ApiMFlags::from_bits(0), &mut pt_alloc)
                .expect("protect RW->RO");
            // Force a full TLB eviction (what a context switch does) so the
            // read below repopulates the TLB from the NEW RO descriptor.
            // Without this, a missed TLBI in protect would be masked by the
            // still-cached RW entry from the initial write.
            vivanta_arch_api::mmu::activate_address_space(kas.root);
            let ro_val = core::ptr::read_volatile(kva as *const u64);
            assert_eq!(ro_val, 0xA11CE, "RW->RO broke the mapping");
            // RO -> RW: stale RO TLB entry would fault the write below.
            kas.protect(kva, 4096, ApiMFlags::read_write(), &mut pt_alloc)
                .expect("protect RO->RW");
            core::ptr::write_volatile(kva as *mut u64, 0xB0B);
            let val = core::ptr::read_volatile(kva as *const u64);
            assert_eq!(val, 0xB0B, "stale TLB permissions survived RO->RW");
        }
        pmm.free_frame(probe);
        println!("  Protect/TLBI transitions PASS (RW->RO->RW, write-after-restore ok)");

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

        // ------------------------------------------------------------------
        // G3 fault-containment test: spawn a user task that deliberately faults.
        // The faulting task must be terminated (user_fault_terminate →
        // thread_exit), its resources reclaimed, and other threads must continue.
        // ------------------------------------------------------------------
        println!();
        println!("G3 fault-containment test: spawning faulting task");
        const FAULT_CODE_VA: usize = 0x5F00_0000;
        let mut faultman = scheduler::task_manager::TaskManager::new();
        let ftid = faultman
            .spawn_user(
                FAULT_CODE_VA,
                0x5F01_1000,
                user_as2,
                &mut pmm,
                system_state.memory_manager_mut(),
                scheduler::thread::Priority::Normal,
                None,
            )
            .expect("spawn fault task");
        println!("  fault task {} spawned, yielding to it", ftid);
        // Let the faulting task run; it faults and is terminated by the kernel.
        scheduler::yield_now();
        scheduler::yield_now();
        println!("  boot thread survived the faulting task (containment OK)");
        let (fa, fb) = (
            PREEMPT_COUNTER_A.load(Ordering::Relaxed),
            PREEMPT_COUNTER_B.load(Ordering::Relaxed),
        );
        println!("  preempt counters before test: A={} B={}", fa, fb);

        // ------------------------------------------------------------------
        // Phase-10 protection audit: hardware-visible EL0 fault scenarios.
        // Each scenario runs in its own address space; every blob faults on
        // its FIRST instruction sequence. Expected outcomes are asserted
        // against the recorded (ESR, FAR) of the actual exception:
        //   exec-nx   : branch to XN stack page -> instruction abort
        //               EC=0b100000(32), permission fault L3 (IFSC 0x0F)
        //   kread     : load from kernel-only AP=00 block -> data abort,
        //               permission fault L2 (DFSC 0x0E)
        //   unmapped  : load where no descriptor exists -> data abort,
        //               translation fault L2 (DFSC 0x06; translation faults
        //               are 0x05/06/07 per level, permission 0x0D/0E/0F)
        // A scenario whose fault is not delivered falls through to exit(7),
        // which fails the exit_code assertion below.
        // ------------------------------------------------------------------
        println!("Phase-10 protection fault scenarios:");
        const SCEN_CODE_VA: u64 = 0x5D00_0000;
        const SCEN_STACK_VA: u64 = 0x5D01_0000;
        let mut run_fault_scenario = |name: &str,
                                      code_src: *const u8,
                                      code_len: usize,
                                      expect_ec: u64,
                                      expect_dfsc: u64,
                                      expect_far: u64| {
            let root = build_root(
                name,
                0,
                0,
                Some((code_src, code_len, SCEN_CODE_VA, SCEN_STACK_VA)),
            );
            let as_id = vmm::register(root, vmm::AddressSpaceFlags::User);
            let mut tm = scheduler::task_manager::TaskManager::new();
            let tid = tm
                .spawn_user(
                    SCEN_CODE_VA as usize,
                    (SCEN_STACK_VA + 4096) as usize,
                    as_id,
                    &mut pmm,
                    system_state.memory_manager_mut(),
                    scheduler::thread::Priority::Normal,
                    None,
                )
                .expect("spawn fault-scenario task");
            scheduler::yield_now();
            scheduler::yield_now();
            let task = tm.get(tid).expect("scenario task missing");
            assert_eq!(
                task.exit_code,
                Some(-1),
                "scenario {}: fault not delivered (exit_code={:?})",
                name,
                task.exit_code
            );
            let (esr, far) = vivanta_arch_api::boot::user::last_el0_fault();
            let ec = (esr >> 26) & 0x3f;
            let dfsc = esr & 0x3f;
            assert_eq!(ec, expect_ec, "scenario {}: wrong EC", name);
            assert_eq!(dfsc, expect_dfsc, "scenario {}: wrong DFSC", name);
            assert_eq!(far, expect_far, "scenario {}: wrong FAR", name);
            println!(
                "  [FAULT] {}: EC={} DFSC={:#x} FAR={:#x} — PASS",
                name, ec, dfsc, far
            );
            tm.reap_zombie(tid);
        };
        {
            let src = &raw const exec_nx_code_start;
            let len =
                (&raw const exec_nx_code_end as usize) - (&raw const exec_nx_code_start as usize);
            run_fault_scenario("exec-nx", src, len, 0b100000, 0x0F, SCEN_STACK_VA);
        }
        {
            let src = &raw const kread_code_start;
            let len = (&raw const kread_code_end as usize) - (&raw const kread_code_start as usize);
            run_fault_scenario("kread", src, len, 0b100100, 0x0E, 0x4020_0000);
        }
        {
            let src = &raw const unmapped_code_start;
            let len =
                (&raw const unmapped_code_end as usize) - (&raw const unmapped_code_start as usize);
            run_fault_scenario("unmapped", src, len, 0b100100, 0x06, 0x7000_0000);
        }

        // ------------------------------------------------------------------
        // M5.1/M5.2 VM lifecycle test: VA allocator + range mapping +
        // partial protect + table reclamation + alias safety, exercised
        // against the live MMU in a dedicated address space.
        //
        // The test address space is ACTIVATED for the duration; IRQs are
        // masked so no scheduler switch can observe the non-kernel TTBR0
        // (the scheduler skips re-activation when next_as == current_as).
        // ------------------------------------------------------------------
        println!("VM lifecycle test:");
        let vm_root = build_root("VmTestAS", 0, 0, None);
        let vm_as = vmm::register(vm_root, vmm::AddressSpaceFlags::User);
        {
            let mrm = system_state.memory_manager_mut();
            let mut as_alloc = memory::AsPageTableAllocator::new(
                mrm as *mut _,
                &raw mut pmm_backend as *mut dyn memory::MemoryBackend,
                vm_as,
            );
            let kas_root = vmm::lookup_root(crate::vmm::KERNEL_ADDRESS_SPACE_ID);
            // ---- activate test AS, IRQs off -----------------------------
            let _irq = vivanta_arch_api::interrupts::disable_interrupts();
            vivanta_arch_api::mmu::activate_address_space(vm_root);

            // 1. allocate + map 3 pages from the VA allocator.
            // OWNERSHIP RULE: mapping 3 pages requires owning 3 frames.
            let frame = pmm
                .alloc_contiguous(3)
                .expect("vm-test contiguous 3 frames");
            let va0 = vmm::address_space_mut_by(vm_as)
                .map_new_range(
                    frame.addr,
                    3 * 4096,
                    vivanta_arch_api::mmu::MappingFlags::read_write(),
                    777,
                    4096,
                    &mut as_alloc,
                )
                .expect("map_new_range");
            assert_eq!(va0 % 4096, 0);
            assert!(va0 >= vmm::USER_VA_BASE && va0 < vmm::USER_VA_END);

            // Write via the new VAs — proves real translation. Frames from
            // the PMM are NOT zeroed, so values are written before any read.
            core::ptr::write_volatile(va0 as *mut u64, 0x1111);
            core::ptr::write_volatile((va0 + 4096) as *mut u64, 0x2222);
            core::ptr::write_volatile((va0 + 8192) as *mut u64, 0x3333);
            assert_eq!(core::ptr::read_volatile(va0 as *const u64), 0x1111);

            // 2. partial-range protect: middle page only -> shadow splits.
            vmm::address_space_mut_by(vm_as)
                .protect(
                    va0 + 4096,
                    4096,
                    vivanta_arch_api::mmu::MappingFlags::from_bits(0),
                    &mut as_alloc,
                )
                .expect("protect middle page");
            assert_eq!(
                core::ptr::read_volatile((va0 + 4096) as *const u64),
                0x2222,
                "RO page must stay readable with its old content"
            );
            core::ptr::write_volatile((va0 + 8192) as *mut u64, 0x3334);
            {
                let aspace = vmm::address_space_mut_by(vm_as);
                let head = aspace.query(va0).expect("head piece");
                let mid = aspace.query(va0 + 4096).expect("covered piece");
                let tail = aspace.query(va0 + 8192).expect("tail piece");
                assert_ne!(head.virt_range.base, mid.virt_range.base);
                assert_ne!(mid.virt_range.base, tail.virt_range.base);
                assert!(head.permissions.is_read_write());
                assert!(!mid.permissions.is_read_write(), "covered piece not RO");
                assert!(tail.permissions.is_read_write());
            }

            // 3. restore RW (TLBI proof: stale RO entry would fault).
            vmm::address_space_mut_by(vm_as)
                .protect(
                    va0 + 4096,
                    4096,
                    vivanta_arch_api::mmu::MappingFlags::read_write(),
                    &mut as_alloc,
                )
                .expect("restore RW");
            core::ptr::write_volatile((va0 + 4096) as *mut u64, 0x2222);
            assert_eq!(core::ptr::read_volatile((va0 + 4096) as *const u64), 0x2222);

            // 4. alias regression: same PA at two VAs; unmapping one must
            //    not affect the other nor free the physical frame.
            let va_alias = vmm::address_space_mut_by(vm_as)
                .map_new_range(
                    frame.addr,
                    4096,
                    vivanta_arch_api::mmu::MappingFlags::read_write(),
                    778,
                    4096,
                    &mut as_alloc,
                )
                .expect("alias map");
            core::ptr::write_volatile(va_alias as *mut u64, 0xABCD);
            assert_eq!(
                core::ptr::read_volatile(va0 as *const u64),
                0xABCD,
                "alias does not share translations"
            );
            vmm::address_space_mut_by(vm_as)
                .unmap_range(va_alias, 4096, &mut as_alloc)
                .unwrap();
            assert_eq!(
                core::ptr::read_volatile(va0 as *const u64),
                0xABCD,
                "unmap of alias broke original mapping"
            );

            // 5. unmap everything -> runtime-created tables become empty
            //    and are reclaimed. Only the L3 is registry-tracked here:
            //    the containing L2 was created at boot (MMIO identity) and
            //    belongs to the intentional-leak model.
            let free_before_unmap = pmm.free_count();
            let tracked_pre = vmm::tables::count_for_as(vm_as);
            vmm::address_space_mut_by(vm_as)
                .unmap_range(va0, 3 * 4096, &mut as_alloc)
                .unwrap();
            let reclaimed_now = tracked_pre - vmm::tables::count_for_as(vm_as);
            let free_delta = pmm.free_count() - free_before_unmap;
            println!(
                "  [VM] unmap reclaimed {} table frames (PMM +{})",
                reclaimed_now, free_delta
            );
            assert!(reclaimed_now >= 1, "expected at least the L3 reclaimed");
            assert_eq!(free_delta, reclaimed_now, "reclaimed frames not in PMM");

            // 6. block-split case: map INTO an identity block region, then
            //    unmap. The L3 keeps 511 split-inherited entries -> must NOT
            //    be reclaimed ("no leaf mappings" != "unreachable").
            let split_va = frame.addr; // identity VA inside a 2 MiB block
            {
                let aspace = vmm::address_space_mut_by(vm_as);
                aspace
                    .map_pages(
                        split_va,
                        split_va,
                        4096,
                        vivanta_arch_api::mmu::MappingFlags::read_write(),
                        &mut as_alloc,
                        779,
                    )
                    .expect("block-split map");
                // Descriptor audit of the split-inherited page: QEMU does
                // not enforce AF, so assert the attribute bits directly.
                let desc = vivanta_arch_api::mmu::mmu_leaf_descriptor(
                    vmm::lookup_root(vm_as).0 as u64,
                    split_va,
                );
                assert!(desc & 1 == 1, "split leaf invalid");
                assert!(
                    desc & 0x400 != 0,
                    "split leaf lost AF (access flag): desc={desc:#x}"
                );
                // Neighbor page is purely split-inherited (its leaf comes
                // from split_l2_block, not from the mapping write).
                let nb = vivanta_arch_api::mmu::mmu_leaf_descriptor(
                    vmm::lookup_root(vm_as).0 as u64,
                    split_va + 0x1000,
                );
                assert!(nb & 1 == 1, "split neighbor invalid");
                assert!(
                    nb & 0x400 != 0,
                    "split-inherited neighbor lost AF: desc={nb:#x}"
                );
                let tables_mid = vmm::tables::total();
                aspace
                    .unmap_pages(split_va, 4096, &mut as_alloc)
                    .expect("block-split unmap");
                assert_eq!(
                    vmm::tables::total(),
                    tables_mid,
                    "split-inherited table was wrongly reclaimed"
                );
            }

            // 7. remap after full unmap — allocator handed the range back.
            //    Structural invariant: the remap MUST install fresh tables
            //    through the ownership registry. If a previous reclamation
            //    skipped the parent unlink, the walk would silently reuse
            //    the freed table and no registry entry would appear.
            {
                let aspace = vmm::address_space_mut_by(vm_as);
                let tables_pre_remap = vmm::tables::count_for_as(vm_as);
                let va_re = aspace
                    .map_new_range(
                        frame.addr,
                        4096,
                        vivanta_arch_api::mmu::MappingFlags::read_write(),
                        780,
                        4096,
                        &mut as_alloc,
                    )
                    .expect("remap after unmap");
                // Registry must show a freshly installed table while the
                // mapping is live; a skipped parent unlink would have made
                // the walk silently reuse the freed frame instead.
                assert!(
                    vmm::tables::count_for_as(vm_as) > tables_pre_remap,
                    "remap reused a stale (unlinked-or-freed) table: registry entry missing"
                );
                core::ptr::write_volatile(va_re as *mut u64, 0x5555);
                assert_eq!(core::ptr::read_volatile(va_re as *const u64), 0x5555);
                aspace.unmap_range(va_re, 4096, &mut as_alloc).unwrap();
            }

            pmm.free_frame(frame);

            // ---- back to kernel AS, IRQs back on ------------------------
            vivanta_arch_api::mmu::activate_address_space(kas_root);
        }
        // 8. teardown: unregister reclaims any remaining tracked tables.
        vmm::unregister(vm_as).expect("unregister vm test AS");
        println!("  [VM] lifecycle PASS");

        // ------------------------------------------------------------------
        // M6.0 lazy VM test: demand-fill, page-granular materialization,
        // mprotect-before-fault, munmap ownership, OOM rollback.
        // ------------------------------------------------------------------
        println!("Lazy VM test:");
        vmm::faults::set_backing_context(
            system_state.memory_manager_mut() as *mut _,
            &raw mut pmm_backend as *mut dyn memory::MemoryBackend,
        );
        let lz_root = build_root("VmLazyAS", 0, 0, None);
        let lz_as = vmm::register(lz_root, vmm::AddressSpaceFlags::User);
        {
            let mrm = system_state.memory_manager_mut();
            let mut as_alloc = memory::AsPageTableAllocator::new(
                mrm as *mut _,
                &raw mut pmm_backend as *mut dyn memory::MemoryBackend,
                lz_as,
            );
            let kas_root = vmm::lookup_root(crate::vmm::KERNEL_ADDRESS_SPACE_ID);
            let _irq = vivanta_arch_api::interrupts::disable_interrupts();
            vivanta_arch_api::mmu::activate_address_space(lz_root);

            use vivanta_arch_api::mmu::MappingFlags as ApiMFlags;
            // 1. reserve 16 KiB lazy RW.
            let va = vmm::address_space_mut_by(lz_as)
                .reserve_lazy(16 * 4096, ApiMFlags::read_write(), 900, 4096)
                .expect("reserve_lazy");
            assert!(va >= vmm::USER_VA_BASE && va < vmm::USER_VA_END);
            vmm::address_space_mut_by(lz_as)
                .verify_hardware_consistency()
                .expect("lazy reservation must have no hardware image");

            // 2. read page 0 -> translation fault -> demand fill -> retry.
            let v0 = core::ptr::read_volatile(va as *const u64);
            assert_eq!(v0, 0, "demand-filled page must be zero-initialized");

            // 3. exactly one page materialized (page-granular, Phase 9).
            {
                let aspace = vmm::address_space_mut_by(lz_as);
                aspace
                    .verify_hardware_consistency()
                    .expect("post-fill verify");
                assert!(
                    aspace
                        .query(va)
                        .is_some_and(|m| m.backing == vmm::mapping::Backing::Present),
                    "faulted page must be Present"
                );
                assert!(
                    aspace
                        .query(va + 4096)
                        .is_some_and(|m| m.backing == vmm::mapping::Backing::LazyAnonymous),
                    "rest of range must stay Lazy"
                );
            }

            // 4. write page 2 -> second demand fill.
            core::ptr::write_volatile((va + 8192) as *mut u64, 0xCAFE);
            assert_eq!(core::ptr::read_volatile((va + 8192) as *const u64), 0xCAFE);

            // 5. mprotect the WHOLE range to RO before page 1 is touched:
            //    metadata changes for Lazy pieces, hardware for Present ones.
            vmm::address_space_mut_by(lz_as)
                .protect(va, 16 * 4096, ApiMFlags::from_bits(0), &mut as_alloc)
                .expect("mprotect RO");
            // 6. read page 1 -> fill must use CURRENT (RO) permissions.
            let _v1 = core::ptr::read_volatile((va + 4096) as *const u64);
            {
                let desc = vivanta_arch_api::mmu::mmu_leaf_descriptor(
                    vmm::lookup_root(lz_as).0 as u64,
                    va + 4096,
                );
                let expected = vivanta_arch_api::mmu::mmu_permission_bits(ApiMFlags::from_bits(0));
                assert_eq!(
                    desc & expected,
                    expected,
                    "demand fill must apply post-mprotect permissions"
                );
                // Privilege policy: anonymous fills are kernel-only in M6.0
                // — no EL0 access may appear regardless of mapping flags.
                assert!(
                    desc & (1 << 6) == 0,
                    "demand fill must not grant EL0 access: desc={desc:#x}"
                );
            }
            vmm::address_space_mut_by(lz_as)
                .verify_hardware_consistency()
                .expect("verify after RO fill");

            // 7. munmap everything: Anonymous frames return to PMM, Lazy
            //    pieces vanish without allocation.
            let free_pre = pmm.free_count();
            let tables_pre = vmm::tables::count_for_as(lz_as);
            vmm::address_space_mut_by(lz_as)
                .unmap_range(va, 16 * 4096, &mut as_alloc)
                .expect("munmap lazy range");
            let delta = pmm.free_count() - free_pre;
            // 3 anonymous frames (pages 0/1/2) + reclaimed fill-time tables.
            let tables_reclaimed = tables_pre - vmm::tables::count_for_as(lz_as);
            assert_eq!(
                delta,
                3 + tables_reclaimed,
                "munmap must return anon frames + reclaimed tables"
            );
            vmm::address_space_mut_by(lz_as)
                .verify_hardware_consistency()
                .expect("no ghost leaves after munmap");

            // 8. OOM rollback (deterministic): failing allocator must leave
            //    the mapping Lazy and untouched.
            struct OomAlloc;
            impl vivanta_arch_api::mmu::PageTableAllocator for OomAlloc {
                fn alloc_page_table_frame(&mut self) -> u64 {
                    panic!("infallible alloc must not be reached on the fault path")
                }
                fn try_alloc_page_table_frame(&mut self) -> Option<u64> {
                    None
                }
            }
            let va2 = vmm::address_space_mut_by(lz_as)
                .reserve_lazy(4096, ApiMFlags::read_write(), 901, 4096)
                .expect("reserve for oom test");
            let mut oom = OomAlloc;
            assert!(
                !vmm::address_space_mut_by(lz_as).resolve_lazy_fault(va2, false, &mut oom),
                "OOM must not resolve"
            );
            vmm::address_space_mut_by(lz_as)
                .verify_hardware_consistency()
                .expect("OOM must leave mapping Lazy/untouched");

            // 9. negative classification (ADR-032 §2.2): unmapped VA and
            //    write-to-RO are never resolvable.
            assert!(!vmm::address_space_mut_by(lz_as).resolve_lazy_fault(
                va2 + 0x100_0000,
                false,
                &mut as_alloc,
            ));
            let va_ro = vmm::address_space_mut_by(lz_as)
                .reserve_lazy(4096, ApiMFlags::from_bits(0), 902, 4096)
                .expect("reserve RO piece");
            assert!(
                !vmm::address_space_mut_by(lz_as).resolve_lazy_fault(va_ro, true, &mut as_alloc),
                "write to a lazy RO piece must never resolve"
            );
            vmm::address_space_mut_by(lz_as)
                .verify_hardware_consistency()
                .expect("rejected fill must leave state untouched");

            // 9b. page-granular proof: fault at a NON-base page of a fresh
            //     piece — base must stay Lazy, faulted page becomes Present.
            let va4 = vmm::address_space_mut_by(lz_as)
                .reserve_lazy(16 * 4096, ApiMFlags::read_write(), 903, 4096)
                .expect("reserve pg-granular");
            let _ = core::ptr::read_volatile((va4 + 4096) as *const u64);
            {
                let aspace = vmm::address_space_mut_by(lz_as);
                assert!(
                    aspace
                        .query(va4 + 4096)
                        .is_some_and(|m| m.backing == vmm::mapping::Backing::Present),
                    "faulted page must be the materialized one"
                );
                assert!(
                    aspace
                        .query(va4)
                        .is_some_and(|m| m.backing == vmm::mapping::Backing::LazyAnonymous),
                    "piece base must stay Lazy"
                );
                aspace
                    .verify_hardware_consistency()
                    .expect("pg-granular verify");
            }
            vmm::address_space_mut_by(lz_as)
                .unmap_range(va4, 16 * 4096, &mut as_alloc)
                .expect("unmap pg-granular");

            // 9c. deterministic VM stress: reserve/fault/protect/unmap
            //     cycles with full verification after every iteration.
            let free_stress_pre = pmm.free_count();
            for i in 0..200u32 {
                let perms_i = if i % 2 == 0 {
                    ApiMFlags::read_write()
                } else {
                    ApiMFlags::from_bits(0)
                };
                let tables_iter = vmm::tables::count_for_as(lz_as);
                let vs = vmm::address_space_mut_by(lz_as)
                    .reserve_lazy(8 * 4096, perms_i, 1000 + i as u64, 4096)
                    .expect("stress reserve");
                // Fault two non-base pages. Access type matches the
                // iteration's permissions: RW iterations write, RO
                // iterations read (a write would be a correct rejection).
                let _ = core::ptr::read_volatile((vs + 4096) as *const u64);
                if perms_i.is_read_write() {
                    core::ptr::write_volatile((vs + 3 * 4096) as *mut u64, i as u64);
                    assert_eq!(
                        core::ptr::read_volatile((vs + 3 * 4096) as *const u64),
                        i as u64
                    );
                } else {
                    let _ = core::ptr::read_volatile((vs + 3 * 4096) as *const u64);
                }
                {
                    let aspace = vmm::address_space_mut_by(lz_as);
                    aspace.verify_hardware_consistency().expect("stress verify");
                    assert!(
                        aspace
                            .query(vs + 4096)
                            .is_some_and(|m| m.backing == vmm::mapping::Backing::Present),
                        "stress {i}: page not materialized"
                    );
                }
                vmm::address_space_mut_by(lz_as)
                    .unmap_range(vs, 8 * 4096, &mut as_alloc)
                    .expect("stress unmap");
                let free_now = pmm.free_count();
                assert_eq!(free_now, free_stress_pre, "stress {i}: frame leak");
                assert_eq!(
                    vmm::tables::count_for_as(lz_as),
                    tables_iter,
                    "stress {i}: table registry leak"
                );
            }
            println!("  [STRESS] 200 reserve/fill/unmap cycles PASS");

            // 10. cleanup: Lazy reservations unmap without any allocation.
            vmm::address_space_mut_by(lz_as)
                .unmap_range(va2, 4096, &mut as_alloc)
                .expect("unmap lazy reservation");
            vmm::address_space_mut_by(lz_as)
                .unmap_range(va_ro, 4096, &mut as_alloc)
                .expect("unmap RO reservation");
            vmm::address_space_mut_by(lz_as)
                .verify_hardware_consistency()
                .expect("clean teardown");
            vmm::address_space_mut_by(lz_as)
                .verify_domain_reverse()
                .expect("reverse scan: ghost leaf after teardown");
            vivanta_arch_api::mmu::activate_address_space(kas_root);
        }
        vmm::unregister(lz_as).expect("unregister lazy AS");
        println!("  [LAZY] demand-fill/mprotect/munmap/OOM PASS");

        // ------------------------------------------------------------------
        // M6 process-lifecycle demo: the demo user task (tid) already ran and
        // exited with code 0 through the new thread_exit path. Observe its Task
        // state transitions and verify reaping returns frames to the PMM
        // (G6-A / G6-B / G6-C).
        // ------------------------------------------------------------------
        println!();
        println!("M6 process-lifecycle verification:");
        let m6_before_free = pmm.free_count();
        if let Some(task) = taskman.get(tid) {
            println!(
                "  [M6] demo Task {} state={:?} exit_code={:?}",
                tid, task.state, task.exit_code
            );
            assert_eq!(task.exit_code, Some(0), "M6 FAIL: demo task exit code != 0");
            assert_eq!(
                task.state,
                scheduler::task::TaskState::Zombie,
                "M6 FAIL: demo task not Zombie after exit"
            );
        }
        if let Some(ft) = faultman.get(ftid) {
            println!(
                "  [M6] fault Task {} state={:?} exit_code={:?}",
                ftid, ft.state, ft.exit_code
            );
        }
        println!(
            "  [M6] running_count={} (demo task exited)",
            taskman.running_count()
        );
        let zombies: alloc::vec::Vec<scheduler::task::TaskId> = taskman.zombies();
        println!("  [M6] zombies before reap: {:?}", zombies);
        let before_reap_free = pmm.free_count();
        let mut reaped = 0;
        for z in zombies {
            if taskman.reap_zombie(z).is_some() {
                reaped += 1;
            }
        }
        let after_reap_free = pmm.free_count();
        println!(
            "  [M6] reaped={} free_before={} free_after={} delta={}",
            reaped,
            before_reap_free,
            after_reap_free,
            after_reap_free - before_reap_free
        );
        assert!(
            after_reap_free >= before_reap_free,
            "M6 FAIL: reaping leaked memory (free decreased)"
        );
        println!(
            "  [M6] process lifecycle demo OK (baseline_free={})",
            m6_before_free
        );

        // ------------------------------------------------------------------
        // G4 preemption test: two live CPU-bound kernel threads, NO voluntary
        // yield. The 100 Hz timer must preempt them and switch A <-> B. The
        // observability log ([PREEMPT] current=.. counter=..) proves real
        // timer-driven context switches (G4 observability sub-gate).
        // ------------------------------------------------------------------
        println!();
        println!("G4 preemption test: spawning 2 CPU-bound threads");
        let mut taskman2 = scheduler::task_manager::TaskManager::new();
        let ta = taskman2
            .spawn_kernel(
                preempt_worker_a,
                0,
                crate::vmm::KERNEL_ADDRESS_SPACE_ID,
                &mut pmm,
                scheduler::thread::Priority::Normal,
                None,
            )
            .expect("spawn preempt A");
        let tb = taskman2
            .spawn_kernel(
                preempt_worker_b,
                0,
                crate::vmm::KERNEL_ADDRESS_SPACE_ID,
                &mut pmm,
                scheduler::thread::Priority::Normal,
                None,
            )
            .expect("spawn preempt B");
        println!("  preempt tasks A={} B={} spawned", ta, tb);

        // Boot thread: monitor the counters, then spin.
        for i in 0..5 {
            scheduler::yield_now();
            let (ca, cb) = (
                PREEMPT_COUNTER_A.load(Ordering::Relaxed),
                PREEMPT_COUNTER_B.load(Ordering::Relaxed),
            );
            vivanta_boot_common::println!(
                "  [MONITOR] iter={} A={} B={} running={} current={}",
                i,
                ca,
                cb,
                scheduler::running_thread_count(),
                scheduler::current_thread_id()
            );
        }

        // G4 running invariant: exactly one Running thread at a time. The enum
        // state is exclusive, so Running ∩ Ready == ∅ holds structurally; the
        // dangerous case (a thread stranded Ready while actually running) is
        // caught by running_count != 1.
        let rcount = scheduler::running_thread_count();
        vivanta_boot_common::println!("  [G4] running_count={} (expect 1)", rcount);
        assert_eq!(rcount, 1, "G4 FAIL: expected exactly one Running thread");

        loop {
            scheduler::yield_now();
        }
    }
}

// ---------------------------------------------------------------------------
// G4 preemption workers — CPU-bound, no voluntary yield. The timer IRQ drives
// the reschedule. Counters are plain unsync statics because preemption is
// single-core and each thread touches only its own counter.
// ---------------------------------------------------------------------------

/// Per-worker iteration counters. Atomics (Relaxed): single-core stats,
/// no ordering requirements — removes the static-mut hazard entirely.
static PREEMPT_COUNTER_A: AtomicU64 = AtomicU64::new(0);
static PREEMPT_COUNTER_B: AtomicU64 = AtomicU64::new(0);

extern "C" fn preempt_worker_a(_arg: usize) {
    loop {
        let c = PREEMPT_COUNTER_A.fetch_add(1, Ordering::Relaxed) + 1;
        if c % 1000000 == 0 {
            vivanta_boot_common::println!(
                "  [PREEMPT] current={} A={}",
                scheduler::current_thread_id(),
                c
            );
        }
    }
}

extern "C" fn preempt_worker_b(_arg: usize) {
    loop {
        let c = PREEMPT_COUNTER_B.fetch_add(1, Ordering::Relaxed) + 1;
        if c % 1000000 == 0 {
            vivanta_boot_common::println!(
                "  [PREEMPT] current={} B={}",
                scheduler::current_thread_id(),
                c
            );
        }
    }
}
