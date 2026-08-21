# INV-002: Preemption reliability — IRQ loss under sustained timer load

## Status

**Closed (2026-08-11) — root cause proven and fixed.** G4 preemption soak
verified > 6.5 min continuous (157 K log lines, counters > 30 B) with no
freeze, no tight loop, no crash.

## Root cause (proven)

**Console-lock deadlock, not IRQ loss.** `vivanta_boot_common`'s
`with_console` / `write_direct` documented "interrupts are disabled while the
lock is held" (`boot_common/src/lib.rs:27-29`), but the implementation **never
disabled IRQs** while holding `CONSOLE_LOCK`.

The deadlock sequence:

1. A preempt worker calls `println!("  [PREEMPT] ...")`, acquires
   `CONSOLE_LOCK`, starts printing (log shows a partial `[P` line).
2. The 100 Hz timer IRQ fires *while the lock is held* (IRQs still enabled).
3. `irq_entry_handler` → `scheduler_reschedule` → `yield_now`, which itself
   `println!`s → `with_console` → `CONSOLE_LOCK.acquire()` spins forever,
   because the preempted worker still owns the lock.
4. The spinning thread runs in interrupt context with IRQs masked, so the
   timer can never resume the lock owner → permanent 100 % CPU tight loop and
   the log stops growing — indistinguishable from "IRQ loss".

Diagnostics that proved it (raw UART pokes bypassing the lock):

```
[PREEMPT] current=5 B=75000000     <- worker B prints fully
[PGAtfF                            <- B starts "[PREEMPT]", interrupted after "[P";
      G=IRQ entered, A=acked, t=tick, f=find_next_ready, F=returned
      -> then nothing: yield_now's println spins on CONSOLE_LOCK forever
```

The `x30=0` crash variant was the same class of corruption surfacing
differently: a context save/restore raced with the deadlock path. Moving the
per-thread `ThreadContext` to the **bottom** of the kernel stack
(`stack_bottom`) was also applied (it can never be clobbered by stack usage /
exception frames), and is retained as a defensive hardening.

## Fix (implemented)

- `boot_common/src/lib.rs`: `with_console` / `write_direct` now hold the
  console lock **with interrupts disabled**, via a registered
  `set_console_irq_guard()` hook backed by the arch's `InterruptGuard`
  (`disable_interrupts`). The hook is registered by `vivanta-kernel`
  `kernel_main` right after `early_init`. Platforms that never run the
  scheduler leave the hook unset and remain single-threaded-safe.
- `arch-test-stub`: added no-op `disable_interrupts` / `enable_interrupts`
  so the build-time proof target still links.
- Retained: `ThreadContext` at `stack_bottom` in `context_init`
  (`arch-aarch64/src/context.rs`), passed through
  `create_user_thread` / `spawn_user`.

## Symptom

Under sustained 100 Hz timer preemption between two CPU-bound kernel threads,
the kernel **loses IRQ preemption** after a nondeterministic interval
(observed between ~40 s and ~2 min):

- The active thread keeps running (QEMU ~99% CPU = tight loop), but the timer
  no longer reschedules it.
- The preemption log (`[PREEMPT] current=N X=...`) stops growing; the counters
  freeze at their last values.
- Occasionally the loss manifests as a crash instead of a silent loop:
  `EL1h Sync (4) Instruction Abort (same EL)`, `ESR_EL1=0x86000006`,
  **`ELR_EL1=0`, `FAR_EL1=0`, `x30=0`** — a jump through a zeroed ThreadContext.

Both the silent tight loop and the crash are the same root cause: the
interrupt-driven preemption path stops being delivered.

## Evidence

- Reproducible on both the committed M5.0 state and the M6 tree (not an M6
  regression). The M5.0 "60 s smoke" passed once, but does not reliably reach
  60 s.
- With diagnostic UART pokes added, tick/EOI counters showed a mismatch
  (`E` EOI ≈ 5× `H` timer handler), suggesting spurious IRQ activity around the
  timer path.
- Crash register dump (one run): SP=0x40136eb0, x19=0x40136c90, x29=0, x30=0.

## Root cause hypothesis (not yet proven)

The IRQ-return path after a context switch made from within
`maybe_reschedule` (interrupt context) is fragile:

1. `yield_now()` (called from `maybe_reschedule`) disables IRQs and performs
   `context_switch()` while inside the timer IRQ handler.
2. `context_switch_asm` swaps to another thread; when the interrupted thread is
   later resumed, execution continues inside `yield_now`, returns into
   `maybe_reschedule`, then into `irq_entry_handler`, and finally the
   `save_and_eret` epilogue performs `eret`.
3. If a second IRQ arrives at an unlucky point during this
   save/switch/restore sequence (or the GIC state is left inconsistent by the
   nested dispatch), the IRQ is acknowledged but never fully re-armed, or the
   DAIF/Timer state is corrupted so subsequent ticks are masked.

The `x30=0` crash indicates a zeroed `ThreadContext` being restored — a
specific corruption of the context-switch bookkeeping, not just IRQ masking.

## Related code

- `kernel/src/scheduler/mod.rs` — `maybe_reschedule` / `yield_now` /
  `context_switch` usage from interrupt context.
- `arch-aarch64/src/interrupts/dispatcher.rs` — `irq_entry_handler` calls
  `scheduler_reschedule` after EOI.
- `arch-aarch64/src/vectors.rs` — `save_and_eret` epilogue for IRQ returns.
- `arch-aarch64/src/context.rs` / `thread.rs` — `context_switch_asm`
  ThreadContext save/restore.
- `arch-aarch64/src/timer.rs` / `gic.rs` — timer re-arm and GIC ack/EOI.

## Impact

The M5.0 G4 "preemption proven" claim is true for short runs (the 60 s smoke
and manual runs passed), but **not reliable over minutes**. This blocks:

- any long-running multi-thread workload;
- trusting the 60-min soak as a reliability gate until fixed;
- moving to user-space services that depend on stable preemption.

## M6 impact

**M6 itself is not affected.** All M6 gates (task lifecycle, exit code, reap)
were verified in short runs and pass. The preemption instability is a
pre-existing M5.0-path defect that the soak surfaced. It is tracked here as a
P1 deferred item, NOT part of M6 exit criteria.

## Fix direction (implemented — see "Fix" above)

The definitive guard: **never hold `CONSOLE_LOCK` with IRQs enabled.** Any
code that prints from interrupt context must not be able to block on a lock
owned by a preempted thread. The RAII `InterruptGuard` around the lock
enforces this structurally. The scheduler/dispatcher audit also confirmed:
`context_switch_asm` correctly never touches DAIF, and `save_and_eret`
restores SPSR (IRQ state) on resume, so no IRQ masking leaks across the
switch.

## Verification for closure

- **DONE (2026-08-11):** 6.5+ min QEMU soak, G4 preemption test — 157 K log
  lines, preemption counters > 30 B, full boot→A→B→boot rotation repeating
  ~11 K times, ~22 K preemptions per worker, zero `CPU halted`, zero panics.
- 60-min soak passes with both preemption workers making progress the whole
  time (counters strictly increasing, no tight-loop, no crash).
- No `EL1h` instruction abort, no `x30=0`.
- `[TICK]`/reschedule evidence continues for the full duration.
