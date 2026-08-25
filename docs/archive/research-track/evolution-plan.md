# Evolution Plan

## M4 — Execution Foundation

### M4.4 Address Spaces (commit 915846e)

Multi-AS model with verified hardware isolation:
- AddressSpace registry (8 slots), KernelAddressSpace singleton
- Thread.address_space: AddressSpaceId, activation on context switch
- D1: Three ASes switching stably
- D2: Independent root tables
- D3.1/D3.2: Hardware-confirmed isolation (Data Abort on cross-AS access)
- D3.3: Fault handler with register dump, graceful halt

### M4.4.5 Execution Contract Freeze (commit 2b0b320)

Unified execution model (ADR-017 supersedes ADR-012):
- Single context_switch(), ExceptionFrame never copied
- ArchContext is #[repr(transparent)] struct, ExecutionLevel enum
- InterruptGuard in arch-api (RAII, #[must_use])
- Zero inline DAIF asm in kernel

#### M4.4.5 External Review

Reviewed by: external adversarial architecture review

Result:
- No Critical issues
- Execution model validated
- EL0 preparation gaps identified (H1, H2, M3)

Deferred to M4.5.0:
- InterruptGuard state preservation (H1)
- User stack ownership (M3)
- EL0 entry trampoline (H2)

### M4.5 — User Execution Foundation

#### M4.5.0 Preparation (ADR-018)
- ADR-018: User Entry Transition Model
- InterruptGuard: save/restore DAIF state
- context_init: user_stack_top parameter
- eret_to_user_stub: architecture-specific primitive (~20 lines)
- ExecutionLevel SPSR invariant assertions

#### M4.5.1 First EL0 entry
- User code page (mov x0, #42; svc #0; b .)
- User stack page
- eret -> EL0 -> SVC -> EL1 handler -> print x0

#### M4.5.2 Syscall ABI
- x8 = syscall number, x0-x5 args, x0 return
- sys_yield, sys_write
