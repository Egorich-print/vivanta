# Vivanta Status

> Last updated: **2026-09-29** (correction pass — no new code, no new silicon
> evidence). This revision fixes five self-contradictions in this file and
> replaces every number with one re-verified against source at HEAD `273f9bc`:
> the syscall count (13/16 live, not 14/16), the QEMU gate count (18 CI
> markers, not 19 or 22), CI status (advisory, not "18/18 PASS"), the RPi3B+
> platform row (kernel linked and `kernel_main` wired, never run on silicon),
> and the INV-002 entry (closed since 2026-08-11, not an open P1 — the
> still-open scheduler race is a *different* bug). Known issues are now the
> verified list from [docs/audit/2026-09-24-global-audit.md](docs/audit/2026-09-24-global-audit.md).
> Prior update: 2026-09-25 (QEMU gate moved to an AArch64 runner, made
> advisory). Prior: 2026-09-09 (P0 corrective mission — see
> [P0 corrective mission](#p0-corrective-mission-2026-09-09) below).
>
> One thing this document has never been able to say honestly: **Vivanta has
> never run a kernel to userspace on any physical board.** QEMU is the only
> working configuration. See [Hardware evidence tiers](#hardware-evidence-tiers).

## Toolchain

**Rust 1.98.0 stable, edition 2024** for the whole workspace, pinned via
`vivanta-boot/rust-toolchain.toml` (includes the `aarch64-unknown-none`
target). No nightly features; host tests and the freestanding kernel build
share one compiler.

`clippy` is pinned in the toolchain file but **CI never runs it**.

`user-init/user-init.elf` is gitignored yet embedded into the kernel by
`include_bytes!` (`vivanta-boot/kernel/src/syscall/process.rs:544-545`), so a
fresh clone cannot build `vivanta-kernel` until
`vivanta-boot/tools/build-user-init.sh` runs. Every entry point does this for
you (`vivanta-boot/build.sh`, `tools/qemu-gates.sh:52-53`,
`tools/soak_test.sh`, `.github/workflows/ci.yml`).

## Build shape (verified, `vivanta-boot/Cargo.toml`)

- Workspace: **20 members** — `vm`, `exec`, `boot-info`, `boot_common`,
  `arch-api`, `arch-aarch64`, `arch-armv7a`, `kernel`,
  `platform-{qemu,rk3568,sdm660,allwinner-h616,amlogic,rpi3b}`,
  `target-{qemu-aarch64,rk3568,x96q,lavender,rpi3b-plus,suma-q5}`.
- `user-init/` is a **separate workspace** (`[workspace]` in its own
  `Cargo.toml`), built by script and embedded.
- Live-tree Rust: **18 365 lines** (20 workspace crates = 18 254, plus the
  111-line `user-init`). Largest: `kernel` 8658, `arch-aarch64` 4172,
  `boot_common` 1680, `vm` 785, `arch-api` 713, `exec` 549.
- Not built, not workspace members (dead, kept per ADR-011): `kernel-memory-frozen/`,
  `user/hello` + `user/libc` (superseded by `user-init`), `archive/boot_legacy/`.
- `arch-armv7a` is an **8-line frozen stub** (ADR-011). There is **no ARMv7
  target**; `target-qemu-armv7a` was deleted 2026-09-25. There is **no PSCI,
  no SMP, no secondary-CPU code** anywhere in the tree.
- **Zero `todo!()` and zero `unimplemented!()`** in the whole tree. Gaps live in
  prose, in `expect()`, and in the audit backlog — not in panicking stubs.

## Virtual memory (M5.1/M5.2, 2026-08-21)

- VA allocator (`vivanta-vm`) — ✅ first-fit intervals, model-checked
  20k-op host stress; user domain `0x0100_0000`–`0x4000_0000`, page-0 guard
- Page-table ownership registry — ✅ every runtime table frame tracked
  (frame/as_id/level/parent/backend); roots + boot-era frames leak by rule
- Table reclamation — ✅ hardware-proven emptiness → IRQ-guarded unlink →
  PMM return; exercised in QEMU with PMM delta asserts
- Range protect/unmap — ✅ partial ranges with transactional shadow
  splitting; `MappingSet` stays an exact hardware image
- Aliasing — ✅ VAs may alias; PA ownership never follows unmap
  (regression-tested)
- ADR-031 ratified: [vivanta-boot/docs/adr/ADR-031-va-page-table-ownership.md](vivanta-boot/docs/adr/ADR-031-va-page-table-ownership.md)

## Paging (verified)

| Property | Reality |
|---|---|
| Page size | **4 KiB only** |
| Levels | L1 → L2 → L3 (no L4) |
| VA width | **39-bit** — `T0SZ=25`, TG0=4K, SH=inner, IPS=3 |
| Registers | `mair_el1 = 0x44_FF`, `tcr_el1 = 25\|IRGN01\|ORGN01\|SH11\|TG0 00\|IPS3` (`arch-aarch64/src/mmu.rs:239-244`) |
| 2 MiB blocks | **boot identity map only**; split back to 4 KiB on demand (`paging/walker.rs`) |
| Table descriptor | `0b11` (spec-reserved) **by default** — QEMU hangs on `0b10`. The `spec-table-desc` feature is for silicon and **hangs QEMU**; never run a bare `cargo build --workspace` |

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
- Mapping state machine — ✅ `Backing::{Present, CoW, LazyAnonymous, Reserved}`
  inside `Mapping`; backing metadata lives in MappingSet (no second
  registry)
- Demand fill — ✅ page-granular materialization; transactional order
  validate → allocate+zero → map → shadow-commit-last; OOM leaves mapping
  Lazy (deterministic unit coverage)
- mprotect/munmap on Lazy — ✅ metadata-only until materialization; fills
  use post-mprotect permissions; anonymous frames return to PMM on unmap
- MappingSet ⇔ hardware verifier — ✅ mechanical per-piece check
  (Present ⇔ valid leaf + exact permission bits; Lazy ⇔ no leaf)
- Limitations: `MappingSet` is a **fixed `[Option<Mapping>; 64]`**
  (`kernel/src/vmm/mapping.rs:147-150`, `MAX_MAPPINGS=64`) — demo-scale,
  heap-backed storage is follow-up. `MAX_ADDRESS_SPACES = 16`
  (`kernel/src/vmm/address_space.rs:19`), not 8 (fault path identifies AS by
  TTBR0 match — no ID reuse possible). EL0-originated lazy fills not yet
  resolved (containment unchanged). `MAX_TABLES=256`, `MAX_COW_FRAMES=256`.

> `vivanta-boot/vm/` (785 lines) is **not a VMM** — it is only a VA-range
> allocator (`VaAllocator`, first-fit interval list, `MAX_FREE_RANGES=256`,
> `PAGE_SIZE=4096`). The real VMM is `kernel/src/vmm/` (1896 lines, 6 files).

## Current milestone

**M5.0 GREEN BASELINE — PASS / CLOSED** (2026-08-11)
Ratified spec: [vivanta-boot/docs/milestones/M5.0-green-baseline.md](vivanta-boot/docs/milestones/M5.0-green-baseline.md)

**Next mission (2026-10) — PLANNED.** Executable plan:
[`docs/planning/mission-2026-10/00-mission-charter.md`](docs/planning/mission-2026-10/00-mission-charter.md)
with six appendices (scheduler race, memory correctness, process lifecycle,
silicon blockers, functional roadmap, CI/repo). Done-criteria: deterministic
race reproduction, honest gates + blocking CI, single teardown + frame
ownership, programmable userland (`execve` transactions, `argv`/`envp`,
signal delivery).

Honest status: **M5.0 QEMU-correct baseline**, NOT "hardware-correct". One
deferred ARM MMU portability issue (L1/L2 table descriptor encoding, see
`vivanta-boot/docs/investigations/MMU-descriptor-encoding-hardware-validation.md`).
60-min soak is tooled (`vivanta-boot/tools/soak_test.sh`, default 3600 s) and
pending a full run.

## M5.0 gates — all PASS (verified on clean clone + QEMU)

- G1 Workspace integrity — PASS
- G2 Physical ownership + reclamation — PASS (churn delta=0)
- G3 User boundary + fault containment — PASS (EFAULT, fault-kill, W^X)
- G4 Scheduler + preemption — PASS (ThreadId current, Running==1, 60s smoke)

## Memory management (verified)

| Subsystem | Reality |
|---|---|
| PMM | **Bitmap** (`FRAME_SIZE=4096`). `alloc_frame` is a **linear scan from frame 0** (`kernel/src/pmm.rs:20-29`) — no buddy, no slab, no zones |
| Kernel heap | **Free-list with coalescing, fixed 64 KiB** (`kernel/src/lib.rs:672`, `KernelHeap::init` in `kernel/src/memory/kernel_heap.rs`). `#[global_allocator]` at `kernel/src/lib.rs:9-10` |
| MRM | Reclamation proven (Drop→deallocate, churn delta=0) |

## Exceptions and interrupts (verified)

- **6 of 16 vectors return via `eret`; 10 halt the CPU**
  (`arch-aarch64/src/vectors.rs:252-269`). The six resumable ones are
  `el1h_sync` (`save_and_eret_sync_el1` — the only resumable EL1 class,
  demand-fill + COW fault resolution), `lower_aarch64_sync`
  (`save_and_eret_sync` — dispatches SVC and EL0 faults), and the four IRQ
  vectors `el1t_irq`, `el1h_irq`, `lower_aarch64_irq`, `lower_aarch32_irq`
  (all `save_and_eret`). Everything else halts — including
  `lower_aarch32_sync`, so there is no AArch32 EL0 support.
- GIC: **v2 and v3 implemented** (`arch-aarch64/src/interrupts/gic.rs`),
  selected by FDT `compatible` containing `gic-v3`.
  **No RWP polling** on distributor writes. `irq_init` uses a hard `.expect()`
  on the FDT interrupt-controller node (`arch-aarch64/src/boot.rs:173`).
- **IRQ masking works** — `disable_interrupts()`/`enable()` use
  `DAIFSet/DAIFClr #2`, which set/clear the IRQ mask (I). The earlier
  "does not work" finding was **disproven** (an `imm4` encoding inversion);
  see [Known open defects](#known-open-defects-verified-2026-09-29), item 1.

## Scheduler (verified)

- **Round-robin over a 5-level priority ladder** (`Realtime, High, Normal,
  Low, Idle`; `kernel/src/scheduler/thread.rs:8-14`). `RunQueue::find_next_ready`
  is a **linear scan** of `Vec<Option<Thread>>` (`scheduler/runqueue.rs:79-122`).
- Timer `TICK_HZ = 100` (`arch-aarch64/src/timer.rs:8`). Every tick requests a
  reschedule — **there is no quantum accounting**.
- Cooperative `yield_now()` is the primary scheduling path.
- `MAX_TASKS = 64` (`scheduler/task.rs:21`), `KERNEL_STACK_SIZE = 65536`.

## Kernel

- PMM (Physical Memory Manager) — ✅ all usable RAM, self-test + stress
- Paging / MMU (aarch64) — ✅ (ADR-030 split; descriptor encoding deferred to HW)
- Memory Resource Manager — ✅ reclamation proven (Drop→deallocate, churn delta=0)
- Kernel heap — ✅ free-list allocator with reclamation (was bump-leak-all)
- Scheduler — ✅ ThreadId-based current, Running invariant, 100 Hz preemption
  **under QEMU only** — see the race in Known issues
- VMM (AddressSpace) — ✅ map/unmap/protect (protect added 2026-08-21 via
  arch-api mmu_protect; whole-mapping granularity; VA allocator post-M5)
- W^X (user pages) — ✅ enforced 2026-08-21 (was silently broken: user code
  pages were EL0-writable; see
  `vivanta-boot/docs/investigations/WX-user-code-ap-encoding.md`;
  boot-time `[WX]` readback verification + EL0 store-to-code-page negative test)
- Identity — ⚠️ nominal only (counter-based UUID in
  `kernel/src/identity/uuid.rs:20-29`; no crypto/Ed25519 — scope fence)
- Process Model — ✅ fork/waitpid/kill/getpid/getppid/exit/execve live (M10.2);
  tombstones reused with generations, live_count excludes Exited (P0 mission).
  `waitpid` does **not** release the child — see Known issues
- Signals — ⚠️ dispositions + SIGKILL/SIGCHLD paths work; async EL0 delivery
  explicitly absent (scope fence, documented in `kernel/src/signal.rs`).
  `MAX_SIG=32`; `pending` is write-only by design
- Syscalls — ✅ **13 of 16 live**, 3 reserved → `-ENOSYS`:
  live = `write(1) exit(2) yield(3) mmap(4) munmap(5) mprotect(6) fork(7)
  waitpid(8) kill(9) getpid(10) getppid(11) execve(12) rt_sigaction(13)`;
  reserved = **`read(0)`, `rt_sigprocmask(14)`, `rt_sigreturn(15)`**
  (`kernel/src/syscall.rs:24-39` numbers, `:110-140` dispatcher, where each
  reserved arm carries its own `-ENOSYS` + comment). ABI frozen in ADR-033:
  number in `x8`, args `x0..x5`, result `x0` (`>=0` ok, `<0` = `-errno`).
  `MAX_MMAP_BYTES = 64 MiB`, `WRITE_BUF_SIZE = 256`.
  mmap/munmap/mprotect + negatives proven from EL0 (M7.2 + user-init exit 42)
- User threads — ✅ EL0 demo, EFAULT test, fault-containment test all pass
- ELF/execve — ✅ genuine EL0 `execve("/init")` → exit(42) proven (M10.3 gate);
  new-image stack at top of user domain. `execve` resolves **only** a
  compile-time `BUILTIN_PROGRAMS` table and **ignores `argv`/`envp`**
  (`kernel/src/syscall/process.rs:544-545, 582-583`)

## Tests (verified)

| Where | Count | In CI? |
|---|---:|---|
| `vm/src/va_tests.rs` | 12 | ✅ |
| `exec/src/elf_tests.rs` | 10 | ✅ |
| `boot-info/src/mmap.rs` | 3 | ❌ host-runnable, never run |
| `boot_common/src/memory_discovery.rs` | 3 | ❌ host-runnable, never run |
| **Total** | **28** (22 run in CI) | |

CI runs `cargo test -p vivanta-vm -p vivanta-exec --target x86_64-unknown-linux-gnu`
(`.github/workflows/ci.yml:53`) — that is the **22**. The other 6 are host-runnable
and CI never invokes them. `kernel` and `arch-aarch64` set `[lib] test = false`
by design: their verification *is* the QEMU gate. No `#[ignore]`, no
`#[should_panic]`, no integration tests anywhere.

## Userspace (verified)

Exactly **one** real program: `user-init/` — 101 lines of `global_asm!` (plus a
10-line `build.rs`), fully naked assembly (no Rust, no stack), exercising
mmap/store/mprotect/munmap/exit(42) and 3 negative-errno cases. The `Hello, Vivanta!` string the gate matches is an
**assembly blob inside the kernel** (`arch-aarch64/src/user.rs:107`), not output
from `user/hello` (which is dead and cannot be built).

## ADRs (verified)

**18 files** in `vivanta-boot/docs/adr/` — ADR-011…015, 017…023, 030…035.
**ADR-016, ADR-024 and ADR-025 do not exist**; any document citing them is
wrong. ADR-035 is still `Proposed`. Two known doc/code drifts: ADR-033 §3
tabulates only syscalls 0–6 (it stops at MPROTECT) while the code defines
0–15 (`kernel/src/syscall.rs:24-39`), and ADR-035 says `MAX_PROCESSES=256`
while the code has `MAX_TASKS=64`.

## Post-M5 deferred artifacts

1. **G4+ soak** — `vivanta-boot/tools/soak_test.sh` (default 60 min). A full
   60-minute run is still pending; the soak must pass consistently before it
   is trusted as a reliability gate.
2. **MMU descriptor encoding** — HW-validation plan documented
   (`vivanta-boot/docs/investigations/MMU-descriptor-encoding-hardware-validation.md`);
   requires physical ARM64 hardware.
3. **Orphan workspace members removed** (`kernel-memory-frozen`, `user/hello`,
   `user/libc`); directories kept per ADR-011.

## Known open defects (verified 2026-09-29)

Source of truth: [docs/audit/2026-09-24-global-audit.md](docs/audit/2026-09-24-global-audit.md).
Line numbers below were re-checked at HEAD `273f9bc`.

### P0 — correctness

1. **IRQ masking is correct (earlier finding disproven).**
   `arch-aarch64/src/interrupts.rs:13-28` — `enable()` executes
   `msr DAIFClr, #2` and `disable_interrupts()` (`:41-47`) executes
   `msr DAIFSet, #2`. The `DAIFSet`/`DAIFClr` `imm4` field order is bit3=D,
   bit2=A (SError), bit1=I (IRQ), bit0=F. `#2` is therefore the IRQ mask (I),
   which is what an IRQ guard needs; `#4` would set the SError mask (A).
   `PSTATE.I` is bit 7, mirrored as bit 1 of `imm4`, so it **is** set/cleared
   via `DAIFSet/Clr #2`. The default `#2` masking path is correct.
   **Correction (2026-09-29):** the audit previously read the `imm4` field in
   the wrong order and concluded `#2` was SError, i.e. that IRQ masking was
   broken. That was wrong (empirically verified by reading DAIF after each
   instruction on QEMU). The project's own `#4` experiment then misfired
   because `#4` sets the SError mask, so `enable()` never cleared the IRQ
   mask and the timer never ticked — a defect in the experiment, **not** a GIC
   delivery bug. There is no evidence of a GIC IRQ delivery defect. See
   `docs/audit/2026-09-24-global-audit.md` (dated correction note).
2. **Scheduler race** — a tick can land inside a context switch, losing a user
   thread's exit code or leaving a forked child unscheduled. This is real and
   unresolved; it is **not** caused by unmasked interrupts (they are masked
   correctly, per item 1). The race window
   is between `CURRENT_THREAD.store(next)` and entry into the new thread: a
   tick there saves the outgoing thread's registers into the *incoming*
   thread's context. Local macOS host: ~1 run in 15–35. GitHub
   `ubuntu-24.04-arm`: the workflow's own comment *estimates* ~1 in 2–5; the
   measurement is **9 failures in 21 non-cancelled `main` runs, ≈1 in 2.3**
   (see "CI honesty" below). The diagnostic branch
   **`diag/exit-code-race`**
   (1 unmerged commit, `6c7a9dc`) carries tracing that **makes the bug
   disappear** (45/45 green) — UART output is slow enough to separate the
   events in time. It is a heisenbug and needs a deterministic failpoint
   (QEMU `-icount`, `-d int`, or a breakpoint), not tracing. **This is the
   root cause of the advisory CI gate below.**
3. **COW ownership bug** — `kernel/src/vmm/address_space.rs:599-635` converts
   any `Backing::Present` into `PhysOwnership::CoWShared { refcount: 2 }`,
   including pieces whose prior ownership was `External` (from a
   `MemoryObject`). The last owner will free a frame the VMM never owned.
   Violates ADR-031.
4. **`waitpid` leaks** — `kernel/src/syscall/process.rs:213+` removes only the
   process-table entry; the child's address space, its mappings and its
   threads survive, and `register_child` panics when the pool is exhausted.
   Every fork/exit/wait cycle loses one address space. The M10.4 gate
   **manually performs the teardown `waitpid` should do**, which masks the
   defect from the gate itself.
5. **No EL0 FP/SIMD context save** — `d8-d15`, FPSR and FPCR are not
   preserved across a context switch. Latent while the EL2-path FP/SIMD defect
   below stands (`CPACR_EL1.FPEN` never leaves reset `0b00` on those paths);
   on the EL1 entry path `FPEN` *is* set, and there it is silent user-state
   corruption.
6. **GICD writes with no RWP polling** — the distributor is enabled and
   targeted without waiting for RWP; CPU IRQs are enabled before the timer is
   registered.
7. **`execve` icache maintenance covers only the entry page**, not the whole
   executable segment, so a multi-page `PF_X` region can execute from a stale
   I-cache on this CPU.
8. **`arch-api`'s `syscall_dispatch` declaration is wrong** —
   `arch-api/src/syscall.rs:18-27` declares 8 parameters, the real signature
   has 9 (it takes `as_root: u64` first). `arch-aarch64` declares its own
   private copy (`arch-aarch64/src/user.rs:468-480`) and the arch-api one is
   never called. It survives only because `extern "Rust"` mangles by arity;
   **anyone using it gets UB.**

### P0 — silicon blockers

- Boot stack is **16 KiB** in `linker.ld` for `x96q`, `suma-q5`, `rk3568`,
  `lavender` (256 KiB only for `qemu-aarch64` and `rpi3b-plus`) — and
  `x96q`/`suma-q5` *do* reach `kernel_main` with the same ~82 KiB `-O0` frame
  that forced the earlier bump.
- **FP/SIMD is never enabled on the EL2 path of 5 of 6 targets**, in two
  independent ways: (a) the EL2 branch writes `msr CPTR_EL2, #(0b11<<20)`, but
  `0b11 << 20` is `CPACR_EL1.FPEN[21:20]` and **`CPTR_EL2.TFP` is bit 10** —
  bits 20:21 of `CPTR_EL2` are RES0, so the store is an **architecturally
  no-op** and FP/SIMD control stays at its reset state; (b) the
  `msr CPACR_EL1` at the branch-fallthrough label is unreachable from the EL2
  entry, so **`CPACR_EL1.FPEN` stays at reset `0b00`, which traps FP/SIMD to
  EL1**. `rpi3b-plus` and `x96q` are called out here because they are the two
  boards nearest to silicon, and `rpi3b-plus` has no `msr CPACR_EL1` at all.
  `target-rk3568:190-191` is the only correct implementation — and the one
  target that never links the kernel. This is invisible under QEMU because
  the gate builds the **debug** profile (which *does* emit NEON in
  `write_volatile`) and still passes. Prescription:
  `docs/architecture/arm-exception-state.md`.
- **`x96q` and `suma-q5` are not bootable on silicon.** `platform-amlogic`
  never calls `set_console`, so `with_console()`'s `.expect(...)` panics —
  **`suma-q5` cannot print a single character**
  (`platform-amlogic/src/lib.rs:105-114`).
- Per-target `.cargo/config.toml` rustflags are **dead config** (cargo reads
  config from the invocation directory; every script and CI job runs from
  `vivanta-boot/`). The effective mechanism is each crate's `build.rs`.
- `flash_rk3568.py` uses 115 200 baud + `mw.l` at 5 ms/word; the canonical
  runbook says **1 500 000 baud** + `mm.l`, 64 words/chunk, ≥60 ms/word. It is
  superseded by `docs/hardware/rk3568/scripts/flash-serial-mm.py`, but
  `build.sh` still points at the old script.

### P1

- Several QEMU gates assert `after >= before` instead of exact values and/or
  live inside `if let`, so a branch that never ran still prints PASS. Gate G4
  checks neither tick frequency nor A↔B alternation nor thread identity.
- `PmmBitmap::free_frame` does not distinguish reserved from allocated frames.
- `PmmBackend::new_dram` hardcodes `MemoryProperties::dram_4gb()` — actual
  DRAM from the DTB is ignored, over-reporting capacity ~8× on a 512 MiB run.
- `MemoryCapability::check()` always returns `true` (declared placeholder).
- `vmm::register` / `register_child` **panic** on pool exhaustion instead of
  returning `Result`.
- `mprotect`/`munmap` break ADR-033 errno semantics (`W|X` returns `-EINVAL`
  where the ADR specifies `-EPERM`).
- `kill` checks no permissions: any process may kill any other by PID.
- **`main` has no branch protection** (verified via the GitHub API: *"Branch
  not protected"*), so a green run never blocked a merge anyway. Release
  workflow runs no gate and does not require green CI on the same SHA, builds
  the debug profile, and pulls RPi firmware from a floating
  `raspberrypi/firmware@master` with no checksums; `mkfs.vfat` runs without
  `--invariant`, so the image is not byte-reproducible.

### P2 (documentation)

- `docs/CHANGELOG.md` still has date gaps — most visibly 2026-07-17 → 2026-07-19
  and 2026-08-29 → 2026-09-04. The header explains that this is a partial journal
  and that gaps are expected, not a defect.
- ~~`docs/OS_MATURITY.md` headlines "35-40% done" and cites
  `StubAllocator`~~ — **resolved 2026-09-29.** The file now opens by retiring
  that headline and carries a table (`docs/OS_MATURITY.md:163-164`) naming both
  the percentage and the `StubAllocator` advice as retired.
- ~~`docs/architecture/repository-layout.md` and `docs/REPOSITORY_GUIDE.md`
  reference paths that do not exist~~ — **resolved.** Both trees now list real
  paths, and `docs/architecture/repository-layout.md:240-251` carries an
  explicit note that the pre-R2 paths it used to cite never existed.
- ~~`ROADMAP.md:44` still says "22/22 gates PASS"~~ — **resolved.** ROADMAP now
  states 18 required serial markers and that the job is advisory.

### Closed — INV-002 is NOT open

Earlier revisions of this file listed **INV-002 (preemption IRQ-loss under
sustained load) as an open P1**. That was wrong and it is removed.
`vivanta-boot/docs/investigations/INV-002-preemption-irq-loss.md:5` states
**"Closed (2026-08-11) — root cause proven and fixed"**, and the investigation
identifies the actual root cause: a **console-lock deadlock, not IRQ loss** —
`with_console`/`write_direct` documented that interrupts were disabled while
`CONSOLE_LOCK` was held, but the implementation never disabled them. Fixed,
then verified over a >6.5-minute soak (157 K log lines, no freeze, no tight
loop, no crash). The 2026-09-24 audit flagged the STATUS.md contradiction and
it was never fixed here.

**INV-002 and the still-open scheduler race are different bugs.** INV-002 was
a lock-discipline bug in the console layer, closed in August. The open race
(Known issue 2) is a preemption-vs-context-switch timing window; it is **not**
caused by unmasked interrupts (IRQ masking is correct). Do not conflate them.

## Platforms

| Target | SoC / board | Links `vivanta-kernel`? | Reaches `kernel_main`? | Verified on silicon? |
|---|---|:--:|:--:|---|
| `target-qemu-aarch64` | QEMU `virt`, cortex-a53 | ✅ | ✅ | **NO — QEMU only, advisory gate** |
| `target-rk3568` | RK3568 NVR | ❌ **no kernel dependency at all** | ❌ — EL2 diagnostic: UART + FDT report + `test_brk` (`src/main.rs:173`) | **YES — Stage 1–2 diagnostic on real UART (validated 2026-07-19)** |
| `target-rpi3b-plus` | BCM2837B0 Pi 3B+ | ✅ (`target-rpi3b-plus/Cargo.toml:17`) | ✅ (`src/main.rs:132`) | **NO — `kernel_main` wired, never run** |
| `target-x96q` | Allwinner H313 (X96Q) | ✅ | ✅ (`:181`) | **NO — "compile-checked only, needs silicon"** |
| `target-lavender` | SDM660 (Redmi Note 7) | deps declared, **unused** | ❌ — 5 `println!`s, then spins forever | **NO — Vivanta never ran** |
| `target-suma-q5` | Amlogic S905L | ✅ | call present (`:175`) but **panics first** | **NO** |
| `target-qemu-armv7a` | — | — | — | **FROZEN** — crate removed 2026-09-25; `arch-armv7a` is an 8-line stub kept per ADR-011 |

`platform-rk3568` is linked by **no** target — a dead crate. `early_mmu.rs` is
commented in `arch-aarch64/src/lib.rs:13` as "used by target-rpi3b-plus" and
has **zero callers**; the RPi3B+ path goes through the normal kernel, not a
standalone identity-map diagnostic. All six `target-*/Cargo.toml` files carry
the comment `# Verified via QEMU boot.` — true only for
`target-qemu-aarch64`; the gate never builds the other five.

## Hardware evidence tiers

Vivanta has **never run a kernel to userspace on any physical board.** No board
in the table above has ever reached `kernel_main`. What exists, by tier:

1. **RK3568** — the only real silicon output from Vivanta code: a Stage 1–2
   **EL2 diagnostic binary** printing on a real UART, validated 2026-07-19,
   decoded string `KC3FGZAB1CONSOLE OK231234WITH CONSOLE OK54X` (each letter
   is a per-stage marker: K after BSS zeroing, C = CPACR_EL1.FPEN readback,
   F/G = an FP/SIMD instruction test, Z = before the first Rust call, …).
   It never calls `kernel_main` and does not link the kernel crate. The board
   was later
   observed in **Maskrom** after a mainline U-Boot flash; boot unconfirmed
   ([docs/hardware/rk3568/board-info.md](docs/hardware/rk3568/board-info.md),
   [uboot-emmc-bringup.md](docs/hardware/rk3568/uboot-emmc-bringup.md)).
2. **lavender (SDM660) / whyred (SDM636)** — thoroughly surveyed with real
   evidence (`/proc/device-tree`, XBL UART capture, pmOS boots,
   `fastboot getvar all`). **Vivanta itself never ran on either.**
3. **AIBOX-1684X (BM1684X)** — a 912-line raw boot log with an
   `[OBSERVED]/[INFERRED]/[UNAVAILABLE]` evidence ledger. Excellent
   methodology. **Research only — no code, not on the roadmap.**
4. **Raspberry Pi 3B+** — the kernel is linked and `kernel_main` is wired, but
   it has never been run. The **working UART evidence from this board came
   from booting BalanSir's Buildroot, not Vivanta** — do not read a Pi 3B+
   serial log as a Vivanta boot.
5. **QEMU** — the only genuinely working configuration, and its gate is
   advisory.

## P0 corrective mission (2026-09-09)

Full log: [docs/history/missions/AUTONOMOUS_MISSION_P0_REPORT.md](docs/history/missions/AUTONOMOUS_MISSION_P0_REPORT.md)
(retired from the repo root). Forensic deep-dive on the stack overflow:
[vivanta-boot/docs/investigations/INV-003-boot-stack-overflow.md](vivanta-boot/docs/investigations/INV-003-boot-stack-overflow.md).

This mission's own notes claimed "19 PASS" and later "22/22 milestone gate
groups". **Both counts are retired**: `vivanta-boot/tools/qemu-gates.sh:28-47`
enforces exactly **18 markers**, and nothing in CI counts "milestone gate
groups". The fixes below are all real; only the counts were wrong.

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
  ELR=0 before); gates M7.12-dual-ELF and M10.4-EL0-fork green.
- `rt_sigreturn` now returns `-ENOSYS` instead of `0` when no delivery path
  exists (2026-09-24 audit) — this is why the live-syscall count is 13, not 14.

Deferred with rationale: `UserPtr` adoption in hot syscalls, `register()`
panic→Result conversion, kernel W^X, MAIR/TCR unification — all touch proven
paths for no P0 gain; revisit after next functional milestone.

## The QEMU gate — what it actually checks

`vivanta-boot/tools/qemu-gates.sh` requires **exactly 18 serial markers** and
fails on `PANIC|EXCEPTION|G4 FAIL` (`:111`). The terminal marker is
`[G4] running_count=1` (`:48`). The 18 markers, in script order, cover:

| # | Marker | What it proves |
|---:|---|---|
| 1 | `M7.12 concurrent ELF gate:` | two concurrent ELF user tasks |
| 2 | `[DUAL] two concurrent ELF tasks exit(42) PASS` | both exit 42 |
| 3 | `M10.4 EL0 fork gate:` | EL0 fork path entered |
| 4 | `[FORK-EL0] fork/waitpid round-trip exit(42) PASS` | fork + waitpid round trip |
| 5 | `[FORK-EL0] child teardown PASS` | child teardown (done **by hand** in the gate — see Known issue 4) |
| 6 | `M10.3 EL0 execve gate:` | execve path entered |
| 7 | `[EXECVE] EL0 execve("/init") -> exit(42) PASS` | real execve → EL0 → exit 42 |
| 8 | `[EXEC2] double load+stack in one AS PASS` | re-exec in the same address space |
| 9 | `[PTABLE] 70 create/reap cycles` | page-table create/reclaim churn |
| 10 | `[KILL] SIGKILL->Zombie + SIGCHLD PASS` | SIGKILL, zombie, SIGCHLD |
| 11 | `[OOM] full-table resolve refuses cleanly PASS` | capacity refusal instead of panic |
| 12 | `[E2E] load/dup/teardown deltas all zero PASS` | MRM frame deltas return to zero |
| 13 | `[ELF] genuine ELF64 userland PASS (exit=42)` | ELF64 loader end-to-end |
| 14 | `[COW] break+isolation PASS` | COW break + parent/child isolation |
| 15 | `[FORK] pid parent=` | fork parent/child pid relation |
| 16 | `[SYS] mmap/store/mprotect/munmap/negatives PASS` | syscalls + negative errno cases |
| 17 | `Hello, Vivanta!` | EL0 user code actually wrote to the console |
| 18 | `[G4] running_count=1` | the G4 scheduler invariant (terminal marker) |

Every one of these is a **serial-log substring check on a QEMU boot**. There
is no silicon, no timing/frequency assertion, and no hardware MMU validation
among them. See the P1 entry on weak assertions in Known issues.

## CI honesty (2026-09-25 → 2026-09-29)

The `qemu` job runs on `ubuntu-24.04-arm` so host and guest share an
architecture; GitHub does not expose `/dev/kvm` on hosted runners, and the job
auto-selects KVM if that changes.

**The job is advisory: `.github/workflows/ci.yml:87-99` sets
`continue-on-error: true`.** It is not deleted, but it does not block. And
`main` has **no branch protection** (verified via the GitHub API), so a green
run never gated a merge regardless.

**Do not read "18/18" as a CI pass rate.** Queried directly from the GitHub
API, the `qemu` job concluded `failure` in **9 of the 21** non-cancelled
`main` runs of the `ci` workflow to date — **≈ 1 in 2.3**. The most recent 4
runs are green, but 4 samples at that rate does not establish a fix. All the
failures are the scheduler race (Known issue 2) — a task's exit code is lost, or
a forked child is never scheduled. It surfaces as a fast panic on
`assert_eq!(task.exit_code, Some(42))` at `kernel/src/lib.rs:1202`, the same
lost-exit-code race on the M6 demo task at `kernel/src/lib.rs:1956`, or a 600 s
hang right after `[DUAL] ... PASS` (unscheduled forked child). The CI
workflow's own comment, the audit, and
[docs/tooling/ci-cd.md](docs/tooling/ci-cd.md) all say the same thing in
slightly different words: this is a heisenbug that local long runs do not
reproduce. The reason CI sees it more often than a local macOS host is
unresolved.

The scheduler race and its diagnosis are in
[docs/audit/2026-09-24-global-audit.md](docs/audit/2026-09-24-global-audit.md).
IRQ masking is correct (`DAIFSet/Clr #2` sets/clears PSTATE.I); the earlier
audit finding to the contrary was disproven (see Known issue 1). The remaining
work is to reproduce the race deterministically (QEMU `-icount`/`-d int`) and
then close the switch window. A "switching" flag was tried and deliberately
rolled back, because the flag is
cleared at the resume point which an EL0 thread's first entry
(`eret_to_user_stub`) never executes, which would leave EL0 threads without
preemption. That is worse than a flake.

## Scope fence (holds through any next milestone until explicitly lifted)

IPC · storage · drivers · networking · distributed AI (ADR-031…039) ·
Ed25519 · BIP-39 · persistent identity · TTBR1/ASID · asynchronous EL0 signal
delivery · new hardware targets · new architectures

Verified against [README.md](README.md), which carries the same fence minus
the ADR range and which now correctly states AArch64-only (`README.md:15-20`);
`arch-armv7a` is an 8-line frozen stub and there is no ARMv7 target.
