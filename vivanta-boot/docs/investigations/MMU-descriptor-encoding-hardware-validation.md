# MMU Descriptor Encoding — Hardware Validation Plan

**Status:** Deferred reliability / hardware-validation artifact (post-M5).
**Related:** `M5.0-green-baseline.md` G3 §5.6, `arch-aarch64/src/mmu.rs` `table_desc`.
**Date:** 2026-08-11

## Summary

Vivanta's AArch64 page tables currently encode L1/L2 **table** descriptors
as `0b11` (`DESC_VALID | DESC_TABLE`) because that is the only encoding QEMU's
`cortex-a53` model boots with. The VMSAv8-64 specification requires table
descriptors at L1/L2 to be `0b10` (bit0 clear, bit1 set); `0b11` is RES0 there.

Experimentally (QEMU, this milestone):

| Encoding | L1/L2 table | L3 page | QEMU cortex-a53 result |
|----------|-------------|---------|------------------------|
| `0b11`   | RES0 per spec | correct `0b11` | **boots** (used today) |
| `0b10`   | correct per spec | `0b11` | hangs at `mmu_activate` (isb) |

On real hardware, `0b11` at L1/L2 is reserved and should raise a translation
fault — meaning the current encoding is **not guaranteed to work on physical
ARM64**. This is why the honest status is "M5.0 QEMU-correct baseline with one
deferred ARM MMU portability issue", NOT "hardware-correct".

## What to validate on real hardware

Target a physical ARM64 board (RK3568 is the declared validation target, or
RPi3B+ which already uses `early_mmu`).

### Step 1 — Prove the failure (optional)

Before changing anything, confirm that `0b11` L1/L2 tables fault on hardware.
This establishes the defect is real on silicon, not only a spec concern.

### Step 2 — Switch encodings coherently

The encoding change is NOT a one-liner: walkers and predicates must agree.
Complete switch set (all must move together):

1. `arch-aarch64/src/mmu.rs` `table_desc()`:
   `DESC_VALID | DESC_TABLE` → `DESC_TABLE` (emit `0b10`).
2. `arch-aarch64/src/paging/walker.rs` `split_l2_block()`: the L2 entry write
   `l3_addr | DESC_VALID | DESC_TABLE` → `l3_addr | DESC_TABLE`.
3. `arch-aarch64/src/paging/descriptor.rs` `desc_is_table()`: currently treats
   any non-block as table (encoding-agnostic). After the switch it can be
   tightened to `desc & DESC_TYPE_MASK == 0b10`.
4. `arch-aarch64/src/early_mmu.rs` `l1_table_desc()`: already emits `0b10`
   (rpi3b path) — keep.
5. Walkers that check "is this a table vs block" (`mapper.rs`,
   `table_or_create` in `mmu.rs`, `walk_to_l3`) use `desc_is_table` /
   `desc_is_block`; they must be re-verified against `0b10` tables.

### Step 3 — Validation on hardware

1. Boot the QEMU target unchanged on the board's U-Boot (`booti`), confirm the
   same boot log (PMM 511 MiB, MMU smoke, EL0 demo, G4 preemption).
2. Specifically confirm `mmu_activate` completes (no hang at `isb`) and the
   `mmu_self_test` translate checks pass.
3. Run the G3 fault-containment and G4 preemption tests on hardware.
4. Record the board, U-Boot version, and exact boot log as evidence.

### Step 4 — Fallback decision

If hardware also rejects `0b10` (unlikely but possible on non-conformant
silicon), the `0b11` encoding stays and this document must record the
non-conformant hardware as a supported quirk. If `0b10` works, remove the
NOTE in `table_desc` and mark the issue resolved.

## Acceptance criteria for closing this issue

- Real hardware boots with the `0b10` table encoding (or a documented
  non-conformant-hardware exception).
- No regressions on QEMU after the switch.
- `desc_is_table` tightened to the spec encoding.
- This plan marked resolved in the M5.0 baseline tracker.

## Files that change when switching

```
arch-aarch64/src/mmu.rs            (table_desc, table_or_create walk)
arch-aarch64/src/paging/descriptor.rs  (desc_is_table)
arch-aarch64/src/paging/walker.rs      (split_l2_block)
arch-aarch64/src/paging/mapper.rs      (verify predicates)
arch-aarch64/src/early_mmu.rs          (already 0b10)
```
