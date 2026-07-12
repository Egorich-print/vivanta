# Bootloader Survey — Xiaomi Redmi Note 7 (lavender, SDM660)

## Status

| Field | Value |
|-------|-------|
| Survey Date | 2026-07-12 |
| Device State | Recovery (Ubuntu Touch) |
| Bootloader Access | Not attempted — device in recovery, requires `adb reboot bootloader` |

## Known from Kernel Cmdline

| Variable | Value | Source |
|----------|-------|--------|
| `androidboot.verifiedbootstate` | `orange` | `/proc/cmdline` |
| `androidboot.secureboot` | `1` | `/proc/cmdline` |
| `androidboot.avb_version` | `1.0` | `/proc/cmdline` |
| `androidboot.vbmeta.avb_version` | `1.0` | `/proc/cmdline` |
| `androidboot.serialno` | `b5fdde57` | `/proc/cmdline` |
| `androidboot.cpuid` | `0x8df62afd` | `/proc/cmdline` |
| `androidboot.bootdevice` | `c0c4000.sdhci` | `/proc/cmdline` |
| `androidboot.dtb_idx` | `11` | `/proc/cmdline` |
| `androidboot.dtbo_idx` | `0` | `/proc/cmdline` |
| `androidboot.hwc` | `Global` | `/proc/cmdline` |
| `androidboot.hwversion` | `1.29.0` | `/proc/cmdline` |
| `androidboot.hwlevel` | `MP` | `/proc/cmdline` |
| `androidboot.keymaster` | `1` | `/proc/cmdline` |
| `androidboot.fpsensor` | `fpc` | `/proc/cmdline` |
| `androidboot.baseband` | `msm` | `/proc/cmdline` |
| `androidboot.dp` | `0x0` | `/proc/cmdline` |
| `androidboot.hardware` | `qcom` | `/proc/cmdline` |

## Boot State Summary

- **Orange state**: Bootloader is unlocked (`verifiedbootstate=orange`). The device can boot unsigned images.
- **Secure boot**: Enabled (`secureboot=1`), but orange state allows booting anyway (verified boot policy is lenient).
- **AVB**: Android Verified Boot v1.0 in use (not AVB 2.0/dm-verity).
- **Slots**: Not determinable without `fastboot getvar current-slot`.

## Boot Partition

| Property | Value |
|----------|-------|
| Device | `/dev/block/mmcblk0p60` |
 | Boot Image Format | **Android Boot Image v1** |
| Header Magic | `ANDROID!` |
| Page Size | 4096 bytes |
| Kernel Load Address | `0x00008000` (from header) |
| Kernel is gzip compressed | Confirmed (magic `1f 8b 08`) |
| Ramdisk present | Yes |
| DTB method | DTBO via `androidboot.dtb_idx=11` |

## Boot Chain

```
BootROM
  ↓
PBL (Primary Boot Loader)
  ↓
ABL (Android Boot Loader) — Little Kernel based
  ↓
boot.img (mmcblk0p60) → decompress kernel → DTB via DTBO idx 11
  ↓
Linux kernel at EL1
```

## Required for Next Phase

- `fastboot getvar all` — needs device in fastboot mode
- `fastboot oem device-info` — needs device in fastboot mode
- Slot configuration (AB slots or A-only) — not yet determined
