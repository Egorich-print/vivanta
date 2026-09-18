# Vivanta Platform Design: `vivanta/platform/aibox_1684x/`

## Status: Realistic Vivanta Experimental Platform Candidate

---

## 1. Boot Entry Strategy

Vivanta replaces the Linux kernel subimage inside the vendor FIT image (`emmcboot.itb`). No changes to BL1, BL2, BL31, or U-Boot are needed.

```text
Vendor Boot ROM (BL1, immutable)
   ↓
Vendor BL2 (DDR training, proprietary)
   ↓
Vendor BL31 / TF-A (GICv2 init, PSCI, CNTFRQ)
   ↓
Vendor U-Boot 2022.10 (loads FIT from eMMC)
   ↓
Vivanta kernel subimage @ 0x300280000 (EL2)
   ↓
[EL2 → EL1 transition or stay at EL2]
   ↓
Early UART @ 0x50118000 (16550A, 115200 baud)
   ↓
"Vivanta is alive"
   ↓
GICv2 init + ARM Generic Timer (50 MHz)
   ↓
MMU init (4 KiB granule, 48-bit VA, ≥40-bit PA)
   ↓
PMM init (carve out NPU/VPU/VPP reserved regions)
   ↓
Scheduler (timer-driven preemption)
   ↓
[Milestone 2] PSCI SMP bring-up (CPU1-7)
   ↓
[Milestone 3] SDHCI eMMC storage
   ↓
[Milestone 4] DWMAC Ethernet + PCIe
   ↓
[Milestone 5] BM1684X accelerator sovereignty investigation
```

---

## 2. Reusable Kernel Mechanisms

| Mechanism | Existing Vivanta Component | Compatible? | Notes |
|---|---|---|---|
| MMU / VMM | ARMv8-A paging | YES | 4 KiB granule, 48-bit VA, ≥40-bit PA |
| GICv2 driver | Existing GICv2 | YES | Base addresses needed from DTB or hardcoded |
| ARM Generic Timer | Existing timer backend | YES | CNTFRQ=50 MHz, physical timer |
| 16550A UART | Existing 16550 driver | YES | `0x50118000`, 32-bit MMIO, IRQ 15 |
| PSCI SMP | Existing PSCI support | YES | v1.1, SMC, v0.2 function IDs |
| Exception vectors | Existing arm64 vectors | YES | VBAR_EL1 or VBAR_EL2 |

---

## 3. AIBOX-Specific Platform Code

### What `vivanta/platform/aibox_1684x/` must provide:

1. **Linker script:** Load/entry at `0x300280000` (or specified in FIT)
2. **Early UART init:** `0x50118000`, 32-bit, divisor for 115200 from 31.25 MHz clock
3. **PMM memory map:** Exclude NPU (`0x124100000`, 3.95 GiB), VPU (`0x380000000`, 2 GiB), VPP (`0x440000000`, 3 GiB), CMA (`0x438000000`, 128 MiB), ramoops (`0x314000000`, 1 MiB)
4. **GICv2 base addresses:** `[UNVERIFIED]` — likely in `0x500X0000` range. Must extract from DTB or hardcode.
5. **PSCI SMC calls:** Standard v0.2 function IDs via SMC conduit
6. **top-intc driver** (for PCIe/USB only, not needed for minimal boot)

### What it does NOT need to provide:

- DDR initialization (BL2 handles)
- Secure world setup (BL31 handles)
- Clock initialization (BL31/BL2 handle CNTFRQ and basic clocks)
- Board type detection (BL2 handles via I2C MCU)
- eMMC driver (not needed for first boot)
- Network driver (not needed for first boot)
- NPU/VPU/VPP drivers (not needed until Milestone 5)
- PCIe/USB drivers (not needed until Milestone 4)

---

## 4. FIT Image Packaging

```text
vivanta-aibox-1684x.itb (FIT image)
├── config-pcb180 (or new config name)
│   ├── kernel = vivanta.bin (AArch64, load/entry 0x300280000)
│   ├── fdt = bm1684x.dtb (25 KiB, original or minimal)
│   └── ramdisk = (optional, can be dummy or omitted)
└── (SHA1 hashes for each subimage)
```

Place as `emmcboot.itb` on eMMC partition 1. `/boot.scr.emmc` already selects `config-pcb180`.

---

## 5. Boot Contract Summary

See `VIVANTA-AIBOX-BOOT-CONTRACT.md` for the complete verified boot contract.

**Key facts:**
- Vivanta enters at **EL2** with MMU OFF
- x0 = DTB at `0x33f1a3000` (or wherever U-Boot loads it)
- CNTFRQ_EL0 = 50 MHz (set by BL31)
- PSCI available via SMC
- UART is immediately usable (no init sequence)
- GICv2 is initialized by BL31 but Vivanta should reconfigure
- 6 GiB of general-purpose RAM available (after carve-outs)
- 8-core Cortex-A53 with two clusters, SMP via PSCI CPU_ON
