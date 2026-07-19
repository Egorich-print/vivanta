// ---------------------------------------------------------------------------
// Hardware Description IR — normalised FDT representation (ADR-011)
//
// This is NOT a device lifecycle or resource model.
// This is a flat data structure for boot-time hardware discovery.
// ---------------------------------------------------------------------------

/// MMIO region discovered from FDT.
#[derive(Debug, Clone, Copy)]
pub struct MmioRegion {
    pub addr: u64,
    pub size: u64,
}

/// A single hardware node discovered from FDT.
///
/// Represents the minimum information needed to initialise a driver during
/// early boot. No lifecycle, no ownership, no graph edges.
#[derive(Debug, Clone, Copy)]
pub struct HardwareNode {
    /// Full compatible string (e.g. "arm,pl011", "ns16550a").
    pub compatible: &'static str,

    /// MMIO range (reg property).
    pub reg: Option<MmioRegion>,

    /// First interrupt number.
    pub irq: Option<u32>,

    /// reg-shift property (NS16550-specific).
    pub reg_shift: Option<u32>,

    /// reg-io-width property.
    pub reg_io_width: Option<u32>,

    /// clock-frequency property.
    pub clock_frequency: Option<u32>,

    /// current-speed property.
    pub current_speed: Option<u32>,
}

impl HardwareNode {
    pub const fn empty() -> Self {
        HardwareNode {
            compatible: "",
            reg: None,
            irq: None,
            reg_shift: None,
            reg_io_width: None,
            clock_frequency: None,
            current_speed: None,
        }
    }

    /// Check whether the node's compatible string matches any of the given
    /// family patterns.
    ///
    /// This is deliberately NOT a driver-binding system — it is a flat list
    /// that grows only when a new physical UART variant is encountered on
    /// real hardware.
    pub fn matches_any(&self, patterns: &[&str]) -> bool {
        patterns.iter().any(|p| self.compatible.contains(p))
    }
}

/// Discovered interrupt controller at boot time.
#[derive(Debug, Clone, Copy)]
pub struct InterruptControllerInfo {
    pub compatible: &'static str,
    pub distributor: MmioRegion,
    pub redistributor: Option<MmioRegion>,
}

/// Known NS16550-family UART compatible strings observed on real hardware.
pub const NS16550_FAMILY: &[&str] = &[
    "ns16550",
    "ns16550a",
    "snps,dw-apb-uart",
    "rockchip,rk3568-uart",
];
