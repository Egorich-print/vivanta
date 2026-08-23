# whyred — Redmi Note 5 Pro (SDM636)

## Platform

- SoC: Qualcomm Snapdragon 636 (SDM636) — same die as SDM660, lower clocks
- CPU: 4× Kryo 260 Gold (Cortex-A73) + 4× Kryo 260 Silver (Cortex-A53)
- Storage: 64 GB eMMC 5.1 (`variant: SDM EMMC`, hw-revision 10000)
- Boot: PBL → XBL → ABL → boot.img
- Fleet sibling: `lavender` (SDM660), see `../lavender/`

## Live surveys

| Experiment | Date | State | Result |
|------------|------|-------|--------|
| EXP-002 | 2026-08-23 | fastboot | full getvar dump captured; **bootloader LOCKED** (`unlocked:no`); ABL wedge on unsupported OEM cmd |

## Documents

- `bootloader.md` — fastboot survey, lock state, anti-rollback, MiUL token
- `partitions.md` — measured GPT layout (authoritative, from live device)
- `EXP-002.md` — engineering report

## Cross-project

Build pipeline & artifacts: `~/ai-workstation/Projects/whyred-pve-uefi`
(GitHub: Egorich-print/whyred-pve-uefi). UEFI payload and Proxmox VE ARM64
rootfs already built; flashing blocked on bootloader unlock.
