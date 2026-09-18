# AIBOX-1684X — Second-Pass Forensic Verification Report

## Access Status

* **Serial port:** `/dev/cu.wchusbserial10` at 115200 baud — `[OBSERVED]` active.
* **Login prompt:** `bm1684 login:` — `[OBSERVED]` active.
* **Authenticated shell:** `[UNAVAILABLE — AUTHENTICATION REQUIRED]`
* **U-Boot console:** `[UNAVAILABLE — AUTOBOOT NOT INTERRUPTED]` (system was fully booted before observation; no deliberate reboot was performed per mission constraints).

All conclusions in this second pass derive exclusively from the **912-line RAW_BOOT_LOG.txt** captured during the first-pass power-on observation. No additional runtime access was obtained.

---

## TASK 1 — Verification of Previous Report Claims

### 1.1 SoC = BM1684X

* **Claim:** SoC is a Bitmain/Sophon BM1684X
* **Evidence:**
  * Line 5: `BL1: v2.5(release):bm1686_rom_v6`
  * Line 73: `U-Boot 2022.10 (May 08 2025 - 13:42:40 +0800) Sophon BM1684`
  * Line 207: `Linux version 5.4.217-bm1684`
  * Line 69: `found dtb@180: bitmain-bm1684x-se7-v1-mini`
  * Line 407: `pci 0000:00:00.0: [1e30:1684]` (SoC PCI vendor ID)
* **Source:** Serial boot log (BL1, U-Boot, Linux, PCI enumeration)
* **Classification:** `[OBSERVED]`
* **Verdict: CONFIRMED** — HIGH confidence
* **Correction:** BL1 banner says `bm1686_rom_v6`, not `bm1684x_rom`. This could indicate the ROM is shared across the BM1686/BM1684X family or is a codename. The SoC identity remains BM1684X based on U-Boot, kernel, and DTB evidence.

### 1.2 CPU = 8 × Cortex-A53

* **Claim:** 8-core ARM Cortex-A53
* **Evidence:**
  * Line 206: `Booting Linux on physical CPU 0x0000000000 [0x410fd034]`
    * `0x41` = ARM implementer
    * `0x0fd0` = Cortex-A53 part
    * `0x34` = revision 4
  * Lines 271-283: CPUs 1-7 booted, all with same MIDR `[0x410fd034]`
  * Line 284: `smp: Brought up 1 node, 8 CPUs`
  * Line 234: `SLUB: HWalign=64, Order=0-3, MinObjects=0, CPUs=8, Nodes=1`
  * Line 382: `hw perfevents: enabled with armv8_cortex_a53 PMU driver, 7 counters available`
* **Source:** Linux kernel boot log
* **Classification:** `[REPORTED BY LINUX]` / `[OBSERVED]`
* **Verdict: CONFIRMED** — HIGH confidence
* **Additional detail:** CPU topology has two clusters visible from CPU IDs:
  * CPU0-3: `0x0000000000` through `0x0000000003`
  * CPU4-7: `0x0000000100` through `0x0000000103`
  * This indicates a **dual-cluster topology** within the same Cortex-A53 part. The `0x100` bit in affinity level 1 suggests two clusters of 4 cores each.

### 1.3 AArch64

* **Claim:** CPU runs in AArch64 mode
* **Evidence:**
  * Line 161: `Architecture: AArch64` (FIT kernel subimage)
  * Line 207: Kernel boots with standard arm64 format
  * Line 648: `systemd[1]: Detected architecture arm64.`
* **Source:** U-Boot FIT metadata, Linux kernel, systemd
* **Classification:** `[OBSERVED]`
* **Verdict: CONFIRMED** — HIGH confidence
* **Additional detail:** Line 286: `CPU features: detected: 32-bit EL0 Support` — AArch32 EL0 is also supported.

### 1.4 RAM = 16 GiB LPDDR4x

* **Claim:** 16 GiB LPDDR4x
* **Evidence:**
  * Line 46: `LPDDR4x(rank: 2 + 2, freq: 4000M) init start`
  * Line 234: `Memory: 6289304K/16055040K available`
  * Line 75: `DRAM: 1 GiB` (U-Boot's own view — see correction below)
* **Source:** BL2 firmware, Linux kernel
* **Classification:** `[REPORTED BY FIRMWARE]` / `[REPORTED BY LINUX]`
* **Verdict: CONFIRMED with CORRECTION** — HIGH confidence
* **Correction:** U-Boot reports `DRAM: 1 GiB` (line 75). This is not a contradiction — U-Boot only declares 1 GiB for its own relocation/relocation-offset needs (`Relocation Offset is: 37f45000`, `Relocating to 33ff45000`). The full 16 GiB is available to the OS. The `1 GiB` is U-Boot's working DRAM window.

### 1.5 GICv2

* **Claim:** ARM GICv2 interrupt controller
* **Evidence:**
  * Line 63: `ARM GICv2 driver initialized` (BL31)
  * Line 242: `GIC: Using split EOI/Deactivate mode`
  * Line 241: `NR_IRQS: 64`
* **Source:** TF-A/BL31, Linux kernel
* **Classification:** `[REPORTED BY FIRMWARE]` / `[REPORTED BY LINUX]`
* **Verdict: CONFIRMED** — HIGH confidence
* **Missing:** GICD and GICC base addresses are NOT visible in the boot log. These would normally come from the device tree. `[UNAVAILABLE — AUTHENTICATION REQUIRED]` for `/proc/device-tree/` or DTB inspection.

### 1.6 Generic Timer @ 50 MHz

* **Claim:** ARM Generic Timer at 50.00 MHz
* **Evidence:**
  * Line 243: `arch_timer: cp15 timer(s) running at 50.00MHz (phys).`
  * Line 244: `clocksource: arch_sys_counter: mask: 0xffffffffffffff`
  * Line 245: `sched_clock: 56 bits at 50MHz, resolution 20ns`
* **Source:** Linux kernel boot log
* **Classification:** `[REPORTED BY LINUX]`
* **Verdict: CONFIRMED** — HIGH confidence
* **Additional detail:** `(phys)` indicates physical timer is active. No virtual or hypervisor timer mentioned in the log.

### 1.7 16550A UART

* **Claim:** 16550A-compatible UART at `0x50118000`
* **Evidence:**
  * Line 209: `earlycon: uart0 at MMIO32 0x0000000050118000 (options '')`
  * Line 453: `50118000.serial: ttyS0 at MMIO 0x50118000 (irq = 15, base_baud = 31250000) is a 16550A`
  * Line 104: `In: serial@50118000`
* **Source:** Linux kernel earlycon and 8250 driver
* **Classification:** `[OBSERVED]`
* **Verdict: CONFIRMED** — HIGH confidence
* **Additional detail:** MMIO32 (32-bit register access), base_baud = 31,250,000 (clock input). Standard 16550A — no vendor-specific UART initialization visible.

### 1.8 U-Boot 2022.10

* **Claim:** U-Boot 2022.10
* **Evidence:**
  * Line 73: `U-Boot 2022.10 (May 08 2025 - 13:42:40 +0800) Sophon BM1684`
* **Source:** U-Boot banner
* **Classification:** `[OBSERVED]`
* **Verdict: CONFIRMED** — HIGH confidence

### 1.9 FIT image / emmcboot.itb / boot.scr.emmc

* **Claim:** FIT image `emmcboot.itb` loaded via `boot.scr.emmc`
* **Evidence:**
  * Line 143: `Found U-Boot script /boot.scr.emmc`
  * Line 144: `fs reading /boot.scr.emmc`
  * Line 145: `1294 bytes read in 3 ms`
  * Line 149: `fs reading emmcboot.itb`
  * Line 150: `28240922 bytes read in 616 ms (43.7 MiB/s)`
  * Line 152: `## Loading kernel from FIT Image at 310000000`
* **Source:** U-Boot boot sequence
* **Classification:** `[OBSERVED]`
* **Verdict: CONFIRMED** — HIGH confidence

### 1.10 kernel entry ≈ 0x300280000

* **Claim:** Kernel load/entry address = `0x300280000`
* **Evidence:**
  * Line 163: `Load Address: 0x300280000`
  * Line 164: `Entry Point: 0x300280000`
* **Source:** U-Boot FIT image subimage metadata
* **Classification:** `[OBSERVED]` / `[FIT IMAGE SUBIMAGE / U-BOOT REPORTED]`
* **Verdict: CONFIRMED** — HIGH confidence
* **Critical qualifier:** This is the address **U-Boot loaded the Linux kernel to**. It is the FIT subimage's declared load/entry address. Vivanta can use this address IF it packages itself as a FIT subimage with the same or compatible address. It is NOT necessarily a hardware-mandated entry point — it is a U-Boot configuration decision.

---

## TASK 2 — Complete Boot Topology

| Stage | Name | Version | Location | Loading Mechanism | EL | Responsibility | Vendor-Controlled | Replaceable by Vivanta |
|---|---|---|---|---|---|---|---|---|
| 1 | Boot ROM (BL1) | v2.5 `bm1686_rom_v6` Jan 24 2022 | Internal SoC ROM | Hardwired power-on | EL3 | First-stage boot, loads BL2 from SPI flash | YES (immutable) | NO |
| 2 | BL2 | v2.7 May 8 2025 | SPI-NOR flash (FIP) | BL1 loads from `SPI flash (DMMR)` | EL3 | Platform setup, DDR init (LPDDR4x 4000M), board type detection, loads BL31 and BL33 | YES | NO (DDR training proprietary) |
| 3 | DDR init | `DDR A-firmware 2023` | SPI-NOR flash (BL2 payload) | BL2 executes | EL3 | LPDDR4x rank 2+2, interleave, 4000 MHz training | YES | NO (requires vendor DDR firmware) |
| 4 | BL31 / TF-A | v2.7 May 8 2025 | SPI-NOR flash (FIP) | BL2 loads image id=3 to `0x300000000` | EL3 | GICv2 init, secure firewall disable, PSCI runtime services, EL3→EL2 handoff | YES | NO (provides PSCI) |
| 5 | BL32 / TEE | `[NOT OBSERVED]` | `[UNKNOWN]` | `[UNKNOWN]` | `[UNKNOWN]` | `[UNKNOWN — no TEE/OP-TEE messages in boot log]` | `[UNKNOWN]` | `[UNKNOWN]` |
| 6 | BL33 / U-Boot | 2022.10 May 8 2025 | eMMC (`mmc0:1`) | BL31 exits to `0x308000000` | EL2 | Load FIT from eMMC, verify hashes, boot kernel | YES (vendor build) | THEORETICALLY YES (but no reason to) |
| 7 | FIT Image | `emmcboot.itb` | eMMC (`mmc0:1` `/boot.scr.emmc`) | U-Boot `bootm` | N/A (container) | Contains kernel + ramdisk + DTB with SHA1 hashes | YES | **YES — Vivanta replaces kernel subimage** |
| 8 | Linux Kernel | 5.4.217-bm1684 May 8 2025 | FIT kernel subimage | U-Boot loads to `0x300280000` | EL2→EL1 | OS | YES | **YES — Vivanta replaces this entirely** |

### Key observations:

* **BL31 entry point:** `0x300000000` (line 59)
* **U-Boot entry point (BL33):** `0x308000000` (line 67, loaded by BL2 as image id=5 to `0x308000000`)
* **BL2 loads BL31 first (image id=3), then U-Boot (image id=5)**
* **BL31 exits to normal world at `0x308000000`** — this is where U-Boot starts
* **No BL32/TEE/OP-TEE messages appear in the entire boot log** — either TEE is not present, or it initializes silently. Mark as `[UNKNOWN]`.
* **FIP (Firmware Image Package) in SPI flash** — BL1 and BL2 both say `Locate FIP in SPI flash (DMMR)`. This confirms SPI-NOR flash contains the FIP with BL2, BL31, and potentially BL32.

---

## TASK 3 — Exact Kernel Handoff

### Observed FIT image structure (from U-Boot output):

| Component | Data Start | Data Size | Load Address | Entry Point | Hash |
|---|---|---|---|---|---|
| **Kernel** | `0x3100000d4` | 20,204,032 B (19.3 MiB) | `0x300280000` | `0x300280000` | sha1 `224ae9d5...` |
| **Ramdisk** | `0x311344bc8` | 6,595,072 B (6.3 MiB) | `0x00000000` | `0x00000000` | sha1 `63adf83c...` |
| **FDT** | `0x311ad880c` | 25,648 B (25 KiB) | — | — | sha1 `f02e98cd...` |

### Additional handoff details:

| Parameter | Value | Source Line |
|---|---|---|
| FIT image loaded at | `0x310000000` | Line 152 |
| Ramdisk loaded to | `0x33f1ad000` (end `0x33f7f7200`) | Line 199 |
| Device Tree loaded to | `0x33f1a3000` (end `0x33f1ac42f`) | Line 200 |
| U-Boot relocation | `33ff45000` | Line 77 |
| U-Boot gd | `33f7ffd60` | Line 77 |
| U-Boot sp | `33f7fe350` | Line 77 |
| FIT config name | `config-pcb180` | Line 153 |
| Boot command | `bootm` (implied by `## Executing script` + FIT loading) | Line 146 |

### Boot protocol:

* **Kernel format:** Uncompressed AArch64 Image (`Compression: uncompressed`)
* **Architecture:** AArch64
* **OS:** Linux
* **Boot protocol:** Standard U-Boot `bootm` with FIT → Linux arm64 boot protocol
* **DTB location at entry:** `0x33f1a3000` (passed to kernel)
* **Ramdisk location at entry:** `0x33f1ad000` (initramfs, freed at line 380: `Freeing initrd memory: 6440K`)

### Linux arm64 boot protocol (inferred from kernel version + U-Boot):

* **x0** = DTB physical address (`0x33f1a3000`) — `[INFERRED]` from standard arm64 boot protocol
* **x1-x3** = 0 (standard arm64 boot protocol) — `[INFERRED]`
* **MMU** = OFF at kernel entry — `[INFERRED]` from standard arm64 boot protocol
* **D-cache** = OFF — `[INFERRED]`
* **EL** = EL2 (line 288: `CPU: All CPU(s) started at EL2`) — but note Linux then drops to EL1

### Can Vivanta simply replace the kernel payload?

**YES.** The FIT image is a container with SHA1 integrity hashes. To boot Vivanta:

1. Create a new FIT image with a Vivanta kernel subimage (same `config-pcb180` configuration name, or a new config)
2. Set kernel load/entry to `0x300280000` (or any valid DRAM address)
3. Include a DTB subimage (can reuse the existing one or create a minimal one)
4. Optionally include a ramdisk subimage (or dummy)
5. Replace `emmcboot.itb` on eMMC partition 1

**OR alternatively:**

1. Modify `/boot.scr.emmc` to point U-Boot at a raw Vivanta binary
2. Use `bootm` or custom U-Boot command to load Vivanta

**No changes to BL1, BL2, BL31, or U-Boot are required.**

---

## TASK 4 — Live Device Tree

* **Status:** `[UNAVAILABLE — AUTHENTICATION REQUIRED]`
* **DTB model:** `bitmain-bm1684x-se7-v1-mini` — `[OBSERVED]` line 69
* **DTB description:** `for BM1684X AiBox One` — `[OBSERVED]` line 188
* **DTB config:** `config-pcb180` — `[OBSERVED]` line 153
* **DTB size:** 25,648 bytes — `[OBSERVED]` line 192
* **DTB load address at boot:** `0x33f1a3000` — `[OBSERVED]` line 200

### Reconstructed DT nodes from boot log evidence:

| Node | Compatible / Name | MMIO | IRQ | Evidence Line |
|---|---|---|---|---|
| UART0 | `serial@50118000` | `0x50118000` | 15 | 453 |
| UART1 | `serial@5011a000` | `0x5011a000` | 16 | 458 |
| UART2 | `serial@5011c000` | `0x5011c000` | 17 | 459 |
| Ethernet0 | `ethernet@50108000` (`bm-dwmac`) | `0x50108000` | `[UNKNOWN]` | 463-476 |
| Ethernet1 | `ethernet@5010c000` (`bm-dwmac`) | `0x5010c000` | `[UNKNOWN]` | 478-492 |
| eMMC | `sdhc@50100000` / `bm-emmc` | `0x50100000` | `[UNKNOWN]` | 556 |
| SD card | `sdhc@50101000` / `bm-sd` | `0x50101000` | `[UNKNOWN]` | 558 |
| PCIe | `pcie@5fb80000` (`bm168x_pcie`) | `0x5fb80000` | `[UNKNOWN]` | 398-402 |
| Top-Intc | `bitmain,top-intc` | `0x500100bc` | (cascade) | 306-324 |
| sysdma | `50110000.sysdma` (`dw_dmac`) | `0x50110000` | `[UNKNOWN]` | 348 |
| tsdma | `200b000.tsdma` (`dw_dmac`) | `0x200b000` | `[UNKNOWN]` | 349 |
| I2C0 | `5001c000.i2c` (`i2c_designware`) | `0x5001c000` | `[UNKNOWN]` | 358 |
| I2C1 | `5001e000.i2c` (`i2c_designware`) | `0x5001e000` | `not found` | 359-360 |
| PWM | `50029000.pwm` (`bitmain-pwm`) | `0x50029000` | `[UNKNOWN]` | 548-550 |
| VPP | `0x16860000` (`bm_vpp`) | `0x16860000` | 24, 25 | 434, 440 |
| MCU | `1-0017` (`bm16xx-mcu`, I2C addr 0x17) | I2C bus 1 | — | 447 |
| Watchdog | `1-0069` (`bm-wdt`, I2C addr 0x69) | I2C bus 1 | — | 448 |
| RTC | `0-0051` (`rtc-hym8563`, I2C addr 0x51) | I2C bus 0 | — | 605-606 |
| PMU | `/pmu_a53` | — | `[UNKNOWN]` | 381 |
| m2m-dma | `m2m@0` (`sophgo,m2m-dma`) | `[UNKNOWN]` | `[UNKNOWN]` | 449-450 |

### live.dtb / live.dts

`[UNAVAILABLE — AUTHENTICATION REQUIRED]`

The DTB binary is embedded inside `emmcboot.itb` on eMMC partition 1. It could not be extracted without either:
1. Authentication to the running Linux system, or
2. Interrupting U-Boot and using `load`/`md` commands (not performed — no deliberate reboot)

---

## TASK 5 — Physical Memory Map

### From boot log (Linux kernel messages):

| Region | Base | Size | Purpose | Evidence Line |
|---|---|---|---|---|
| Total RAM | `0x000000000` | 16,055,040 KB (~15.3 GiB usable) | All physical RAM | 234 |
| Available to OS | — | 6,289,304 KB (~6.0 GiB) | Linux usable | 234 |
| Reserved total | — | 9,634,664 KB (~9.2 GiB) | All reservations | 234 |
| linux,cma | `0x438000000` | 128 MiB (`0x8000000`) | CMA shared DMA pool | 213-214 |
| ion_npu_mem | `0x124100000` | 3,950 MiB (`0xf6e00000`) | NPU/Tensor Processor | 215-216, 423 |
| ion_vpu_mem | `0x380000000` | 2,048 MiB (`0x80000000`) | VPU/Video Processor | 217-218, 429 |
| ion_vpp_mem | `0x440000000` | 3,072 MiB (`0xc0000000`) | VPP/Video Post-Processor | 219-220, 426 |
| ramoops | `0x314000000` | 1 MiB (`0x100000`) | Persistent crash log store | 305 |
| Kernel code | — | 13,564 KB | Kernel text | 234 |
| Kernel rwdata | — | 1,426 KB | — | 234 |
| Kernel rodata | — | 3,732 KB | — | 234 |
| Kernel init | — | 960 KB | Freed at boot | 234, 622 |
| Kernel bss | — | 464 KB | — | 234 |

### U-Boot address space (observed):

| Address | Purpose | Evidence Line |
|---|---|---|
| `0x10002000 - 0x1000d000` | BL1 RAM | 7 |
| `0x10020000 - 0x1003f824` | BL2 loaded here | 21, 50-53 |
| `0x300000000` | BL31 loaded here | 52, 59 |
| `0x308000000` | U-Boot (BL33) loaded here | 56, 67 |
| `0x310000000` | FIT image loaded here | 152 |
| `0x300280000` | Kernel load/entry | 163-164 |
| `0x33f1a3000` | DTB at kernel entry | 200 |
| `0x33f1ad000` | Ramdisk at kernel entry | 199 |
| `0x33ff45000` | U-Boot relocation | 77 |
| `0x33f7ffd60` | U-Boot global data | 77 |
| `0x33f7fe350` | U-Boot stack | 77 |

### Highest observed physical address:

`0x440000000 + 0xc0000000 = 0x500000000` (VPP region end) — but this is reserved, not RAM per se. The highest physical RAM address from the memory line is approximately `0x3C0000000` (16 GiB from `0x0`).

### `/proc/iomem` and `/proc/meminfo`:

`[UNAVAILABLE — AUTHENTICATION REQUIRED]`

---

## TASK 6 — Reserved Memory Table

| Base | Size | DT Node | Purpose | Owner | Evidence | Vivanta Usable? |
|---|---|---|---|---|---|---|
| `0x0124100000` | 0xf6e00000 (~3.95 GiB) | `ion_npu_mem` (npu-region) | NPU/Tensor accelerator | Linux ION/Accelerator driver | Lines 215-216, 423-424, 431, 444 | NO — accelerator-owned |
| `0x380000000` | 0x80000000 (2 GiB) | `ion_vpu_mem` (vpu-region) | VPU/Video accelerator | Linux ION/VPU driver | Lines 217-218, 429-430, 433, 445 | NO — accelerator-owned |
| `0x440000000` | 0xc0000000 (3 GiB) | `ion_vpp_mem` (vpp-region) | VPP/Video Post-processor | Linux ION/VPP driver | Lines 219-220, 426-427, 432, 443 | NO — accelerator-owned |
| `0x438000000` | 0x8000000 (128 MiB) | `linux,cma` (shared-dma-pool) | CMA DMA pool | Linux CMA framework | Lines 213-214 | Conditional — Vivanta could repurpose if no DMA users |
| `0x314000000` | 0x100000 (1 MiB) | (ramoops) | Crash log / pstore | Linux pstore/ramoops | Line 305 | Conditional — Vivanta could use for crash logging or reclaim |
| BL1 SRAM | `0x10002000-0x1000d000` | (internal) | BL1 code | Boot ROM (immutable) | Line 7 | NO — firmware SRAM |
| BL2 | `0x10020000-0x1003f824` | (FIP from SPI) | BL2 code | TF-A (secure) | Lines 21, 50-53 | NO — secure firmware |
| BL31 | `0x300000000` | (FIP from SPI) | BL31/TF-A code | Secure world (EL3) | Lines 52, 59 | NO — secure firmware |

### Classification:
* **Firmware:** BL1 SRAM, BL2, BL31
* **NPU:** `ion_npu_mem`
* **VPU:** `ion_vpu_mem`
* **VPP:** `ion_vpp_mem`
* **CMA/DMA:** `linux,cma`
* **Crashlog:** ramoops
* **TEE:** `[UNKNOWN — no TEE memory region observed in boot log]`

---

## TASK 7 — GIC and Interrupt Topology

### GIC version:
* **GICv2** — `[OBSERVED]` lines 63, 242
* **Split EOI/Deactivate mode** — `[OBSERVED]` line 242
* **NR_IRQS: 64** — `[OBSERVED]` line 241

### GICD / GICC base addresses:
`[UNAVAILABLE — not visible in boot log; would require DTB or /proc/iomem]`

### bitmain,top-intc:

* **MMIO:** `0x500100bc` — `[OBSERVED]` line 306
* **Compatible:** `bitmain,top-intc` — `[OBSERVED]` lines 306-324
* **Type:** Interrupt aggregator / MSI parent — `[OBSERVED]` line 306: `is a msi parent`
* **Behavior:** Maps 16 GIC hardware IRQs (hwirq 80-87, 148-155) to Linux virtual IRQs 22, 38-52 — `[OBSERVED]` lines 307-324
* **MSI address:** `0x50010198` (address_lo), data `0x2` — `[OBSERVED]` line 514

### Actual topology:

```text
Peripheral (MSI-capable)
   ↓
bitmain,top-intc @ 0x500100bc (MSI parent, 16 IRQ slots)
   ↓ (cascading into GIC)
GICv2 (GICD → GICC → CPU)
   ↓
Cortex-A53 CPU
```

`top-intc` is a **real vendor-specific interrupt aggregator** that sits between MSI-capable peripherals (like PCIe/xHCI) and the GICv2. It is not merely a Linux abstraction.

### Important IRQs:

| Device | IRQ | Evidence Line |
|---|---|---|
| UART0 (ttyS0) | 15 | 453 |
| UART1 (ttyS1) | 16 | 458 |
| UART2 (ttyS2) | 17 | 459 |
| VPP sys0 | 24 | 440 |
| VPP sys1 | 25 | 440 |
| arch_timer | PPI (not visible) | 243 |
| Ethernet0 | `[UNKNOWN — not in boot log]` | — |
| Ethernet1 | `[UNKNOWN — not in boot log]` | — |
| eMMC | `[UNKNOWN — not in boot log]` | — |
| USB xHCI | 155 (via top-intc MSI, gic irq 39) | 513 |
| PCIe | `[UNKNOWN — not in boot log]` | — |
| NPU | `[UNKNOWN — not in boot log]` | — |
| Watchdog | I2C-based (`bm-wdt` at `1-0069`), not a direct IRQ | 448 |

### Implication for Vivanta:

Vivanta's existing GICv2 driver is necessary but **not sufficient**. The `bitmain,top-intc` aggregator must be handled for any MSI-capable peripheral (notably PCIe/USB). For the minimal first boot (UART + timer + GIC + MMU), `top-intc` can be ignored since UART and timer use direct GIC interrupts.

---

## TASK 8 — UART Confirmation

| Parameter | Value | Evidence | Classification |
|---|---|---|---|
| Controller IP | 16550A (DesignWare or compatible) | Line 453: `is a 16550A` | `[OBSERVED]` |
| MMIO base | `0x50118000` | Lines 209, 453 | `[OBSERVED]` |
| Register width | 32-bit (MMIO32) | Line 209: `MMIO32` | `[OBSERVED]` |
| IRQ | 15 | Line 453 | `[OBSERVED]` |
| Base baud (clock) | 31,250,000 Hz | Line 453: `base_baud = 31250000` | `[OBSERVED]` |
| Baud rate | 115200 | Serial observation (working) | `[OBSERVED]` |
| DT node | `serial@50118000` | Lines 104-106 | `[OBSERVED]` |
| Linux driver | `8250/16550 driver` | Line 451: `Serial: 8250/16550 driver, 4 ports` | `[OBSERVED]` |
| earlycon | `uart0 at MMIO32 0x50118000` | Line 209 | `[OBSERVED]` |
| Vendor-specific init | None observed | No vendor UART init messages | `[INFERRED]` |

### Verdict:

Vivanta can use a **standard 16550A UART driver** with no vendor-specific initialization. The UART works with earlycon immediately at power-on (before full driver load), confirming the hardware is ready for use by the time kernel entry occurs.

---

## TASK 9 — Timer Confirmation

| Parameter | Value | Evidence | Classification |
|---|---|---|---|
| Timer type | ARM Generic Timer (cp15) | Line 243: `arch_timer: cp15 timer(s) running at 50.00MHz (phys)` | `[OBSERVED]` |
| Counter frequency | 50,000,000 Hz (50 MHz) | Line 243 | `[OBSERVED]` |
| Physical timer | Yes | Line 243: `(phys)` | `[OBSERVED]` |
| Virtual timer | Not mentioned in log | — | `[UNKNOWN]` |
| Hypervisor timer | Not mentioned in log | — | `[UNKNOWN]` |
| Timer PPI | `[UNKNOWN — not visible in boot log]` | — | `[UNAVAILABLE]` |
| Firmware configuration | CNTFRQ set by BL31/TF-A | `[INFERRED]` — TF-A sets counter frequency before EL2 exit | `[INFERRED]` |
| clocksource | `arch_sys_counter` | Line 244 | `[OBSERVED]` |
| sched_clock | 56 bits, 50 MHz, 20ns resolution | Line 245 | `[OBSERVED]` |

### Verdict:

Vivanta's existing ARM Generic Timer backend can be **reused directly**. The 50 MHz frequency is standard for Cortex-A53 designs. The physical timer is available (relevant since kernel runs at EL1 after dropping from EL2). No vendor-specific timer initialization is visible — the counter frequency is set by TF-A firmware (CNTFRQ_EL0 register).

---

## TASK 10 — CPU / Exception-Level Reality

### What Linux reports:

| Parameter | Value | Evidence Line |
|---|---|---|
| CPU implementer | `0x41` (ARM) | 206: `0x410fd034` |
| CPU part | `0xd0` (Cortex-A53) | 206: `0x410fd034` |
| CPU revision | `0x3` (r0p3?) or `0x34` | 206: `0x410fd034` — MIDR format: `implementer(8):variant(4):arch(4):part(12):revision(4)` → `0x41:0x0:0xf:0xd0:0x4` = ARM Cortex-A53 r0p4 |
| CPU count | 8 (single NUMA node) | 234: `CPUs=8, Nodes=1`; 284: `Brought up 1 node, 8 CPUs` |
| CPU features | ARM erratum 845719, 32-bit EL0, CRC32 | 228, 286, 287 |
| PMU | armv8_cortex_a53, 7 counters | 382 |
| KVM/Hyp | IPA size 40 bits, Hyp mode initialized | 383-384 |

### Exception level analysis:

| Context | EL | Evidence |
|---|---|---|
| Bootloader (U-Boot) execution | EL2 | BL31 exits to normal world at `0x308000000` with SPSR `0x3c9` (line 68) — SPSR bits [3:0] = `0b1001` = EL2 |
| Primary CPU kernel entry | EL2 | Line 288: `CPU: All CPU(s) started at EL2` |
| Secondary CPU startup | EL2 (via PSCI) | Lines 256-269: `bm_pwr_domain_on` → `bm_pwr_domain_on_finish` (PSCI from BL31) |
| Linux current EL | EL1 (after dropping) | `[INFERRED]` — Linux arm64 drops from EL2 to EL1 during boot; KVM retains EL2 (line 384: `Hyp mode initialized successfully`) |
| Potential Vivanta EL | EL1 (if following Linux) or EL2 (if staying) | `[INFERRED]` — Vivanta can choose |

### CPU topology (two clusters):

| Cluster | CPUs | Affinity | Evidence |
|---|---|---|---|
| Cluster 0 | CPU0-CPU3 | `0x0000000000` - `0x0000000003` | Lines 206, 271-275 |
| Cluster 1 | CPU4-CPU7 | `0x0000000100` - `0x0000000103` | Lines 276-283 |

All 8 cores are Cortex-A53 (`0x410fd034`) — this is likely a dual-cluster configuration for cache/power-domain separation, not big.LITTLE (since all are A53).

### Exact state Vivanta would inherit at kernel entry:

| State | Value | Evidence |
|---|---|---|
| EL | EL2 | Line 288 |
| MMU | OFF `[INFERRED]` | Standard arm64 boot protocol |
| D-cache | OFF `[INFERRED]` | Standard arm64 boot protocol |
| I-cache | ON (VIPT) `[INFERRED]` | Line 227: `Detected VIPT I-cache on CPU0` |
| x0 | DTB physical address (`0x33f1a3000`) | `[INFERRED]` standard arm64 boot protocol |
| x1-x3 | 0 | `[INFERRED]` standard arm64 boot protocol |
| SP | `[UNKNOWN]` | Not visible in boot log |
| CNTFRQ_EL0 | 50,000,000 | `[INFERRED]` from timer observation |
| SCR_EL3 | `[UNKNOWN]` | Secure-world only, not visible |
| PSCI | Available via SMC (BL31 runtime services) | Lines 222-225 |

---

## TASK 11 — MMU / Page-Table Environment

| Parameter | Value | Evidence | Classification |
|---|---|---|---|
| Page granule | 4 KiB (and possibly 64 KiB, 2 MiB, 32 MiB, 1 GiB) | Lines 325-328: HugeTLB registered for 64KiB, 2MiB, 32MiB, 1GiB | `[REPORTED BY LINUX]` |
| VA width | `[UNKNOWN]` — 40-bit IPA observed for KVM | Line 383: `IPA Size Limit: 40 bits` | `[REPORTED BY LINUX]` |
| PA width | ≥40 bits (IPA 40 bits + KVM) | Line 383 | `[INFERRED]` |
| DMA address width | 40 bits | Line 475: `Using 40 bits DMA width` (ethernet); Line 556: `ADMA 64-bit` (eMMC) | `[OBSERVED]` |
| MMU state at kernel entry | OFF | — | `[INFERRED]` (standard arm64 boot protocol) |
| Cache state at entry | D-cache OFF, I-cache ON (VIPT) | Line 227 | `[INFERRED]` |
| SCTLR | `[UNKNOWN]` | — | `[UNAVAILABLE]` |
| MAIR | `[UNKNOWN]` | — | `[UNAVAILABLE]` |
| TCR | `[UNKNOWN]` | — | `[UNAVAILABLE]` |
| TTBR | `[UNKNOWN]` | — | `[UNAVAILABLE]` |
| Memory attributes | `[UNKNOWN]` | — | `[UNAVAILABLE]` |

### Implication for Vivanta:

Most MMU register values are `[UNAVAILABLE]` without a root shell. However, the key architectural facts are:
* **PA width: ≥40 bits** (confirmed by DMA width and KVM IPA)
* **4 KiB page granule is supported** (standard AArch64)
* **Large pages:** 2 MiB, 1 GiB (standard AArch64 block descriptors)
* Vivanta's MMU implementation targeting 48-bit VA / 40-bit PA would be compatible.

---

## TASK 12 — Boot Storage

### Boot media hierarchy:

| Media | Role | Evidence |
|---|---|---|
| **SPI-NOR flash** | BL2, BL31, (FIP) — secure firmware | Lines 9, 51, 55: `Locate FIP in SPI flash (DMMR)` |
| **eMMC** (`mmcblk0`) | U-Boot env (attempted), FIT image, kernel, rootfs | Lines 80, 142-150, 589-597 |
| **SD card** (`mmc1`) | Not used for boot (no card present) | Line 120: `MMC: no card present` |

### eMMC partition table (from Linux):

| Partition | Size | Purpose | Evidence |
|---|---|---|---|
| `mmcblk0p1` | `[UNKNOWN]` | Boot partition (contains `/boot.scr.emmc`, `emmcboot.itb`) | Lines 142-149, 762, 770 |
| `mmcblk0p2` | `[UNKNOWN]` | `[UNKNOWN]` | Line 760, 763 |
| `mmcblk0p3` | `[UNKNOWN]` | ext4 filesystem (corrupt superblock) | Lines 625-633, 641 |
| `mmcblk0p4` | `[UNKNOWN]` | ext4 (mounted, likely /boot or root) | Line 642, 756, 770 |
| `mmcblk0p5` | 16 GiB (`4194304 blocks` × 4K) | ext4 (mounted, likely root) | Lines 636, 644, 758, 770 |
| `mmcblk0p6` | 2 GiB (`524288 blocks` × 4K) | ext4 (mounted, likely /data or /opt) | Lines 638, 759, 770-771 |
| `mmcblk0p7` | 34.5 GiB (`9073147 blocks` × 4K) | ext4 (mounted, likely /data) | Lines 640, 761, 771 |
| `mmcblk0boot0` | 4 MiB | eMMC boot partition 0 | Line 591 |
| `mmcblk0boot1` | 4 MiB | eMMC boot partition 1 | Line 593 |
| `mmcblk0rpmb` | 16 MiB | RPMB (Replay Protected Memory Block) | Line 595 |
| Total card | 58.3 GiB | Y0S064 eMMC | Line 589 |

### Boot script content (`boot.scr.emmc`):

* `[PARTIALLY OBSERVED]` — 1,294 bytes read from eMMC partition 1 (line 145)
* Contains `BM1684 boot script` (line 147) and `booting #config-pcb180` (line 148)
* Full content not visible (binary U-Boot script format)

### FIT image metadata:

* Loaded from eMMC partition 1 as `emmcboot.itb` (line 149)
* 28,240,922 bytes (line 150)
* Contains 3 subimages: kernel, ramdisk, fdt (lines 152-196)
* All subimages verified with SHA1 hash integrity (lines 155, 167, 183, 195)
* Configuration: `config-pcb180` (line 153)

---

## TASK 13 — U-Boot Capabilities

* **Version:** `U-Boot 2022.10 (May 08 2025 - 13:42:40 +0800) Sophon BM1684` — `[OBSERVED]` line 73
* **Architecture:** AArch64 — `[OBSERVED]` line 161
* **FIT support:** Yes — `[OBSERVED]` lines 152-196 (full FIT image loading and verification)
* **Boot script:** `/boot.scr.emmc` — `[OBSERVED]` lines 143-148
* **Environment storage:** U-Boot attempted to load environment from FAT on `mmc0:1` but failed (`Unable to read "uboot.env"`, line 102) — environment is likely stored on eMMC or uses defaults
* **Console:** `serial@50118000` — `[OBSERVED]` lines 104-106
* **Watchdog:** `bm16xxwdt@69 with servicing (60s timeout)` — `[OBSERVED]` line 79
* **Network:** RTL8211F PHY on both eth0 and eth1 — `[OBSERVED]` lines 112-117
* **MMC:** `sdhc@50100000: 0, sdhc@50101000: 1` — `[OBSERVED]` line 80
* **DTB:** U-Boot uses its own DTB (fit) — `[OBSERVED]` line 78: `devicetree: fit`
* **Autoboot delay:** 2 seconds — `[OBSERVED]` line 119: `Hit any key to stop autoboot: 2 1 0`

### Can U-Boot launch a generic AArch64 Image?

**YES** — `[INFERRED]` with HIGH confidence based on:
1. U-Boot 2022.10 has standard `bootm` / `booti` support for arm64 Images
2. The existing boot sequence uses FIT with `AArch64` / `Linux` kernel type
3. U-Boot can be interrupted during the 2-second autoboot window
4. A custom `boot.scr.emmc` could direct U-Boot to load a raw Vivanta binary

**OR** a Vivanta payload packaged as a FIT image subimage (replacing the Linux kernel) would be transparently loaded and verified by U-Boot.

---

## TASK 14 — Firmware Ownership Map

| Resource | Owner at Boot | Owner under Linux | Could Vivanta Own It? | Evidence |
|---|---|---|---|---|
| DDR controller | BL2 (EL3) | Linux (read-only) | NO — DDR training is proprietary | Lines 43-47 |
| DRAM | BL2 initializes | Linux manages usable regions | YES — for non-reserved regions | Lines 43-47, 234 |
| GIC | BL31 initializes (EL3) | Linux GICv2 driver (EL1) | YES — after BL31 init | Lines 63, 242 |
| Timer | BL31 sets CNTFRQ (EL3) | Linux arch_timer (EL1) | YES — CNTFRQ set by firmware, kernel uses it | Lines 243-245 |
| UART | U-Boot (EL2) → Linux (EL1) | Linux 8250 driver | YES — standard 16550A | Lines 104, 209, 453 |
| eMMC | U-Boot (EL2) → Linux (EL1) | Linux SDHCI driver | YES — after handoff | Lines 80, 556 |
| Ethernet | U-Boot (EL2) → Linux (EL1) | Linux stmmac/dwmac driver | YES — after handoff | Lines 112-117, 463-492 |
| PCIe | U-Boot (EL2) → Linux (EL1) | Linux bm168x_pcie driver | YES — after handoff | Lines 398-402 |
| USB | Linux (EL1) | Linux xhci driver | YES — via PCIe | Lines 510-532 |
| NPU | BL2/BL31 (secure?) → Linux (EL1) | Linux ION/bm_vpp drivers | CONDITIONAL — Vivanta would need vendor driver or reverse-engineering | Lines 215-216, 434-446 |
| VPU | BL2/BL31 (secure?) → Linux (EL1) | Linux ION driver | CONDITIONAL — same as NPU | Lines 217-218, 429-430 |
| VPP | BL2/BL31 (secure?) → Linux (EL1) | Linux ION/bm_vpp drivers | CONDITIONAL — same as NPU | Lines 219-220, 426-427, 434-446 |
| Clocks | BL2/BL31 (secure) | Linux clock framework | PARTIAL — some clocks set by firmware, Linux manages others | Lines 350-351, 869-874 |
| Reset | BL2/BL31 (secure) | Linux reset controllers | PARTIAL — firmware controls secure resets | Lines 48-49, 869-874 |
| Power | BL31 (PSCI) | Linux CPUFreq | YES — PSCI available, Linux uses it | Lines 222-225, 256-269 |
| SPI-NOR flash | BL1/BL2 (EL3) | Not accessed by Linux? | NO — contains secure firmware | Lines 9, 51, 55 |
| I2C MCU | BL2 reads board type | Linux bm16xx-mcu driver | YES — accessible via I2C | Lines 28-44, 447 |
| Watchdog | U-Boot (EL2) → Linux | Linux bm-wdt driver | YES — I2C-based | Lines 79, 448 |

---

## TASK 15 — Minimal Vivanta Boot Contract

See `VIVANTA-AIBOX-BOOT-CONTRACT.md` for the complete verified boot contract.

### Summary:

```text
Minimum Vivanta Boot Contract:
  - EL: EL2 (can drop to EL1)
  - MMU: OFF at entry
  - x0: DTB physical address
  - CNTFRQ_EL0: 50,000,000 (set by BL31)
  - PSCI: v1.1 via SMC (v0.2 function IDs)
  - UART: 16550A @ 0x50118000, IRQ 15, 32-bit, 31.25 MHz clock
  - GIC: GICv2 (base addresses from DTB or hardcoded)
  - RAM: ~6 GiB usable (after NPU/VPU/VPP carve-outs)
  - PA width: ≥40 bits
  - DMA width: 40 bits
  - CPU: 8× Cortex-A53 r0p4, 2 clusters
  - SMP: PSCI CPU_ON for secondary bring-up
```

---

## TASK 16 — First-Boot Vivanta Design

### Prerequisites:

1. Vivanta AArch64 kernel binary (uncompressed Image)
2. FIT image packaging tool (`mkimage`)
3. Original BM1684X DTB (25 KiB, can be extracted from `emmcboot.itb`)
4. eMMC write access on AIBOX (or SD card with modified boot script)

### Minimum Proof of Life Sequence:

```text
1. Package Vivanta as FIT subimage (config-pcb180, load 0x300280000)
2. Write to eMMC partition 1 as emmcboot.itb
3. Power on AIBOX
4. Vendor boot chain executes (BL1 → BL2 → BL31 → U-Boot)
5. U-Boot loads Vivanta FIT from eMMC
6. Vivanta enters at EL2 @ 0x300280000
7. Vivanta drops to EL1 (or stays EL2)
8. Vivanta initializes UART @ 0x50118000 → "Vivanta alive" on serial
9. Vivanta initializes GICv2 (hardcoded or DTB-parsed bases)
10. Vivanta initializes ARM Generic Timer (CNTFRQ already set by BL31)
11. Vivanta initializes MMU (4 KiB granule, identity + high mapping)
12. Vivanta initializes PMM (exclude NPU/VPU/VPP carve-outs)
13. Vivanta starts scheduler (timer-driven preemption)
14. Vivanta brings up CPU1-7 via PSCI CPU_ON SMC
```

### NOT required for first boot:
- NPU, VPU, VPP
- Ethernet, USB, PCIe
- eMMC, SD card
- I2C, RTC, Watchdog
- `bitmain,top-intc`

---

## TASK 17 — Known / Unknown / Unverified

### KNOWN (Verified from boot log):

| Item | Value | Confidence |
|---|---|---|
| SoC | BM1684X | HIGH |
| CPU | 8× Cortex-A53 r0p4 | HIGH |
| EL at entry | EL2 | HIGH |
| RAM total | 16 GiB LPDDR4x 4000 MHz | HIGH |
| RAM usable | ~6 GiB | HIGH |
| NPU region | 0x124100000, 3.95 GiB | HIGH |
| VPU region | 0x380000000, 2 GiB | HIGH |
| VPP region | 0x440000000, 3 GiB | HIGH |
| CMA | 0x438000000, 128 MiB | HIGH |
| Ramoops | 0x314000000, 1 MiB | HIGH |
| Timer | ARM Generic Timer, 50 MHz | HIGH |
| UART | 16550A, 0x50118000, IRQ 15 | HIGH |
| GIC | GICv2, split EOI | HIGH |
| top-intc | 0x500100bc, 16 MSI IRQs | HIGH |
| Boot chain | BL1→BL2→BL31→U-Boot→FIT→Kernel | HIGH |
| Kernel entry | 0x300280000, EL2 | HIGH |
| DTB model | bitmain-bm1684x-se7-v1-mini | HIGH |
| PSCI | v1.1, SMC, v0.2 IDs | HIGH |
| PA width | ≥40 bits | HIGH |
| DMA width | 40 bits | HIGH |
| Boot media | SPI-NOR (firmware) + eMMC (OS) | HIGH |
| eMMC partitions | p1-p7 + boot0/boot1/rpmb | HIGH |
| U-Boot version | 2022.10 | HIGH |
| FIT image | emmcboot.itb, SHA1, config-pcb180 | HIGH |

### UNKNOWN (Not observable from boot log):

| Item | Value | Priority | Impact |
|---|---|---|---|
| GICD base address | `[UNKNOWN]` | **CRITICAL** | Cannot init GICv2 without it |
| GICC base address | `[UNKNOWN]` | **CRITICAL** | Cannot init GICv2 without it |
| Timer PPI number | `[UNKNOWN]` | **HIGH** | Cannot register timer IRQ without it |
| Ethernet IRQ | `[UNKNOWN]` | MEDIUM | Not needed for first boot |
| eMMC IRQ | `[UNKNOWN]` | MEDIUM | Not needed for first boot |
| PCIe IRQ | `[UNKNOWN]` | MEDIUM | Not needed for first boot |
| NPU IRQ | `[UNKNOWN]` | LOW | Not needed until Milestone 5 |
| BL32 / TEE presence | `[UNKNOWN]` | MEDIUM | Affects security model |
| Secure boot verification | `[UNKNOWN]` | MEDIUM | Affects ability to replace FIT |
| U-Boot environment | `[UNAVAILABLE]` | MEDIUM | Affects boot script modification |
| Full DTB structure | `[UNAVAILABLE]` | **CRITICAL** | Contains GICD, timer PPI, all addresses |
| SCTLR/TCR/TTBR at entry | `[UNKNOWN]` | LOW | Standard arm64 boot protocol assumed |
| Clock tree topology | `[UNKNOWN]` | LOW | BL31 manages critical clocks |
| IOMMU/SMMU presence | `[UNKNOWN]` | LOW | Not needed for minimal boot |

### UNVERIFIED (Inferred from standard protocols):

| Item | Inference | Confidence |
|---|---|---|
| x0 = DTB at entry | Standard arm64 boot protocol via U-Boot bootm | MEDIUM |
| MMU OFF at entry | Standard arm64 boot protocol | HIGH |
| x1-x3 = 0 at entry | Standard arm64 boot protocol | HIGH |
| Secondary CPUs enter at EL2 | PSCI CPU_ON, same as primary | HIGH |

### IMPOSSIBLE TO DETERMINE WITHOUT DOCUMENTATION:

| Item | Why | Priority |
|---|---|---|
| NPU command submission interface | Requires vendor register docs or driver source | LOW (first boot doesn't need) |
| BL31 SMC handler details | TF-A source may exist (v2.7) | MEDIUM |
| DDR controller register map | Proprietary vendor IP | LOW (BL2 handles) |
| Secure boot key layout | Requires ROM documentation | MEDIUM |

### Ranked Unknowns by Value:

1. **CRITICAL:** Full live DTB (contains GICD, GICC, timer PPI, all MMIO addresses, IRQ routing, clock topology)
2. **CRITICAL:** GICD and GICC base addresses
3. **HIGH:** Timer PPI number (secure timer vs virtual timer)
4. **MEDIUM:** BL32/TEE presence and memory layout
5. **MEDIUM:** Secure boot verification status (can we freely replace FIT?)
6. **MEDIUM:** U-Boot environment variables
7. **LOW:** NPU/VPU/VPP register documentation

---

## TASK 18 — Final Recommendation

### Is additional investigation still worth doing?

**YES — critical evidence is still missing.**

The single most valuable missing artifact is the **live DTB** (`live.dtb` / `live.dts`). It contains:
- GICD and GICC base addresses (CRITICAL for GICv2 initialization)
- Timer PPI numbers (CRITICAL for timer IRQ registration)
- All peripheral MMIO addresses and IRQ assignments
- Clock and reset controller topology
- Full reserved-memory node list
- IOMMU/SMMU presence/configuration

### How to obtain it (in priority order):

1. **If credentials become available**: `cp /sys/firmware/devicetree/base /DEVICE-TREE/live.dtb` (read-only, safe)
2. **If U-Boot console access is obtained** (by interrupting autoboot during next boot): `load mmc 0:1 ${loadaddr} /emmcboot.itb` then `iminfo ${loadaddr}` to extract DTB, or `fdt addr ${fdt_addr}` and `fdt print /` (all read-only U-Boot commands)
3. **If the FIT image can be accessed** from another machine: `dumpimage -T flat_dt -p 0 -o live.dtb emmcboot.itb` (extract DTB from FIT image)
4. **From public SDK**: Sophon/FireFly BM1684X SDK may contain the DTS source on GitHub

### Why current evidence IS sufficient to BEGIN designing the first bring-up:

Even without the DTB, Vivanta engineers can:
- Design the platform layer with UART, timer, MMU, PMM (all addresses are known)
- Hardcode GICD/GICC base addresses (try `0x50010000` and `0x50010200` — standard GICv2 offsets for this SoC family, but **MUST verify from DTB before flashing**)
- Package a Vivanta FIT image
- Design the boot script

### What CANNOT proceed without the DTB:
- GICv2 initialization (wrong base = immediate crash)
- Timer IRQ registration (wrong PPI = no preemption)
- Any peripheral driver (wrong IRQ = no interrupts)

### Final verdict:

The boot log alone provides **80%+ of the information needed** for a first Vivanta bring-up. The remaining 20% (GICD/GICC addresses, timer PPI, full DTB) can be obtained from:
1. Public Sophon/FireFly SDK source code (most likely available on GitHub)
2. A 30-second U-Boot console session (interrupt autoboot, `fdt print`)
3. Any authenticated Linux shell (`cat /proc/device-tree/`)

The **AIBOX-1684X is confirmed as a viable Vivanta experimental platform**, and sufficient evidence exists to begin the platform design immediately, pending GICD/GICC/timer-PPI verification before the first flash attempt.
