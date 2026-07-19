> **Note:** At the time of writing, the project was named TheseusOS. The content reflects the historical state and is preserved as-is.
>
# ADR-017: Unified Execution Context

## Status

Accepted

## Date

2026-07-17

## Supersedes

- ADR-012 (Execution Model — ThreadContext vs ExceptionFrame)

## Related ADRs

- ADR-011 Phase Transition — Research Prototype → Engineering Platform
- ADR-014 Architectural Boundaries
- ADR-015 Arch Boundary Contracts

---

## Context

ADR-012 established a dual-path execution model:

1. **Cooperative switching** via `ThreadContext` (callee-saved registers + SP) — used by `yield_now()`
2. **Preemptive switching** via `ExceptionFrame` manipulation — used by the timer IRQ path

During M4 execution foundation (M4.4), the preemptive implementation was exercised
for the first time in a multi-threaded context. It revealed that copying an `ExceptionFrame`
between thread stacks creates an ownership violation:

- An `ExceptionFrame` represents the CPU state **at the moment of exception entry**.
- It is bound to the interrupted thread's kernel stack by physical position (SP_EL1).
- Copying it to another thread's stack implies that the new thread "inherits" the
  interrupted execution state, but the new thread's `ThreadContext` (callee-saved
  registers + SP_EL1) still belongs to the original interrupted context.
- This mismatch made the preemptive path unsound. The implementation could not be
  stabilised on QEMU.

Three independent architecture reviews converged on the same root cause:
**the problem is not a bug in the implementation — it is a flaw in the execution model.**

In addition, the dual-path design introduced two maintenance concerns:

- Two different mechanisms for the same operation (context switch).
- Architectural inline assembly (`DAIFSet`/`DAIFClr`) in the architecture-independent kernel.

## Decisions

### 1. Unified context switch

Replace the two-path model with a single `context_switch()`:

```rust
// arch-api/src/context.rs
extern "Rust" {
    pub fn context_switch(old: *mut ArchContext, new: ArchContext);
}
```

- `context_switch` operates solely on `ThreadContext` (callee-saved registers + SP_EL1).
- It is the same function whether called from `yield_now()` or from a future timer reschedule path.
- Cooperative and (future) preemptive switching use the same mechanism.

### 2. ExceptionFrame is never copied between threads

A thread's `ExceptionFrame` exists only on its own kernel stack:

- Exception entry (e.g., timer IRQ): the hardware writes the frame to the **current** SP_EL1 stack.
- `save_and_eret` macro: reads the frame from the **current** stack, restores registers, `eret`.
- Context switching uses `ThreadContext`, not `ExceptionFrame`.
- **No `memcpy` of `ExceptionFrame` between stacks.**

This restores ownership clarity: each thread owns its kernel stack, and the
`ExceptionFrame` on that stack is a transient artefact of exception handling.

### 3. ExecutionLevel

```rust
pub enum ExecutionLevel {
    Kernel,  // SPSR = 0x345 (EL1h, DAIF masked)
    User,    // SPSR = 0x000 (EL0t)
}
```

- `ExecutionLevel` describes **execution privilege**, not scheduling class or lifecycle.
- It determines the SPSR value at thread creation and the target exception level on `eret`.
- It is NOT:
  - A scheduler priority indicator.
  - A process/user classification.
  - A resource ownership boundary.

### 4. Interrupt abstraction

All interrupt control is moved from the kernel to `arch-api`:

```rust
// arch-api/src/interrupts.rs
extern "Rust" {
    pub fn disable_interrupts() -> InterruptGuard;
    pub fn enable_interrupts();
}
```

- `InterruptGuard` is an RAII guard defined in `arch-api` with `Drop` in `arch-api`.
- The arch implementation provides the `#[no_mangle]` functions.
- The kernel calls `arch_api::interrupts::disable_interrupts()` instead of inline `DAIFSet` asm.

This removes the last remaining architecture-specific inline assembly from the
kernel crate.

---

## Consequences

### Positive

- Single, well-defined context switch mechanism.
- No ownership ambiguity for `ExceptionFrame`.
- `ExecutionLevel` cleanly separates privilege from scheduler concerns.
- Kernel no longer contains architecture-specific interrupt asm.
- The unified model makes future preemption architecturally possible
  without reworking the context switch API.

### Negative

- ADR-012 is superseded — any documentation relying on the dual-path model
  must be updated.
- `InterruptGuard` uses a function pointer (`fn()`) to restore interrupts —
  negligible overhead (one indirect call per `Drop`).
- The preemptive switching path is not yet enabled; only the architectural
  foundation is in place.

### Risk Mitigation

- The function pointer in `InterruptGuard` is a plain `fn()` (no closure, no
  allocation) — zero cost in practice with LTO.
- `ExecutionLevel` is additive — all existing threads are `ExecutionLevel::Kernel`;
  no behavioural change.
- QEMU validation confirms cooperative switching is unaffected.
