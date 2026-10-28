# 70 — Execution plan: research findings, verified root cause, and development paths

**Date:** 2026-10-26
**Scope:** full-project research pass, verified root cause for the open
scheduler race, wave 0 + wave 1.1 execution.
**Supersedes:** nothing. Complements `00-mission-charter.md` (which sets the
rules) and `10`/`20`/`30`/`40`/`50`/`60` (which set the candidate content).

---

## 0. Executive summary

1. **F-DAIF is settled and externally confirmed.** The `DAIFSet`/`DAIFClr`
   `uimm4` field is bit3=D, bit2=A, bit1=I, bit0=F, so `msr DAIFSet, #2`
   masks IRQ. The old in-tree claim that "`#2` serialises SError, not IRQ"
   and that there is a "GIC IRQ delivery defect" is **disproven** — the Arm
   Architecture Reference Manual states the bit order explicitly, and a
   direct QEMU read-back of DAIF confirms it on the real target. Every
   masking site in the tree already uses `#2` correctly.

2. **The race is now reproduced deterministically.** A feature-gated
   failpoint injects a nested scheduler entry into the resume window, and
   the kernel loses the affected thread's saved context **3 runs out of 3**.
   The control build (harness compiled in, nothing armed) boots clean —
   so the harness itself is not the cause.

3. **The mechanism is a nested scheduler entry inside the resume window**,
   not the IRQ-masking question and not the identity-substitution window.
   `yield_now()` re-enables interrupts (`enable_interrupts()`) *before* the
   exception-handler chain that led to the switch has unwound. The victim's
   `ExceptionFrame` is still live at that moment, and a second scheduler
   entry replaces the thread's saved context. The thread then resumes with a
   destroyed register set and never completes its exit path — which is
   exactly the reported symptom class ("lost exit code", "stranded child").

4. **Two harness gaps were blocking the whole feedback loop** and are now
   fixed: `cargo test` did not work at all locally (the workspace pins
   `build.target = "aarch64-unknown-none"`), and clippy had never been run
   despite being installed by the pinned toolchain.

5. **Wave 0 and wave 1.1 are executed.** The next wave is the actual fix,
   which the charter forbids starting before the failpoint fires — that
   precondition is now met.

---

## 1. Verified facts (evidence attached)

### 1.1 F-DAIF — confirmed externally

> Arm ISA, `MSR DAIFSet`/`DAIFClr`: *"Uses uimm4 as a bitmask to select the
> clearing of one or more of the DAIF exception mask bits: bit 3 selects the
> D mask, bit 2 the A mask, bit 1 the I mask and bit 0 the F mask."*
> `PSTATE` layout: D = bit 9, A = bit 8, I = bit 7, F = bit 6.

`msr DAIFSet, #2` therefore sets the **IRQ** mask (`PSTATE.I`). The tree's
masking code is correct as written. The in-tree comment claiming otherwise
was corrected in this pass.

### 1.2 Baseline measurements (local, QEMU 11.1.2, TCG, cortex-a53)

| Measurement | Value | How |
|---|---|---|
| Workspace build | clean | `cargo build --workspace` |
| QEMU gate matrix | **18/18 PASS** | `tools/qemu-gates.sh` |
| Host unit tests | 28 pass across 5 crates | `tools/host-test.sh` |
| Context switches per boot | 43 | failpoint counters |
| … of which preempted an EL0 thread | 2 | `SWITCH_TIMER_FROM_EL0` |
| Resume-path interrupt unmask events | 29 | `RESUME_UNMASKS` |
| Reschedule requests consumed without a switch | 29 | `CONSUMED_NO_SWITCH` |
| Clippy correctness lints | 0 (after fixes below) | `-D clippy::correctness` |

### 1.3 The resume window — the defect

`kernel/src/scheduler/mod.rs`, `yield_now()`:

```
    // Perform context switch
    unsafe { ... context_switch(...) }

    // We are running as `next` again ...
    unsafe { vivanta_arch_api::interrupts::enable_interrupts() };   // ← unmask HERE
    core::mem::forget(_guard);
```

The unmask happens while the CPU is **still inside the exception-handler
call chain** that led to the switch. That chain has not yet unwound past
the vector epilogue, so:

* the victim's `ExceptionFrame` (at `[SP_EL1 − 272]`) is still live and
  still holds the victim's EL0 state — `elr_el1`, `spsr_el1`, `sp_el0`,
  `x0`–`x30`;
* a second scheduler entry from inside that window **replaces the victim's
  saved `ThreadContext`** (`context_switch_asm` saves `x19`–`x30` + SP into
  the outgoing thread's slot).

When the chain finally unwinds and `eret`s, the victim resumes with a
destroyed context. The failpoint build faults with **`x0`–`x29` all zero**
on the faulting resume — the signature of a replaced context.

### 1.4 Two candidate causes that are now RULED OUT

* **"DAIF `#2` masks SError, so IRQs are never masked"** — disproven (§1.1).
  The masking in `yield_now` is real and correct.
* **"The EL0 save contract is broken because SP at switch time is 464 bytes
  below the frame base"** — *this looked like a defect under measurement and
  is **not** one.* The offset is the depth of the exception-handler call
  chain, and the chain unwinds normally on resume, restoring SP to the
  frame base before the vector epilogue reads the frame. The measurement is
  now recorded as a **chain-depth invariant check** (`EL0_SP_DELTA`), not as
  a defect indicator. *This correction is why the failpoint targets the
  nested entry rather than the save path.*

### 1.5 A real, separate defect found while measuring

`maybe_reschedule()` clears `NEED_RESCHEDULE` **before** it knows whether a
switch will happen; `yield_now()` then returns early when no eligible
thread exists. Measured: **29 flag consumptions against 13 timer switches**
in one boot — the pending reschedule is silently dropped and only re-happens
if another tick arrives. It is an amplifier (jitter, and a lost wakeup if
the tick source ever stalls), not the primary cause. Fix: make
`NEED_RESCHEDULE` a counter, and re-check after a no-switch return.

---

## 2. Development paths

### Path A — close the scheduler race (gating everything else)

The QEMU CI job stays `continue-on-error: true` until Path A lands. Order
is fixed by the charter: **failpoint → fix → gate flip**.

| Step | Change | Owner (file) | Status |
|---|---|---|---|
| A1 | Failpoint harness + counters + report | `kernel/src/scheduler/failpoint.rs`, `scheduler/mod.rs` | **DONE** |
| A2 | `tools/race-harness.sh` (control / fire modes) | `vivanta-boot/tools/` | **DONE** |
| A3 | Fix: defer the unmask out of the live-frame window | `scheduler/mod.rs` + `arch-aarch64/src/vectors.rs` | next |
| A4 | `NEED_RESCHEDULE` bool → counter, re-check on no-switch | `scheduler/mod.rs` | next |
| A5 | Single atomic `block_then_yield` (close the wait→yield hole) | `scheduler/mod.rs`, `syscall/process.rs` | next |
| A6 | Split `CURRENT_THREAD` identity from the save-target identity; remove the synthetic identity swap from the fork gate | `scheduler/mod.rs`, `kernel/src/lib.rs` | next |
| A7 | Flip the QEMU CI job to blocking; add a repeated-boot gate (N boots per CI run) | `.github/workflows/ci.yml`, `tools/` | after A3–A6 |

**The candidate fix for A3** (to be developed against the reproducer, not
blind): the unmask must not happen while an exception frame is live. Two
shapes are on the table —

* *arch-side*: move the unmask into the vector epilogue (unmask immediately
  before `eret`, and clear the DAIF bits in the restored SPSR so an EL1
  resume comes back with IRQs on);
* *kernel-side*: track IRQ-handler depth and defer the reschedule while it
  is non-zero, so the nested entry cannot happen at all.

Both are small; the arch-side one touches assembly and needs a second
reviewer (charter §5-5).

### Path B — unified teardown and frame ownership

The two are coupled: eager exit-time teardown only stops the
address-space pool leak if frame ownership is accurate, because the unmap
path decrements ownership counts.

| Step | Change | Owner (file) |
|---|---|---|
| B1 | Unified `release_task(task_id, Release)` teardown path; call it from `thread_exit`, `user_fault_terminate`, `sys_kill` | `kernel/src/scheduler/teardown.rs` (new) |
| B2 | `register`/`register_child` return `Result<AddressSpaceId, VmmError>` instead of `panic!` | `kernel/src/vmm/address_space.rs` |
| B3 | `AddressSpace.root_owned: Option<PhysFrame>` so `unregister` can free the root | `kernel/src/vmm/address_space.rs` |
| B4 | `kernel/src/memory/frame_owner.rs` replaces `vmm/cow_refcount.rs`; `Mapping.phys` becomes `FrameOwner` | `kernel/src/memory/`, `kernel/src/vmm/mapping.rs` |
| B5 | Fix the three tautologies: `verify_pmm` (compare the bitmap, not the arithmetic), `verify_hardware_consistency` (check the PA correspondence — `ADDR_MASK = 0x0000_FFFF_FFFF_F000` exists at `arch-aarch64/src/paging/descriptor.rs:113`, but there is **no** `pa_of` helper yet), `MappingSet::len()` (count live entries, not the high-water mark) | `kernel/src/verify.rs`, `kernel/src/vmm/` |

The concrete defect this path closes: **the 7th user fork panics** in
`register_child` (`kernel/src/vmm/address_space.rs:1079-1089`,
`panic!("address space registry full")`) because boot gates leave 9 of the
16 slots occupied and **no exit path ever calls `unregister`**. `Zombie`
tasks still own a full address space and all of its mappings until reaped.

### Path C — repository and gate hygiene

| Step | Change | Status |
|---|---|---|
| C1 | `cargo test` works locally (explicit `--target <host>`), all 5 testable crates exercised | **DONE** |
| C2 | clippy runs in CI with a correctness-only policy | **DONE** (found and fixed 4 real `not_unsafe_ptr_arg_deref` findings) |
| C3 | `if: always()` on the lint steps, so a failing rustfmt no longer silently skips the remaining linters | **DONE** |
| C4 | The stale DAIF comment in the CI workflow removed | **DONE** |
| C5 | The uncommitted work reviewed and committed | **OPEN — needs a decision** |

**The scope of "uncommitted" is larger than it looks.** As of this pass:

* `git log -- docs/planning/` is **empty** — the entire 2026-10 mission
  planning set (`00-mission-charter.md` … `60-build-ci-repo.md`, plus this
  file — roughly 500 KB) has **never been committed**. It exists only in the
  working tree.
* On top of that sits the September documentation correction pass: ~40
  staged and ~16 unstaged documentation files, uncommitted since 2026-09-29,
  on top of a green `main` (last commit `273f9bc`, 2026-09-25).

So the mission plan this pass researched and executed is itself unversioned
work. It should be committed before the next wave lands — otherwise the
next diff is mixed with two unrelated bodies of change, and the plan that
governs the work is not recoverable from history.

### Path D — capability growth (after A and B)

Not started, deliberately: the crash-consistency work (W4), the CoW frame
double-release (W9), the ELF execution model (W11) and the RPi3B bring-up
(W12) all depend on a scheduler that is not racing and a memory model whose
ownership is honest. Attempting them first would produce heisenbug-shaped
false positives.

---

## 3. What was executed in this pass

### 3.1 Failpoint harness (wave 1.1)

* `vivanta-boot/kernel/src/scheduler/failpoint.rs` (new) — feature-gated
  counters, F1/F2/F3 failpoints, one-shot report. No printing inside an
  armed window (the earlier `diag/exit-code-race` attempt failed exactly
  because it printed there).
* `vivanta-boot/kernel/Cargo.toml` — `sched-failpoint` (observe) and
  `sched-failpoint-fire` (arm F3). With both off, every counter and branch
  is compiled out.
* `vivanta-boot/kernel/src/scheduler/mod.rs` — switch sites now record
  their origin (`SwitchOrigin::{Voluntary,Timer,Exit}`) and the exception
  frame base is threaded down to the switch site.
* `vivanta-boot/kernel/src/lib.rs` — the report prints after the G4 gate,
  outside every armed window.
* `vivanta-boot/tools/race-harness.sh` (new) — `control` and `fire` modes.

**Result: `control` clean, `fire` faults 3/3.**

### 3.2 Tooling and CI

* `vivanta-boot/tools/host-test.sh` (new) — detects the host triple and
  passes it explicitly; **parses the test count out of cargo's own output**
  rather than hardcoding it, and refuses to report success if zero tests
  ran.
* `.github/workflows/ci.yml` — `if: always()` on the lint steps; the `test`
  job runs `host-test.sh`; **new `clippy` job** with
  `-D clippy::correctness` (correctness only, by project policy — 236 style
  warnings remain un-gated); the `build` job now depends on `clippy`.
* `vivanta-boot/tools/ci-local.sh` — runs the same host tests CI runs.

### 3.3 Correctness fixes found by the new clippy gate

`sys_fork` and `sys_execve` were public **safe** functions that dereference
a caller-supplied raw pointer (`kernel/src/syscall/process.rs`). They are
now `unsafe fn` with `# Safety` contracts, and the two call sites in the
dispatcher carry SAFETY comments. The dispatcher itself keeps the project's
arch-api `safe fn` convention with a justified
`#[allow(clippy::not_unsafe_ptr_arg_deref)]`.

---

## 4. Open questions that need a decision

1. **The uncommitted correction pass** (C5). Roughly 40 staged and 16
   unstaged documentation files, untouched since 2026-09-29, sitting on top
   of a green `main`. It looks like a coherent documentation correction
   pass, but it has never been reviewed or committed. It should be
   committed (or reverted) before the next wave lands, otherwise every
   later diff is mixed with it.
2. **Fix shape for A3** — arch-side epilogue change versus kernel-side
   deferral. The arch-side change is more correct but touches assembly and
   needs the second reviewer the charter asks for.
3. **Scope of Path B.** Eager teardown plus frame ownership is a large
   change to the memory subsystem. It is the only way to make the 16-slot
   address-space registry and the 64-slot mapping table non-demo-scale, but
   it should be sequenced behind Path A so a scheduler race is not
   misdiagnosed as a memory bug.

---

## 5. Method notes

* **Instrument, do not guess.** The decisive facts here came from counters
  added to the kernel and read out of the serial log, not from reading the
  code. One measurement (the 464-byte SP offset) *looked* like a smoking
  gun and turned out to be normal; the honest reading is recorded in §1.4
  so it is not re-discovered as a false lead.
* **A control is not optional.** The fire build faulting is only evidence
  if the control build (same code, nothing armed) is clean. It is.
* **Sub-agent findings were verified, not trusted.** The two design reports
  were checked against the code before being folded into this plan; the
  7th-fork panic site, the three tautologies and the DAIF encoding were all
  confirmed independently.
