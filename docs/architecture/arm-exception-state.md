# ARM Exception State Initialization

**Purpose:** Document ARMv8-A exception level startup state requirements for Vivanta.
**Validated:** 2026-07-19 (RK3568 Cortex-A55, QEMU AArch64)

---

## Floating Point / SIMD Initialization

### Problem

On ARMv8-A, FP/SIMD instructions trap at EL1 if `CPACR_EL1.FPEN` is not configured.
When entering from EL2, `CPTR_EL2.TFP` can additionally route all FP/SIMD traps to EL2
(instead of EL1). If the EL2 vector table is not set up (which is typical after
dropping to EL1), the trap causes an unrecoverable hang.

### Required Initialization

Before any Rust code executes (including `write_volatile` alignment checks that
may use NEON in debug builds):

```asm
// EL1 entry path (or before eret in EL2 path):
mov x5, #(0b11 << 20)       // FPEN = 0b11
msr CPACR_EL1, x5           // FP enabled at EL1 and EL0

// EL2 entry path ONLY (before eret):
msr CPTR_EL2, xzr           // TFP=0: no FP/SIMD traps to EL2
```

### Register Details

| Register | Field | Value | Effect |
|----------|-------|-------|--------|
| `CPACR_EL1` | FPEN[21:20] | `0b11` | Full FP/SIMD access at EL1 and EL0 |
| `CPACR_EL1` | FPEN[21:20] | `0b00` | Trap FP/SIMD at EL1 and EL0 to EL1 (reset value) |
| `CPTR_EL2` | TFP[20] | `0` | No traps to EL2 (FP/SIMD controlled by CPACR_EL1) |
| `CPTR_EL2` | TFP[20] | `1` | All FP/SIMD traps to EL2 regardless of CPACR_EL1 |

### Symptom

- Debug builds: `write_volatile` calls `precondition_check` → `is_aligned_to` → NEON
  (`fmov`, `cnt`, `addv`) → synchronous exception → hang
- Release builds (`opt-level = "z"` or `release`): may work because debug checks
  are omitted and no NEON instructions are generated
- QEMU with `-kernel` at EL2 exhibits the same failure as RK3568 hardware

### Affected Targets

All ARMv8-A targets entering at EL2:
- RK3568 (Cortex-A55) — confirmed
- QEMU AArch64 virt — confirmed (QEMU 11.0.2, `-kernel` boots at EL2)
- Raspberry Pi 3B+ (Cortex-A53) — likely, not yet tested

### Verification

Check at EL1 after initialization:
```asm
mrs x6, cpacr_el1
lsr x6, x6, #20
and x6, x6, #3
// x6 = 3 → FPEN configured correctly
```
