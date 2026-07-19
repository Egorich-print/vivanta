#![no_std]
#![no_main]

extern crate vivanta_arch_aarch64;

use core::panic::PanicInfo;

use vivanta_boot_common::fdt::FdtScanner;
use vivanta_boot_common::hardware::{HardwareNode, NS16550_FAMILY};
use vivanta_boot_common::ns16550::Ns16550;
use vivanta_boot_common::{MemoryMap, println, set_console};
use vivanta_boot_info::BootInfo;

mod platform;

use platform::qemu::Pl011Uart;

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

#[allow(static_mut_refs)]
unsafe fn init_console(node: &HardwareNode) {
    let base = node.reg.map(|r| r.addr as usize).expect("console has no reg");

    if node.compatible.contains("pl011") {
        static mut UART: Option<Pl011Uart> = None;
        UART = Some(Pl011Uart::new(base));
        UART.as_ref().unwrap().init();
        set_console(UART.as_ref().unwrap());
    } else if node.matches_any(NS16550_FAMILY) {
        static mut UART: Option<Ns16550> = None;
        let shift = node.reg_shift.unwrap_or(0) as u8;
        UART = Some(Ns16550::new(base as *mut u8, shift));
        set_console(UART.as_ref().unwrap());
    } else {
        panic!("unsupported console: {}", node.compatible);
    }
}

#[no_mangle]
pub unsafe extern "C" fn adapter_main(dtb_addr: usize) -> ! {
    let dtb_ptr = dtb_addr as *const u8;

    // Console must be discovered and initialised BEFORE any println! calls
    // (FdtScanner::console() does not use println!; FdtScanner::probe() does).
    let console_node = FdtScanner::console(dtb_ptr).expect("no console in FDT");
    init_console(&console_node);

    // ------- FDT Validation Report + Memory discovery ----------------------
    let mut mem_map = MemoryMap::new();
    let cpu_count = FdtScanner::report(dtb_ptr, &mut mem_map);

    println!();
    println!(
        "\u{2500}\u{2500}\u{2500}\u{2500} Theseus Boot Adapter (AArch64/QEMU) \u{2500}\u{2500}\u{2500}\u{2500}"
    );
    println!("  DTB at 0x{:x}", dtb_addr);
    let console_reg = console_node.reg.unwrap().addr;
    if console_node.matches_any(NS16550_FAMILY) {
        println!("  Console: {} @ 0x{:x} (reg-shift={:?}, class=NS16550)",
            console_node.compatible, console_reg, console_node.reg_shift);
    } else {
        println!("  Console: {} @ 0x{:x} (class=PL011)",
            console_node.compatible, console_reg);
    }
    println!();

    // ------- Assemble BootInfo ---------------------------------------------
    let mut mem_map_buf: core::mem::MaybeUninit<MemoryMap> =
        core::mem::MaybeUninit::uninit();
    let mut boot_info_buf: core::mem::MaybeUninit<BootInfo> =
        core::mem::MaybeUninit::uninit();

    mem_map_buf.as_mut_ptr().write(mem_map);
    let mem_map_ref: &'static MemoryMap = &*mem_map_buf.as_ptr();

    boot_info_buf.as_mut_ptr().write(BootInfo {
        memory_map: mem_map_ref,
        mmio_regions: &[],
        interrupt_controller: None,
        cpu_count,
        dtb: Some(dtb_addr),
    });
    vivanta_kernel::kernel_main(&*boot_info_buf.as_ptr());
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    // Fallback panic handler — if console is not yet initialised, QEMU output
    // may not appear; the early-UART '!' character in the ASM entry confirms
    // the board is alive.
    loop {
        core::hint::spin_loop();
    }
}
