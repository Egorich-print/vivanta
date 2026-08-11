# Vivanta Status

> Last updated: 2026-08-11

## Current milestone

**M5.0 GREEN BASELINE — PASS / CLOSED** (2026-08-11)
Ratified spec: `vivanta-boot/docs/milestones/M5.0-green-baseline.md`

Honest status: **M5.0 QEMU-correct baseline**, NOT "hardware-correct". One
deferred ARM MMU portability issue (L1/L2 table descriptor encoding, see
`docs/investigations/MMU-descriptor-encoding-hardware-validation.md`).
60-min soak is tooled (`tools/soak_test.sh`) and pending a full run.

## M5.0 gates — all PASS (verified on clean clone + QEMU)

- G1 Workspace integrity — PASS
- G2 Physical ownership + reclamation — PASS (511 MiB managed, churn delta=0)
- G3 User boundary + fault containment — PASS (EFAULT, fault-kill, W^X)
- G4 Scheduler + preemption — PASS (ThreadId current, Running==1, 100 Hz A↔B, 60s smoke)

## Kernel

- PMM (Physical Memory Manager) — ✅ full usable RAM, self-test + stress
- Paging / MMU (aarch64) — ✅ (ADR-030 split; descriptor encoding deferred to HW)
- Memory Resource Manager — ✅ reclamation proven (Drop→deallocate, churn delta=0)
- Kernel heap — ✅ free-list allocator with reclamation (was bump-leak-all)
- Scheduler — ✅ ThreadId-based current, Running invariant, timer preemption
- VMM (AddressSpace) — ⚠️ `protect()` still `todo!` (needs arch-api mmu_protect);
  no VA allocator (post-M5)
- Identity — ⚠️ nominal only (counter-based UUID; no crypto/Ed25519 — scope fence)
- Process Model — ⚠️ lifecycle incomplete: `Task::exit()` never called,
  `running_count()` always 0, exit_code/zombie/reap are dead APIs (candidate M6)
- Signals — ⚠️ enum only, no delivery path (scope fence)
- Syscalls — ⚠️ SYS_READ is a stub returning 0; mmap returns -ENOMEM (post-M5)
- User threads — ✅ EL0 demo, EFAULT test, fault-containment test all pass

## Post-M5 deferred artifacts

1. **G4+ soak** — `tools/soak_test.sh` (default 60 min), validated on 30s run;
   full run pending.
2. **MMU descriptor encoding** — HW-validation plan documented; requires
   physical ARM64 hardware.
3. **Orphan workspace members removed** (`kernel-memory-frozen`, `user/hello`,
   `user/libc`); directories kept per ADR-011.

## Platforms

| Platform | Status |
|----------|--------|
| qemu-aarch64 | Active, boots to kernel_main, EL0 demo + preemption work |
| rk3568 | Diagnostic only (does not link vivanta-kernel) |
| rpi3b+ | Standalone diagnostic (early_mmu identity map) |
| qemu-armv7a | Frozen (arch-armv7a is an empty stub; removed from workspace members) |
| allwinner-h616 / amlogic / sdm660 | Stalled / planned |

## Scope fence (holds through any next milestone until explicitly lifted)

IPC · storage · drivers · distributed AI (ADR-031…039) · Ed25519 · BIP-39 ·
persistent identity · TTBR1/ASID · signal delivery · new hardware targets ·
new architectures
