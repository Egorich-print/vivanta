# ARM Exception State Initialization

**Purpose:** Document ARMv8-A exception-level startup state requirements for
Vivanta, and record where the code diverges from them.
**Last verified against code:** 2026-09-29, HEAD `273f9bc`.

> **Status of the prescription below: architecturally correct, and NOT what the
> code does.** Five of the six targets get FP/SIMD enablement wrong in **two
> independent ways**: a `CPTR_EL2` store that is an architectural no-op, and a
> `CPACR_EL1` store that is unreachable from the EL2 path. See
> [Open blocker](#open-blocker-fpsimd-left-at-reset-on-five-targets).

---

## Floating Point / SIMD Initialization

### Problem

On ARMv8-A, FP/SIMD instructions trap at EL1 if `CPACR_EL1.FPEN` is not
configured. When entering from EL2, `CPTR_EL2.TFP` can additionally route all
FP/SIMD traps to EL2 instead of EL1. If the EL2 vector table is not set up —
which is the normal situation after dropping to EL1 — the trap causes an
unrecoverable hang.

### Required Initialization

Before any Rust code executes (including `write_volatile` alignment checks,
which emit NEON in debug builds):

```asm
// EL1 entry path — or, on an EL2 path, after eret has landed you in EL1:
mov x5, #(0b11 << 20)       // FPEN = 0b11
msr CPACR_EL1, x5           // FP enabled at EL1 and EL0

// EL2 entry path, before eret:
msr CPTR_EL2, xzr           // TFP=0: no FP/SIMD traps to EL2
```

Both stores are required on **every** entry path. `msr CPACR_EL1` is what
actually enables FP/SIMD at EL1 and EL0; `msr CPTR_EL2, xzr` only stops EL2
from intercepting the traps that result. Dropping the first leaves
`CPACR_EL1.FPEN` at its reset value `0b00`, which traps FP/SIMD to EL1.

These are two different registers, and the `0b11 << 20` constant means
something only in one of them: `CPACR_EL1.FPEN` is **bits 21:20**, while
**`CPTR_EL2.TFP` is bit 10** — bits 20:21 of `CPTR_EL2` are RES0. Writing
`0b11 << 20` to `CPTR_EL2` is therefore **an architecturally no-op**: it does
not set TFP, and it does not clear it either. `TFP` is simply left at its
reset value.

`vivanta-boot/target-rk3568/src/main.rs:190-191` (`msr CPTR_EL2, xzr` then
`msr CPACR_EL1, #(0b11<<20)`) is the only place in the tree that implements
this.

### Register Details

| Register | Field | Value | Effect |
|----------|-------|-------|--------|
| `CPACR_EL1` | FPEN[21:20] | `0b11` | Full FP/SIMD access at EL1 and EL0 |
| `CPACR_EL1` | FPEN[21:20] | `0b00` | Trap FP/SIMD at EL1 and EL0 to EL1 (reset value) |
| `CPTR_EL2` | TFP[10] | `0` | No traps to EL2 (FP/SIMD governed by CPACR_EL1) |
| `CPTR_EL2` | TFP[10] | `1` | All FP/SIMD traps to EL2 regardless of CPACR_EL1 |
| `CPTR_EL2` | [21:20] | any | RES0 — writing `0b11 << 20` here has no effect at all |

### Symptom

- Debug builds: `write_volatile` calls `precondition_check` → `is_aligned_to` →
  NEON (`fmov`, `cnt`, `addv`) → FP/SIMD exception, taken to EL1 because
  `CPACR_EL1.FPEN` is still `0b00` → `el1_sync_handler` resolves only same-EL
  data aborts (`arch-aarch64/src/exceptions.rs:90-116`), so an FP exception
  falls through to `exception_handler`, which dumps and halts
  (`:118-119`, `:124-134`).
- Release builds (`opt-level = "z"`): may work, because the debug checks are
  omitted and no NEON is generated.
- QEMU does **not** surface this today: the gate is green in exactly the
  profile that should trap. See the blocker below.

### Verification

Read back at EL1 after initialization:

```asm
mrs x6, cpacr_el1
lsr x6, x6, #20
and x6, x6, #3
// x6 = 3 → FPEN configured correctly
```

---

## Open blocker: FP/SIMD left at reset on five targets

This is a **P0 silicon blocker**, recorded in
[`docs/audit/2026-09-24-global-audit.md:76-87`](../audit/2026-09-24-global-audit.md).
That audit entry labels it "`CPTR_EL2.TFP=1`"; the label is wrong (see
[Register Details](#register-details)) — the register is never set — but the
defect and its severity are real.

Two independent defects, both present on the EL2 entry path:

1. **The wrong register is written.** `mov x5, #(0b11 << 20)` +
   `msr CPTR_EL2, x5`. `0b11 << 20` is the encoding of `CPACR_EL1.FPEN[21:20]`.
   It is being written to `CPTR_EL2`, where `TFP` is bit 10 and bits 20:21 are
   RES0 — so the store is **an architecturally no-op**. It does not set TFP.
   The net effect is that FP/SIMD control is left at its reset state.
2. **The `CPACR_EL1` store is dead code on the EL2 path.** The unconditional
   `b 7f` (or, on `x96q`, the `eret` that returns to label `7f`) jumps past
   the `msr CPACR_EL1` at label 6, so `CPACR_EL1.FPEN` stays at its reset
   value `0b00`, which **traps FP/SIMD to EL1**.

Verified 2026-09-29 by reading every `main.rs` EL-detection block:

| Target (all under `vivanta-boot/`) | EL2 path — `src/main.rs` line in brackets | EL2-path `CPTR_EL2` write | Does the EL2 path set `CPACR_EL1`? |
|--------------------------------------|----------|----------|-------------------------------------|
| `target-qemu-aarch64/src/main.rs:31` | `msr CPTR_EL2, #(0b11<<20)` | no-op ❌ | `:35` exists but is skipped — `b 7f` at `:32` |
| `target-suma-q5/src/main.rs:60` | `msr CPTR_EL2, #(0b11<<20)` | no-op ❌ | `:64` exists but is skipped — `b 7f` at `:61` |
| `target-lavender/src/main.rs:40` | `msr CPTR_EL2, #(0b11<<20)` | no-op ❌ | `:44` exists but is skipped — `b 7f` at `:41` |
| `target-rpi3b-plus/src/main.rs:42` | `msr CPTR_EL2, #(0b11<<20)` | no-op ❌ | **no `msr CPACR_EL1` instruction anywhere in the target** |
| `target-x96q/src/main.rs:53` | `msr CPTR_EL2, #(0b11<<20)` | no-op ❌ | `:68` exists, on the skipped path — `eret` at `:65` returns to label `7f` (`:69`) |
| `target-rk3568/src/main.rs:190` | `msr CPTR_EL2, xzr` | **TFP cleared** ✅ | `:191` ✅ |

**Only `target-rk3568` implements this document's prescription.** And
`target-rk3568` is the one target that never links the kernel.

Consequences, in order of severity:

1. Every target that reaches `kernel_main` via the EL2 path boots with
   `CPACR_EL1.FPEN=0b00` and `CPTR_EL2.TFP` at its reset value. Any NEON
   emitted by `write_volatile` in a debug build traps at EL1 before the first
   banner character and halts in `exception_handler`.
2. Both halves of the fix are needed, and on four of the five affected
   targets **neither half is in effect**: the `CPTR_EL2` store does nothing
   and the `CPACR_EL1` store is unreachable. Fixing only the register name
   would leave `FPEN` at `0b00`.
3. `target-rpi3b-plus` is the worst case: it performs the no-op `CPTR_EL2`
   write and has no `msr CPACR_EL1` instruction at all, so there is no second
   code path to fall back on.

Why the QEMU gate passes anyway: the gate builds the **debug** profile
(build line `vivanta-boot/tools/qemu-gates.sh:55-56`, output under
`vivanta-boot/target/aarch64-unknown-none/debug/`),
which is exactly the configuration in which `write_volatile` emits NEON and
should trap on `CPACR_EL1.FPEN=0b00`. It does not trap, and the gate is green.
So QEMU does not surface this defect — neither the failed `CPTR_EL2` store nor
the unreachable `CPACR_EL1` store is exercised there. It is a **silicon**
blocker, not a logic error — which is precisely why it survived: the only
configuration anyone runs never exposed it.

Fix per target: in the EL2 branch replace `mov x5, #(0b11 << 20)` +
`msr CPTR_EL2, x5` with `msr CPTR_EL2, xzr`, **and** make the
`msr CPACR_EL1, #(0b11<<20)` store run on every entry path rather than sitting
behind a label that the EL2 branch jumps past. Correct reference
implementation: `vivanta-boot/target-rk3568/src/main.rs:190-191`.

### Related defect — corrected 2026-09-29

`vivanta-boot/arch-aarch64/src/interrupts.rs:13-28`: `enable()` executes
`msr DAIFClr, #2` and `disable_interrupts()` executes `msr DAIFSet, #2`. The
`DAIFSet`/`DAIFClr` `imm4` field order is bit3=D, bit2=A (SError), bit1=I (IRQ),
bit0=F; `#2` is therefore the **IRQ** mask (I), which is what an IRQ guard
needs. `PSTATE.I` is bit 7, mirrored as bit 1 of `imm4`, so it **is** set/cleared
via `DAIFSet/Clr #2`. The code is correct.

**Correction (2026-09-29):** an earlier revision of this section claimed "bit 2
is SError, not IRQ", that IRQ masking needed `#4`, and that `PSTATE.I` was never
manipulated. That inverted the `imm4` encoding and is **wrong**. It was disproven
empirically by reading DAIF after each instruction on QEMU (`DAIFSet #2` → I=1;
`DAIFSet #4` → A=1). The project's `#4` experiment then failed to clear the IRQ
mask at all, which is why the timer stopped — a defect in the experiment, not a
GIC delivery bug. There is **no evidence of a GIC IRQ delivery defect**, and this
is not the cause of the advisory CI gate; the scheduler race is. Recorded at
[`docs/audit/2026-09-24-global-audit.md`](../audit/2026-09-24-global-audit.md)
(dated correction note).

**A related gap: there is no EL0 FP/SIMD context save.** `d8-d15`, FPSR and FPCR
are not preserved across EL0 entry/exit anywhere in
`vivanta-boot/arch-aarch64/src/`. Until `CPACR_EL1.FPEN` is set on these paths
this is latent; once FP is enabled for EL0 it becomes silent user-state
corruption (`docs/audit/2026-09-24-global-audit.md:74-75`).

---

## Exception vector coverage

`vivanta-boot/arch-aarch64/src/vectors.rs:253-269` defines all 16 vectors.
**6 return, 10 halt.**

| Outcome | Vectors |
|---------|---------|
| Returns | `el1t_irq` (`:266`), `el1h_irq` (`:267`), `lower_aarch64_irq` (`:268`), `lower_aarch32_irq` (`:269`), `el1h_sync` (`:256`, `save_and_eret_sync_el1`), `lower_aarch64_sync` (`:259`, `save_and_eret_sync`) |
| Halts | `el1t_sync` (`:253`), `el1t_fiq` (`:254`), `el1t_serror` (`:255`), `el1h_fiq` (`:257`), `el1h_serror` (`:258`), `lower_aarch64_fiq` (`:260`), `lower_aarch64_serror` (`:261`), `lower_aarch32_sync` (`:262`), `lower_aarch32_fiq` (`:263`), `lower_aarch32_serror` (`:264`) |

Notes:

- `el1h_sync` is the only resumable EL1 fault class: demand-fill and COW
  resolution (ADR-032).
- `lower_aarch64_sync` dispatches SVC and EL0 faults.
- `lower_aarch32_sync` **halts**, so there is no AArch32 EL0 support. An
  `lower_aarch32_irq` vector exists and returns, but no AArch32 EL0 thread can
  reach it.
- Every FIQ and every SError halts. `el1h_serror` halting means an SError at EL1
  is unrecoverable by design.
- There is no EL2 vector table in the live tree. That is no longer what makes
  this fatal — `CPTR_EL2.TFP` is never actually set, so the traps go to EL1
  and the live vector table does catch them — but it does mean a *correctly
  written* `CPTR_EL2` fix must not be allowed to regress into setting TFP.

---

## Platform verification status

The previous version of this file claimed:

> **Validated:** 2026-07-19 (RK3568 Cortex-A55, QEMU AArch64)

That is a category error and is narrowed here. What the evidence actually
supports:

| Platform | What was actually demonstrated | Source |
|----------|--------------------------------|--------|
| **RK3568 (Cortex-A55)** | Real UART output from a Vivanta **EL2-stage diagnostic** on silicon, 2026-07-19. Decoded signature `KC3FGZAB1CONSOLE OK231234WITH CONSOLE OK54X`; the `C3` marker is the FPEN=3 readback and the `Z` marker precedes the adapter call, so the FP/SIMD fix in `target-rk3568` genuinely was exercised on hardware. **It never reached `kernel_main`.** | `docs/hardware/rk3568/board-info.md:409-437` (`:519-520` for the FPEN/TFP readback claims) |
| **QEMU AArch64 (cortex-a53)** | The kernel runs and the 18-marker gate passes — but under a `continue-on-error: true` CI job, at ~1 failure in 2.4 runs. Reproducible on demand via `vivanta-boot/tools/qemu-gates.sh`; no committed log is stored, and none should be treated as a static artefact. | `.github/workflows/ci.yml:87-99` |
| **Raspberry Pi 3B+ (Cortex-A53)** | **No Vivanta code has run on this board.** The 2026-08-14 verification was booting **BalanSir's Buildroot Linux image** and reaching a login prompt on the GPIO mini-UART. It established the board's UART routing (PL011 → Bluetooth, mini-UART → GPIO 14/15) — real and useful — but says nothing about this kernel. `vivanta-boot/target-rpi3b-plus` links the kernel, calls `kernel_main` at `vivanta-boot/target-rpi3b-plus/src/main.rs:132`, and has never been executed. | `docs/hardware/rpi3b/uart-bringup-verified.md:1-9` |
| **X96Q (Allwinner H313)** | Compile-checked only; needs silicon. | `docs/audit/2026-09-24-global-audit.md:66-70` |
| **Sumavision S905L (Amlogic)** | Cannot print: `platform-amlogic` never calls `set_console`, so `with_console()`'s `.expect(...)` panics first. | `vivanta-boot/platform-amlogic/src/lib.rs:105-114` |

The former "Raspberry Pi 3B+ — likely, not tested" line understated what was
verified about the *board* while overstating what was verified about *Vivanta*.
Both halves are stated above.
