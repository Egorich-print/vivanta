# Bootloader Survey — Xiaomi Redmi Note 5 Pro (whyred, SDM636)

## Status

| Field | Value |
|-------|-------|
| Survey Date | 2026-08-23 |
| Device State | fastboot mode (`adb reboot bootloader` by owner) |
| USB Serial | `4699bca9` (enumerated; differs from directive's "19680/68UA04603") |
| Method | `fastboot getvar all` (complete dump) |

## Critical State

| Variable | Value | Significance |
|----------|-------|--------------|
| **unlocked** | **no** | **Bootloader is LOCKED — all flash operations will be rejected until Mi Unlock is completed** |
| anti | 4 | Anti-rollback fused at version 4: firmware with lower anti-version refuses to boot. Custom payloads are unaffected (ABL checks only signed firmware images), but stock downgrades below the V12-era are permanently blocked |
| crc | 1 | CRC check enabled |
| token | `VQEBIAEQbe2B4CWYYLkJqKlJFQj+fwMGd2h5cmVkAgQOoF7o` | Base64 MiUL unlock token; ASN.1-ish structure contains literal "whyred" device marker + account binding data. This is what Mi Unlock Tool signs against |
| hw-revision | 10000 | |
| battery-voltage | 4148 mV | healthy for flashing |
| off-mode-charge | 0 | |
| erase-block-size / logical-block-size | 0x200 / 0x200 | 512B LUN blocks |
| variant | SDM EMMC | eMMC not UFS |
| version-baseband / version-bootloader | (empty) | ABL does not report them |

## Lock implications (verified reasoning)

1. `fastboot flash <partition>` will fail with `(remote: 'Failed to validate...')`
   or similar on a locked whyred — MIUI ABL enforces AVB for boot/recovery and
   refuses writes to protected partitions entirely.
2. Unlock path: bind Mi account in developer options → Windows **Mi Unlock
   Tool** → 168 h binding wait → unlock (wipes userdata).
3. `fastboot oem device-info` **is NOT supported** by this ABL build — it
   wedges the fastboot handler (see EXP-002 §incident). Lock state must be
   read from `getvar unlocked`.

## Boot chain

```
PBL (Qualcomm)
  ↓
XBL (+ xblbak fallback, 3.5 MB each)
  ↓
ABL (aboot) — MIUI, anti-rollback v4, locked
  ↓ boot.img @ mmcblk0p<boot> 64 MB, header v1/v0 accepted
Linux kernel EL1
```

## Fastboot variables of interest

Full dump preserved in EXP-002.md appendix and in pipeline repo
(`whyred-pve-uefi/docs/`). Notable: no `current-slot`/`slot-count` reported —
device is A-only (consistent with MIUI 8.1 launch firmware lineage).
