# Interrupt Model

## IRQ Lifecycle

```
Peripheral asserts IRQ line
        ↓
GIC distributor routes interrupt to CPU interface
        ↓
GIC CPU interface signals IRQ to core
        ↓
Core checks PSTATE.I (IRQ mask)
        ↓
If unmasked: core enters exception state
        ↓
ELR_EL1 ← return address
SPSR_EL1 ← saved PSTATE
PSTATE.DAIF set (IRQs disabled in handler)
        ↓
Vector table lookup (VBAR_EL1 + offset for EL1h IRQ)
        ↓
save_and_eret macro (vectors.rs):
  - saves x0-x30, ELR_EL1, SPSR_EL1 on stack
  - calls irq_entry_handler
        ↓
irq_entry_handler (dispatcher.rs):
  - gic::acknowledge() → reads GICC_IAR (GICv2) or ICC_IAR1_EL1 (GICv3)
  - dispatches to registered handler via IRQ_TABLE[id]
  - gic::eoi(id) → writes GICC_EOIR or ICC_EOIR1_EL1
        ↓
save_and_eret epilogue:
  - restores ELR_EL1, SPSR_EL1, x0-x30
  - eret → returns to interrupted code
        ↓
PSTATE restored from SPSR_EL1 (IRQs re-enabled if they were before)
```

## Critical Section Pattern

```
let _irq = IrqGuard::new();     // save DAIF, disable IRQs
let guard = SPINLOCK.lock();    // acquire spinlock
// ... critical section ...
// drop(guard) → release spinlock
// drop(_irq)  → restore DAIF (IRQs re-enabled if they were before)
```

## Context Rules

| State           | Boot | Thread | Interrupt |
|-----------------|------|--------|-----------|
| PmmBitmap       | ✓    | ✓      | ✗         |
| SpinLock        | ✓    | ✓      | ✓(!)      |
| IrqGuard        | ✓    | ✓      | ✓         |
| GIC statics     | ✓    | ✗(1)   | ✓         |
| IRQ_TABLE       | ✓(2) | ✗(1)   | ✓         |
| Console (UART)  | ✓    | ✓      | ✓(!)      |

(!) = only with IrqGuard
(1) = written once at boot, read-only after
(2) = via register_irq

## Synchronization Primitives

- **IrqGuard**: RAII guard that saves DAIF and disables IRQs. Always use for interrupt-unsafe shared state access from thread context.
- **SpinLock**: Busy-wait lock. Must be combined with IrqGuard when used across interrupt/thread boundary.
- **Pattern**: `let _irq = IrqGuard::new(); let guard = lock.lock();`

## Dispatcher Rules

1. `irq_entry_handler` runs with IRQs disabled (hardware does this on exception entry).
2. Handlers must not block (no spinlocks without prior IrqGuard).
3. Handlers must not allocate from PMM (PmmBitmap is not IRQ-safe).
4. EOI must be called exactly once per acknowledged interrupt.
5. Spurious interrupts (IAR returns 0x3FF) must be handled without calling the handler.
