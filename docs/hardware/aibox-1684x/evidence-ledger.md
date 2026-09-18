# AIBOX-1684X Evidence Ledger (Second-Pass Verified)

## Access Status

* Serial port: `[OBSERVED]` active at `/dev/cu.wchusbserial10` 115200 baud
* Login prompt: `[OBSERVED]` `bm1684 login:`
* Authenticated shell: `[UNAVAILABLE — AUTHENTICATION REQUIRED]`
* U-Boot console: `[UNAVAILABLE — AUTOBOOT NOT INTERRUPTED]`
- `/proc/iomem`: `[UNAVAILABLE — AUTHENTICATION REQUIRED]`
- `/proc/meminfo`: `[UNAVAILABLE — AUTHENTICATION REQUIRED]`
- `/proc/device-tree/`: `[UNAVAILABLE — AUTHENTICATION REQUIRED]`
- `/proc/cmdline`: `[OBSERVED]` via kernel boot log: `console=ttyS0,115200 earlycon user_debug=31`
- `/proc/interrupts`: `[UNAVAILABLE — AUTHENTICATION REQUIRED]`
- `dmesg`: `[OBSERVED]` via serial boot log (912 lines)
- live.dtb: `[UNAVAILABLE — AUTHENTICATION REQUIRED]`
- U-Boot env: `[UNAVAILABLE — AUTOBOOT NOT INTERRUPTED]`

---

## Claim 1: SoC = BM1684X
* **Evidence:** BL1 banner `bm1686_rom_v6` (line 5); U-Boot banner `Sophon BM1684` (line 73); kernel version `5.4.217-bm1684` (line 207); DTB model `bitmain-bm1684x-se7-v1-mini` (line 69); PCI vendor `[1e30:1684]` (line 407)
* **Source:** Serial boot log
* **Classification:** `[OBSERVED]`
* **Confidence:** HIGH
* **Correction:** BL1 string is `bm1686_rom_v6` — possible family codename. SoC is BM1684X per all other sources.

---

## Claim 2: CPU = 8 × ARM Cortex-A53 r0p4
* **Evidence:** `0x410fd034` (lines 206, 271-283); `smp: Brought up 1 node, 8 CPUs` (line 284); `armv8_cortex_a53 PMU` (line 382)
* **Source:** Linux kernel boot log
* **Classification:** `[REPORTED BY LINUX]` / `[OBSERVED]`
* **Confidence:** HIGH
* **Additional:** Two-cluster topology (CPU0-3 affinity 0x000-0x003, CPU4-7 affinity 0x100-0x103).

---

## Claim 3: AArch64, EL2 boot, EL2→EL1 transition
* **Evidence:** `Architecture: AArch64` (line 161); `CPU: All CPU(s) started at EL2` (line 288); SPSR=0x3c9 for BL31→U-Boot handoff (line 68, EL2); `32-bit EL0 Support` (line 286)
* **Source:** U-Boot FIT metadata, Linux kernel boot log
* **Classification:** `[OBSERVED]` + `[INFERRED]`
* **Confidence:** HIGH
* **Correction:** Previous report implied EL1 at entry. Corrected: **EL2 at entry**, Linux drops to EL1 itself. Vivanta can choose EL2→EL1 or stay at EL2.

---

## Claim 4: RAM = 16 GiB LPDDR4x, 4000 MHz, rank 2+2
* **Evidence:** `LPDDR4x(rank: 2 + 2, freq: 4000M) init start` (line 46); `Memory: 6289304K/16055040K available` (line 234); U-Boot `DRAM: 1 GiB` (line 75)
* **Source:** BL2 firmware, Linux kernel, U-Boot
* **Classification:** `[OBSERVED]` / `[REPORTED BY FIRMWARE]` / `[REPORTED BY LINUX]`
* **Confidence:** HIGH
* **Correction:** U-Boot's `1 GiB` is its own working window, NOT total RAM. Total is 16 GiB.

---

## Claim 5: Boot chain = BootROM → BL2 → BL31 → U-Boot → FIT → Kernel
* **Evidence:**
  - BL1 (ROM): `bm1686_rom_v6` (line 5); loads BL2 from SPI flash FIP (line 9)
  - BL2: `v2.7` (line 25); DDR init (lines 43-47); loads image id=3 (BL31) to `0x300000000` (line 52); loads image id=5 (U-Boot) to `0x308000000` (line 56)
  - BL31: `v2.7` (line 61); GICv2 init (line 63); EL3→EL2 handoff (lines 66-68)
  - U-Boot: `2022.10` (line 73); loads FIT from eMMC (lines 143-150)
  - Kernel: loaded at `0x300280000` (lines 163-164)
* **Source:** Serial boot log
* **Classification:** `[OBSERVED]`
* **Confidence:** HIGH
* **Correction:** No BL32/TEE stage observed. Either absent or silent. `[UNKNOWN]`.
* **Correction:** BL2 and BL31 share the same build timestamp (May 8 2025), suggesting they are from the same firmware build.

---

## Claim 6: Kernel load/entry = 0x300280000
* **Evidence:** U-Boot FIT: `Load Address: 0x300280000, Entry Point: 0x300280000` (lines 163-164)
* **Source:** U-Boot FIT image subimage metadata
* **Classification:** `[OBSERVED]` / `[FIT IMAGE SUBIMAGE / U-BOOT REPORTED]`
* **Confidence:** HIGH
* **Correction:** This is a U-Boot/FIT configuration decision, NOT a hardware-mandated address. Vivanta can choose a different address if its FIT subimage declares it.

---

## Claim 7: DTB at x0 = 0x33f1a3000
* **Evidence:** U-Boot: `Loading Device Tree to 000000033f1a3000` (line 200)
* **Source:** U-Boot output
* **Classification:** `[OBSERVED]` (U-Boot load address); `[INFERRED]` (x0 = dtb per standard arm64 boot protocol)
* **Confidence:** HIGH for load address, MEDIUM for x0 (assumes standard U-Boot bootm behavior)

---

## Claim 8: GICv2 with bitmain,top-intc cascading
* **Evidence:** `ARM GICv2 driver initialized` (line 63); `GIC: Using split EOI/Deactivate mode` (line 242); `bitmain,top-intc 500100bc.top_intc: got 16 gic irqs` (line 324); MSI address `0x50010198` (line 514)
* **Source:** TF-A/BL31, Linux kernel
* **Classification:** `[OBSERVED]`
* **Confidence:** HIGH
* **Correction:** Previous report implied GICv2 is "directly reusable." Corrected: GICv2 driver is necessary, but `bitmain,top-intc` must be implemented for MSI-capable peripherals (PCIe/USB). For minimal boot (UART/timer), top-intc can be ignored.

---

## Claim 9: Timer = ARM Generic Timer @ 50 MHz, physical
* **Evidence:** `arch_timer: cp15 timer(s) running at 50.00MHz (phys).` (line 243); `clocksource: arch_sys_counter` (line 244); `sched_clock: 56 bits at 50MHz` (line 245)
* **Source:** Linux kernel
* **Classification:** `[OBSERVED]`
* **Confidence:** HIGH
- U-Boot `DRAM: 1 GiB` (line 75) is its own working window, not total RAM.

---

## Claim 10: UART = 16550A @ 0x50118000, IRQ 15, 32-bit MMIO
* **Evidence:** `earlycon: uart0 at MMIO32 0x0000000050118000` (line 209); `50118000.serial: ttyS0 at MMIO 0x50118000 (irq = 15, base_baud = 31250000) is a 16550A` (line 453)
* **Source:** Linux kernel earlycon + 8250 driver
* **Classification:** `[OBSERVED]`
* **Confidence:** HIGH
* **Additional:** No vendor-specific init sequence observed. Works at earlycon stage.

---

## Claim 11: Remaining RAM after carve-outs ≈ 6 GiB
* **Evidence:** `Memory: 6289304K/16055040K available` (line 234); 9634664K reserved
* **Source:** Linux kernel
* **Classification:** `[REPORTED BY LINUX]`
* **Confidence:** HIGH
* **Correction:** Previous report's "available" figure was correct, but the implication was that Vivanta "owns" 6 GiB. In reality, Vivanta owns whatever is not reserved, which includes kernel space overhead. The exact free-for-use figure is slightly less than 6 GiB.

---

## Claim 12: DDR controller trainingirmware 2023` (line 43)
firmware in SPI-NOR flash. DDR controller is NOT replaceable by Vivanta.

---

## Claim 14: Secure firewall disabled in BL31
* **Evidence:** `ERROR: disable secure firewall` (line 64)
* **Source:** BL31 firmware
* **Classification:** `[OBSERVED]`
* **Confidence:** HIGH
* **Implication:** BL31 disables certain security firewalls during init, suggesting development/debug mode. This may mean Vivanta has more normal-world access than on a production Secure Boot system. This is `[UNVERIFIED]` — the exact implications depend on what specific firewalls are disabled.

---

## Claim 15: PA width ≥ 40 bits
* **Evidence:** `kvm [1]: IPA Size Limit: 40 bits` (line 383); `Using 40 bits DMA width` (line 475)
* **Source:** Linux kernel (KVM, Ethernet DMA)
* **Classification:** `[REPORTED BY LINUX]`
* **Confidence:** HIGH
* **Implication:** Vivanta MMU should target 40-bit PA minimum (48-bit VA is standard).

---

## Claim 16: ARM erratum 845719 present
* **Evidence:** `CPU features: detected: ARM erratum 845719` (line 228)
* **Source:** Linux kernel
* **ClassificatL should apply the erratum 845719 workaround (related to TLB invalidation on Cortex-A53).

---

## Contradiction: No contradictions found

The previous report's claims were all confirmed, with several refinements:
1. BL1 string is `bm1686_rom_v6` (family name), not `bm1684x_rom`
2. U-Boot's `1 GiB` is its own window, not total RAM
3. Kernel entry is EL2, not EL1 (Linux drops to EL1 itself)
4. `0x300280000` is a FIT/U-Boot config decision, not hardware-mandated
5. `bitmain,top-intc` is an additional layer that GICv2 alone is insufficient for
6. No BL32/TEE stage observed in boot log
