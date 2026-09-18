# Device Tree Reconnaissance: AIBOX-1684X

## Access Status

`[UNAVAILABLE — AUTHENTICATION REQUIRED]`

The live DTB could not be exported from the running Linux system. No authenticated shell was obtained during the investigation.

## What IS known (from boot log evidence):

### DTB Identity
- **Model:** `bitmain-bm1684x-se7-v1-mini` — `[OBSERVED]` line 69
- **Description:** `for BM1684X AiBox One` — `[OBSERVED]` line 188
- **Config:** `config-pcb180` — `[OBSERVED]` line 153
- **Size:** 25,648 bytes (25 KiB) — `[OBSERVED]` line 192
- **Location at boot:** Loaded to `0x33f1a3000` — `[OBSERVED]` line 200
- **Embedded in:** `emmcboot.itb` on eMMC partition 1

### DTB SHA1 Hash
`f02e98cd5dc3018cfbd4cf51dcd57ca3a557a887` — `[OBSERVED]` line 195

### Reconstructed DT Nodes (from Linux driver probing)

| Node | Compatible | MMIO | IRQ | Driver | Evidence |
|---|---|---|---|---|---|
| `serial@50118000` | 16550A | `0x50118000` | 15 | `8250/16550` | Lines 209, 453 |
| `serial@5011a000` | 16550A | `0x5011a000` | 16 | `8250/16550` | Line 458 |
| `serial@5011c000` | 16550A | `0x5011c000` | 17 | `8250/16550` | Line 459 |
| `ethernet@50108000` | `bm-dwmac` | `0x50108000` | `[UNKNOWN]` | `stmmac/dwmac` | Lines 463-477 |
| `ethernet@5010c000` | `bm-dwmac` | `0x5010c000` | `[UNKNOWN]` | `stmmac/dwmac` | Lines 478-492 |
| `bm-emmc` / `sdhc@50100000` | `sdhci_bm` | `0x50100000` | `[UNKNOWN]` | `sdhci_bm` | Line 556 |
| `bm-sd` / `sdhc@50101000` | `sdhci_bm` | `0x50101000` | `[UNKNOWN]` | `sdhci_bm` | Line 558 |
| `pcie@5fb80000` | `bm168x_pcie` | `0x5fb80000` | `[UNKNOWN]` | `bm168x_pcie` | Lines 398-402 |
| `top_intc` | `bitmain,top-intc` | `0x500100bc` | (cascade) | `bitmain,top-intc` | Lines 306-324 |
| `50110000.sysdma` | `dw_dmac` | `0x50110000` | `[UNKNOWN]` | `dw_dmac` | Line 348 |
| `200b000.tsdma` | `dw_dmac` | `0x200b000` | `[UNKNOWN]` | `dw_dmac` | Line 349 |
| `5001c000.i2c` | `i2c_designware` | `0x5001c000` | `[UNKNOWN]` | `i2c_designware` | Line 358 |
| `5001e000.i2c` | `i2c_designware` | `0x5001e000` | `not found` | `i2c_designware` | Lines 359-360 |
| `50029000.pwm` | `bitmain-pwm` | `0x50029000` | `[UNKNOWN]` | `bitmain-pwm` | Lines 548-550 |
| bm_vpp | `bm_vpp` | `0x16860000` | 24, 25 | `bm_vpp` | Lines 434, 440 |
| `1-0017` | `bm16xx-mcu` | I2C bus 1, addr 0x17 | — | `bm16xx-mcu` | Line 447 |
| `1-0069` | `bm-wdt` | I2C bus 1, addr 0x69 | — | `bm-wdt` | Line 448 |
| `0-0051` | `rtc-hym8563` | I2C bus 0, addr 0x51 | — | `rtc-hym8563` | Lines 605-606 |
| `/pmu_a53` | PMU | — | `[UNKNOWN]` | `armv8_cortex_a53` | Line 381 |
| `m2m@0` | `sophgo,m2m-dma` | `[UNKNOWN]` | `[UNKNOWN]` | `sophgo,m2m-dma` | Lines 449-450 |

### Reserved Memory Nodes (from kernel boot log)

| Node | Base | Size | Compatible | Evidence |
|---|---|---|---|---|
| `linux,cma` | `0x438000000` | `0x8000000` (128 MiB) | `shared-dma-pool` | Lines 213-214 |
| `ion_npu_mem` | `0x124100000` | `0xf6e00000` (~3.95 GiB) | `npu-region` | Lines 215-216 |
| `ion_vpu_mem` | `0x380000000` | `0x80000000` (2 GiB) | `vpu-region` | Lines 217-218 |
| `ion_vpp_mem` | `0x440000000` | `0xc0000000` (3 GiB) | `vpp-region` | Lines 219-220 |

### Missing Information (requires authenticated shell or U-Boot console)

- `live.dtb` binary — `[UNAVAILABLE]`
- `live.dts` decompiled source — `[UNAVAILABLE]`
- GICD / GICC base addresses — `[UNKNOWN]`
- Timer PPI numbers — `[UNKNOWN]`
- All IRQ assignments for Ethernet, eMMC, PCIe, NPU — `[UNKNOWN]`
- Clock tree topology — `[UNKNOWN]`
- Reset controller registers — `[UNKNOWN]`
- GPIO/pinctrl configuration — `[UNKNOWN]`
- Ethernet MDIO/PHY wiring — `[UNKNOWN]`
- IOMMU/SMMU presence — `[UNKNOWN]`
- Full reserved-memory node list — `[PARTIALLY KNOWN]`
