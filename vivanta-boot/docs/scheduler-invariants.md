# Scheduler Invariants

## Core Principles

1. **Uniqueness**: A thread can be in only one `ThreadState` at any given time.
2. **Ownership**: The `RUNQUEUE` static array owns all active `Thread` objects. Scheduler manages array lifecycle (insertion, termination/removal).
3. **Running Invariant**: For every online CPU, exactly one thread is `Running`. The `Running` thread is never present in the Ready Queue.
4. **Ready Queue Invariant**: Every `Ready` thread appears exactly once in the Ready Queue. No `Running` thread appears in the Ready Queue. `Blocked`, `Sleeping`, and `Terminated` threads never appear in the Ready Queue.
5. **Context Ownership**: The CPU register state always belongs to exactly one Thread. Before a context switch: `Running → Context Save`. After a context switch: `Context Restore → Running`.
6. **Transition Rules**: Valid state transitions are:
   - `Ready → Running` (selected by scheduler)
   - `Running → Ready` (preempted)
   - `Running → Blocked` (waiting on resource)
   - `Running → Sleeping` (time-based yield)
   - `Blocked/Sleeping → Ready` (event/time trigger)
   - `Running/Ready → Terminated` (exit)

   Forbidden transitions:
   - `Sleeping → Running` (must go via `Ready`)
   - `Blocked → Running` (must go via `Ready`)

7. **Thread Identity**: `ThreadId` is immutable and never reused while references may exist.
8. **Time Slice**: Only the `Running` thread consumes time slice. `Ready`, `Blocked`, and `Sleeping` threads never consume CPU budget.
9. **Idle Thread**: The Idle thread cannot terminate, block, or sleep. It always has the lowest priority and runs only when no `Ready` thread exists.
10. **Scheduler Authority**: The Scheduler is the only subsystem allowed to modify `ThreadState`. Other kernel subsystems request transitions through Scheduler APIs (e.g., `scheduler.wake(thread)`, `scheduler.block(thread)`).

## Execution Model

- The scheduler is timer-driven and preemptive (`NEED_RESCHEDULE` atomic flag).
- `maybe_reschedule()` is called from the IRQ dispatcher to perform context switching if `NEED_RESCHEDULE` is set.
- All scheduler operations must be atomic regarding interrupts (`disable_interrupts()` guard).

## Ownership Rules

- `Thread` objects are managed in static array.
- A thread is considered "active" if it exists in the `RUNQUEUE` (i.e., `RUNQUEUE[i].is_some()`).
- The `IDLE_SLOT` is special: it must always be present (or ready) if no other thread is runnable.
