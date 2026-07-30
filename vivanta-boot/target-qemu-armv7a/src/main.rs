// ---------------------------------------------------------------------------
// target-qemu-armv7a — QEMU ARMv7-A vivanta_kernel binary
// ---------------------------------------------------------------------------

#![no_std]
#![no_main]

extern crate vivanta_arch_armv7a;

use core::panic::PanicInfo;

use vivanta_boot_common::{FmtAdapter, MemoryMap,
                  println, set_console, with_console};
use vivanta_boot_info::BootInfo;

// ARMv7 entry point (from separate entry.S to avoid LLVM integrated assembler quirks)
core::arch::global_asm!(
    ".section .text._start, \"ax\"",
    ".global _start",
    ".type _start, %function",
    "_start:",
    "ldr r0, stack_top_ptr",
    "add sp, r0, #0",
    "b rust_main",
    "stack_top_ptr:",
    ".word __stack_top",
);

extern "C" {
    static __bss_start: u8;
    static __bss_end: u8;
}

unsafe fn bss_zero() {
    let start = &__bss_start as *const u8 as usize;
    let end = &__bss_end as *const u8 as usize;
    if end > start {
        core::ptr::write_bytes(start as *mut u8, 0, end - start);
    }
}

unsafe fn fp_enable() {
    let cpacr: u32;
    core::arch::asm!("mrc p15, 0, {0}, c1, c0, 2", out(reg) cpacr);
    core::arch::asm!("mcr p15, 0, {0}, c1, c0, 2", in(reg) cpacr | (0xFu32 << 20));
}

unsafe fn uart_hello() {
    let uart = 0x0900_0000 as *mut u32;
    core::ptr::write_volatile(uart, 0x21);
}

/// Simple FDT scanner for ARMv7 (duplicated from vivanta_boot_common due to
/// ARMv7 limitations; to be unified in a future sprint).
unsafe fn scan_fdt(dtb_addr: *const u8, mem: &mut MemoryMap) {
    if dtb_addr.is_null() { return; }
    let magic = core::ptr::read_volatile(dtb_addr as *const u32);
    if magic != 0xEDFE0DD0 { return; }
    // Count memory nodes
    let off_dt_struct = core::ptr::read_volatile(dtb_addr.add(8) as *const u32);
    let off_dt_strings = core::ptr::read_volatile(dtb_addr.add(12) as *const u32);
    let size_dt_struct = core::ptr::read_volatile(dtb_addr.add(36) as *const u32);
    let struct_addr = dtb_addr.add(u32::from_be(off_dt_struct) as usize);
    let struct_end = struct_addr.add(u32::from_be(size_dt_struct) as usize);
    let strings_addr = dtb_addr.add(u32::from_be(off_dt_strings) as usize);

    // Use vivanta_boot_common FdtScanner if the FDT is at a valid address
    if !dtb_addr.is_null() {
        vivanta_boot_common::fdt::FdtScanner::probe(dtb_addr, mem);
    }
}

#[no_mangle]
pub unsafe extern "C" fn rust_main() -> ! {
    uart_hello();
    bss_zero();
    fp_enable();

    // Initialise PL011 UART
    use vivanta_boot_common::pl011::Pl011;
    static UART: Pl011 = Pl011::new(0x0900_0000);
    UART.init(24_000_000, 115200);
    set_console(&UART);

    println!();
    println!("\u{2500}\u{2500}\u{2500}\u{2500} Vivanta Boot Adapter (ARMv7/QEMU) \u{2500}\u{2500}\u{2500}\u{2500}");

    // Try DTB from r2 (ARM boot convention), or scan
    let mut dtb_addr: usize;
    let mut r0_val: usize;
    let mut r1_val: usize;
    core::arch::asm!("mov {0}, r0", out(reg) r0_val);
    core::arch::asm!("mov {0}, r1", out(reg) r1_val);
    core::arch::asm!("mov {0}, r2", out(reg) dtb_addr);
    println!("  Boot regs: r0=0x{:x}  r1=0x{:x}  r2=0x{:x}", r0_val, r1_val, dtb_addr);

    let r2_val = dtb_addr;
    if !(r2_val >= 0x4000_0000 && r2_val < 0x6000_0000
        && core::ptr::read_volatile(r2_val as *const u32) == 0xEDFE0DD0)
    {
        if core::ptr::read_volatile(0x5000_0000usize as *const u32) == 0xEDFE0DD0 {
            dtb_addr = 0x5000_0000;
            println!("FDT: DTB found at hard-coded 0x50000000");
        } else {
            dtb_addr = 0;
            println!("FDT: no DTB found");
        }
    }

    let dtb_ptr = dtb_addr as *const u8;
    let mut mem_map = MemoryMap::new();
    scan_fdt(dtb_ptr, &mut mem_map);

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
        cpu_count: 1,
        dtb: Some(dtb_addr),
    });

    vivanta_kernel::kernel_main(&*boot_info_buf.as_ptr());
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    use core::fmt::Write;
    with_console(|c| {
        let mut w = FmtAdapter(c);
        let _ = writeln!(w, "\n!!! PANIC: {}", info);
    });
    loop { core::hint::spin_loop(); }
}
