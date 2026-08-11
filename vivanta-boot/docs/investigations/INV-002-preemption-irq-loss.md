# INV-002: Preemption reliability — IRQ loss under sustained timer load

## Status

Open (P1 reliability). Found by the M6 soak test run (2026-08-11).

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

## Fix direction (not implemented — investigation)

1. Make the IRQ-return path re-entrancy-safe: decide whether
   `maybe_reschedule` should defer the switch to the `save_and_eret` epilogue
   instead of switching inside the handler, or ensure the IRQ state is
   restored consistently across the switch.
2. Audit `save_and_eret` for the case where the interrupted thread was
   switched away and resumed (restore order, ELR/SPSR, and the `sp` reload).
3. Investigate the spurious IRQ activity seen in EOI-vs-timer counts.
4. Add a soak that fails loudly on the tight-loop condition (log not growing
   while CPU is high), so future regressions are caught.

## Verification for closure

- 60-min soak passes with both preemption workers making progress the whole
  time (counters strictly increasing, no tight-loop, no crash).
- No `EL1h` instruction abort, no `x30=0`.
- `[TICK]`/reschedule evidence continues for the full duration.
