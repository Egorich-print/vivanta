// ---------------------------------------------------------------------------
// platform-qemu — QEMU virt platform support (PL011 UART, FDT console)
// ---------------------------------------------------------------------------

#![no_std]

use vivanta_boot_common::{Console, set_console, println};
use vivanta_boot_common::fdt::FdtScanner;
use vivanta_boot_common::hardware::HardwareNode;

// PL011 driver lives in `vivanta_boot_common::pl011`

// ---------------------------------------------------------------------------
// Platform initialization
// ---------------------------------------------------------------------------

/// Initialise console from the FDT. Supports PL011 and NS16550.
/// # Safety
/// Must be called with a valid DTB pointer.
pub unsafe fn init_console(node: &HardwareNode) {
    use vivanta_boot_common::ns16550::Ns16550;
    use vivanta_boot_common::hardware::NS16550_FAMILY;
    use vivanta_boot_common::pl011::Pl011;

    let base = node.reg.map(|r| r.addr as usize).expect("console has no reg");

    if node.compatible.contains("pl011") {
        const QEMU_PL011_CLOCK: u32 = 24_000_000;
        static mut UART: Option<Pl011> = None;
        UART = Some(Pl011::new(base));
        UART.as_ref().unwrap().init(QEMU_PL011_CLOCK, 115200);
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

/// Discover console from FDT and initialise it.
/// Returns the HardwareNode for the console.
pub unsafe fn init_console_from_fdt(dtb_addr: *const u8) -> HardwareNode {
    let node = FdtScanner::console(dtb_addr).expect("no console in FDT");
    init_console(&node);
    node
}

/// Build memory map and count CPUs from FDT.
pub unsafe fn build_memory_map(dtb_addr: *const u8) -> (vivanta_boot_common::MemoryMap, usize) {
    let mut mem_map = vivanta_boot_common::MemoryMap::new();
    let cpu_count = FdtScanner::report(dtb_addr, &mut mem_map);
    (mem_map, cpu_count)
}
