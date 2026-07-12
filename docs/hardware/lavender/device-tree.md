# Device Tree Survey — Xiaomi Redmi Note 7 (lavender, SDM660)

## Status

| Field | Value |
|-------|-------|
| Survey Date | 2026-07-12 |
| Source | `/proc/device-tree` from running Ubuntu Touch kernel (Linux 4.4.156) |
| DTB Index | 11 (`androidboot.dtb_idx=11`) |
| DTBO Partition | `/dev/block/mmcblk0p52` (not directly inspected) |

## Root Node

| Property | Value |
|----------|-------|
| `model` | `Qualcomm Technologies, Inc. SDM 660 PM660 + PM660L MTP F7A overlay` |
| `compatible` | (Qualcomm SDM660 MTP) |
| `#address-cells` | 2 |
| `#size-cells` | 2 |
| `interrupt-parent` | phandle 0x01 (GICv3) |

## CPUs

8 cores, big.LITTLE:

| Core | Type | CPU Part | Implementer |
|------|------|----------|-------------|
| 0-3 | Cortex-A73 (Kryo 360 Gold) | `0x801` | `0x51` (Qualcomm) |
| 4-7 | Cortex-A53 (Kryo 360 Silver) | `0x800` | `0x51` (Qualcomm) |

- CPU nodes: `cpu@0`, `cpu@1`, `cpu@2`, `cpu@3` (little cluster), `cpu@100`, `cpu@101`, `cpu@102`, `cpu@103` (big cluster)
- CPU-map present in DTB
- All CPUs start at EL1 per dmesg

## Memory

### Physical Layout (from `/proc/iomem`)

| Region | Start | End | Size |
|--------|-------|-----|------|
| System RAM (bank 0) | `0x40000000` | `0x855FFFFF` | ~1,116 MB |
| System RAM (bank 1) | `0x88F00000` | `0x8ABFFFFF` | ~45 MB |
| System RAM (bank 2) | `0x95000000` | `0xFEABFFFF` | ~1,693 MB |
| **Total usable** | | | **~2,854 MB** |

### DTB Memory Node

Two banks per `/proc/device-tree/memory/reg`:
1. Base `0x40000000`, size `0x60000000` (1.5 GB — total available, includes reserved regions)
2. Base `0xA0000000`, size `0x5EAC0000` (~1.5 GB — high memory)

Total: ~3 GB detected by DTB (remaining ~1 GB reserved for modem/adsp/firmware).

### Reserved Memory Regions

| Region | Address | Size | Purpose |
|--------|---------|------|---------|
| `removed_regions@85800000` | `0x85800000` | `0x370000` (~3.5 MB) | Platform reserved |
| `wlan_msa_guard@85600000` | `0x85600000` | — | WLAN MSA guard |
| `wlan_msa_mem@85700000` | `0x85700000` | — | WLAN MSA memory |
| `modem_fw_region@8ac00000` | `0x8AC00000` | — | Modem firmware |
| `adsp_fw_region@92a00000` | `0x92A00000` | — | ADSP firmware |
| `cdsp_fw_region@94a00000` | `0x94A00000` | — | CDSP firmware |
| `splash_region@9d400000` | `0x9D400000` | — | Boot splash screen |
| `pstore_reserve_mem_region@0` | `0xA0000000` | `0x400000` (4 MB) | pstore/ramoops |

### Kernel Memory (from iomem)

| Region | Address | Size |
|--------|---------|------|
| Kernel code | `0x40080000` | `0x1B8000` |
| Kernel data | `0x42600000` | `0x9AE000` |

## UART (console)

### Node

Path: `/soc/serial@0c170000`

| Property | Value |
|----------|-------|
| Address | `0x0C170000` |
| Size | `0x100000` (1 MB) |
| Compatible | `qcom,msm-uartdm-v1.4`, `qcom,msm-uartdm` |
| Interrupt | SPI 108, level-triggered (GICv3) |
| Clocks | 2: core (idx 45), iface (idx 35), from clock-controller phandle 0xA1 |
| Clock names | `core`, `iface` |
| Status | `ok` |
| Pinctrl | `default` state |

### Console Configuration

| Parameter | Value |
|-----------|-------|
| Console device | `ttyMSM0` |
| Baud rate | 115200 |
| Data bits | 8 |
| Parity | none |
| Stop bits | 1 |
| Earlycon | `msm_serial_dm,0xc170000` |
| Aliased as | `serial0` (from `/aliases`) |

### Register Layout (MSM UART DM v1.4)

Based on the `qcom,msm-uartdm-v1.4` compatible string, the UART DM register layout is:
- `+0x00`: TX FIFO / RX FIFO (depending on access direction)
- `+0x08`: Status Register (SR), bit 2 = TX_READY
- Full register map requires Qualcomm UART DM documentation

## Interrupt Controller

| Property | Value |
|----------|-------|
| Type | GICv3 |
| CPUs supported | 8 |
| Detected from | `/proc/interrupts` |

## Timers

| Property | Value |
|----------|-------|
| Timer type | Generic architected timer (CP15 + MMIO) |
| Frequency | 19.20 MHz (virt) |
| Interrupt | PPI 19 (Edge-triggered) |

## PSCI

| Property | Value |
|----------|-------|
| Version | PSCI v1.0 |
| Conduit | SMC (from DT) |
| Function IDs | Standard PSCI v0.2 |
| CPU suspend | Available |
| CPU off | Available |
| CPU on | Available |

## Kernel Boot Info

| Property | Value |
|----------|-------|
| Boot CPU | 0x0 |
| Exception level | **EL1** |
| MMU state | Enabled (kernel maps memory) |
| Kernel version | 4.4.156-danctnix+ |
| SMP | 8 processors |
| HMP scheduling | Enabled |

## Boot Arguments (from `chosen/bootargs`)

Full command line: `console=ttyMSM0,115200,n8 androidboot.console=ttyMSM0 earlycon=msm_serial_dm,0xc170000 androidboot.hardware=qcom user_debug=31 msm_rtb.filter=0x37 ehci-hcd.park=3 lpm_levels.sleep_disabled=1 sched_enable_hmp=1 sched_enable_power_aware=1 service_locator.enable=1 swiotlb=1 firmware_class.path=/vendor/firmware_mnt/image loop.max_part=7 androidboot.avb_version=1.0 androidboot.vbmeta.avb_version=1.0 console=tty0 apparmor=1 security=apparmor root=PARTUUID=50d69e1d-8239-78b7-6a82-098762fd8b7f androidboot.bootdevice=c0c4000.sdhci androidboot.serialno=b5fdde57 androidboot.cpuid=0x8df62afd androidboot.dp=0x0 androidboot.baseband=msm mdss_mdp.panel=1:dsi:0:qcom,mdss_dsi_nt36672a_tianma_fhdplus_video:config0:1:none:cfg:single_dsi rootwait ro init=/init androidboot.dtbo_idx=0 androidboot.dtb_idx=11 androidboot.fpsensor=fpc androidboot.secureboot=1 androidboot.hwc=Global androidboot.hwversion=1.29.0 androidboot.hwlevel=MP`

### Notable Boot Arguments

| Argument | Value | Significance |
|----------|-------|-------------|
| `console` | `ttyMSM0,115200,n8` | Debug UART configuration |
| `androidboot.dtb_idx` | `11` | DTB index used for this hardware revision |
| `androidboot.verifiedbootstate` | `orange` | Bootloader unlocked (can boot unsigned) |
| `androidboot.secureboot` | `1` | Secure boot enabled, but in lenient policy |
| `root` | `PARTUUID=50d69e1d...` | Root filesystem by PARTUUID |
| `swiotlb` | `1` | SW IOMMU bounce buffer enabled |
| `init` | `/init` | Init process path |
