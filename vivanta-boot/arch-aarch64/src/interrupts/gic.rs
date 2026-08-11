// ---------------------------------------------------------------------------
// GICv2/v3 driver
// ---------------------------------------------------------------------------

#![allow(dead_code)]

use crate::barrier;
use crate::mmio;
use vivanta_boot_common::{hardware::InterruptControllerInfo, println};

// GICv2 offsets
const GICD_CTLR: usize = 0x0000;
const GICD_TYPER: usize = 0x0004;
const GICD_IGROUPR: usize = 0x0080;
const GICD_ISENABLER: usize = 0x0100;
const GICD_ICENABLER: usize = 0x0180;
const GICD_ISPENDR: usize = 0x0200;
const GICD_ICPENDR: usize = 0x0280;
const GICD_IPRIORITYR: usize = 0x0400;
const GICD_SGIR: usize = 0x0F00;
const GICD_CTLR_ENABLE_GRP0: u32 = 1 << 0;
const GICD_CTLR_ENABLE_GRP1: u32 = 1 << 1;

// GICv2 CPU interface
const GICC_CTLR: usize = 0x0000;
const GICC_PMR: usize = 0x0004;
const GICC_IAR: usize = 0x000C;
const GICC_EOIR: usize = 0x0010;
const GICC_CTLR_ENABLE_GRP0: u32 = 1 << 0;
const GICC_CTLR_ENABLE_GRP1: u32 = 1 << 1;

// GICv3 additional
const GICD_CTLR_ARE_NS: u32 = 1 << 4;
const GICR_SGI_BASE: usize = 0x10000;
const GICR_WAKER: usize = 0x0014;
const GICR_IGROUPR0: usize = 0x0080;
const GICR_ISENABLER0: usize = 0x0100;
const GICR_ICENABLER0: usize = 0x0180;
const GICR_ICPENDR0: usize = 0x0280;
const GICR_IPRIORITYR0: usize = 0x0400;
const WAKER_PROCESSOR_SLEEP: u32 = 1 << 2;
const WAKER_CHILDREN_ASLEEP: u32 = 1 << 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GicVersion {
    V2,
    V3,
}

// Module-level statics (used by dispatcher)
static mut GIC_CPU_BASE: u64 = 0;
static mut GIC_USE_SYSREG: bool = false;

pub(crate) unsafe fn set_gic_cpu_base(base: u64) {
    GIC_CPU_BASE = base;
    GIC_USE_SYSREG = false;
}

pub(crate) unsafe fn set_gic_sysreg_mode() {
    GIC_CPU_BASE = 0;
    GIC_USE_SYSREG = true;
}

pub(crate) unsafe fn acknowledge() -> u32 {
    if GIC_USE_SYSREG {
        let irq: u64;
        core::arch::asm!("mrs {0}, ICC_IAR1_EL1", "isb", out(reg) irq, options(nostack));
        (irq & 0xFFFFFF) as u32
    } else {
        let cpu = GIC_CPU_BASE as *mut u8;
        mmio::mmio_read32(cpu.add(0x0C) as *const u32) & 0x3FF
    }
}

pub(crate) unsafe fn eoi(irq: u32) {
    if GIC_USE_SYSREG {
        core::arch::asm!("msr ICC_EOIR1_EL1, {0}", "isb", in(reg) irq as u64, options(nostack));
    } else {
        let cpu = GIC_CPU_BASE as *mut u8;
        mmio::mmio_write32(cpu.add(0x10) as *mut u32, irq);
    }
}

pub struct Gic {
    version: GicVersion,
    dist_base: *mut u8,
    cpu_base: u64,
}

unsafe fn read32(addr: *mut u8, offset: usize) -> u32 {
    mmio::mmio_read32(addr.add(offset) as *const u32)
}
unsafe fn write32(addr: *mut u8, offset: usize, val: u32) {
    mmio::mmio_write32(addr.add(offset) as *mut u32, val)
}

impl Gic {
    pub unsafe fn new(info: &InterruptControllerInfo) -> Self {
        let version = if info.compatible.contains("gic-v3") {
            GicVersion::V3
        } else {
            GicVersion::V2
        };
        let cpu_base = info.redistributor.map_or(0, |r| r.addr);
        if version == GicVersion::V2 {
            println!("  GICv2: Distributor  @ 0x{:x}", info.distributor.addr);
            println!("  GICv2: CPU I/F     @ 0x{:x}", cpu_base);
        }
        Gic {
            version,
            dist_base: info.distributor.addr as *mut u8,
            cpu_base,
        }
    }

    pub unsafe fn init(&self) {
        if self.version == GicVersion::V3 {
            self.init_v3();
        } else {
            self.init_v2();
        }
    }

    unsafe fn init_common_dist(&self) {
        let typer = read32(self.dist_base, GICD_TYPER);
        let it_lines = ((typer >> 0) & 0x1f) + 1;
        let n_spis = it_lines * 32 - 32;
        println!(
            "  GICv{}: {} SPIs",
            if self.version == GicVersion::V3 { 3 } else { 2 },
            n_spis
        );
        write32(self.dist_base, GICD_CTLR, 0);
        let total_regs = (32 + n_spis + 31) / 32;
        for i in 0..total_regs {
            let off = (i as usize) * 4;
            write32(self.dist_base, GICD_ICPENDR + off, 0xFFFFFFFF);
            write32(self.dist_base, GICD_ICENABLER + off, 0xFFFFFFFF);
            write32(self.dist_base, GICD_IGROUPR + off, 0);
        }
        for i in 0..(total_regs * 8) {
            write32(
                self.dist_base,
                GICD_IPRIORITYR + (i as usize) * 4,
                0x80808080,
            );
        }
    }

    unsafe fn init_v2(&self) {
        self.init_common_dist();
        write32(
            self.dist_base,
            GICD_CTLR,
            GICD_CTLR_ENABLE_GRP0 | GICD_CTLR_ENABLE_GRP1,
        );
        let _ = read32(self.dist_base, GICD_CTLR);
        let cpu = self.cpu_base as *mut u8;
        if cpu.is_null() {
            return;
        }
        write32(cpu, GICC_PMR, 0xFF);
        write32(
            cpu,
            GICC_CTLR,
            GICC_CTLR_ENABLE_GRP0 | GICC_CTLR_ENABLE_GRP1,
        );
        let _ = read32(cpu, GICC_CTLR);
        write32(self.dist_base, GICD_ISENABLER, 0x0000FFFF);
        set_gic_cpu_base(self.cpu_base);
        println!("  GICv2: CPU interface enabled");
    }

    unsafe fn init_v3(&self) {
        self.init_common_dist();
        write32(
            self.dist_base,
            GICD_CTLR,
            GICD_CTLR_ENABLE_GRP1 | GICD_CTLR_ARE_NS,
        );
        let _ = read32(self.dist_base, GICD_CTLR);
        let redist = self.cpu_base as *mut u8;
        if redist.is_null() {
            return;
        }
        let waker = read32(redist, GICR_WAKER);
        if waker & WAKER_PROCESSOR_SLEEP != 0 {
            write32(redist, GICR_WAKER, waker & !WAKER_PROCESSOR_SLEEP);
            let mut timeout = 10000;
            while (read32(redist, GICR_WAKER) & WAKER_CHILDREN_ASLEEP) != 0 {
                timeout -= 1;
                if timeout == 0 {
                    break;
                }
            }
        }
        let sgi = redist.add(GICR_SGI_BASE);
        write32(sgi, GICR_ICPENDR0, 0xFFFFFFFF);
        write32(sgi, GICR_ICENABLER0, 0xFFFFFFFF);
        write32(sgi, GICR_IGROUPR0, 0xFFFFFFFF);
        for i in 0..8 {
            write32(sgi, GICR_IPRIORITYR0 + (i as usize) * 4, 0x80808080);
        }
        write32(sgi, GICR_ISENABLER0, 0x0000FFFF);
    }

    pub unsafe fn enable_cpu_interface(&self) {
        if self.version == GicVersion::V3 {
            core::arch::asm!(
                "mrs x0, ICC_SRE_EL1",
                "orr x0, x0, #1",
                "msr ICC_SRE_EL1, x0",
                "isb",
                options(nostack)
            );
            core::arch::asm!("msr ICC_PMR_EL1, {0}", "isb", in(reg) 0xFFu64, options(nostack));
            core::arch::asm!("msr ICC_IGRPEN1_EL1, {0}", "isb", in(reg) 1u64, options(nostack));
            set_gic_sysreg_mode();
            println!("  GICv3: CPU interface enabled");
        }
    }

    pub unsafe fn send_sgi(&self, sgi_num: u32) {
        if self.version == GicVersion::V2 {
            let val = (2 << 24) | sgi_num;
            write32(self.dist_base, GICD_SGIR, val);
            barrier::dsb_sy();
            barrier::isb();
        } else {
            let val = ((sgi_num as u64) << 24) | 1;
            core::arch::asm!("msr ICC_SGI1R_EL1, {0}", "isb", in(reg) val, options(nostack));
        }
    }

    pub unsafe fn enable_irq(&self, irq: u32) {
        let idx = irq as usize;
        write32(
            self.dist_base,
            GICD_ISENABLER + (idx / 32) * 4,
            1 << (idx % 32),
        );
    }

    pub unsafe fn disable_irq(&self, irq: u32) {
        let idx = irq as usize;
        write32(
            self.dist_base,
            GICD_ICENABLER + (idx / 32) * 4,
            1 << (idx % 32),
        );
    }

    pub unsafe fn acknowledge(&self) -> u32 {
        if self.version == GicVersion::V2 {
            let cpu = self.cpu_base as *mut u8;
            read32(cpu, GICC_IAR) & 0x3FF
        } else {
            let irq: u64;
            core::arch::asm!("mrs {0}, ICC_IAR1_EL1", "isb", out(reg) irq, options(nostack));
            (irq & 0xFFFFFF) as u32
        }
    }

    pub unsafe fn eoi(&self, irq: u32) {
        if self.version == GicVersion::V2 {
            let cpu = self.cpu_base as *mut u8;
            write32(cpu, GICC_EOIR, irq);
        } else {
            core::arch::asm!("msr ICC_EOIR1_EL1, {0}", "isb", in(reg) irq as u64, options(nostack));
        }
    }
}
