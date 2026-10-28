# Changelog

> **This is a partial journal, not a commit log.** Entries are written at
> **milestone scale** — the work that closed a milestone, changed a contract, or
> corrected a published claim. Day-to-day commits (debug traces, refactors,
> `chore:` sweeps, formatting passes) are not recorded. **Gaps in dates are
> expected and are not a defect**; where a gap looked like a defect before, it
> has been backfilled from `git log` and is now labelled as such. For the
> authoritative current state see [`STATUS.md`](../STATUS.md) and
> [`ROADMAP.md`](../ROADMAP.md); for what the QEMU gate actually verifies, see
> `vivanta-boot/tools/qemu-gates.sh` (**18** required serial markers, and the CI
> job runs `continue-on-error: true` — advisory, not blocking).

## 2026-07-11

### R2 Repository Reorganization

Added:
- `docs/architecture/repository-layout.md` — target directory structure
- `.gitignore` — exclude `target/`, `*.bin`, `.DS_Store`
- `docs/hardware/lavender/` — initial hardware notes for SDM660 target
- `docs/adr/ADR-001-rfc-freeze.md` — placeholder *(removed 2026-08-25,
  `d6735c7`; the ADR series was renumbered from ADR-011 and now lives in
  `vivanta-boot/docs/adr/`)*
- `docs/adr/ADR-002-state-versioning.md` — placeholder *(removed 2026-08-25,
  same commit; see ADR-011…ADR-015 in `vivanta-boot/docs/adr/`)*
- `specs/schemas/` — placeholder for state/environment schemas *(as added on
  2026-07-11, the two files were 5-line `TBD` stubs, and **no machine-readable
  schema has ever existed in this project** — not then, not now. Both files were
  rewritten on 2026-09-29 into honest notes: they now state that the format was
  never specified, why the defining RFCs disagree with each other and with the
  only reference implementation, and that the work is **blocked behind the scope
  fence** (no filesystem, no crypto), not pending. See
  [`specs/schemas/`](../specs/schemas/))

Changed (git mv):
- Milestone documents moved under `docs/milestones/`
- RFCs moved under `specs/rfc/`
- ADRs moved under `docs/adr/`
- `Goals/`, `MindMap/`, `research/`, `OPEN_QUESTIONS.md` archived
- `docs/architecture/master-roadmap.md` committed (`4e8c276`, 2026-07-12)

Commits: `e2139ab`, `6a62c7b`, `bbbecde`

## 2026-07-14

### ACS — Architecture Cleanup Sprint

- Repository restructuring: `boot/` → `vivanta-boot/archive/boot_legacy/`,
  `kernel/src/memory/` → `kernel-memory-frozen/`
- Arch-api: `extern "Rust"` bidirectional contract layer
- Kernel/arch/platform/target split with strict dependency direction (ADR-014)
- `arch-test-stub` / `target-test`: *claimed* build-time arch independence proof
  — **retracted 2026-09-25** (`273f9bc`): `target-test` never referenced
  `kernel_main`, so the kernel was never linked, and the stub was missing 13
  `arch-api` symbols. Both crates are deleted; the rule itself is enforced by
  dependency direction. See `docs/audit/2026-09-25-cleanup.md`.
- ADR-011 through ADR-015 — now at `vivanta-boot/docs/adr/`
  (`vivanta-boot/docs/architecture/acs-summary.md` carries the sprint's own
  record, also dated 2026-07-14)

## 2026-07-16

### M4 — Execution Foundation (Complete)

- Cooperative round-robin scheduling (3 threads)
- Thread lifecycle: create, exit, trampoline, cleanup, idle
- Timer infrastructure (CNTP; `TICK_HZ = 100` —
  `vivanta-boot/arch-aarch64/src/timer.rs:8`)
- Thread exit and cleanup verified
- Repository layout finalized
- Tag: `M4` (`4a44971`)

Next:
- M4.4 Address Spaces — fill AddressSpace, mmap/munmap/mprotect

### 2026-07-17 — 2026-07-19: RK3568 silicon, rename, RPi3B+ bootstrap

- `4a6056c` — **first UART byte on real RK3568 silicon** (Stage 1 bring-up).
  This remains the only Vivanta code that has ever executed on hardware.
- `2d6d916`, `6b034bc` — M4.5.2 diagnostic results; ARM FP/SIMD documentation
- `3cbfde2`, `2f31104` — post-audit decisions appendix and roadmap v1.1
  (4 external LLM audits consolidated; the prompt template is archived at
  `docs/history/audit-prompts/`)
- `940832b` — **project rename: TheseusOS → Vivanta**
- `44206e1`, `6b6a911` — RPi3 B+ bootstrap target, PL011 init, `EarlyPlatformInfo`

## 2026-07-20 — 2026-07-23: V0.1 and V1.1 runtime identity

- `6fdf08c`, `10e4a91`, `e9f8e8b`, `9cbaf79` — V1.1 Runtime Identity, BootInfo
  escape fix, close V1.1: RK3568 boot flow via `kernel_main`
  (identity was later frozen; `state::identity` removed in `04e4720`)

## 2026-07-28 — 2026-07-30: paging mechanism/policy split, history restructure

- `07a04d6` — `archive/` restructured into `docs/history/`; `STATUS.md` and
  `docs/REPOSITORY_GUIDE.md` added
- `7fffe63`, `768403c`, `54a1ea0` — split paging **mechanism** (arch) from
  **policy** (VMM), runtime smoke tests, and **ADR-030** (paging architecture)

## 2026-08-01 — 2026-08-03: scheduler, signals, first EL0 round-trip, public repo

- Rebuild of the execution core: `ThreadState`/invariants, priority levels,
  dynamic `RunQueue`, sleep/wake, task↔thread relations, process table, SVC
  dispatch, minimal signal model, EL0 round-trip verified (`c1a5733` … `2797e6e`)
- Memory routed entirely through MRM; VMM backend integrated; `AddressSpace`
  map/unmap/query API (`8860473` … `a6292f0`)
- Timer-driven **preemptive** reschedule wired (`b6900b6`)
- `499c430` — fix EL0 syscall return (double `ELR` increment)
- `215b8fc`, `6916026` — LICENSE (GPLv3), public README, ROADMAP, Russian README
  with language switcher, cluster research note

## 2026-08-11 — 2026-08-14: M5.0 and M6 closed; preemption proven

- M5.0: workspace integrity, physical-memory ownership and reclamation, kernel
  heap fix, user-memory boundary + fault containment (G3, `EFAULT`), scheduler
  identity repair and **timer preemption proven on QEMU** (G4, WP-04)
  (`375cc4d` … `9d5c4c8`)
- Soak-test tooling added (`tools/soak_test.sh`); orphan workspace members
  removed; `Cargo.lock` committed
- M6: process lifecycle wiring (task state, exit code, reaping) — **PASS/CLOSED**
  (`8ce1378`, `fe60729`)
- `c773dc1` — soak test surfaced the **preemption IRQ-loss** defect (P1),
  documented as INV-002; closed the same day by `4a46ceb` (console lock held with
  IRQs disabled; `ThreadContext` at stack bottom)
- `f2af711` — RPi3B+ UART bring-up **VERIFIED**. Note: the evidence is a boot of
  *BalanSir's* Buildroot image reaching a login prompt, **not** Vivanta.

## 2026-08-21: toolchain policy, W^X, and the VMM foundation

- `a094ce9`, `bab8130` — workspace migrated to **Rust 1.98.0 / edition 2024**
- `67d5b87` — W^X enforced for AArch64 user pages; `mmu_protect` + VMM
  `protect()` implemented
- `4a46ceb` — INV-002 closed (see above)
- `5861d8f` — provenance hygiene: extern statics via `&raw const`
- `df5702b` — **fix**: page-table frames were being freed while the MMU walked
  them
- `eea939a`, `4550846`, `df671ed` — page-table **ownership protocol** +
  missing-table creation, range mapping, partial protect, safe table
  reclamation, and the new `vivanta-vm` **VA range allocator** (host-proven:
  12 of today's 22 host tests live in `vivanta-boot/vm/src/va_tests.rs`)
- `9c0aa6e` — post-M5 protection audit: permission matrix, TLBI transitions
- `2ceef95` — ADR-031 ratification notes

## 2026-08-23 — 2026-08-25: M6.0 user VM, syscall ABI freeze, G-M7

- `d69a516` — **ADR-032** fault-resolution semantics written *before* the
  implementation (classification, retry proof, state machine, OOM ordering)
- `bd3d46b`, `62328fa`, `02da886` — lazy reservation, demand-fill fault
  resolution, forward **verifier**, fallible frame allocation; QEMU lifecycle,
  stress and mutation battery
- `f1eb6c9`, `5fec9e6` — M6.0 mission report and status/roadmap sync
- `69f556b` — **reverse-direction** INV-VM-001 verifier (ghost-leaf detection),
  closing a known limitation the M6.0 report had listed as backlog
- `72eb73b` — **syscall ABI frozen (ADR-033)**; `mmap`/`munmap`/`mprotect`
  implemented on the proven VMM primitives (M7.1 + M7.2)
- `08859f1` — generation-protected process handles, deterministic capacity (M7.4)
- `09dcc99` — ADR-032 §2.3 amendment: EL0 translation faults resolve through the
  same validator
- `a35bb41` — G-M7 acceptance report (now archived with a RETIRED banner at
  `docs/history/missions/` — it predates COW and the ELF loader)
- `d6735c7` — quarantined research track archived; `docs/adr/ADR-001`/
  `ADR-002` placeholders removed

## 2026-08-26 — 2026-08-28: ELF loader, COW, and the process-syscall track

- `f8039b0` — **ELF64 AArch64 loader** (`vivanta-boot/exec/`, ADR-033 policy:
  rejects W/X segments, overlaps, entry-outside-exec, `memsz < filesz`) plus
  the first genuine ELF userland (G-M8)
- `6fd73d8` — host-test imports so the loader is host-testable
- `e883af7`, `661e22e` — **copy-on-write** for anonymous private memory
  (G-M9, ADR-034)
- `8d5f757` … `71c5f1f` — G-M10: `fork`, `waitpid`, `getppid`, SIGCHLD delivery,
  `munmap`/`mprotect`, `execve` with a built-in program registry, and the P0
  fixes underneath them (fork SP bug, PID domain, `waitpid`/`kill`/`execve`
  semantics, scheduler hardening, COW tail)
- `b58adbe` — G-M11 signals: `sigaction`, blocked mask, unblockable `SIGKILL`
- `978391b` then `eacfe20` — G-M10.1 true COW, **reverted** the same day to keep
  the eager copy, so the QEMU COW gate stayed meaningful
- `1e4c2a4` then `e48b52e` — fork/`waitpid` PID-domain gate, **then disabled**:
  it polluted the AS namespace and broke downstream gates

## 2026-09-05: M10.2 real EL0 fork

- `9757f55` — kernel-half clone, frame-level COW refcount, IRQ-safe teardown.
  Also the tip of `backup/pre-p0-mission-2026-09-09`.

## 2026-09-15: P0 corrective mission, RK3568 RPi3B+ to kernel_main

- `f6ed78f` — P0 correctness: execve stack + return path + icache flush,
  `unmap_all` VA leak, `unmap_all` vs lazy pieces, process-table tombstone
  reuse, signal scope fence, MappingSet fault-capacity pre-check, gate-hygiene fix
- `cb17b7a` — six regression gates extracted as `#[inline(never)]` fns
  (**19/19** at the time; the gate has since changed — see 2026-09-18)
- `5005542` — `spec-table-desc` feature (0b11→0b10 table descriptor, for silicon
  only — it hangs QEMU, so never build it workspace-wide) and the x96q
  EL2→EL1 drop (compile-checked only, needs silicon)
- `795f119`, `b8b26c3` — audit sync, INV-003 (boot-stack overflow) report
- `5eb7936`, `aef1fcb`, `b84d58f` — signal-mask hardening, EL0 fork round-trip +
  dual-ELF gates, 64 KiB kernel stacks, EL0 child `x30`
- `251b981`, `958af2c` — `target-rpi3b-plus` brought to `kernel_main` (**never
  run on silicon**) and a one-command SD image (MBR + FAT32, EL2 drop)

## 2026-09-18

### CI/CD + repository hygiene

Added:
- `.github/workflows/ci.yml` — lint, host tests, kernel build, QEMU gate matrix,
  and the Raspberry Pi 3B+ SD image on every push/PR.
- `.github/workflows/release.yml` — `workflow_dispatch` release that publishes
  the SD image, `kernel8.img`, the QEMU kernel ELF, a flat RK3568 binary and the
  EL0 image with checksums.
- `vivanta-boot/tools/build-user-init.sh` — builds the embedded EL0 image
  (`user-init.elf` is gitignored, so this must run before any kernel build).
- `vivanta-boot/tools/qemu-gates.sh` — boots the kernel under QEMU and verifies
  **18** serial-log markers, locally and in CI. *(The "22/22 gate matrix"
  framing this entry previously carried was retired by the 2026-09-24 audit;
  the script requires 18 markers and the CI job is advisory — see 2026-09-25.)*
- `vivanta-boot/tools/ci-local.sh` — one-command local replay of the pipeline.
- `vivanta-boot/tools/rust-objcopy.sh` — resolves `rust-objcopy` from the
  toolchain sysroot (the `llvm-tools` component, not on PATH by default).
- `docs/tooling/ci-cd.md` — pipeline, release flow, local ritual, known traps.
- `vivanta-boot/tools/soak_test.sh` portability fix and macOS `mktemp` fix.

Changed:
- `tools/make-rpi3b-image.sh` is now portable: FAT32 via `mkfs.vfat` + `mtools`
  instead of macOS-only `hdiutil`, so the same script builds the image locally
  and on the CI runner.
- `vivanta-boot/rust-toolchain.toml` pins `rustfmt`, `clippy` and `llvm-tools`
  alongside the toolchain and target. *(clippy is pinned but CI never runs it.)*
- `docs/hardware/rk3568/` promoted to the canonical U-Boot/eMMC reference
  (README, runbook, scripts, evidence); the old `vivanta-boot/` copy is marked
  superseded.
- AIBOX-1684X reconnaissance migrated into `docs/hardware/aibox-1684x/`.
- README/README.ru/ROADMAP refreshed: live CI/release badges, working quick
  start, status aligned with `STATUS.md`.
- Release hardening: unique checksum asset names, `gh release create -R`,
  package step repaired after line-fold corruption, `rust-objcopy` resolved from
  the sysroot.

Gate status churn on this day, in order: added blocking (`3011c39`) → demoted to
advisory after an x86-64-TCG fork hang (`50f7909`) → moved to a native AArch64
runner with KVM (`50fecb7`) → made blocking again (`c8a14c9`). The demotion in
`029057d` (2026-09-25) is the one that still stands.

## 2026-09-24

### Global audit + three defect-fix passes

- `99a1ec9` — kernel hardware defects: `context_fork` must take the real
  `SP_EL0`; `wake_waiters` must only wake `Blocked` threads; `kill` must reject
  already-exited children and reject out-of-range signal numbers; `rt_sigreturn`
  returns `-ENOSYS`; unmap and mprotect must program hardware for **COW** runs,
  not just `Present` (use-after-free of a PMM frame)
- `16c7134` — clean-clone bootstrap (`build.sh`, `make-rpi3b-image.sh`,
  `soak_test.sh`, CI, release) and release hardening
- `a1ed7d7` — public status claims corrected; global-audit backlog opened at
  `docs/audit/2026-09-24-global-audit.md`

## 2026-09-25

### Scheduler race found, gate demoted, cleanup, doc retractions

- `4ddbbfe`, `d083f8e`, `465cbf4`, `82a2b94`, `602fe7b` — scheduler and VM fixes
  aimed at the exit-code race: do not restore a stale DAIF on the incoming
  thread; never schedule a thread before its `Task` record exists; publish
  `CURRENT_THREAD` immediately before the switch; `reserve()` must cover one
  free interval transactionally; the M10.4 teardown gate made strict and
  `waitpid` now returns `ECHILD`
- `358c692` — the race is recorded as **still open**: a timer tick can land
  inside a context switch and lose a user thread's exit code or leave a forked
  child unscheduled. Printing a trace makes it vanish (a heisenbug), which is why
  the diagnosis is hard.
- `029057d` — **the QEMU gate is now advisory** (`continue-on-error: true`) on
  the AArch64 runner. Reason: the scheduler timing race. `main` has no branch
  protection, so a green run never blocked a merge anyway. **Do not report
  "gates green" or "18/18 in CI" without this caveat.**
- `6e78a11` — `docs/tooling/ci-cd.md` records the advisory status and why
- `b4b30a0` — cleanup: build artifacts, a vendored Mach-O `rkdeveloptool` binary
  and dead code removed (118 MB → 14 MB). Deleted `target-qemu-armv7a/`,
  `arch-test-stub/` and `target-test/`. Details:
  `docs/audit/2026-09-25-cleanup.md`
- `65eff7f` — documentation and CI gaps found by independent verification closed
- `273f9bc` — the `target-test` "build-time ISA-independence proof" claim
  **retracted everywhere it is asserted**, including the archived M4 acceptance
  criteria and the archived external-review prompt.

