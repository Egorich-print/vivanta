# Execution Context Model

> Part of M4.4.5 Execution Contract Freeze, M4.5.0 Execution Preparation,
> and M4.5.1 First EL0 Entry.
> Companion to ADR-017 (Unified Execution Context), ADR-018 (User Entry Transition Model),
> and ADR-019 (User Page Permissions).

## Thread Ownership Model

A `Thread` in Vivanta owns:

| Resource | Description | Created by | Destroyed by |
|----------|-------------|------------|--------------|
| Kernel stack | 16 KiB region allocated from PMM | `create_kernel_thread` | `cleanup` (on thread exit) |
| Execution context | Opaque `ArchContext` handle referencing stack layout | `context_init` / `context_capture_current` | thread termination (stack freed) |
| Address space | `AddressSpaceId` mapping to a hardware page table | `vmm::register()` | (currently static) |
| Execution level | `ExecutionLevel::Kernel` or `ExecutionLevel::User` | `context_init` | thread termination |

Threads do NOT own:
- A separate user stack (user stack is part of the address space, allocated separately)
- An `ExceptionFrame` (one exists on the kernel stack at exception entry, but it is creation-time state, not a permanent resource)

## Context Layers

### 1. Software Context (`ThreadContext`)

Created and managed by the cooperative scheduler. No hardware involvement.

| Field | Size | Saved by | Restored by |
|-------|------|----------|-------------|
| x19–x30 (callee-saved) | 96 B (12 × 8) | `context_switch_asm` | `context_switch_asm` |
| SP_EL1 | 8 B | `context_switch_asm` | `context_switch_asm` |
| **Total** | **104 B** | | |

Purpose: save/restore execution state between voluntary context switches.

### 2. Exception Context (`ExceptionFrame`)

Created by the hardware on exception entry (or synthesised by `context_init` for new threads).

| Field | Size | Saved by | Restored by |
|-------|------|----------|-------------|
| x0–x30 | 248 B (31 × 8) | hardware exception entry | `save_and_eret` macro (via `eret`) |
| SP_EL0 | 8 B | `save_and_eret` macro | `save_and_eret` macro |
| ELR_EL1 | 8 B | hardware exception entry | `eret` |
| SPSR_EL1 | 8 B | hardware exception entry | `eret` |
| **Total** | **272 B (34 × 8)** | | |

Purpose: architectural CPU state at the moment of exception entry.

## AArch64 Stack Layout

One contiguous region per thread, allocated by `create_kernel_thread`:

```
  High address (stack_top)
  │
  ├──────────────────────────────────┐
  │  Synthetic Initial Frame          │  ← ArchContext points here
  │  (layout = ExceptionFrame,        │     context_init fills:
  │   272 B)                          │     x[30] = entry point
  │                                   │     elr = entry point
  │                                   │     sp  = stack_top
  │                                   │     spsr = level->SPSR
  │                                   │     x[0..29] = 0
  │                                   │
  │                                   │     NOT a hardware exception frame.
  │                                   │     Created by software at thread init.
  │                                   │
  ├──────────────────────────────────┤
  │                                   │
  │  tc_loc(ArchContext) =            │
  │    ArchContext - 104              │
  │                                   │
  ├──────────────────────────────────┤
  │  ThreadContext (104 B)            │  ← context_switch_asm saves/restores
  │                                   │     context_init fills: x30 = entry,
  │                                   │     sp = stack_top
  ├──────────────────────────────────┤
  │                                   │
  │  ↓ free stack grows downward      │
  │                                   │
  ▼ Low address
```

On exception entry (timer IRQ, SVC, abort), the hardware appends a REAL ExceptionFrame
above the current SP_EL1. This is temporary and is consumed by `eret`.

```
  Before IRQ:                         During IRQ:
  ┌─────────────┐                     ┌─────────────┐
  │ Synth Frame │                     │ Synth Frame │
  ├─────────────┤                     ├─────────────┤
  │ ThreadCtx   │                     │ ThreadCtx   │
  ├─────────────┤                     ├─────────────┤
  │ free stack  │ ← SP_EL1            │ free stack  │
  └─────────────┘                     ├─────────────┤
                                       │ Exception   │ ← SP_EL1
                                       │ Frame       │
                                       │ (saved by   │
                                       │ hardware)   │
                                       └─────────────┘
```

After `eret` the ExceptionFrame is discarded and SP_EL1 returns to its pre-exception value.

## User Thread Entry Flow (ADR-018)

### First entry to EL0

```
context_switch_asm
    restore SP_EL1 = kernel_stack_top
    restore x30 = eret_to_user_stub
    ret -> eret_to_user_stub

eret_to_user_stub
    SP_EL1 -= 272            (-> synthetic frame base)
    load SP_EL0              (from frame.sp)
    load ELR_EL1             (from frame.elr = user entry point)
    load SPSR_EL1            (from frame.spsr = 0x000)
    restore x0-x30            (from frame.x[0..30])
    eret -> EL0
```

After the transition, SP_EL1 settles at `kernel_stack_top - 272`. Subsequent
exception entries (SVC, IRQ) save their frames below this point. The synthetic
frame is a one-time-use structure and becomes dead space after eret.

### x30 routing (context_init)

| ExecutionLevel | ThreadContext.x30       | Purpose                                   |
|----------------|------------------------|-------------------------------------------|
| Kernel         | `thread_trampoline`    | Calls the real entry, handles exit        |
| User           | `eret_to_user_stub`    | Transitions from EL1 to EL0 via eret     |

The synthetic frame for user threads contains:
- `sp` = `user_stack_top` (becomes SP_EL0)
- `elr` = `entry` (becomes ELR_EL1 = user code address)
- `spsr` = `0x000` (EL0t, interrupts enabled)
- `x[0..30]` = `0` (clean register state for user code)

For kernel threads the synthetic frame is dead space — the thread never erets
from it.

### EL0 Transition Ownership (ADR-019)

Only the following code paths may transition from EL1 to EL0:

| Code path | Role | Allowed? |
|-----------|------|----------|
| `eret_to_user_stub` | First EL1→EL0 entry for a user thread | ✅ |
| `save_and_eret_sync` | Return to EL0 after handling an EL0 exception | ✅ |
| `save_and_eret` | Return to interrupted EL1/EL0 after IRQ | ✅ |
| Inline `eret` in kernel code | Ad-hoc privilege transition | ❌ |
| `UserBootstrap::enter()` | Duplicate entry path | ❌ (removed) |
| Scheduler `x30 = entry` for User | Bypasses eret_to_user_stub | ❌ |

## Invariants

### ADR-017 Invariants

1. **ThreadContext and synthetic frame belong to the same Thread and live on the same kernel stack.**
   - Created together in `context_init`, accessed via `ArchContext` + offset.

2. **SP_EL1 always refers to the owning thread's kernel stack.**
   - `context_switch` changes SP_EL1 to the selected thread's stack.
   - The kernel never borrows another thread's stack.

3. **Scheduler never modifies the contents of any frame directly.**
   - `context_switch` modifies `ThreadContext` (save old, restore new).
   - Exception frame contents are owned by the exception handler, not the scheduler.

4. **Kernel does not know the AArch64 register layout.**
   - `ArchContext` is an opaque handle (`#[repr(transparent)] struct` over `usize`).
   - Kernel passes `ArchContext` values but never inspects them.

5. **`context_switch` is the single mechanism for all context switching.**
   - No separate cooperative/preemptive paths.
   - Whether triggered by `yield_now` or future timer reschedule, the same function is used.

6. **Synthetic initial frame is NOT a hardware ExceptionFrame.**
   - Created by software (`context_init`) for initial thread state.
   - Real ExceptionFrames are created by the hardware on exception entry.

7. **ExceptionFrame is never copied between thread stacks.**
   - The frame saved by exception entry is consumed by `eret` on the same stack.
   - Context switching uses `ThreadContext`, not `ExceptionFrame`.

### ADR-018 Invariants

8. **`eret_to_user_stub` is the ONLY component allowed to transform a kernel
   execution context into an EL0 execution context.**
   - The scheduler sets `x30 = eret_to_user_stub` and lets `context_switch_asm`
     ret into it.
   - No other code path creates EL0 state.

9. **EL0 execution may only be entered through `eret` and may only return
   through architecturally defined exception paths (SVC, IRQ, abort).**
   - The kernel never uses `ret` to enter user code.
   - `context_init(x30 = user_entry)` would execute user code in EL1, which is
     an invariant violation.
