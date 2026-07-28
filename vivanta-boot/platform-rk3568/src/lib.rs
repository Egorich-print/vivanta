// ---------------------------------------------------------------------------
// platform-rk3568 — Rockchip RK3568 platform support (NS16550 UART)
// ---------------------------------------------------------------------------

#![no_std]

use vivanta_boot_common::set_console;
use vivanta_boot_common::fdt::FdtScanner;
use vivanta_boot_common::ns16550::Ns16550;
use vivanta_boot_common::hardware::NS16550_FAMILY;

/// Hardcoded console init — no FDT dependency, always works.
pub fn init_console_hardcoded() {
    static UART: Ns16550 = Ns16550::new(0xFE66_0000 as *mut u8, 2);
    set_console(&UART);
}

/// Initialise console from FDT, falling back to hardcoded NS16550.
/// Returns whether console was initialised via FDT (true) or fallback (false).
pub unsafe fn init_console_from_fdt(dtb_addr: *const u8) -> bool {
    let console_node = FdtScanner::console(dtb_addr);

    if let Some(node) = console_node {
        if node.matches_any(NS16550_FAMILY) {
            let base = node.reg.unwrap().addr as *mut u8;
            let shift = node.reg_shift.unwrap_or(2) as u8;
            static mut UART: Option<Ns16550> = None;
            let uart_ptr = core::ptr::addr_of_mut!(UART);
            uart_ptr.write(Some(Ns16550::new(base, shift)));
            if let Some(ref u) = *uart_ptr {
                set_console(u);
            }
            return true;
        }
    }

    // Fallback
    init_console_hardcoded();
    false
}

/// Build memory map and count CPUs from FDT.
pub unsafe fn build_memory_map(dtb_addr: *const u8) -> (vivanta_boot_common::MemoryMap, usize) {
    let mut mem_map = vivanta_boot_common::MemoryMap::new();
    let cpu_count = FdtScanner::report(dtb_addr, &mut mem_map);
    (mem_map, cpu_count)
}
