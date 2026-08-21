# WX-001: User code pages were EL0-writable (W^X AP-encoding divergence)

## Status

**Closed (2026-08-21) — encoding fixed, enforced and verified in QEMU.**

## Summary

The M5.0 G3 acceptance criteria require `user code = RX`, `user data = RW`,
`no RWX` (M5.0-green-baseline §6, §32-G3). The descriptor encoders did **not**
satisfy this: every user page was encoded with AP[2:1] = `0b01`
(EL0 read/**write**) regardless of the requested writability. User code pages
mapped as "read-only + executable" were therefore writable+executable from
EL0 in practice — an RWX page — while the documentation claimed RX.

This is a documentation/code divergence of the exact kind the M5.0 audit
process is designed to catch: the gate was declared PASS on the strength of
the EFAULT/fault-containment tests, which never exercised a write to a
user *code* page.

## Root cause

All three AArch64 descriptor builders used the same lossy pattern:

```rust
if flags.user {
    d |= 1 << 6;            // AP[1] = 1 → EL0 access
} else if !flags.writable {
    d |= 2 << 6;            // kernel RO → AP = 0b10
}
```

For a user read-only page (`user=true, writable=false`) this yields
AP = `0b01` = **EL0 read-write**, not AP = `0b11` (EL0 read-only). The
`writable: false` intent of `PageFlags::USER_READ_EXEC` was silently dropped.

Affected encoders (all fixed):

| Encoder | Path |
|---------|------|
| `paging::MappingFlags::to_descriptor_bits` | early identity map, `PageTable::map/map_region` |
| `mmu::block_or_page_desc` | `PageTableBuilder::map` — boot user image (`mmu_map_user_pages`) |
| `mmu::flags_to_desc_bits` | runtime `mmu_map_object` |

Note: ADR-019 (Proposed) codified the lossy table (`user=true → AP=01`)
and listed `USER_READ_WRITE_EXEC` as the user-code flag; the later M5.0 W^X
policy ("user code → RX") superseded it without updating the encoding.
ADR-019 has been amended accordingly.

## Fix (implemented 2026-08-21)

1. **Single source of truth** — `paging/descriptor.rs::ap_bits(user, writable)`:
   EL1 RW→`0b00`, EL1 RO→`0b10`, EL0 RW→`0b01`, EL0 RO→`0b11`. All three
   encoders now route through it.
2. **Permission-rewrite mechanism** — `paging/walker.rs::leaf_with_permissions()`
   rewrites AP/XN/PXN bits of an existing leaf descriptor, preserving PA,
   type, AF, shareability and ATTRIDX (pure mechanism per ADR-030).
3. **arch-api `mmu_protect` contract** + arch-aarch64 implementation:
   change permissions of a mapped range at page granularity, splitting 2 MiB
   blocks where required, TLBI for the range afterwards.
4. **VMM wiring** — `AddressSpace::protect()` implemented (was `todo!()`):
   exact-match mapping validation → hardware rewrite → software shadow
   (`Mapping.permissions`) commit. Whole-mapping granularity only;
   partial-range protection requires mapping splitting (post-M5 backlog,
   needs a VA allocator).
5. **Boot-time verification** — `wx_verify_user_as(root, code_va, stack_va)`
   reads back live leaf descriptors and asserts code = AP=11/XN=0/PXN=1,
   stack = AP=01/XN=1. Runs for both user address spaces during boot
   (`[WX] ... PASS` lines). Encoding regression matrix added to the MMU
   smoke test (`test_wx_encoding`).
6. **Behavioral negative test** — the G3 fault task now first attempts a
   store to its **own code page** before the historical store-to-VA-0.
   Post-fix it data-aborts with `FAR = <code page VA>` (permission fault);
   if the W^X encoding ever regresses, that store succeeds and the fault
   moves to `FAR = 0` — making the regression observable in the boot log.

## Evidence (QEMU cortex-a53, 2026-08-21)

```text
[WX] root=0x40119000 code_va=0x5e000000 desc=...c3 AP=0b11 XN=0 PXN=1
[WX] root=0x40119000 stack_va=0x5e010000 desc=...43 AP=0b1 XN=1
[WX] user AS W^X verification PASS          (both user ASes)
EL0 fault: ESR=0x9200004f EC=36 FAR=0x5f000000 ELR=0x5f000004 — terminating task
```

ESR DFSC = `0b001111` → permission fault at L3 on the code-page store:
the MMU itself enforces read-only user code. Demo task, M6 lifecycle,
fault containment and preemption all unchanged; 95 s soak clean
(37 K log lines, zero panics).

## Impact / scope notes

- Only user+read-only mappings changed behaviour (previously none existed
  except user code pages, which were unintentionally RWX). Kernel and
  user-RW encodings are bit-identical to before.
- `access_ok(Write)` already keyed off AP bit 7, so it now correctly denies
  writes to code pages (before the fix it allowed them).
- Hardware-validation caveat: descriptor semantics were verified against
  QEMU's cortex-a53 model. The deferred L1/L2 table-descriptor question
  (`table_desc` 0b11 vs spec 0b10) is orthogonal and remains tracked in
  `MMU-descriptor-encoding-hardware-validation.md`; AP-bit semantics are
  architecturally defined (VMSAv8-64) and not affected by that issue.

## Follow-ups

- Partial-range `protect()` (sub-mapping granularity) once a VA allocator
  exists.
- Full-block fast path in `mmu_protect` (rewrite block AP bits when the
  range covers a whole 2 MiB block) — optimisation only.
- Consider `mprotect`-style syscall once syscall numbers leave the scope fence.
