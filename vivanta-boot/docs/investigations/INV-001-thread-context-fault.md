# INV-001: Thread context switch fault on RUNQUEUE access

## Status

Open

## Date

2026-07-13

## Bug

Thread `yield_now()` from Thread A (running on its own 16KB BSS stack) 
crashes at the address of `RUNQUEUE` (a `static mut` in BSS) with 
ESR=0x02000000 (EC=0, Unknown reason).

Boot → Thread A via `yield_now()` **works**. Thread A → Boot via 
`yield_now()` **crashes**.

## Symptoms

- ELR = 0x4021ef70 (address of `RUNQUEUE` in BSS)
- x0 = 0x4021ef70 (same — x0 was used to compute RUNQUEUE address)
- x30 = 0x4021ef70 (LR also points to RUNQUEUE)
- SP = valid address within Thread A's stack (0x4022xxxx)
- x19-x28 = 0, x29 = 0, x30 = RUNQUEUE address
- FAR_EL1 = 0x0000000000000000
- ESR_EL1 = 0x0000000002000000
  - EC[31:26] = 0b000000 = 0x00 (Unknown reason)
  - ISS = 0

## Observations

1. `boot_common::println!("  [entry yield]")` at the top of `yield_now` 
   **succeeds** when called from Thread A — UART write works.
2. The crash occurs on the very next instruction after the println 
   returns — `let cur = CURRENT;`.
3. The same code path from Boot's context works perfectly.
4. The crash is **deterministic** — same address every run.
5. Removing the `let cur = CURRENT` and replacing with `let cur = 0` 
   still crashes (but only if we later access RUNQUEUE via 
   `find_next_ready`).

## Hypotheses

### H1: Stack not mapped by MMU

Thread A's stack is a `static mut` array in kernel BSS
(0x4022xxxx range). MMU identity-maps 0x40000000-0x5FFFFFFF.
BSS is within this range. **Low probability** — the stack IS mapped.

### H2: SP invalid after context_switch

The context_switch restores SP from Thread A's context (set to 
stack_top = base + 16384). SP = stack_top means the first push 
will write BELOW stack_top, which is within the stack array. 
**Low probability** — stack_top is valid.

### H3: Simple static access also crashes

If accessing any static from Thread A's context crashes, the issue 
is not RUNQUEUE-specific but context-related. **To test.**

### H4: ABI violation in context_switch

Context_switch saves x19-x30 + SP but doesn't maintain the ABI 
invariant that x18 (platform register) or other reserved registers 
are handled. If the compiler uses x18 as a base register for static 
access, and x18 is corrupted, the next static access will fault. 
**To test — this is the leading hypothesis.**

## Experiments

### EXP-001: Test simple static access from Thread A

Thread A reads a plain `static TEST: u64 = 42` (NOT `static mut`)
and prints it. If this crashes, the issue is context/ABI, not 
RUNQUEUE-specific.

### EXP-002: Print SP and x30 from Thread A

Thread A reads its own SP and LR and prints them to verify 
they're valid.

### EXP-003: Test with global_asm! for yield

Replace the Rust `yield_now()` with a `global_asm!` function that 
saves/restores registers using AAPCS64 rules, avoiding any 
compiler-generated code in the path.

## Resolution

### Root cause

Two independent issues were involved:

1. **`context_switch` as inline `asm!` with `options(nostack)`**: The inline asm changed SP but told the compiler it didn't (`options(nostack)`). This caused the compiler to generate incorrect prologue/epilogue code that corrupted the callee-saved registers (x19-x30) and the LR (x30). When `yield_now()` returned to `kernel_main`, x30 had the RUNQUEUE data address instead of the correct return address, causing a jump to data memory.

2. **`save_and_eret` doing `add sp, sp, #272`**: The macro restored the original SP by adding 272 to the current SP. When `maybe_reschedule` switched threads, the macro restored the NEW thread's registers but kept the OLD thread's SP. The new thread ran on the wrong stack.

### Fixes

| Issue | Fix | File |
|-------|-----|------|
| Inline asm with `options(nostack)` | `context_switch_asm` as `global_asm!` + `extern "C"` (proper AAPCS64 function) | `thread.rs` |
| `add sp, sp, #272` keeps old SP | Macro loads new SP from ExceptionFrame field, computes new frame address, switches SP | `vectors.rs` |
| maybe_reschedule writes to old frame only | Added `copy_nonoverlapping(next.full, next.full.sp - 272)` | `thread.rs` |

### Post-resolution status

After the fixes:
- Cooperative yield: Boot → A → B → Boot works (verified)
- Preemptive timer: A ↔ B ↔ A at 100 Hz works (verified for 8+ seconds)
- Console lock: prevents interleaving during preemption (verified)
- Stress test: 3 threads, 5+ minutes, stable (verified)
- Idle thread: wfi loop, falls back when no other thread ready (verified)
