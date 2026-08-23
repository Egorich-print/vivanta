# Vivanta Status

> Last updated: 2026-08-21 (mission 2)

## Toolchain

**Rust 1.98.0 stable, edition 2024** for the whole workspace, pinned via
`vivanta-boot/rust-toolchain.toml` (includes the `aarch64-unknown-none`
target). No nightly features; host tests, clippy and the freestanding
kernel build share one compiler.

## Virtual memory (M5.1/M5.2, 2026-08-21)

- VA allocator (`vivanta-vm`) — ✅ first-fit intervals, model-checked
  20k-op host stress; user domain 0x01000000–0x40000000, page-0 guard
- Page-table ownership registry — ✅ every runtime table frame tracked
  (frame/as_id/level/parent/backend); roots + boot-era frames leak by rule
- Table reclamation — ✅ hardware-proven emptiness → IRQ-guarded unlink →
  PMM return; exercised in QEMU with PMM delta asserts
- Range protect/unmap — ✅ partial ranges with transactional shadow
  splitting; `MappingSet` stays an exact hardware image
- Aliasing — ✅ VAs may alias; PA ownership never follows unmap
  (regression-tested)
- ADR-031 ratified: `vivanta-boot/docs/adr/ADR-031-va-page-table-ownership.md`

## User VM / fault-driven mapping (M6.0, 2026-08-21)

**M6.0 CLOSED** — закрытие базовых fault/recovery и transactional paging
semantics (не «lazy paging implementation»). Зафиксированные свойства:

- `MappingSet` — authoritative VM state; hardware tables — производная
  материализация, проверяемая механически (INV-VM-001);
- ровно один возобновляемый класс EL1 faults (ADR-032); retry без
  изменения ELR — скрытый skip instruction доказуемо отсутствует;
- LazyAnonymous как транзакция: commit-last, OOM оставляет Lazy;
- ownership разделён: mapping ≠ physical frame; VMM не освобождает PA.

Здоровые границы (не архитектурные дыры): OOM при реальном исчерпании
512 MiB не тестировался; M4 rollback = structural; ASID и per-VA TLBI —
backlog за консервативным full-flush.

- Fault policy ADR-032 — ✅ one resolvable class (EL1 data-abort
  translation fault on a LazyAnonymous piece, access ⊆ perms); permission
  faults, instruction aborts, Reserved pieces and OOM stay fatal;
  same-instruction retry with unmodified ELR (no `elr += 4` anywhere)
- Mapping state machine — ✅ `Backing::{Present, LazyAnonymous, Reserved}`
  inside `Mapping`; backing metadata lives in MappingSet (no second
  registry)
- Demand fill — ✅ page-granular materialization; transactional order
  validate → allocate+zero → map → shadow-commit-last; OOM leaves mapping
  Lazy (deterministic unit coverage)
- mprotect/munmap on Lazy — ✅ metadata-only until materialization; fills
  use post-mprotect permissions; anonymous frames return to PMM on unmap
- MappingSet ⇔ hardware verifier — ✅ mechanical per-piece check
  (Present ⇔ valid leaf + exact permission bits; Lazy ⇔ no leaf)
- Limitations: MappingSet fixed at 64 slots (demo-scale, heap-backed
  storage is follow-up); MAX_ADDRESS_SPACES=8 retained (fault path
  identifies AS by TTBR0 match — no ID reuse possible); EL0-originated
  lazy fills not yet resolved (containment unchanged)

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
- VMM (AddressSpace) — ✅ map/unmap/protect (protect added 2026-08-21 via
  arch-api mmu_protect; whole-mapping granularity; VA allocator post-M5)
- W^X (user pages) — ✅ enforced 2026-08-21 (was silently broken: user code
  pages were EL0-writable; see
  `vivanta-boot/docs/investigations/WX-user-code-ap-encoding.md`;
  boot-time `[WX]` readback verification + EL0 store-to-code-page negative test)
- Identity — ⚠️ nominal only (counter-based UUID; no crypto/Ed25519 — scope fence)
- Process Model — ⚠️ lifecycle incomplete: `Task::exit()` never called,
  `running_count()` always 0, exit_code/zombie/reap are dead APIs (candidate M6)
- Signals — ⚠️ enum only, no delivery path (scope fence)
- Syscalls — ⚠️ SYS_READ is a stub returning 0; mmap returns -ENOMEM (post-M5)
- User threads — ✅ EL0 demo, EFAULT test, fault-containment test all pass

## Post-M5 deferred artifacts

1. **G4+ soak** — `tools/soak_test.sh` (default 60 min). **Note: soak run
   surfaced INV-002 (preemption IRQ-loss under sustained load) — see below.
   Soak must pass consistently before it is trusted as a reliability gate.**
2. **MMU descriptor encoding** — HW-validation plan documented; requires
   physical ARM64 hardware.
3. **Orphan workspace members removed** (`kernel-memory-frozen`, `user/hello`,
   `user/libc`); directories kept per ADR-011.

## Known issues

- **P1 — INV-002: preemption IRQ-loss under sustained load** (pre-existing
  M5.0-path defect, surfaced by the soak). Under long timer preemption the
  kernel loses ticks (tight loop at ~99% CPU) or crashes
  (EL1h Instruction Abort, `x30=0`). See
  `vivanta-boot/docs/investigations/INV-002-preemption-irq-loss.md`.
  Blocks long-running multi-thread workloads; must be fixed before the
  preemption claim is considered reliable.
- M5.0 G4 "preemption proven" is true for short runs (60 s smoke, manual
  runs) but is NOT reliable over minutes until INV-002 is resolved.

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
