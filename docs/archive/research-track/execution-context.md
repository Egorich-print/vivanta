# Execution Context Model

## Thread Ownership and Stack Layout

Each thread owns exactly one kernel stack. The kernel stack layout,
from high to low addresses:

    kernel_stack_top
        synthetic frame (user threads only, 272 B, one-time use)
        exception frames (grow downward during EL0 execution)
        ThreadContext (saved x19-x30, SP_EL1)
    kernel_stack_bottom

## Invariants (ADR-017)

1. ExceptionFrame is never copied between thread stacks.
2. ThreadContext is the only state moved by context_switch().
3. ArchContext is a newtype over a single usize (ThreadContext address).
4. ExecutionLevel determines initial SPSR at thread creation.
5. InterruptGuard is the only mechanism for disabling interrupts.
6. Kernel code contains zero inline DAIFSet/DAIFClr asm.
7. context_switch() is the single entry point for both cooperative and
   preemptive switching.

## User Thread Entry Flow (ADR-018)

### First entry to EL0

    context_switch_asm
        restore SP_EL1 = kernel_stack_top
        restore x30 = eret_to_user_stub
        ret -> eret_to_user_stub

    eret_to_user_stub
        SP_EL1 -= 272 (-> frame base)
        load SP_EL0, ELR_EL1, SPSR_EL1 from frame
        restore x0-x30
        eret -> EL0

### Invariant 8

eret_to_user_stub is the ONLY component allowed to transform a
kernel execution context into an EL0 execution context. The scheduler
never creates EL0 state directly.

### Invariant 9

EL0 execution may only be entered through eret and may only return
through architecturally defined exception paths (SVC, IRQ, abort).
The kernel never uses ret to enter user code.
