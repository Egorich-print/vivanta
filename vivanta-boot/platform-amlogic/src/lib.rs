// ---------------------------------------------------------------------------
// platform-amlogic — Amlogic GXL/GXBB family (S905L, S905M, S905X)
//
// Target device: Sumavision Q5 / WIFIRE Q5 / Мегафон Q5
//   - SoC:      Amlogic S905L (GXL family, p212 reference board)
//   - CPU:      4× Cortex-A53 @ 1.2–1.5 GHz (ARMv8-A, 64-bit)
//   - RAM:      1 GB DDR3 (base: 0x00000000)
//   - Storage:  8 GB eMMC
//   - WiFi/BT:  RTL8822CS (USB interface)
//   - Ethernet: 100 Mbps
//   - GPU:      Mali-450 MP3
//
// Amlogic GXL Memory Map (from Linux meson-gxl.dtsi):
//   UART_AO:    0xc81004e0  (always-on domain, used for debug console)
//   UART_A:     0xc11084c0  (main UART block)
//   GIC:        GIC-400 (GICv2)
//     Distributor:     0xc4301000
//     CPU Interface:   0xc4302000
//   Timer (ARM): Generic ARMv8 timer (CNTFRQ = 24 MHz typical)
//   DRAM:       0x00000000 – 0x3fffffff (1 GB)
//   Kernel load: 0x01080000 (standard Amlogic kernel load address)
//
// Boot chain:
//   BootROM → BL2 → BL30 → BL31 (ATF) → BL33 (U-Boot) → Vivanta
//
// U-Boot commands for chainloading Vivanta:
//   fatload mmc 0:1 0x01080000 vivanta-suma-q5.bin
//   go 0x01080000
//
// Key unknowns to verify on hardware:
//   - UART_AO register layout (Meson UART, not PL011/NS16550)
//   - Exact GIC addresses (verify via U-Boot or device tree)
//   - DRAM size (should be 1 GB, verify via U-Boot `bdinfo`)
//   - CPU numbering (which core is core 0?)
//   - Boot state at kernel entry (EL2 or EL3? or already at EL1S?)
//   - MPIDR values for PSCI/Affinity
//
// Amlogic UART (Meson UART) register layout:
//   Unlike PL011 or NS16550, Amlogic uses a custom UART IP block
//   with 32-bit register access and 5-bit register offsets.
//
//   Registers (WO/WFIFO – Write-Only FIFO level, RO/RFIFO – Read-Only FIFO level):
//     Offset 0x04:  CONTROL    — baud rate, parity, stop bits
//     Offset 0x08:  STATUS     — TX/RX FIFO status, errors
//     Offset 0x0C:  MISC       — misc control
//     Offset 0x10:  WFIFO      — write data (transmit FIFO)
//     Offset 0x14:  RFIFO      — read data (receive FIFO)
//
//   STATUS register bits:
//     Bit 20: TX FIFO empty (when set, can write to WFIFO)
//     Bit 21: TX FIFO full
//     Bit 22: TX FIFO count (uses 0-63 values)
//
//   Baud rate calculation: baud = XTAL / (divisor * 4)
//   Where XTAL = 24 MHz, divisor from CONTROL register fields.
//
// Reference: Linux kernel driver (drivers/tty/serial/meson_uart.c)
// Reference: U-Boot driver (drivers/serial/serial_meson.c)
// ---------------------------------------------------------------------------

#![no_std]

use vivanta_boot_common::println;

// ---------------------------------------------------------------------------
// Known hardware addresses (Amlogic GXL / p212 reference board)
// TO VERIFY on actual hardware via U-Boot `fdt addr` and device tree dump
// ---------------------------------------------------------------------------

/// Amlogic GXL UART_AO base address (always-on domain).
/// This is the primary debug UART on most Amlogic boards.
pub const UART_AO_BASE: u64 = 0xc810_04e0;

/// Amlogic GXL GIC-400 distributor base address.
pub const GIC_DIST_BASE: u64 = 0xc430_1000;

/// Amlogic GXL GIC-400 CPU interface base address.
pub const GIC_CPU_BASE: u64 = 0xc430_2000;

/// Amlogic GXL DRAM base address.
pub const DRAM_BASE: u64 = 0x0000_0000;

/// Standard Amlogic kernel load address (used by U-Boot).
pub const KERNEL_LOAD_ADDR: u64 = 0x0108_0000;

/// UART address for early boot output (before FDT parsing).
/// Uses UART_AO which is in the always-on power domain.
pub const EARLY_UART_BASE: u64 = UART_AO_BASE;

/// Amlogic GXL Platform Info — call during early boot before full Vivanta init.
///
/// This configures the early console on UART_AO so that panic messages
/// are visible even before the full kernel boots.
pub unsafe fn init_early_platform() {
    vivanta_boot_common::set_early_platform(vivanta_boot_common::EarlyPlatformInfo {
        uart_base: EARLY_UART_BASE as usize,
    });
}

/// Initialise console from FDT, with fallback to hardcoded Meson UART at
/// UART_AO_BASE. Returns true if FDT was used, false if fallback.
///
/// NOTE: Meson UART driver not yet implemented.
/// Currently stubbed — will need a new `meson_uart.rs` in boot_common.
pub unsafe fn init_console_from_fdt(dtb_addr: *const u8) -> bool {
    // TODO: Implement Meson UART driver in boot_common
    // For now, print the known UART base for reference
    let _ = dtb_addr;
    println!(
        "  [Amlogic] UART_AO base: 0x{:x} (Meson UART — driver TBD)",
        UART_AO_BASE
    );
    false
}

/// Build memory map and count CPUs from FDT.
pub unsafe fn build_memory_map(dtb_addr: *const u8) -> (vivanta_boot_common::MemoryMap, usize) {
    let mut mem_map = vivanta_boot_common::MemoryMap::new();
    let cpu_count = vivanta_boot_common::fdt::FdtScanner::report(dtb_addr, &mut mem_map);
    (mem_map, cpu_count)
}
