// ---------------------------------------------------------------------------
// target-rk3568 — Rockchip RK3568 vivanta_kernel binary
// ---------------------------------------------------------------------------

#![no_std]
#![no_main]

use core::panic::PanicInfo;

use vivanta_boot_common::println;
use vivanta_boot_info::{BootInfo, MmioRegion, MmioKind};

extern crate vivanta_arch_aarch64;

// ARM64 entry point with Image header for U-Boot booti
core::arch::global_asm!(
    ".section .text._start",
    ".global _start",
    "_start:",
    // ARM64 Image header (64 bytes)
    "b _real_start",
    ".word 0",
    ".quad 0x00280000",
    ".quad 0",
    ".quad 0x0a",
    ".quad 0",
    ".quad 0",
    ".quad 0",
    ".word 0x644d5241",
    ".word 0",

    "_real_start:",
    "msr daifset, #0xf",
    // Disable MMU (U-Boot may leave it on)
    "dsb sy",
    "isb",
    "mrs x5, CurrentEL",
    "and x5, x5, #0xC",
    "cmp x5, #(2 << 2)",
    "b.eq 10f",
    "cmp x5, #(1 << 2)",
    "b.eq 11f",
    "b 7f",
    "10:",
    // EL2: disable MMU at both EL2 and EL1
    "mrs x6, sctlr_el2",
    "bic x6, x6, #1",
    "msr sctlr_el2, x6",
    "dsb sy",
    "isb",
    "tlbi alle2",
    "dsb sy",
    "isb",
    "mrs x6, sctlr_el1",
    "bic x6, x6, #1",
    "msr sctlr_el1, x6",
    "dsb sy",
    "isb",
    "tlbi vmalle1",
    "dsb sy",
    "isb",
    "mov x5, #(0b11 << 20)",
    "msr CPACR_EL1, x5",        // FPEN=3: enable FP at EL1 and EL0
    "msr CPTR_EL2, xzr",        // clear TFP — no FP/SIMD traps to EL2
    // Configure HCR_EL2 for EL1 compatibility, then drop to EL1
    "mov x5, #(1 << 31)",    // RW bit: AArch64 for EL1
    "msr hcr_el2, x5",
    "mov x5, #0x3c5",        // SPSR_EL2: EL1h, DAIF masked
    "msr spsr_el2, x5",
    "adr x5, 12f",           // return address in EL1
    "msr elr_el2, x5",
    "eret",                   // → EL1 at 12f
    "11:",
    // EL1: disable MMU
    "mrs x6, sctlr_el1",
    "bic x6, x6, #1",
    "msr sctlr_el1, x6",
    "dsb sy",
    "isb",
    "tlbi vmalle1",
    "dsb sy",
    "isb",
    "mov x5, #(0b11 << 20)",
    "msr CPACR_EL1, x5",
    "12:",
    // Set up a safe exception vector table (all entries: branch to self)
    "adr x6, 8f",
    "msr vbar_el1, x6",
    "isb",
    "b 7f",
    ".balign 2048",
    "8:",
    ".rept 16",
    "b 8b",
    ".endr",
    "7:",
    "adrp x1, __stack_top",
    "add x1, x1, :lo12:__stack_top",
    "mov sp, x1",
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
    // Reinitialize UART: set LCR = 0x03 (8N1, DLAB=0)
    "mov x4, #0x0",
    "movk x4, #0xfe66, lsl #16",
    "mov w5, #0x03",
    "strb w5, [x4, #(3 << 2)]",
    // Wait for TX completion (read LSR, bit 5 = THR empty)
    "1:",
    "ldr w5, [x4, #(5 << 2)]",
    "tst w5, #(1 << 5)",
    "b.eq 1b",
    // Store DTB address (x0 from U-Boot) into BOOT_CONTEXT
    "adrp x1, BOOT_CONTEXT",
    "add x1, x1, :lo12:BOOT_CONTEXT",
    "str x0, [x1]",
    "bl adapter_main",
    "3:",
    "wfi",
    "b 3b",
);

#[no_mangle]
pub unsafe extern "C" fn adapter_main() -> ! {
    // Read DTB address from BOOT_CONTEXT (stored by ASM entry)
    let dtb_addr = vivanta_boot_common::BOOT_CONTEXT.dtb;
    let dtb_ptr = dtb_addr as *const u8;

    // Configure early platform info for boot debug output
    vivanta_boot_common::set_early_platform(
        vivanta_boot_common::EarlyPlatformInfo { uart_base: 0xFE66_0000 },
    );

    // Platform init: console from FDT, fallback to hardcoded NS16550
    vivanta_platform_rk3568::init_console_from_fdt(dtb_ptr);

    // FDT validation report and memory discovery
    let (mem_map, cpu_count) = vivanta_platform_rk3568::build_memory_map(dtb_ptr);

    println!();
    println!("\u{2500}\u{2500}\u{2500}\u{2500} Vivanta Boot Adapter (RK3568) \u{2500}\u{2500}\u{2500}\u{2500}");
    println!("  DTB at 0x{:x}", dtb_addr);

    // Build MMIO regions (RK3568: UART NS16550 at 0xFE660000)
    static MMIO_REGIONS: [MmioRegion; 1] = [
        MmioRegion { base: 0xFE66_0000, size: 0x1000, kind: MmioKind::Device },
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
