# ADR-019: User Page Permissions and EL0 Memory Model

## Status
Proposed

## Date
2026-07-17

## Related ADRs
- ADR-017 Unified Execution Context
- ADR-018 User Entry Transition Model
- ADR-013 Privilege Transition Model

## Context

ADR-017 and ADR-018 defined how a thread transitions from EL1 to EL0 via
eret_to_user_stub. M4.5.1 (First EL0 Entry) requires user code and stack
pages to be mapped with correct page permissions.

The existing PageFlags model (ADR-014 era) has three fields — `writable`,
`executable`, `user` — and only one Execute-Never bit (UXN, bit 54).
This is insufficient for proper user page permissions because:

1. **PXN is missing** — user pages are executable from EL1, violating
   kernel/user separation.
2. **No distinction between permission intent and descriptor encoding**
   — the API hardcodes AArch64-specific bits rather than expressing
   architectural intent.

Additionally, the TTBR0/TTBR1 split is not yet implemented. M4.5.1 uses
a unified TTBR0 address space for both kernel and user mappings. This
requires explicit documentation so it is not mistaken for the final model.

## Decisions

### 1. Permission intent vs descriptor encoding

`PageFlags` describes the **intended access semantics** of a memory region.
The architecture backend translates these semantics into ISA-specific
descriptor bits:

| PageFlags field    | AArch64 encoding           |
|--------------------|----------------------------|
| `executable=true`  | UXN = 0 (executable)       |
| `executable=false` | UXN = 1 (non-executable)   |
| `privileged_executable=true` | PXN = 0 (EL1-executable) |
| `privileged_executable=false`| PXN = 1 (EL1-non-executable) |
| `user=true`        | AP[2:1] = 01 (EL0 RW)      |
| `user=false`       | AP[2:1] = 00 (EL1 only)    |

When a RISCV64 or ARMv7 backend is added, the same `PageFlags` fields
produce the corresponding page-table entries for those ISAs.

### 2. PageFlags model

```rust
pub struct PageFlags {
    pub writable: bool,
    pub executable: bool,
    pub user: bool,
    pub privileged_executable: bool,
}
```

Pre-defined constants:

| Constant | writable | executable | user | privileged_executable | Usage |
|----------|----------|------------|------|----------------------|-------|
| `READ_ONLY` | false | false | false | true | kernel rodata |
| `READ_WRITE` | true | false | false | true | kernel data, stacks |
| `READ_WRITE_EXEC` | true | true | false | true | kernel code |
| `USER_READ_WRITE` | true | false | true | false | user stack |
| `USER_READ_WRITE_EXEC` | true | true | true | false | user code |

Key invariants for user pages:

- `privileged_executable = false` → PXN = 1 — kernel cannot execute user
  pages directly.
- `user = true` → AP = EL0 RW — user mode can read and write.
- Kernel pages always have `privileged_executable = true` and `user = false`.

### 3. TTBR0-only unified model (intentional)

M4.5.1 intentionally uses a unified TTBR0 address space for both kernel
and user mappings.

This is acceptable because M4.5.1 validates:
- EL1 → EL0 privilege transition via eret_to_user_stub
- Exception return path (SVC → EL1 handler → eret → EL0)
- User execution state (CurrentEL = EL0, SPSR = 0x000)
- SVC roundtrip (user → kernel → user)

M4.5.1 does NOT claim:
- Kernel/user page table isolation
- Separate kernel address space (TTBR1)
- ASID-based process switching
- Independent user address spaces per process

These are deferred to a future milestone (M5.x — Kernel Address Space
Split). The TTBR0-only model is temporary and will be replaced when the
kernel gains a dedicated high-VA mapping region in TTBR1.

### 4. EL0 Transition Ownership

Only the following code paths may transition execution from EL1 to EL0:

| Code path | Role | Allowed? |
|-----------|------|----------|
| `eret_to_user_stub` | First EL1 → EL0 entry for a user thread | ✅ |
| `save_and_eret_sync` | Return to EL0 after handling an EL0 exception | ✅ |
| `save_and_eret` | Return to interrupted EL1/EL0 after IRQ | ✅ |
| Any kernel function executing `eret` directly | — | ❌ Forbidden |
| `UserBootstrap::enter()` or equivalent | — | ❌ Forbidden (removed in M4.5.1) |
| Scheduler creating `x30 = entry` for a User thread | — | ❌ Forbidden (must use eret_to_user_stub) |

Invariant 8 from ADR-018 is reinforced: `eret_to_user_stub` is the ONLY
component allowed to create a fresh EL0 execution context.

### 5. SVC return path

When a user thread executes `svc #N`:

1. Hardware traps to `lower_aarch64_sync` vector.
2. `save_and_eret_sync` macro saves x0–x30, SP_EL0, ELR_EL1, SPSR_EL1
   to the current kernel stack (at SP_EL1 - 272).
3. `el0_sync_handler` is called with `&mut ExceptionFrame`, ESR_EL1,
   FAR_EL1.
4. Handler:
   - Reads syscall number from `frame.x[8]` (future — M4.5.2).
   - Sets return value in `frame.x[0]`.
   - Advances `frame.elr += 4` to skip the SVC instruction on return.
5. `save_and_eret_sync` restores registers from the frame and `eret`.
6. User thread continues at the instruction after `svc`.

This contract applies to all synchronous exceptions from EL0.
For M4.5.1, the only SVC number is effectively `0` (no syscall ABI yet).

### 6. M4.5.1 acceptance criteria

| # | Criterion | Verification |
|---|-----------|-------------|
| A0 | User thread reaches EL0 via `eret_to_user_stub` | QEMU output: first SVC handled |
| A1 | CurrentEL == EL0 during user execution | `mrs x1, CurrentEL` in user code (printed via SVC) |
| A2 | SVC roundtrip: user → kernel → user | Two consecutive SVCs handled |
| A3 | User code: UXN=0, PXN=1 | Page table inspection |
| A4 | User stack: UXN=1, PXN=1 | Page table inspection |
| A5 | kernel_page: PXN=0, user=false | Page table inspection |
| A6 | SPSR_EL1 = 0x000 at first eret | Exception frame dump (debug) |
| A7 | SP_EL1 != SP_EL0 during SVC handler | `frame.sp` vs `SP_EL1` |
| A8 | `cargo build -p target-qemu-aarch64` passes | Build |

## Consequences

### Positive
- Clear separation between permission intent and ISA encoding.
- PXN prevents kernel from executing user pages.
- Single well-defined SVC return path.
- TTBR0-only keeps M4.5.1 focused on privilege transition, not memory model.
- Decoupled architecture: PageFlags changes do not affect scheduler.

### Negative
- Unified TTBR0 means kernel and user share the same VA space.
  User can read kernel data (though not execute kernel code).
- `privileged_executable` is an additional field that all page-mapping
  call sites must consider.
- TTBR1 split will require revisiting all page permission assignments.

### Risk mitigation
- Unified TTBR0 is explicitly temporary — documented and tracked.
- PXN is an additional protection even without TTBR split.
- All kernel page table entries set `privileged_executable = true`;
  the risk of forgetting is low in practice.
- Future TTBR1 split will add a separate root for kernel mappings,
  providing true isolation even without PTE permissions.
