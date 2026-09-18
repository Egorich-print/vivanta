# VIVANTA-AIBOX-BOOT-CONTRACT.md
## Minimum Verified Boot Contract for AIBOX-1684X Vivanta Bring-Up

This document defines the **exact state** Vivanta will inherit at kernel entry on the FireFly AIBOX-1684X, based exclusively on verified evidence from the serial boot log. It is the contract between vendor firmware and Vivanta.

---

## Boot Chain (Verified)

```text
Power On
   ↓
[EL3] Boot ROM (BL1 v2.5, bm1686_rom_v6) — immutable
   ↓ loads from SPI-NOR flash (FIP)
[EL3] BL2 (v2.7) — DDR training (LPDDR4x 4000M), board type detection
   ↓ loads image id=3 + image id=5 from SPI-NOR FIP
[EL3] BL31 / TF-A (v2.7) — GICv2 init, PSCI setup, CNTFRQ set
   ↓ exits to normal world (SPSR=0x3c9 → EL2)
[EL2] U-Boot 2022.10 — loads FIT from eMMC (mmc0:1)
   ↓ bootm with FIT image (emmcboot.itb)
[EL2] Kernel Entry @ 0x300280000
   ↓
[EL2→EL1] Vivanta (replacing Linux kernel subimage)
```

---

## CPU State at Kernel Entry

| Register / State | Value | Evidence | Classification |
|---|---|---|---|
| **EL** | EL2 | `CPU: All CPU(s) started at EL2` (line 288) | `[OBSERVED]` |
| **MMU** | OFF | Standard arm64 boot protocol | `[INFERRED]` |
| **D-cache** | OFF | Standard arm64 boot protocol | `[INFERRED]` |
| **I-cache** | ON (VIPT) | `Detected VIPT I-cache on CPU0` (line 227) | `[INFERRED]` |
| **x0** | DTB physical address | Standard arm64 boot protocol (U-Boot bootm) | `[INFERRED]` |
| **x1, x2, x3** | 0 | Standard arm64 boot protocol | `[INFERRED]` |
| **SP** | `[UNKNOWN]` | Not visible in boot log | `[UNKNOWN]` |
| **CNTFRQ_EL0** | 50,000,000 (50 MHz) | `arch_timer: cp15 timer(s) running at 50.00MHz` (line 243) | `[OBSERVED]` |
| **CNTKCTL_EL1** | `[UNKNOWN]` | — | `[UNKNOWN]` |
| **CPUECTLR_EL1** | `[UNKNOWN]` | — | `[UNKNOWN]` |
| **SCTLR_EL2** | `[UNKNOWN]` | — | `[UNKNOWN]` |
| **HCR_EL2** | `[UNKNOWN]` | — | `[UNKNOWN]` |
| **VBAR_EL2** | `[UNKNOWN]` | — | `[UNKNOWN]` |

### CPU identification:
* **MIDR_EL1:** `0x410fd034` (ARM Cortex-A53 r0p4) — `[OBSERVED]`
* **CPU count:** 8 — `[OBSERVED]`
* **Topology:** 2 clusters × 4 cores (affinity: CPU0-3 = `0x000-0x003`, CPU4-7 = `0x100-0x103`) — `[OBSERVED]`
* **PA size:** ≥40 bits (KVM IPA limit) — `[OBSERVED]`
* **DMA width:** 40 bits — `[OBSERVED]`

---

## Memory State at Kernel Entry

### DRAM initialized by BL2:
* **Type:** LPDDR4x, rank 2+2, interleave mode 1, 4000 MHz — `[OBSERVED]`
* **Total size:** 16,055,040 KB (~15.3 GiB) — `[REPORTED BY LINUX]`
* **Available to OS:** 6,289,304 KB (~6.0 GiB) — `[REPORTED BY LINUX]`

### Vivanta must NOT touch these regions:

| Base | Size | Purpose | Evidence |
|---|---|---|---|
| `0x0124100000` | `0xf6e00000` (~3.95 GiB) | NPU accelerator memory | `[OBSERVED]` lines 215-216 |
| `0x380000000` | `0x80000000` (2 GiB) | VPU accelerator memory | `[OBSERVED]` lines 217-218 |
| `0x440000000` | `0xc0000000` (3 GiB) | VPP accelerator memory | `[OBSERVED]` lines 219-220 |
| `0x438000000` | `0x8000000` (128 MiB) | CMA DMA pool | `[OBSERVED]` lines 213-214 |
| `0x314000000` | `0x100000` (1 MiB) | Ramoops/pstore | `[OBSERVED]` line 305 |

### Vivanta may use:

* **General-purpose RAM:** ~6 GiB (everything not reserved above, from `0x0` to approximately `0x3C0000000`)
* **Vivanta load address:** `0x300280000` (as a valid DRAM address, matching FIT kernel subimage convention)

---

## DTB Handoff

| Parameter | Value | Evidence | Classification |
|---|---|---|---|
| DTB physical address in x0 | `0x33f1a3000` | U-Boot loaded DTB to `0x33f1a3000` (line 200) | `[OBSERVED]` (U-Boot load address; x0 = dtb is `[INFERRED]` from standard arm64 boot protocol) |
| DTB size | 25,648 bytes | `Data Size: 25648 Bytes` (line 192) | `[OBSERVED]` |
| DTB model | `bitmain-bm1684x-se7-v1-mini` | `found dtb@180` (line 69) | `[OBSERVED]` |
| DTB description | `for BM1684X AiBox One` | line 188 | `[OBSERVED]` |

**Note:** Vivanta does NOT need to use this DTB if it hardcodes platform addresses. But the DTB is available at x0 if Vivanta wants to parse it for device discovery.

---

## Timer State

| Parameter | Value | Evidence | Classification |
|---|---|---|---|
| Counter frequency (CNTFRQ_EL0) | 50,000,000 Hz (50 MHz) | `arch_timer: cp15 timer(s) running at 50.00MHz` (line 243) | `[OBSERVED]` |
| Timer type | ARM Generic Timer (physical) | `cp15 timer(s) running at 50.00MHz (phys)` | `[OBSERVED]` |
| Configured by | BL31 / TF-A (EL3 firmware) | Standard TF-A behavior | `[INFERRED]` |

**Vivanta action:** Read CNTFRQ_EL0 at entry (or trust the 50 MHz value). Use physical timer (CNTPT_EL1 / CNTP_CVAL_EL1 / CNTP_TVAL_EL1 / CNTP_CTL_EL1) for EL1 operation.

---

## GIC State

| Parameter | Value | Evidence | Classification |
|---|---|---|---|
| GIC version | GICv2 | `ARM GICv2 driver initialized` (line 63, BL31) | `[OBSERVED]` |
| EOI mode | Split EOI/Deactivate | `GIC: Using split EOI/Deactivate mode` (line 242) | `[OBSERVED]` |
| GICD base | `[UNKNOWN]` | Not visible in boot log | `[UNAVAILABLE]` |
| GICC base | `[UNKNOWN]` | Not visible in boot log | `[UNAVAILABLE]` |
| GICR | N/A (GICv2 has no GICR) | GICv2 architecture | `[INFERRED]` |
| NR_IRQS | 64 | `NR_IRQS: 64` (line 241) | `[OBSERVED]` |

**Vivanta action:** Initialize GICv2 driver (GICD + GICC). Base addresses must be obtained from DTB or hardcoded (likely in the `0x50000000` MMIO range, possibly `0x50010000` or similar — `[UNVERIFIED]`).

**top-intc:** For minimal boot, ignore `bitmain,top-intc` at `0x500100bc` — it only handles MSI-capable peripherals (PCIe/USB). UART and timer use direct GIC IRQs.

---

## UART State

| Parameter | Value | Evidence | Classification |
|---|---|---|---|
| Controller | 16550A | `is a 16550A` (line 453) | `[OBSERVED]` |
| MMIO base | `0x50118000` | Lines 209, 453 | `[OBSERVED]` |
| Register width | 32-bit | `MMIO32` (line 209) | `[OBSERVED]` |
| IRQ | 15 | Line 453 | `[OBSERVED]` |
| Input clock | 31,250,000 Hz | `base_baud = 31250000` (line 453) | `[OBSERVED]` |
| Baud rate | 115200 | Serial observation | `[OBSERVED]` |
| Vendor init needed | NO | UART works at earlycon before any driver | `[INFERRED]` |

**Vivanta action:** Write standard 16550A driver targeting `0x50118000` with 32-bit register access. Set divisor for 115200 baud from 31.25 MHz input clock. No vendor-specific initialization sequence required.

---

## PSCI State

| Parameter | Value | Evidence | Classification |
|---|---|---|---|
| PSCI version | 1.1 | `PSCIv1.1 detected in firmware` (line 222) | `[OBSERVED]` |
| Conduit | SMC (not HVC) | `SMC Calling Convention v1.0` (line 225) | `[OBSERVED]` |
| Function IDs | Standard PSCI v0.2 | `Using standard PSCI v0.2 function IDs` (line 223) | `[OBSERVED]` |
| MIGRATE_INFO_TYPE | Not supported | `MIGRATE_INFO_TYPE not supported` (line 224) | `[OBSERVED]` |
| CPU_ON | Available (used for secondary bring-up) | Lines 256-269: `bm_pwr_domain_on` / `bm_pwr_domain_on_finish` | `[OBSERVED]` |

**Vivanta action:** Use SMC calls to PSCI for secondary CPU bring-up (CPU_ON), power management (CPU_OFF, CPU_SUSPEND), and system reset (SYSTEM_RESET). Standard PSCI v0.2 function IDs.

---

## Cache State

| Parameter | Value | Evidence | Classification |
|---|---|---|---|
| I-cache | VIPT, enabled at entry | `Detected VIPT I-cache on CPU0` (line 227) | `[OBSERVED]` |
| D-cache | OFF at entry | Standard arm64 boot protocol | `[INFERRED]` |
| Cache line size | 64 bytes | `HWalign=64` (line 234) | `[OBSERVED]` |
| Erratum 845719 | Present | `CPU features: detected: ARM erratum 845719` (line 228) | `[OBSERVED]` |

---

## Secondary CPU Bring-Up Contract

| Parameter | Value | Evidence | Classification |
|---|---|---|---|
| Method | PSCI CPU_ON via SMC | Lines 256-269 | `[OBSERVED]` |
| Secondary entry | Set by PSCI caller | — | `[INFERRED]` |
| Secondary EL | EL2 (same as primary) | `All CPU(s) started at EL2` (line 288, includes all 8) | `[OBSERVED]` |
| Secondary state at entry | MMU OFF, caches OFF | Standard arm64 secondary kernel protocol | `[INFERRED]` |

---

## What Vivanta Must Implement

### P0 — Minimal Proof of Life:

1. **EL2→EL1 transition** (or stay at EL2)
2. **Early UART** (16550A @ `0x50118000`, 32-bit, 115200 baud, 31.25 MHz clock)
3. **Exception vector table** (VBAR_EL1 or VBAR_EL2)
4. **GICv2 initialization** (GICD + GICC, base addresses from DTB or hardcoded)
5. **ARM Generic Timer** (CNTFRQ = 50 MHz, use physical timer at EL1)
6. **MMU initialization** (4 KiB granule, 48-bit VA, 40-bit PA)
7. **PMM initialization** (carve out NPU/VPU/VPP/CMA/reserved regions)
8. **Scheduler** (timer-driven preemption)

### P1 — Multi-Core:

9. **PSCI CPU_ON** for secondary bring-up (SMC calls to BL31)

### NOT required for first boot:
- NPU, VPU, VPP
- Ethernet, USB, PCIe
- eMMC, SD card
- I2C, RTC, Watchdog
- `bitmain,top-intc` (ignore until PCIe/USB needed)

---

## What Vivanta Delegates to Vendor Firmware

| Responsibility | Delegated to | Why |
|---|---|---|
| DDR training (LPDDR4x 4000M) | BL2 (SPI-NOR) | Proprietary vendor DDR firmware |
| Secure world (EL3) | BL31 / TF-A (SPI-NOR) | Provides PSCI, CNTFRQ, GICv2 init |
| Boot loading | U-Boot 2022.10 (eMMC) | Loads Vivanta payload from FIT image |
| Board type detection | BL2 | I2C-based board ID (type 180/0x33/0x0) |
| MCU communication | BL2 / Linux driver | I2C MCU at address 0x17 (bm16xx-mcu) |

---

## FIT Image Packaging for Vivanta

To boot Vivanta on AIBOX-1684X:

1. Build Vivanta as an AArch64 uncompressed Image
2. Create a FIT image (`.its` file) with:
   - `config-pcb180` configuration (or new name)
   - kernel subimage: Vivanta binary, load/entry `0x300280000`, type `Kernel Image`, arch `AArch64`, os `Vivanta` (or `Linux` for U-Boot compatibility)
   - fdt subimage: original BM1684X DTB (25 KiB)
   - ramdisk subimage: dummy or omit (set as optional)
3. Sign with SHA1 (matching U-Boot's verification expectations)
4. Place as `emmcboot.itb` on eMMC partition 1
5. Ensure `/boot.scr.emmc` selects the correct config

**No firmware modifications needed.** Vendor BL1, BL2, BL31, and U-Boot remain untouched.
