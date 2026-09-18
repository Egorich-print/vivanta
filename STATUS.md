# Vivanta Status

> Last updated: 2026-09-18 (CI/CD added; QEMU gate matrix re-verified locally
> at 22/22 PASS — see `docs/tooling/ci-cd.md`)
> Prior update: 2026-09-09 (P0 mission — see "P0 corrective mission" below)

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
`vivanta-boot/docs/investigations/MMU-descriptor-encoding-hardware-validation.md`).
60-min soak is tooled (`vivanta-boot/tools/soak_test.sh`) and pending a full run.

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
- Process Model — ✅ fork/waitpid/kill/getpid/getppid/exit/execve live (M10.2);
  tombstones reused with generations, live_count excludes Exited (P0 mission)
- Signals — ⚠️ dispositions + SIGKILL/SIGCHLD paths work; async EL0 delivery
  explicitly absent (scope fence, documented in `kernel/src/signal.rs`)
- Syscalls — ✅ 14/16 live (READ + SIGPROCMASK reserved → ENOSYS per ADR-033);
  mmap/munmap/mprotect + negatives proven from EL0 (M7.2 + user-init exit 42)
- User threads — ✅ EL0 demo, EFAULT test, fault-containment test all pass
- ELF/execve — ✅ genuine EL0 `execve("/init")` → exit(42) proven (M10.3 gate);
  new-image stack at top of user domain; icache maintained on exec path

## Post-M5 deferred artifacts

1. **G4+ soak** — `vivanta-boot/tools/soak_test.sh` (default 60 min). **Note: soak run
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
| qemu-aarch64 | Active, boots to kernel_main, 22/22 gates PASS (0 panics) |
| rk3568 | Diagnostic only (does not link vivanta-kernel) |
| rpi3b+ | Standalone diagnostic (early_mmu identity map) |
| qemu-armv7a | Frozen (arch-armv7a is an empty stub; removed from workspace members) |
| allwinner-h616 / amlogic / sdm660 | Stalled / planned (x96q gained an EL2→EL1 drop, compile-checked only) |

## P0 corrective mission (2026-09-09)

Full log: `AUTONOMOUS_MISSION_P0_REPORT.md`. Forensic deep-dive on the
stack overflow: `vivanta-boot/docs/investigations/INV-003-boot-stack-overflow.md`.

Fixed all §3.1-class defects found by the 2026-09-08 audit, with regression
gates in the QEMU boot matrix (now 19 PASS, 0 panics):

- `execve` stack moved into the user VA domain (was above `USER_VA_END`, every
  execve failed); new image gets icache maintenance; frame.sp no longer
  overwritten (SVC epilogue restores SP_EL1 from it — the old code hung the
  return path); user SP installed via `msr sp_el0`.
- `unmap_all` releases VA reservations (`va.free`) and skips HW unmap for
  Lazy/Reserved pieces (previously panicked the walker on never-touched Lazy).
- Process table: `live_count`/`count` exclude `Exited` tombstones; `create`
  reuses tombstone slots preserving bumped generations (was a lifetime cap of
  64 tasks plus unbounded Vec growth → heap OOM).
- Signals: async EL0 delivery documented as absent (scope fence);
  `SignalState::take()` removed; self-SIGKILL now terminates via `thread_exit`.
- Fault paths: capacity pre-check before HW mutation (was `.expect()` panic).
- Boot stack 128→256 KiB: the 82 KiB `-O0` kernel_main frame plus gate
  temporaries overflowed into `.bss` (silent console death pre-dump); gates
  now live in `#[inline(never)]` fns.
- MMU table encoding behind `spec-table-desc` feature (default QEMU `0b11`
  unchanged; feature build hangs at MMU enable exactly as documented).
- Follow-up (same day, YOLO): thread stacks 16→64 KiB (fork path measured
  18 KiB past the bottom — second INV-003 instance); `context_fork` sets
  child x30 to `eret_to_user_stub` (first scheduled fork child jumped to
  ELR=0 before); gates M7.12-dual-ELF and M10.4-EL0-fork green. Matrix at
  22/22 PASS (13 pre-existing + 9 new), 0 panics.

Deferred with rationale: `UserPtr` adoption in hot syscalls, `register()`
panic→Result conversion, kernel W^X, MAIR/TCR unification — all touch proven
paths for no P0 gain; revisit after next functional milestone.

## CI QEMU gate (2026-09-18)

The gate boots the kernel under QEMU on an **AArch64 runner** (`ubuntu-24.04-arm`),
so host and guest share an architecture and the run is fast and deterministic.
GitHub does not expose `/dev/kvm` on hosted runners; the job auto-selects KVM if
that changes. The job is **blocking**.

History: on an x86_64 runner the gate ran under cross-ISA TCG and intermittently
deadlocked at the M10.4 EL0-fork gate (`fork()` returns, then the parent's
`waitpid` is never satisfied). Re-running on an AArch64 runner cleared it —
same-arch TCG, no cross-ISA translation. See `docs/tooling/ci-cd.md`.

## Scope fence (holds through any next milestone until explicitly lifted)

IPC · storage · drivers · distributed AI (ADR-031…039) · Ed25519 · BIP-39 ·
persistent identity · TTBR1/ASID · signal delivery · new hardware targets ·
new architectures
