# FireFly AIBOX-1684X Hardware & Boot-Chain Reconnaissance Report (Verified)

## Executive Summary

This report documents the verified hardware and boot-chain reconnaissance of the **FireFly AIBOX-1684X** platform, conducted via direct serial observation (`/dev/cu.wchusbserial10` at 115200 baud) during a single power-on boot and 912-line raw boot log capture.

No authenticated shell was obtained. All evidence comes from the serial boot stream (BL1, BL2, BL31, U-Boot, Linux kernel). `/proc/iomem`, `/proc/device-tree/`, `live.dtb`, U-Boot console, and all runtime Linux information remain `[UNAVAILABLE — AUTHENTICATION REQUIRED]`.

Despite this limitation, the boot log provides **sufficient verified evidence** to design a first Vivanta bare-metal bring-up. The AIBOX-1684X is reclassified as a **realistic Vivanta experimental platform candidate**.

---

## 1. Exact Hardware Identification

| Parameter | Value | Evidence | Classification |
|---|---|---|---|
| Platform | FireFly AIBOX-1684X | Board type `180/0x33/0x0` (line 44), config `config-pcb180` (line 148) | `[OBSERVED]` |
| SoC | Bitmain/Sophon BM1684X | U-Boot banner (line 73), kernel version (line 207), DTB model (line 69), PCI vendor `[1e30:1684]` (line 407) | `[OBSERVED]` |
| RAM | 16,055,040 KB (~15.3 GiB) LPDDR4x | BL2: `LPDDR4x(rank: 2 + 2, freq: 4000M)` (line 46); Linux: `16055040K` (line 234) | `[OBSERVED]` |
| Storage | 58.3 GiB eMMC (Y0S064) | `mmcblk0: mmc0:0001 Y0S064 58.3 GiB` (line 589) | `[OBSERVED]` |
| Serial Console | 16550A UART @ `0x50118000` | Lines 209, 453 | `[OBSERVED]` |

---

## 2. Complete Boot Chain (Verified)

```text
Power On
   ↓
[EL3] Boot ROM (BL1 v2.5, "bm1686_rom_v6", Jan 24 2022) — immutable internal ROM
   ↓ loads BL2 from SPI-NOR flash (FIP)
[EL3] BL2 (v2.7, May 8 2025) — DDR training, board type detection, loads BL31 + U-Boot
   ↓ loads image id=3 (BL31) to 0x300000000
   ↓ loads image id=5 (U-Boot) to 0x308000000
[EL3] BL31 / TF-A (v2.7, May 8 2025) — GICv2 init, PSCI setup, CNTFRQ=50MHz, EL3→EL2 handoff
   ↓ SPSR=0x3c9 (EL2), entry 0x308000000
[EL2] U-Boot 2022.10 (May 8 2025, Sophon BM1684) — loads FIT from eMMC
   ↓ bootm with FIT image (emmcboot.itb) at 0x310000000
   ↓ loads kernel to 0x300280000, DTB to 0x33f1a3000, ramdisk to 0x33f1ad000
[EL2] Kernel/Vivanta Entry @ 0x300280000
   ↓
[EL2→EL1] OS (Linux drops to EL1; Vivanta can choose)
```

### Boot Stage Table

| Stage | Name | Version | Location | EL | Replaceable by Vivanta |
|---|---|---|---|---|---|
| 1 | Boot ROM (BL1) | v2.5 `bm1686_rom_v6` | Internal SoC ROM | EL3 | **NO** (immutable) |
| 2 | BL2 | v2.7 | SPI-NOR flash (FIP) | EL3 | **NO** (DDR training proprietary) |
| 3 | DDR init | `DDR A-firmware 2023` | BL2 payload (SPI-NOR) | EL3 | **NO** (vendor firmware) |
| 4 | BL31 / TF-A | v2.7 | SPI-NOR flash (FIP) | EL3 | **NO** (provides PSCI, CNTFRQ) |
| 5 | BL32 / TEE | `[NOT OBSERVED]` | `[UNKNOWN]` | `[UNKNOWN]` | `[UNKNOWN]` |
| 6 | BL33 / U-Boot | 2022.10 | eMMC (`mmc0:1`) | EL2 | Theoretically yes, no reason to |
| 7 | FIT Image | `emmcboot.itb` | eMMC (`mmc0:1`) | N/A | **YES — Vivanta replaces kernel subimage** |
| 8 | Kernel | 5.4.217-bm1684 | FIT subimage | EL2→EL1 | **YES — Vivanta replaces entirely** |

---

## 3. CPU Architecture & Execution Levels

| Parameter | Value | Evidence |
|---|---|---|
| Implementer | ARM (`0x41`) | Line 206: `0x410fd034` |
| Part | Cortex-A53 (`0xd0`) | Line 206 |
| Revision | r0p4 | Line 206: MIDR decoded |
| Core count | 8 | Line 284: `smp: Brought up 1 node, 8 CPUs` |
| Clusters | 2 × 4 cores | Affinity: 0x000-0x003 + 0x100-0x103 (lines 206, 271-283) |
| AArch64 | Yes | Line 161: `Architecture: AArch64` |
| AArch32 EL0 | Yes | Line 286: `32-bit EL0 Support` |
| CRC32 | Yes | Line 287 |
| Erratum 845719 | Present | Line 228 |
| PMU | armv8_cortex_a53, 7 counters | Line 382 |
| KVM | IPA 40 bits, Hyp mode initialized | Lines 383-384 |
| **Kernel entry EL** | **EL2** | Line 288: `CPU: All CPU(s) started at EL2` |
| Linux current EL | EL1 (drops from EL2) | `[INFERRED]` standard arm64 |

---

## 4. Physical Memory Map

### Total RAM: ~15.3 GiB (16,055,040 KB)

| Region | Base | Size | Purpose | Owner | Vivanta Usable? |
|---|---|---|---|---|---|
| General-purpose RAM | `0x000000000` | ~6 GiB | OS | Linux | **YES** |
| ion_npu_mem | `0x0124100000` | `0xf6e00000` (~3.95 GiB) | NPU/Tensor accelerator | Linux ION | NO |
| ion_vpu_mem | `0x380000000` | `0x80000000` (2 GiB) | VPU/Video accelerator | Linux ION | NO |
| ion_vpp_mem | `0x440000000` | `0xc0000000` (3 GiB) | VPP/Video Post-processor | Linux ION | NO |
| linux,cma | `0x438000000` | `0x8000000` (128 MiB) | CMA DMA pool | Linux CMA | Conditional |
| ramoops | `0x314000000` | `0x100000` (1 MiB) | Crash log / pstore | Linux pstore | Conditional |
| BL1 SRAM | `0x10002000-0x1000d000` | ~44 KiB | Boot ROM code | Immutable | NO |
| BL2 | `0x10020000-0x1003f824` | ~160 KiB | BL2 code | Secure firmware | NO |
| BL31 | `0x300000000` | ~40 KiB | TF-A code | Secure firmware | NO |

### PA width: ≥40 bits (`[OBSERVED]` line 383: KVM IPA limit)
### DMA width: 40 bits (`[OBSERVED]` line 475)

---

## 5. Interrupt Architecture

### GICv2 + bitmain,top-intc

```text
Peripheral (MSI-capable: PCIe, xHCI)
   ↓
bitmain,top-intc @ 0x500100bc (MSI parent, 16 IRQ slots)
   ↓ cascading into GIC
GICv2 (GICD → GICC)
   ↓
Cortex-A53 CPU
```

| Parameter | Value | Evidence |
|---|---|---|
| GIC version | GICv2 | Lines 63, 242 |
| EOI mode | Split EOI/Deactivate | Line 242 |
| NR_IRQS | 64 | Line 241 |
| GICD base | `[UNKNOWN]` | Not in boot log |
| GICC base | `[UNKNOWN]` | Not in boot log |
| top-intc MMIO | `0x500100bc` | Line 306 |
| top-intc MSI addr | `0x50010198` | Line 514 |
| top-intc IRQ count | 16 GIC IRQs | Line 324 |

### Key IRQs:

| Device | IRQ | Evidence |
|---|---|---|
| UART0 | 15 | Line 453 |
| UART1 | 16 | Line 458 |
| UART2 | 17 | Line 459 |
| VPP sys0/sys1 | 24/25 | Line 440 |
| USB xHCI | 155 (via top-intc, GIC IRQ 39) | Lines 513-514 |
| Ethernet | `[UNKNOWN]` | — |
| eMMC | `[UNKNOWN]` | — |
| arch_timer | PPI `[UNKNOWN]` | — |

**Vivanta implication:** GICv2 driver necessary. `top-intc` driver needed for PCIe/USB but NOT for minimal boot (UART/timer use direct GIC IRQs).

---

## 6. Timer

| Parameter | Value |
|---|---|
| Type | ARM Generic Timer (cp15) |
| Frequency | 50,000,000 Hz (50 MHz) |
| Physical timer | Yes |
| Virtual timer | `[UNKNOWN]` |
| Configured by | BL31/TF-A (CNTFRQ_EL0) |
| clocksource | arch_sys_counter (56-bit) |

**Vivanta implication:** Standard ARM Generic Timer backend, no vendor-specific code needed.

---

## 7. UART

| Parameter | Value |
|---|---|
| Controller | 16550A |
| MMIO base | `0x50118000` |
| Register width | 32-bit (MMIO32) |
| IRQ | 15 |
| Input clock | 31,250,000 Hz |
| Baud rate | 115200 |
| earlycon | Yes (`uart0`) |
| Vendor init | None needed |

**Vivanta implication:** Standard 16550A driver, no vendor-specific initialization.

---

## 8. Storage

| Media | Controller | MMIO | Boot role |
|---|---|---|---|
| SPI-NOR flash | (FIP) | — | BL2, BL31 firmware |
| eMMC | `bm-emmc` (SDHCI) | `0x50100000` | U-Boot env, FIT image, rootfs |
| SD card | `bm-sd` (SDHCI) | `0x50101000` | Not used (no card) |

### eMMC partitions:

| Partition | Size | Purpose |
|---|---|---|
| p1 | `[UNKNOWN]` | Boot (boot.scr.emmc, emmcboot.itb) |
| p2 | `[UNKNOWN]` | `[UNKNOWN]` |
| p3 | `[UNKNOWN]` | ext4 (corrupt superblock) |
| p4 | `[UNKNOWN]` | ext4 (mounted) |
| p5 | ~16 GiB | ext4 (root) |
| p6 | ~2 GiB | ext4 (/opt or /data) |
| p7 | ~34.5 GiB | ext4 (/data) |
| boot0 | 4 MiB | eMMC boot partition 0 |
| boot1 | 4 MiB | eMMC boot partition 1 |
| rpmb | 16 MiB | Replay Protected Memory Block |

---

## 9. PCIe / USB

| Controller | MMIO | Device | Evidence |
|---|---|---|---|
| PCIe root | `0x5fb80000` | `bm168x_pcie` | Lines 398-402 |
| PCI bridge | `[1e30:1684]` | SoC internal | Line 407 |
| USB xHCI | `0x81000000` (via PCIe) | ASMedia ASM2142 `[1b21:2142]` | Lines 410, 512 |

---

## 10. Network

| Controller | MMIO | Driver | PHY | Evidence |
|---|---|---|---|---|
| Ethernet0 | `0x50108000` | `bm-dwmac` (DWMAC4/5) | RTL8211F | Lines 463-477 |
| Ethernet1 | `0x5010c000` | `bm-dwmac` (DWMAC4/5) | RTL8211F | Lines 478-492 |

DMA width: 40 bits (line 475).

---

## 11. Accelerator (NPU/VPU/VPP)

| Engine | Memory base | Memory size | IRQ | Evidence |
|---|---|---|---|---|
| NPU | `0x124100000` | ~3.95 GiB | `[UNKNOWN]` | Lines 215-216 |
| VPU | `0x380000000` | 2 GiB | `[UNKNOWN]` | Lines 217-218 |
| VPP | `0x440000000` + `0x16860000` (regs) | 3 GiB | 24, 25 | Lines 219-220, 434-440 |
| MCU | I2C 1-0017 (addr 0x17) | — | — | Line 447 |
| Watchdog | I2C 1-0069 (addr 0x69) | — | — | Line 448 |

TPU clock: 950 MHz init rate, max 2.3 GHz (lines 551, 609).

---

## 12. Security Architecture

| Element | Status | Evidence |
|---|---|---|
| Boot ROM (BL1) | Immutable | Line 5 |
| BL2 from SPI-NOR | Vendor-controlled | Lines 9, 51 |
| BL31 from SPI-NOR | Vendor-controlled | Lines 52, 61 |
| Secure firewall | DISABLED in BL31 | Line 64: `ERROR: disable secure firewall` |
| TEE / OP-TEE | `[NOT OBSERVED]` | No TEE messages in boot log |
| Secure boot | `[UNKNOWN]` — SHA1 in FIT, but no signature verification of FIT itself observed |
| RPMB | Present (16 MiB on eMMC) | Line 595 |

---

## 13. U-Boot Capabilities

| Feature | Value | Evidence |
|---|---|---|
| Version | 2022.10 (May 8 2025) | Line 73 |
| Architecture | AArch64 | Line 161 |
| FIT support | Yes (SHA1 verification) | Lines 152-196 |
| Boot script | `/boot.scr.emmc` (1294 bytes) | Lines 143-148 |
| Autoboot delay | 2 seconds | Line 119 |
| Console | `serial@50118000` | Lines 104-106 |
| MMC | `sdhc@50100000: 0, sdhc@50101000: 1` | Line 80 |
| Network | RTL8211F on eth0/eth1 | Lines 112-117 |
| Environment | `uboot.env` on FAT mmc0:1 (failed to read → defaults) | Line 102 |
| Watchdog | `bm16xxwdt@69`, 60s timeout | Line 79 |

**Can launch Vivanta:** YES — via FIT subimage replacement or custom boot script.

---

## 14. Kernel Handoff Summary

| Parameter | Value | Classification |
|---|---|---|
| Kernel load address | `0x300280000` | `[OBSERVED]` (U-Boot FIT) |
| Kernel entry address | `0x300280000` | `[OBSERVED]` (U-Boot FIT) |
| DTB address | `0x33f1a3000` | `[OBSERVED]` (U-Boot load) |
| x0 at entry | DTB address | `[INFERRED]` (arm64 boot protocol) |
| EL at entry | EL2 | `[OBSERVED]` (Linux log) |
| MMU at entry | OFF | `[INFERRED]` (arm64 boot protocol) |
| CNTFRQ_EL0 | 50,000,000 | `[OBSERVED]` (timer) |
| PSCI | v1.1, SMC conduit, v0.2 function IDs | `[OBSERVED]` |
| PA width | ≥40 bits | `[OBSERVED]` (KVM/DMA) |
| DMA width | 40 bits | `[OBSERVED]` |
| Ramdisk | `0x33f1ad000` (optional) | `[OBSERVED]` |
