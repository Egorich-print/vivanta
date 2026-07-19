#![no_std]
#![no_main]

use vivanta_boot_common::fdt::FdtScanner;
use vivanta_boot_common::hardware::NS16550_FAMILY;
use vivanta_boot_common::ns16550::Ns16550;
use vivanta_boot_common::{println, set_console, BOOT_CONTEXT};
use core::panic::PanicInfo;

core::arch::global_asm!(
    ".section .text._start",
    ".global _start",
    "_start:",

    // ARM64 Image header (64 bytes, U-Boot booti format)
    "b _real_start",            // code0 (4 bytes)
    ".word 0",                  // code1 (4 bytes)
    ".quad 0x00280000",         // text_offset (8 bytes) — vivanta_kernel linked at DRAM_base + 2MiB + 512KiB
    ".quad 0",                  // image_size (8 bytes) — 0 → U-Boot assumes 16MiB
    ".quad 0x0a",               // flags (8 bytes) — bit 1=1 (LE), bit 3=1 (position-dependent)
    ".quad 0",                  // res2 (8 bytes)
    ".quad 0",                  // res3 (8 bytes)
    ".quad 0",                  // res4 (8 bytes)
    ".word 0x644d5241",         // magic "ARMd" (4 bytes)
    ".word 0",                  // res5 (4 bytes)

    "_real_start:",
    // Mask all interrupts
    "msr daifset, #0xf",

    // Enable FP/SIMD (CPACR_EL1 or CPTR_EL2 depending on EL)
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

    // Clear BSS
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

    // Preserve x0 (DTB) into BOOT_CONTEXT
    "adrp x1, BOOT_CONTEXT",
    "add x1, x1, :lo12:BOOT_CONTEXT",
    "str x0, [x1]",

    // Enter Rust
    "bl adapter_main",

    // Should never return
    "3:",
    "wfi",
    "b 3b",
);

#[no_mangle]
pub unsafe extern "C" fn adapter_main() -> ! {
    let dtb_addr = BOOT_CONTEXT.dtb as *const u8;

    // Console must be discovered and initialised BEFORE any println! calls
    // (FdtScanner::console() does not use println!; FdtScanner::probe() does).
    let console_node =
        FdtScanner::console(dtb_addr).expect("no console in FDT");

    if console_node.matches_any(NS16550_FAMILY) {
        let base = console_node.reg.unwrap().addr as *mut u8;
        let shift = console_node.reg_shift.unwrap_or(2) as u8;
        static mut UART: Option<Ns16550> = None;
        let uart_ptr = core::ptr::addr_of_mut!(UART);
        uart_ptr.write(Some(Ns16550::new(base, shift)));
        if let Some(ref u) = *uart_ptr {
            set_console(u);
        }
    } else {
        // Fallback: hardcoded NS16550 at 0xFE66_0000, reg-shift=2
        static UART: Ns16550 = Ns16550::new(0xFE66_0000 as *mut u8, 2);
        set_console(&UART);
    }

    // ------- FDT Validation Report + Memory discovery ----------------------
    let mut mem_map = vivanta_boot_common::MemoryMap::new();
    let cpu_count = FdtScanner::report(dtb_addr, &mut mem_map);

    println!();
    println!("\u{2500}\u{2500}\u{2500}\u{2500} Theseus Boot v0.1 \u{2500}\u{2500}\u{2500}\u{2500}");
    println!("  Arch:      AArch64");
    println!("  Platform:  Rockchip RK3568");

    let console_reg = console_node.reg.map(|r| r.addr).unwrap_or(0xFE66_0000);
    if console_node.matches_any(NS16550_FAMILY) {
        println!("  Console:   {} @ 0x{:x} (reg-shift={:?}, class=NS16550)",
            console_node.compatible, console_reg, console_node.reg_shift);
    } else {
        println!("  Console:   {} @ 0x{:x}", console_node.compatible, console_reg);
    }

    let total_mib = mem_map
        .regions()
        .iter()
        .filter(|r| r.kind == vivanta_boot_common::MemoryRegionKind::Usable)
        .map(|r| r.size >> 20)
        .sum::<u64>();
    println!("  Memory:    {} MiB across {} region(s)", total_mib, mem_map.regions().len());
    println!("  CPUs:      {} core(s)", cpu_count);
    println!("  Status:    Stage 1 \u{2713}");
    println!();

    loop {
        core::hint::spin_loop();
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        core::hint::spin_loop();
    }
}
