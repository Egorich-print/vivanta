# Vivanta Status

> Last updated: 2026-07-28

## Current milestone

M2 — Virtual Memory

## Kernel

- PMM (Physical Memory Manager) — ✅
- Early MMU (aarch64) — ✅
- Paging API — ✅
- Memory Resource Manager — in progress (ADR-025)
- System State Encapsulation — ADR-021, draft

## Platforms

| Platform | Status |
|----------|--------|
| rk3568 | Active |
| rpi3b+ | Active |
| qemu-aarch64 | Active |
| qemu-armv7a | Active |
| allwinner-h616 | Stalled |
| amlogic | Stalled |
| sdm660 | Stalled |

## Blocked

- Storage driver
- Persistent Identity model
- Userspace bootstrap

## Next

1. Finalize Memory Resource Manager
2. Scheduler state machine
3. Identity separation model (ADR-024)
