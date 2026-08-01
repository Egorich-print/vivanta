# Vivanta Status

> Last updated: 2026-08-01

## Current milestone

M2 — Virtual Memory (completed)
M3 — Process Model (in progress)

## Kernel

- PMM (Physical Memory Manager) — ✅
- Early MMU (aarch64) — ✅
- Paging API — ✅ (ADR-030: mechanism/policy split)
- Memory Resource Manager — ✅ (all allocation through MRM)
- Scheduler — ✅ (priority-based preemptive, dynamic RunQueue, sleep/wake)
- VMM (AddressSpace) — ✅ (map, unmap, query; protect deferred)
- Identity — ✅ (ADR-024: RuntimeIdentity migrated)
- System State Encapsulation — ✅ (ADR-021)
- Process Model — ✅ (Task lifecycle, parent-child, process table)
- Signals — ✅ (minimal: SIGHUP, SIGINT, SIGKILL, SIGSEGV, SIGTERM)
- Syscalls — ✅ (SVC handler: read, write, exit, yield, mmap)

## Architecture

| Layer | Status |
|-------|--------|
| PMM | ✅ PmmBitmap, self-test |
| MRM | ✅ MemoryResourceManager, MemoryObject |
| VMM | ✅ AddressSpace, MappingSet |
| Paging | ✅ descriptor, walker, mapper (ADR-030) |
| Scheduler | ✅ priority-based, dynamic RunQueue, sleep/wake |
| Process | ✅ Task, TaskManager, ProcessTable |
| Identity | ✅ RuntimeIdentity, BootIdentity, UUID |
| Signals | ✅ Signal enum, SignalState |
| Syscalls | ✅ SVC dispatch (5 syscalls) |

## Scheduler v2

- ThreadState: Created, Ready, Running, Blocked, Sleeping, Terminated
- Priority: Realtime, High, Normal, Low, Idle
- RunQueue: dynamic Vec-based, no MAX_THREADS limit
- Sleep/wake: timer-based with check_sleeping_threads()
- Preemptive: timer tick → NEED_RESCHEDULE → maybe_reschedule

## Process Model

- Task owns multiple threads (Vec<ThreadId>)
- Task owns AddressSpace (memory isolation)
- Task owns MemoryObjects (resource ownership)
- Parent-child relationships (process hierarchy)
- Process table (global registry)
- Exit codes (zombie state for parent collection)

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
- protect() (requires arch-api mmu_protect)

## Next

1. IPC primitives (message passing, shared memory)
2. User-space libc (minimal syscall wrappers)
3. First user-space program (hello world)
4. Storage driver
5. Persistent Identity
