# ADR-013: Privilege Transition Model — EL1 ↔ EL0

## Status

Accepted

## Date

2026-07-13

## Context

Vivanta currently runs all code in EL1 (kernel privilege level). To support userspace execution, the kernel must be able to:

1. Enter EL0 (userspace) with controlled state
2. Handle exceptions from EL0 (SVC, page faults, IRQs)
3. Return to EL0 after handling the exception

Unlike traditional context switching (which operates within EL1), EL1↔EL0 transitions require:

- **Different stack pointers**: `SP_EL1` for kernel, `SP_EL0` for userspace
- **Different page table permissions**: EL0-inaccessible pages for kernel, EL0-accessible pages for user
- **Exception return via `eret`**: This is the only way to enter a lower exception level
- **SPSR_EL1**: Stores the target exception level and PSTATE for `eret`

## Decision

### Privilege transition model

```
EL1 (kernel)
    │
    │ 1. Prepare ExceptionFrame (ELR_EL1, SPSR_EL1, SP_EL0)
    │ 2. eret
    ▼
EL0 (user)
    │
    │ 3. Execute user code
    │ 4. svc #n
    │
    ▼
EL1 (exception vector)
    │
    │ 5. Handle exception (SVC, IRQ, etc.)
    │ 6. Modify saved ExceptionFrame if needed
    │ 7. eret
    ▼
EL0 (user, resumed)
```

### Rule 1: EL0 state is an ExceptionFrame

The state of a userspace thread is represented as an `ExceptionFrame`:

```
ELR_EL1:   address of next user instruction
SPSR_EL1:  PSTATE to restore (must set M[3:0] = 0b0000 for EL0)
SP_EL0:    user stack pointer
x0-x30:    user registers
```

This is the same structure used for preemptive kernel thread switching. The difference is that `SPSR_EL1.M[3:0]` determines the target exception level.

### Rule 2: Kernel owns the ExceptionFrame, user owns the registers

- The kernel prepares the `ExceptionFrame` before entering EL0
- While in EL0, the kernel cannot access user registers directly
- On exception (SVC, IRQ, etc.), the hardware saves the user state to the kernel stack as an `ExceptionFrame`
- The kernel may modify the `ExceptionFrame` before returning to EL0 via `eret`

### Rule 3: EL0 cannot access kernel memory

All kernel pages (code, data, stack, page tables, MMIO) are mapped with `AP[2:1] = 0b00` (EL1 only).
User pages are mapped with `AP[2:1] = 0b01` (EL0 + EL1 accessible).

The MMU enforces this separation. Any attempt by EL0 code to access a kernel page causes a page fault.

### Rule 4: SVC is the only entry to EL1 from EL0

Currently, only the `svc` instruction is defined as a valid entry from EL0 to EL1.
Other exceptions (page faults, undefined instructions, IRQs) currently halt the system.

### Rule 5: SVC handler is a kernel function

The SVC handler runs in EL1 with the kernel stack. It receives the `ExceptionFrame` (saved user state) and the SVC number. It may modify the `ExceptionFrame` to change the user's return state.

### Current implementation limitations

- No ELF loader — user code is a pre-compiled binary blob embedded in the kernel
- No separate user address space — user code and stack are identity-mapped with EL0 permissions
- No page fault handling — user crashes are fatal
- No system call dispatch — only SVC #0 is recognized

## Consequences

### Positive

- The ExceptionFrame-based model is already proven by preemptive kernel thread switching
- The eret path is already tested by the timer IRQ return
- Adding EL0 support requires minimal changes to the existing codebase
- The privilege boundary is explicit and hardware-enforced

### Negative

- Without an ELF loader, user code must be embedded in the kernel binary
- Without page fault handling, user bugs crash the kernel
- Without address space switching, all user code shares the identity mapping

### Risk mitigation

- EL0 support is additive — it does not change the existing kernel thread model
- The first user code is a minimal test (print + SVC), not a full application
- Address space switching and ELF loading are deferred to later stages