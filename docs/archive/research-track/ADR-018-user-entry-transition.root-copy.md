# ADR-018: User Entry Transition Model

## Status
Proposed

## Date
2026-07-17

## Related ADRs
- ADR-017 Unified Execution Context
- ADR-013 Privilege Transition Model

## Context

ADR-017 froze the execution contract for context switching but did not
define how a thread transitions from EL1 to EL0. M4.5 (User Execution
Foundation) requires this transition.

The question: after context_switch() restores a user thread's
ThreadContext and ret's into x30, how does execution reach EL0?

Two options were considered:

### Variant A — SP_EL1-relative synthetic frame (CHOSEN)

The synthetic initial frame (created by context_init) lives at the top
of the kernel stack. eret_to_user_stub finds it via SP_EL1.

### Variant B — frame pointer in x19 (REJECTED)

context_init would store the frame pointer in x19 (callee-saved).
eret_to_user_stub reads x19 to locate the frame.

Rejected because:
- Reserves a callee-saved register permanently
- Complicates AAPCS64 ABI compliance
- Conflicts with future TLS / CPU-local pointer usage
- Unnecessary: SP_EL1 already uniquely identifies the frame location

## Decision

### 1. Synthetic frame location

The synthetic frame occupies [kernel_stack_top - FRAME_SIZE, kernel_stack_top).
FRAME_SIZE = 272 (34 × 8 bytes, matching ExceptionFrame layout).

After context_switch_asm restores SP_EL1 = kernel_stack_top, the frame
is directly accessible at [SP_EL1 - FRAME_SIZE, SP_EL1).

### 2. eret_to_user_stub

A single architecture-specific primitive will perform the EL1 to EL0
transition. The implementation will be provided by
arch-aarch64::eret_to_user_stub:

    eret_to_user_stub:
        sub sp, sp, #FRAME_SIZE      ; SP_EL1 -> frame base
        ldr x30, [sp, #(31*8)]; msr sp_el0, x30    ; user stack
        ldr x30, [sp, #(32*8)]; msr elr_el1, x30   ; user entry
        ldr x30, [sp, #(33*8)]; msr spsr_el1, x30  ; EL0t
        restore x0-x30 from frame
        eret                         ; -> EL0

### 3. x30 selection in context_init

    Kernel thread: x30 = thread_trampoline (or entry)
    User thread:   x30 = eret_to_user_stub

context_switch_asm ret's to x30. For user threads, this enters
eret_to_user_stub, which performs eret to EL0.

### 4. SP_EL0 / SP_EL1 separation

SP_EL1 points below the synthetic frame after the transition.
The exact offset is implementation-defined by the trampoline and must
remain within the owning kernel stack.

SP_EL0 = user_stack_top (from frame.sp).

This design does not preclude future changes such as red zones, guard
pages, separate exception stacks, or per-thread IRQ stacks.

### 5. SPSR invariants

    Kernel: SPSR = 0x345 (EL1h, DAIF masked, SP_EL1)
    User:   SPSR = 0x000 (EL0t)

Initial user execution starts with interrupts enabled.
If later security or debugging requirements require masking,
the transition policy can be changed independently.

M[3:0] determines target exception level on eret.
IL bit (20) = 0, SS bit (21) = 0 for both.

### 6. eret_to_user_stub is the ONLY EL1 -> EL0 transition path

> INVARIANT: eret_to_user_stub is the only component allowed to
> transform a kernel execution context into an EL0 execution context.

The scheduler never creates EL0 state directly. It sets x30 =
eret_to_user_stub and lets context_switch_asm ret into it.

This prevents ad-hoc user-mode entry from kernel code:

    kernel scheduler
            |
            x  (never directly creates EL0 state)
            |
    arch-aarch64::eret_to_user_stub
            |
            v
           EL0

### 7. EL0 cannot return through ret

> INVARIANT: EL0 execution may only be entered through eret and may
> only return through architecturally defined exception paths (SVC,
> IRQ, abort). The kernel never uses ret to enter user code.

This prevents a future scenario where context_init(x30 = user_entry)
is used for user threads, which would execute user code in EL1.

### 8. Frame layout constants

FRAME_SIZE is defined once in arch-aarch64 and used by:
- context_init (frame placement)
- eret_to_user_stub (frame access)
- Layout assertions (compile-time)

The assembly macro save_and_eret_sync uses #(34 * 8) — same constant.
Changing FRAME_SIZE requires updating all three.

## Consequences

### Positive
- Single, well-defined EL0 entry point
- No reserved registers (AAPCS64 fully respected)
- Frame location derived from SP_EL1 (natural AArch64 model)
- Kernel threads unaffected (x30 = trampoline, not stub)
- Scheduler has no EL0-specific logic

### Negative
- eret_to_user_stub couples to FRAME_SIZE and frame layout
- Synthetic frame is 272 B dead space per user thread
- SP_EL1 settles 272 B below stack_top (minor stack accounting)

### Risk mitigation
- FRAME_SIZE asserted at compile time
- Frame layout == ExceptionFrame (already asserted)
- Dead space is 1.7% of 16 KiB kernel stack
- SP_EL1 offset is constant and documented
