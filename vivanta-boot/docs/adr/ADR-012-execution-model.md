# ADR-012: Execution Model — ThreadContext vs ExceptionFrame

## Status

Superseded by ADR-017 (2026-07-17)

## Date

2026-07-13

## Context

Vivanta now supports two distinct context-switching mechanisms:

1. **Cooperative switching** via `context_switch_asm` — used by `yield_now()`
2. **Preemptive switching** via `ExceptionFrame` manipulation in `save_and_eret` — used by the timer IRQ handler

Both mechanisms save and restore CPU state, but they operate on different data structures and have different invariants. During Stage 5 development, we discovered that conflating these mechanisms (or violating the assumptions of one while using the other) led to stack corruption, callee-saved register corruption, and crashes at the `RUNQUEUE` data address.

This ADR formalizes the execution model to prevent such issues going forward.

## Decision

### Two distinct context structures, two distinct mechanisms

```text
ThreadContext

  Purpose:    save/restore thread state between cooperative switches
  Saved:      x19-x30 (callee-saved per AAPCS64), SP
  Used by:    context_switch_asm (global_asm! AAPCS function)
  Mechanism:  regular function call + ret
  Stack:      SP is part of the context — the switch changes SP
  IRQ state:  caller must manage IRQ state (via IrqGuard) if needed

ExceptionFrame

  Purpose:    architectural CPU state at the moment of exception entry
  Saved:      x0-x30, SP, ELR_EL1, SPSR_EL1
  Used by:    save_and_eret macro (vector table epilogue)
  Mechanism:  exception entry + eret
  Stack:      SP is loaded from ExceptionFrame field (saved original SP)
  IRQ state:  SPSR_EL1 restores the original PSTATE (including IRQ mask)
```

### Rule 1: Preemption always passes through ExceptionFrame

When a timer IRQ preempts a running thread, the flow must be:

```
Timer IRQ
  ↓
save_and_eret saves ExceptionFrame on current thread's stack
  ↓
irq_entry_handler → timer_handler → scheduler_tick
  ↓
maybe_reschedule saves ExceptionFrame → current_thread.full
  ↓
maybe_reschedule copies next_thread.full → next_thread's stack (stack_top - 272)
  ↓
save_and_eret restores from ExceptionFrame (now on next thread's stack)
  ↓
eret → next thread continues
```

The `full` field in `Thread` stores a complete `ExceptionFrame`. This is the source of truth for preemptive switching.

### Rule 2: Cooperative switching uses ThreadContext only

When a thread voluntarily yields:

```
yield_now()
  ↓
save current x19-x30, SP → current_thread.ctx
  ↓
load next x19-x30, SP → next_thread.ctx
  ↓
ret → next thread continues
```

The `ctx` field in `Thread` stores callee-saved registers only. This is the source of truth for cooperative switching.

### Rule 3: Never mix the two mechanisms

- After a thread is entered via ERET (preemption), it may call `yield_now()`. This is safe because `yield_now()` saves/restores the callee-saved subset, which is valid regardless of how the thread was entered.
- A thread entered via `yield_now()` may be preempted by a timer IRQ. This is safe because the IRQ handler saves the full ExceptionFrame, which captures all registers.
- However, `context_switch_asm` must NEVER be called from within an IRQ handler. The `save_and_eret` macro owns the exception frame, and the `context_switch_asm` would corrupt it.

### Rule 4: save_and_eret must switch SP when switching threads

The `save_and_eret` macro originally did `add sp, sp, #272` to restore the original SP. When `maybe_reschedule` modifies the ExceptionFrame to point to a different thread, the macro must:

1. Load the new SP from the ExceptionFrame's SP field (offset 31*8)
2. Compute the new frame address (SP - 272)
3. Switch SP to the new frame
4. Load x30 and all registers from the new frame
5. `add sp, sp, #272` to restore the new thread's SP
6. `eret`

This was the key fix for Stage 5D — without it, the resumed thread ran on the PREVIOUS thread's stack.

### Rule 5: context_switch_asm must be a proper AAPCS function

The original `context_switch` was an inline `asm!` with `options(nostack)`. This told the compiler the asm didn't touch the stack, but it actually changed SP. This violated the compiler's contract, causing incorrect prologue/epilogue generation and callee-saved register corruption.

The fix: use `global_asm!` + `extern "C"` to create a proper AAPCS64 function. The compiler saves/restores callee-saved registers around the call, and the function's `ret` returns to the new thread's saved LR.

## Consequences

### Positive

- Clear separation of concerns: cooperative vs preemptive switching use different data structures
- Each mechanism is self-contained and independently testable
- The `ExceptionFrame` is the natural source of truth for preemption (hardware already saves this state)
- The `ThreadContext` is lightweight (104 bytes vs 272 bytes for ExceptionFrame)
- Both mechanisms can coexist: a thread can be entered via ERET and later call `yield_now()`

### Negative

- Two context-switching mechanisms mean two code paths to maintain
- `maybe_reschedule` must copy 272 bytes (`ExceptionFrame`) to the next thread's stack on every preemption — small but measurable overhead
- The `copy_nonoverlapping` in `maybe_reschedule` is necessary because the save_and_eret macro reads from the stack after switching SP

### Risk mitigation

- The two mechanisms are well-isolated: `context_switch_asm` is never called from IRQ context
- Performance impact of the 272-byte copy is negligible at 100 Hz timer frequency
- The `global_asm!` approach prevents compiler interference with the stack pointer

## Relationship to other ADRs

- **ADR-011**: This ADR extends the engineering rules with the execution model invariant
- **INV-001**: The investigation documented the root cause of the context switch fault, which led to this ADR