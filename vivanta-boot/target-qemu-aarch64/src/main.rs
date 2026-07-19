// ---------------------------------------------------------------------------
// target-qemu-aarch64 — QEMU AArch64 vivanta_kernel binary
// ---------------------------------------------------------------------------

#![no_std]
#![no_main]

use core::panic::PanicInfo;

use vivanta_boot_common::println;
use vivanta_boot_info::{BootInfo, MmioRegion, MmioKind};

// Force link arch-aarch64 for extern "Rust" symbol resolution
extern crate vivanta_arch_aarch64;

// Inline ARM64 entry point
core::arch::global_asm!(
    ".section .text._start",
    ".global _start",
    "_start:",
    "mrs x5, CurrentEL",
    "and x5, x5, #0xC",
    "lsr x5, x5, #2",
    "cmp x5, #2",
    "b.eq 5f",
    "cmp x5, #1",
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
    "ldr x1, =__stack_top",
    "mov sp, x1",
    "ldr x1, =__bss_start",
    "ldr x2, =__bss_end",
    "sub x3, x2, x1",
    "cbz x3, 2f",
    "mov x2, xzr",
    "1:",
    "str x2, [x1], #8",
    "subs x3, x3, #8",
    "b.gt 1b",
    "2:",
    "ldr x4, =0x09000000",
    "mov w5, #0x21",
    "str w5, [x4]",
    "ldr x1, =0x40000000",
    "ldr w2, [x1]",
    "ldr x3, =0xEDFE0DD0",
    "cmp x2, x3",
    "b.eq 3f",
    "mov x1, x0",
    "3:",
    "mov x0, x1",
    "bl adapter_main",
    "4:",
    "wfi",
    "b 4b",
);

/// Platform entry point called from ASM _start.
#[no_mangle]
pub unsafe extern "C" fn adapter_main(dtb_addr: usize) -> ! {
    let dtb_ptr = dtb_addr as *const u8;

    // Platform init: console from FDT
    let console_node = vivanta_platform_qemu::init_console_from_fdt(dtb_ptr);

    // FDT validation report and memory discovery
    let (mut mem_map, cpu_count) = vivanta_platform_qemu::build_memory_map(dtb_ptr);

    println!();
    println!("\u{2500}\u{2500}\u{2500}\u{2500} Vivanta Boot Adapter (AArch64/QEMU) \u{2500}\u{2500}\u{2500}\u{2500}");
    println!("  DTB at 0x{:x}", dtb_addr);

    let console_reg = console_node.reg.unwrap().addr;
    if console_node.compatible.contains("pl011") {
        println!("  Console: {} @ 0x{:x} (class=PL011)", console_node.compatible, console_reg);
    } else {
        println!("  Console: {} @ 0x{:x} (class=NS16550)", console_node.compatible, console_reg);
    }
    println!();

    // Build MMIO regions (QEMU virt: UART user-accessible, GIC vivanta_kernel-only)
    static MMIO_REGIONS: [MmioRegion; 2] = [
        MmioRegion { base: 0x0900_0000, size: 0x1000, kind: MmioKind::UserDevice },
        MmioRegion { base: 0x0800_0000, size: 0x10_0000, kind: MmioKind::Device },
    ];

    // Assemble BootInfo
    let mut mem_map_buf: core::mem::MaybeUninit<vivanta_boot_common::MemoryMap> =
        core::mem::MaybeUninit::uninit();
    let mut boot_info_buf: core::mem::MaybeUninit<BootInfo> =
        core::mem::MaybeUninit::uninit();

    mem_map_buf.as_mut_ptr().write(mem_map);
    let mem_map_ref: &'static vivanta_boot_common::MemoryMap = &*mem_map_buf.as_ptr();

    boot_info_buf.as_mut_ptr().write(BootInfo {
        memory_map: mem_map_ref,
        mmio_regions: &MMIO_REGIONS,
        interrupt_controller: None,
        cpu_count,
        dtb: Some(dtb_addr),
    });

    vivanta_kernel::kernel_main(&*boot_info_buf.as_ptr());
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop { core::hint::spin_loop(); }
}
