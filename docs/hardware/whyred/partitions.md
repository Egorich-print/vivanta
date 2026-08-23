# whyred — Measured GPT / eMMC layout (live device, 2026-08-23)

Source: `fastboot getvar all` on the actual unit. Sizes in hex bytes as
reported by ABL. This supersedes the estimated table in
`whyred-pve-uefi/docs/01-partitions.md`.

## Flash targets (our pipeline)

| Partition | Size (hex) | Size | Type | Our use |
|-----------|-----------:|------|------|---------|
| **boot** | 0x4000000 | 64 MiB | raw | UEFI payload / kernel boot.img |
| **userdata** | 0xCD77F7E00 | **54.9 GiB** | ext4 | PVE ARM64 rootfs (8 GiB image fits ×6) |
| cache | 0x10000000 | 256 MiB | ext4 | reuse candidate |
| recovery | 0x4000000 | 64 MiB | raw | keep stock (unbrick path) |

## Full layout (descending)

| Partition | Size (hex) | Size | Type |
|-----------|-----------:|------|------|
| userdata | 0xCD77F7E00 | 54.94 GiB | ext4 |
| vendor | 0x80000000 | 2 GiB | raw |
| system | 0xC0000000 | 3 GiB | ext4 |
| cust | 0x34000000 | 832 MiB | raw |
| cache | 0x10000000 | 256 MiB | ext4 |
| rawdump | 0x8000000 | 128 MiB | raw |
| modem | 0xC000000 | 192 MiB | raw |
| recovery | 0x4000000 | 64 MiB | raw |
| boot | 0x4000000 | 64 MiB | raw |
| logdump | 0x4000000 | 64 MiB | raw |
| splash | 0x4000000 | 64 MiB | raw |
| persistbak | 0x2000000 | 32 MiB | raw |
| persist | 0x2000000 | 32 MiB | raw |
| bk2 | 0x2000000 | 32 MiB | raw |
| mdtp | 0x2000000 | 32 MiB | raw |
| bk1 | 0x1800000 | 24 MiB | raw |
| dsp | 0x1000000 | 16 MiB | raw |
| modemst1/2, fsg, logfs, devinfo | 0x800000 each | 8 MiB each | raw |
| misc | 0x400000 | 4 MiB | raw |
| xbl, xblbak | 0x380000 each | 3.5 MiB each | raw |

## Corrections vs earlier estimates

| Partition | Was (estimated) | Actual |
|-----------|----------------|--------|
| vendor | 800 MiB (LineageOS boardconfig) | **2 GiB** |
| cust | ~570 MiB | **832 MiB** |
| splash | ~20 MiB | **64 MiB** |
| userdata | "~52 GiB" | **54.9 GiB exact** |

No `dtbo` partition exists in this dump — whyred A/B-less MIUI layout
confirmed at GPT level (DTBO was a lavender-only feature among our devices;
whyred DTB rides inside boot.img).
