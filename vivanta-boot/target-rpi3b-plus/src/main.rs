#![no_std]
#![no_main]

use core::panic::PanicInfo;
use vivanta_boot_common::pl011::Pl011;
use vivanta_boot_common::{
    EarlyPlatformInfo, MemoryMap, MemoryRegion, MemoryRegionKind, println, set_console,
    set_early_platform,
};
use vivanta_boot_info::{BootInfo, MmioKind, MmioRegion};
use vivanta_platform_rpi3b::init_uart_gpio;

// Force link arch-aarch64 for extern "Rust" symbol resolution
extern crate vivanta_arch_aarch64;

const PL011_BASE: usize = 0x3F20_1000;

core::arch::global_asm!(
    ".section .text._start",
    ".global _start",
    "_start:",
    "b _real_start",
    ".word 0",
    ".quad 0x00000000",
    ".quad 0",
    ".quad 0x00080000",
    ".quad 0",
    ".quad 0",
    ".quad 0",
    ".word 0x644d5241",
    ".word 0",
    "_real_start:",
    "msr daifset, #0xf",
    // The firmware enters at EL2; the kernel is EL1-only. Drop EL2->EL1h
    // (same sequence as the x96q target). x0 (firmware DTB pointer)
    // survives eret untouched. Already-EL1 boards skip this.
    "mrs x5, CurrentEL",
    "and x5, x5, #0xC",
    "cmp x5, #(2 << 2)",
    "b.ne 8f",
    "mov x5, #(0b11 << 20)",
    "msr CPTR_EL2, x5",
    "mrs x5, HCR_EL2",
    "orr x5, x5, #(1 << 31)",
    "msr HCR_EL2, x5",
    "msr CNTVOFF_EL2, xzr",
    "mov x5, #0x3c5",
    "msr SPSR_EL2, x5",
    "adr x5, 8f",
    "msr ELR_EL2, x5",
    "eret",
    "8:",
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
    "1: str x2, [x1], #8",
    "subs x3, x3, #8",
    "b.gt 1b",
    "2:",
    "adrp x1, BOOT_CONTEXT",
    "add x1, x1, :lo12:BOOT_CONTEXT",
    "str x0, [x1]",
    "bl adapter_main",
    "3: wfi",
    "b 3b",
);

static UART: Pl011 = Pl011::new(PL011_BASE);

#[unsafe(no_mangle)]
pub extern "C" fn adapter_main() -> ! {
    unsafe {
        init_uart_gpio();
        UART.init(250_000_000, 115_200);
        set_console(&UART);
        set_early_platform(EarlyPlatformInfo {
            uart_base: PL011_BASE,
        });

        println!();
        println!("──── Vivanta Boot Adapter (RPi3B+/BCM2837) ────");
        println!(
            "  Console: PL011 @ 0x{:x} (250 MHz, 115200 baud)",
            PL011_BASE
        );
        println!();

        // BCM2837: 1 GiB DRAM at 0x0. Top 64 MiB belong to the VideoCore
        // GPU with the default gpu_mem split — keep clear of it. (If your
        // config.txt sets a different gpu_mem, adjust RPI_USABLE_END.)
        const RPI_USABLE_END: u64 = 0x3C00_0000; // 960 MiB
        let mut mem_map = MemoryMap::new();
        mem_map.push(MemoryRegion {
            start: 0,
            size: RPI_USABLE_END,
            kind: MemoryRegionKind::Usable,
        });

        // MMIO: PL011 UART only (user-accessible, like QEMU virt).
        // BCM2837 has no GIC — interrupt_controller stays None, so the
        // kernel skips GIC/timer init and runs cooperatively (no
        // preemption; the G4 gate reports SKIP instead of failing).
        static MMIO_REGIONS: [MmioRegion; 1] = [MmioRegion {
            base: PL011_BASE as u64,
            size: 0x1000,
            kind: MmioKind::UserDevice,
        }];

        // Assemble BootInfo (same MaybeUninit dance as the QEMU adapter:
        // the references must outlive this frame).
        let mut mem_map_buf: core::mem::MaybeUninit<MemoryMap> = core::mem::MaybeUninit::uninit();
        let mut boot_info_buf: core::mem::MaybeUninit<BootInfo> = core::mem::MaybeUninit::uninit();

        mem_map_buf.as_mut_ptr().write(mem_map);
        let mem_map_ref: &'static MemoryMap = &*mem_map_buf.as_ptr();

        boot_info_buf.as_mut_ptr().write(BootInfo {
            memory_map: mem_map_ref,
            mmio_regions: &MMIO_REGIONS,
            interrupt_controller: None,
            cpu_count: 4,
            dtb: None,
        });

        vivanta_kernel::kernel_main(&*boot_info_buf.as_ptr());
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("PANIC: {}", info);
    loop {}
}
