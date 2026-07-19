// ---------------------------------------------------------------------------
// target-x96q — X96Q TV box (Allwinner H313) vivanta_kernel binary
//
// Boot chain: BROM -> SPL -> U-Boot -> booti -> Vivanta
// U-Boot command:
//   load mmc 0:1 0x40280000 vivanta-x96q.bin
//   booti 0x40280000 - ${fdt_addr_r}
//
// DRAM base: 0x40000000  |  text_offset: 0x00280000  |  load address: 0x40280000
// UART0:     0x05000000  |  GIC-400 (GICv2): 0x03000000
// ---------------------------------------------------------------------------

#![no_std]
#![no_main]

use core::panic::PanicInfo;

use vivanta_boot_common::println;
use vivanta_boot_info::{BootInfo, MmioRegion, MmioKind, InterruptControllerInfo};

// Force link arch-aarch64 for extern "Rust" symbol resolution
extern crate vivanta_arch_aarch64;

// ARM64 entry point with Image header for U-Boot booti
core::arch::global_asm!(
    ".section .text._start",
    ".global _start",
    "_start:",
    // ARM64 Image header (64 bytes) — U-Boot booti compatible
    "b _real_start",
    ".word 0",
    ".quad 0x00280000",             // text_offset
    ".quad 0",                      // image_size (0 = unknown, U-Boot reads ELF segments)
    ".quad 0x0a",                   // flags: LE, P-DEP
    ".quad 0",
    ".quad 0",
    ".quad 0",
    ".word 0x644d5241",             // magic = "ARMd"
    ".word 0",

    "_real_start:",
    // Mask all interrupts
    "msr daifset, #0xf",
    // Detect current EL and enable FP/SIMD
    "mrs x5, CurrentEL",
    "and x5, x5, #0xC",
    "cmp x5, #(2 << 2)",
    "b.eq 5f",
    "cmp x5, #(1 << 2)",
    "b.eq 6f",
    "b 7f",
    "5:",
    "mov x5, #(0b11 << 20)",
    "msr CPTR_EL2, x5",
    "b 7f",
    "6:",
    "mov x5, #(0b11 << 20)",
    "msr CPACR_EL1, x5",
    "7:",
    // Set stack pointer
    "adrp x1, __stack_top",
    "add x1, x1, :lo12:__stack_top",
    "mov sp, x1",
    // Zero BSS
    "adrp x1, __bss_start",
    "add x1, x1, :lo12:__bss_start",
    "adrp x2, __bss_end",
    "add x2, x2, :lo12:__bss_end",
    "sub x3, x2, x1",
    "cbz x3, 2f",
    "mov x2, xzr",
    "1:",
    "str x2, [x1], #8",
    "subs x3, x3, #8",
    "b.gt 1b",
    "2:",
    // Save DTB pointer (x0) into BOOT_CONTEXT
    "adrp x1, BOOT_CONTEXT",
    "add x1, x1, :lo12:BOOT_CONTEXT",
    "str x0, [x1]",
    // Call adapter_main
    "bl adapter_main",
    "3:",
    "wfi",
    "b 3b",
);

/// Platform entry point called from ASM _start.
#[no_mangle]
pub unsafe extern "C" fn adapter_main() -> ! {
    let dtb_addr = vivanta_boot_common::BOOT_CONTEXT.dtb as *const u8;

    // Configure early platform info for boot debug output
    vivanta_boot_common::set_early_platform(
        vivanta_boot_common::EarlyPlatformInfo { uart_base: 0x0500_0000 },
    );

    // Platform init: console from FDT
    vivanta_platform_allwinner_h616::init_console_from_fdt(dtb_addr);

    // FDT validation report and memory discovery
    let (mem_map, cpu_count) = vivanta_platform_allwinner_h616::build_memory_map(dtb_addr);

    println!();
    println!("\u{2500}\u{2500}\u{2500}\u{2500} Vivanta v0.1 \u{2500}\u{2500}\u{2500}\u{2500}");
    println!("  Arch:      AArch64");
    println!("  Platform:  Allwinner H313 (X96Q)");

    let total_mib = mem_map
        .regions()
        .iter()
        .filter(|r| r.kind == vivanta_boot_common::MemoryRegionKind::Usable)
        .map(|r| r.size >> 20)
        .sum::<u64>();
    println!("  Memory:    {} MiB across {} region(s)", total_mib, mem_map.regions().len());
    println!("  CPUs:      {} core(s)", cpu_count);
    println!("  DTB:       0x{:x}", dtb_addr as usize);
    println!();

    // Build MMIO regions for Allwinner H616/H313
    //   UART0:  0x05000000 (user-accessible for early console)
    //   GIC-400: 0x03000000 (distributor), 0x03010000 (CPU interface)
    static MMIO_REGIONS: [MmioRegion; 2] = [
        MmioRegion { base: 0x0500_0000, size: 0x1000, kind: MmioKind::UserDevice },
        MmioRegion { base: 0x0300_0000, size: 0x2_0000, kind: MmioKind::Device },
    ];

    // Interrupt controller: GIC-400 (GICv2)
    let interrupt_controller = InterruptControllerInfo {
        compatible: "arm,cortex-a15-gic",
        distributor_base: 0x0300_0000,
        distributor_size: 0x1_0000,
        redistributor_base: Some(0x0301_0000),
        redistributor_size: Some(0x1_0000),
    };

    // Assemble BootInfo and pass to vivanta_kernel
    let mut mem_map_buf: core::mem::MaybeUninit<vivanta_boot_common::MemoryMap> =
        core::mem::MaybeUninit::uninit();
    let mut boot_info_buf: core::mem::MaybeUninit<BootInfo> =
        core::mem::MaybeUninit::uninit();

    mem_map_buf.as_mut_ptr().write(mem_map);
    let mem_map_ref: &'static vivanta_boot_common::MemoryMap = &*mem_map_buf.as_ptr();

    boot_info_buf.as_mut_ptr().write(BootInfo {
        memory_map: mem_map_ref,
        mmio_regions: &MMIO_REGIONS,
        interrupt_controller: Some(interrupt_controller),
        cpu_count,
        dtb: Some(dtb_addr as usize),
    });

    println!("\u{2500}\u{2500}\u{2500}\u{2500} Handing off to vivanta_kernel \u{2500}\u{2500}\u{2500}\u{2500}");
    println!();

    vivanta_kernel::kernel_main(&*boot_info_buf.as_ptr());
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop { core::hint::spin_loop(); }
}
