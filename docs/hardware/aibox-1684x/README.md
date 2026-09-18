# AIBOX-1684X hardware reconnaissance

FireFly **AIBOX-1684X** (SOPHGO BM1684X) — hardware/boot-chain research for a
possible Vivanta bring-up target. Vendor firmware boots the SoC; Vivanta would
replace the Linux kernel subimage inside the vendor FIT image.

> **Provenance.** Migrated 2026-09-18 from loose files in the navigation hub
> (`~/Documents/New OpenCode Project/`) into canonical project storage, per
> `Knowledge/System/File Organization.md`. Backup of the originals:
> `~/ai-workstation/Archives/Vivanta/hub-migration-2026-09-18.tar.gz`.
> Hub keeps symlinks under the original names.

## Documents

| File | What it is |
|------|------------|
| `hardware-report.md` | Hardware + boot-chain reconnaissance (verified) |
| `second-pass.md` | Second-pass forensic verification of the report |
| `boot-contract.md` | Minimum verified boot contract: exact state Vivanta inherits at kernel entry |
| `platform-bringup.md` | `vivanta/platform/aibox_1684x/` platform design + bring-up plan |
| `evidence-ledger.md` | Evidence ledger with `[OBSERVED]` / `[UNAVAILABLE]` markers |
| `device-tree-recon.md` | DTB identity (model, hash, where it is embedded) |
| `raw-boot-log.txt` | Raw serial boot log (primary evidence) |

## Status

Research only — **no code, no bring-up attempted**. This target is not in the
milestone roadmap; the declared silicon-validation target remains RK3568 and
RPi3B+. See `AUTONOMOUS_MISSION_P0_REPORT.md` and `STATUS.md`.
