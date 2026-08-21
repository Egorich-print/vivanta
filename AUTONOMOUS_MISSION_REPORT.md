# AUTONOMOUS MISSION REPORT — M5.1/M5.2 Virtual Address Space & Page-Table Lifecycle

**Date:** 2026-08-21 (mission 3)
**Repository:** https://github.com/Egorich-print/vivanta (`main`)
**Base:** `bab8130` (mission-2 final)

Tags: `FIXED` / `VERIFIED` / `KNOWN LIMITATION` / `BACKLOG` / `NOT TESTED`.

---

## BASELINE

`bab8130`: check/test/fmt clean, clippy = pre-existing style debt only, all
6 targets build, QEMU full gate suite PASS (W^X ×2, protect/TLBI, 3 fault
scenarios, M6, G4), 95 s stress clean (40K lines, 0 panics). Pre-existing
foreign files (`plan-next-phase` edit, untracked research docs) untouched.

## ARCHITECTURE

New layering, respecting ADR-030's mechanism/policy split:

```text
vivanta-vm (pure, host-proven)     VA intervals, range math, invariants
        ↓ used by
kernel::vmm                        AddressSpace + shadow + VA allocator,
                                   table ownership registry (policy)
        ↓ arch-api
arch-aarch64                       descriptor mechanism: walk (now reports
                                   Missing*), install_child_table, split,
                                   valid-leaves count, leaf readback
```

## VA ALLOCATOR — VERIFIED

First-fit interval allocator over `[USER_VA_BASE=0x0100_0000, USER_VA_END=
0x4000_0000)`; page 0 guard; kernel AS uses a disabled allocator. Free list
is canonical (sorted+merged) and the single truth for "allocated" — overlap
is impossible by construction; double-free/foreign-range are deterministic
errors; all arithmetic overflow-checked. Proven by 7 host tests including a
20 000-operation model-checked lifecycle stress. Three real allocator bugs
found and fixed during self-testing: uninitialized const-constructor slot,
spurious overflow for already-aligned near-TOP addresses, and a merge pass
whose swap-remove broke sort order (fragmented drains).

## PAGE-TABLE OWNERSHIP — VERIFIED

Registry model (`vmm::tables`, ADR-031): arch notifies
`table_installed(frame, parent, index, level)` on every child-table
install; kernel records `{frame, as_id, level, parent_table, parent_index,
backend}`. Roots and boot-era frames never enter the registry → leak by
rule; registry overflow degrades to leak. Chosen over refcounting (no
shared tables, single-core, simplest provable model).

## MAPPING LIFECYCLE — VERIFIED

`map_new_range` (alloc VA → map → rollback VA on failure), `unmap_range`
(unmap → free VA), both built on range-correct `map_pages`/`unmap_pages`.
`mmu_map_object` now creates missing intermediate tables through the
allocator (with ownership notification); unmap/protect keep the strict
"must be mapped" contract.

## PROTECT — VERIFIED

Partial ranges supported: coverage proof + transactional shadow splitting
(head/covered/tail pieces, capacity pre-checked before any mutation),
hardware programmed once, shadow committed last. QEMU: middle-page RO
protect verified readable-with-content, shadow shows exactly 3 pieces with
correct permissions, RW restore write proves no stale permissions.

## UNMAP — VERIFIED

Range semantics mirror protect. After clears, `reclaim_empty_tables`
runs to fixpoint under the IRQ guard. QEMU asserts exact PMM deltas
(reclaimed count == free_count increase).

## TLB — VERIFIED within emulation limits

Per-operation obligations documented in ADR-031 §6. Table unlink needs no
additional TLBI (subtree already uninstantiated). Stale-permission proof
after RW→RO→RW re-verified in the new AS. **KNOWN LIMITATION**: missing
TLBI after permission *widening* remains unobservable under QEMU
(mission-2 M4 finding unchanged); narrowing bounded by tlbi_all on AS
switch.

## ALIASES — VERIFIED

Policy: VAs may alias one PA; physical ownership never follows unmap (the
VMM has no PA-free path at all). QEMU regression: alias map → write via
alias → original VA observes value → unmap alias → original still
translates; PMM deltas exact through steps 4–5 prove no premature free.
COW/shared ownership: BACKLOG (must extend ADR-031).

## BUGS FOUND

1. **`walk_to_l3` could not report missing intermediates** — allocator-
   chosen VAs outside boot-mapped regions were unmappable (panic). FIXED:
   `MissingL2`/`MissingL3` variants + creation path with ownership notify;
   strict panic preserved for unmap/protect.
2. **`unmap_pages` was whole-mapping-only** — orphaned shadow pieces after
   partial protect (caught by the new VM test as `AddressSpaceBusy`).
   FIXED: range semantics.
3. **Stack exhaustion regression (self-inflicted, caught immediately)**:
   `AddressSpace` grew to ~8.8 KB with the embedded VaAllocator; stack
   temporaries corrupted `.bss` (silent hang in boot println via console-
   lock deadlock — INV-002's symptom pattern). FIXED: boot stack 32→64 KiB
   (linker script, M6-precedent comment).
4. Allocator defects (const-slot init, round_up overflow, merge swap-remove)
   — FIXED during host-test hardening.

## MUTATION TESTING

| mutation | expected | observed |
|---|---|---|
| M1 free active table frame (drop obj) | accounting failure | `PANIC: reclaimed 1 table frames (PMM +0)` ✅ |
| M2 reclaim non-empty table | wrongful reclamation visible | `PANIC: intermediate table missing` ✅ |
| M3 omit parent unlink | stale-table reuse detectable | `PANIC: remap reused a stale table: registry entry missing` ✅ (needed new structural assert — plain QEMU missed it) |
| M4 corrupt VA overlap check | host test failure | `free_merge_and_double_free FAILED` ✅ |
| M5 skip TLBI (widening) | — | KNOWN LIMITATION (QEMU re-walk) |
| M6 split attribute loss | descriptor audit fires | scenario failure ✅ + neighbor AF audit added (QEMU ignores AF, so audit is structural) |
| M7 free PA while alias exists | structurally excluded | VERIFIED by construction + exact PMM deltas |
| M8 cross user/kernel domain | host test failure | `free_merge_and_double_free FAILED` ✅ |
| M9 root reclamation | structurally impossible | VERIFIED (roots never registered; count asserts) |

All mutations reverted; final tree verified mutation-free.

## QEMU EVIDENCE

Full boot: W^X ×2 PASS → protect/TLBI PASS → 3 fault scenarios PASS →
**VM lifecycle test**: map 3 pages via allocator (`va0=0x01000000`,
in-domain) → partial protect w/ shadow-split check → RW restore → alias
regression → unmap with reclamation (`reclaimed 1 table frames, PMM +1`)
→ block-split case (split-inherited table NOT reclaimed; neighbor AF
audit) → remap (registry reinstall asserted) → unregister teardown.
M6 demo OK, G4 running=1, 95 s stress: 38K lines, 0 panics, preempt
counters ~7×10⁹.

## REGRESSION

check/test/clippy/fmt clean (host + aarch64-unknown-none); 13/13 host
tests; all 6 targets build; no new warnings; M5/M6/containment/preemption
unchanged.

## KNOWN LIMITATIONS

- TLBI-widening invisibility under QEMU (see above) — hardware validation
  pending (pre-existing deferral).
- Registry capacity 256 frames; overflow leaks (deterministic, safe).
- Reclamation is O(registry-scan) per unmap — fine at kernel scale.

## BACKLOG

- mmap/munmap/mprotect syscalls on top of the new primitives (API-ready:
  reserve/map_new_range/protect/unmap_range; needs syscall-number scope
  lift + SYS_READ work).
- COW / shared page tables → extend ADR-031 ownership model.
- Physical-frame ownership transfer for user MemoryObjects (user flag in
  MemRights).

## COMMITS

```text
branch: main
base:   bab8130
feat(vm): add vivanta-vm virtual address allocator (host-proven)
refactor(mmu): page-table ownership protocol + missing-table creation
feat(vm): range mapping, partial protect, table reclamation
test(vm): QEMU VM lifecycle test + mutation battery
docs(vm): ADR-031 + status/roadmap sync + mission report
(see git log for final hashes)
```

## FINAL GIT STATUS

Committed: all mission work in logical commits. Untouched foreign state:
`plan-next-phase` modification + untracked research docs (as found).
