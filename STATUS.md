# Vivanta Status

> Last updated: 2026-07-30

## Current milestone

M2 — Virtual Memory

## Kernel

- PMM (Physical Memory Manager) — ✅
- Early MMU (aarch64) — ✅
- Paging API — ✅ (ADR-030: mechanism/policy split)
- Memory Resource Manager — ✅ (all allocation through MRM)
- Scheduler — ✅ (timer-driven preemptive reschedule)
- VMM (AddressSpace) — ✅ (map, unmap, query; protect deferred)
- Identity — ✅ (ADR-024: RuntimeIdentity migrated)
- System State Encapsulation — ✅ (ADR-021)

## Architecture

| Layer | Status |
|-------|--------|
| PMM | ✅ PmmBitmap, self-test |
| MRM | ✅ MemoryResourceManager, MemoryObject |
| VMM | ✅ AddressSpace, MappingSet |
| Paging | ✅ descriptor, walker, mapper (ADR-030) |
| Scheduler | ✅ round-robin, 8 threads, preemptive |
| Identity | ✅ RuntimeIdentity, BootIdentity, UUID |

## Platforms

| Platform | Status |
|----------|--------|
| qemu-aarch64 | Active, boots to kernel_main |
| rk3568 | Active (stuck at Stage 1) |
| rpi3b+ | Active |
| qemu-armv7a | Active |
| allwinner-h616 | Stalled |
| amlogic | Stalled |
| sdm660 | Stalled |

## Blocked

- Storage driver
- Persistent Identity model
- Userspace bootstrap
- protect() (requires arch-api mmu_protect)

## Next

1. Scheduler v2 (priority, sleeping/blocked states)
2. Process model (Task → Thread → AddressSpace)
3. EL0 / Userspace support
4. IPC primitives
