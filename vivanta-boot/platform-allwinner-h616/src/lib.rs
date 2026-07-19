// ---------------------------------------------------------------------------
// platform-allwinner-h616 — Allwinner H616/H313/H618 platform (NS16550 UART)
//
// H313 is a cost-reduced H616 (identical die, lower frequency).  H618 is a
// higher-clocked variant.  All three use the same UART, GIC-400, and FDT
// layout.  This crate supports all of them.
//
// UART0 base: 0x05000000  (reg-shift=2, NS16550-compatible)
// GIC:        GIC-400 (GICv2) at 0x03000000
// DRAM base:  0x40000000
// ---------------------------------------------------------------------------

#![no_std]

use vivanta_boot_common::{set_console, println};
use vivanta_boot_common::fdt::FdtScanner;
use vivanta_boot_common::ns16550::Ns16550;
use vivanta_boot_common::hardware::NS16550_FAMILY;

/// Allwinner H616 UART0 hardcoded fallback address.
const H616_UART0_BASE: u64 = 0x0500_0000;
const H616_UART0_REG_SHIFT: u8 = 2;

/// Initialise console from FDT, falling back to hardcoded NS16550 at
/// 0x05000000 (the standard UART0 on all Allwinner H616-family SoCs).
///
/// Returns whether console was initialised via FDT (true) or fallback (false).
pub unsafe fn init_console_from_fdt(dtb_addr: *const u8) -> bool {
    let console_node = FdtScanner::console(dtb_addr);

    if let Some(node) = console_node {
        // Match: "ns16550", "ns16550a", "snps,dw-apb-uart",
        //        or any allwinner,sun50i-h616-uart variant
        let allwinner_compat = node.compatible.contains("allwinner");
        if node.matches_any(NS16550_FAMILY) || allwinner_compat {
            let base = node.reg.unwrap().addr as *mut u8;
            let shift = node.reg_shift.unwrap_or(2) as u8;
            static mut UART: Option<Ns16550> = None;
            let uart_ptr = core::ptr::addr_of_mut!(UART);
            uart_ptr.write(Some(Ns16550::new(base, shift)));
            if let Some(ref u) = *uart_ptr {
                set_console(u);
                println!("  Console: {} @ 0x{:x} (class=NS16550, shift={})",
                    node.compatible, base as u64, shift);
            }
            return true;
        }
    }

    // Fallback: hardcoded NS16550 at 0x05000000, reg-shift=2
    static UART: Ns16550 = Ns16550::new(H616_UART0_BASE as *mut u8, H616_UART0_REG_SHIFT);
    set_console(&UART);
    println!("  Console: fallback NS16550 @ 0x{:x} (shift={})",
        H616_UART0_BASE, H616_UART0_REG_SHIFT);
    false
}

/// Build memory map and count CPUs from FDT.
pub unsafe fn build_memory_map(dtb_addr: *const u8) -> (vivanta_boot_common::MemoryMap, usize) {
    let mut mem_map = vivanta_boot_common::MemoryMap::new();
    let cpu_count = FdtScanner::report(dtb_addr, &mut mem_map);
    (mem_map, cpu_count)
}
