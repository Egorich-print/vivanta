# AUTONOMOUS MISSION REPORT — M6.0 User Virtual Memory & Fault-Driven Mapping Foundation

**Date:** 2026-08-21 (mission 4)
**Repository:** https://github.com/Egorich-print/vivanta (`main`)
**Base:** `2ceef95` (mission-3 final)

Tags: `FIXED` / `VERIFIED` / `KNOWN LIMITATION` / `BACKLOG` / `NOT TESTED`.

---

## BASELINE

`2ceef95`: check/test/fmt clean (host + freestanding), all 6 targets build,
QEMU full suite PASS, 95 s stress clean. Foreign files untouched
(`plan-next-phase`, untracked research docs).

## FAULT ADR

ADR-032 (`vivanta-boot/docs/adr/ADR-032-user-vm-fault-policy.md`) written
**before implementation**: fault classification table (§2), retry
semantics proof (§3), mapping state machine (§1), ownership (§4), OOM
transaction ordering (§5), capacity notes (§6). `VERIFIED`.

## MAPPING STATE MACHINE

`Mapping` extended in place (INV-VM-002): `Backing::{Present,
LazyAnonymous, Reserved}` + `pa: u64` + `PhysOwnership::{External,
Anonymous}`. No parallel registry exists — demand-fill splits the shadow
piece so piece granularity always equals hardware granularity.
Transitions verified per ADR table. `VERIFIED`.

## MAPPINGSET STORAGE

**Option A chosen**: fixed 64 slots retained for M6.0, documented as
demo-scale limitation. Rationale: heap-backed storage breaks
`AddressSpace: Copy` (boot-path ripple) and no current consumer needs
>64 mappings. Compile-time size guards added (`AddressSpace ≤ 12 KiB`,
`MappingSet ≤ 5 KiB`) so future growth cannot silently repeat the
mission-2 stack overflow. Heap-backed migration = `BACKLOG`.
`MAX_ADDRESS_SPACES=8` retained with analysis: fault path identifies the
active AS by TTBR0 match against registered roots — no current-AS global,
no ID reuse (monotonic ids), exhaustion panics deterministically at
registration. `VERIFIED` as explicit limitation.

## BACKING OWNERSHIP

`mapping ownership ≠ physical-frame ownership`, now explicit per piece:
`External` (caller/MemoryObject owns PA; VMM never frees — alias policy of
ADR-031 intact) vs `Anonymous` (VM layer allocated on demand; frame is
reachable only through its mapping; unmap returns it to PMM). Anonymous
frames cannot be aliased (PA never published). `VERIFIED` (QEMU exact PMM
deltas).

## LAZY PAGING

`reserve_lazy()` creates VA reservation without hardware image;
first access demand-fills **exactly one page** (page-granular proven:
non-base fill leaves base Lazy — dedicated QEMU sub-test); zero-initialized;
retry succeeds. `VERIFIED`.

## FAULT HANDLER

`el1h_sync` switched to a resumable vector (`save_and_eret_sync_el1`) +
`el1_sync_handler`: classifies EC/DFSC/WnR, reads TTBR0, resolves the
single approved class via arch-api hook → kernel resolver → VMM
primitives. Everything else falls through to the pre-existing fatal dump.
EL0 containment path untouched. `VERIFIED`.

## RETRY SEMANTICS

Return-from-exception restores ELR **unmodified**; resolution makes the
faulting instruction legal instead of skipping it; no TLBI needed for the
filled page (the first walk missed → nothing cached) but descriptor writes
are cache-cleaned and ordered (see BUGS #1). Mutation M2 proved that
`elr += 4` would be caught (skipped load → garbage value assert).
`VERIFIED`.

## MPROTECT

Existing `protect()` reused; hardware programming restricted to Present
runs (`for_present_runs`), Lazy pieces change metadata only; fills apply
post-mprotect permissions (hard rule #10 asserted against live leaf bits).
RW→RO→RW transitions re-verified. `VERIFIED`.

## MUNMAP

Range semantics over Lazy (no allocation, metadata removal), Present
Anonymous (frame released to PMM), External (untouched). Partial munmap
supported by the same transactional splitting; anonymous whole-piece
release rule documented. Table reclamation runs after every unmap.
`VERIFIED`.

## OOM

Deterministic coverage: a failing allocator stub drives
`resolve_lazy_fault` → returns false, mapping stays Lazy, verifier passes,
no frame lost. Real-allocator behavior: `try_alloc_page_table_frame`
returns None → `[VM] OOM during demand fill` → fatal (per ADR §5).
Behavioral test under true 512 MB exhaustion: `NOT TESTED` (impractical);
resolver contract: `VERIFIED` (mutation M9).

## HARDWARE VERIFIER

`verify_hardware_consistency()`: per-piece mechanical check — Present ⇔
valid leaf whose AP/PXN/XN bits exactly match `mmu_permission_bits(flags)`
; Lazy/Reserved ⇔ no leaf. Runs after every mutation-sensitive step in the
QEMU tests. Known blind spot: cannot see leaves *outside* tracked pieces
in unmapped regions (reverse scan deferred — `BACKLOG`). `VERIFIED` for
forward direction.

## MUTATION TESTING

| mutation | expected | observed |
|---|---|---|
| M1 resolve write-to-RO lazy | direct-call assert | `PANIC: write to a lazy RO piece must never resolve` ✅ |
| M2 advance ELR instead of retry | value assert | `PANIC: demand-filled page must be zero-initialized` ✅ |
| M3 claim Present before hw map | ghost-PTE / refault | second fault → state≠Lazy → fatal ✅ |
| M4 rollback frame loss | — | structural: no failure point between alloc and map (map panic = kernel-fatal) — `KNOWN LIMITATION` |
| M5 stale Lazy permissions after mprotect | leaf-bits assert | `PANIC: demand fill must apply post-mprotect permissions` ✅ |
| M6 materialize wrong page | non-base discriminator | `PANIC: piece base must stay Lazy` ✅ |
| M7 resolve outside MappingSet | direct-call assert | `PANIC: assert !resolve(unmapped)` ✅ |
| M8 EL0-accessible fill | privilege-policy assert | `PANIC: demand fill must not grant EL0 access` ✅ |
| M9 ignore OOM | unit contract | resolver-contract asserts ✅ (real-allocator exhaustion `NOT TESTED`) |
| M10 direct PTE write outside VMM | repo audit | clean — writers exist only in arch mechanism, invoked only via VMM ✅ |

All mutations reverted; final tree verified clean.

## QEMU EVIDENCE

Boot → W^X ×2 → protect/TLBI → 3 fault scenarios → **lifecycle test**
(map/partial-protect/split-shadow/alias/reclaim/remap/unregister) →
**lazy test**: reserve(16K) → read-fill (zeroed ✓) → page-granular check →
write-fill → mprotect-RO → RO-fill (post-mprotect perms asserted on live
leaf + kernel-only policy assert) → munmap (3 anon frames + reclaimed
tables back to PMM, exact deltas) → OOM rollback → negative classification
→ pg-granular non-base fill → **stress: 200 reserve/fill/unmap cycles with
verifier + leak asserts** → teardown. M5/M6/containment/preemption
unchanged. 95 s stress: ~41K lines, 0 panics, preempt counters >7×10⁹.

## REGRESSION

check/clippy/fmt clean both targets; 13/13 host tests; all 6 targets build;
no new warnings; existing gates unchanged.

## BUGS FOUND (all FIXED)

1. **Runtime descriptor writes lacked cache clean-to-PoC** —
   `walker::write_desc` was plain volatile; ARM walkers don't snoop D-cache
   (builder path always did `dc civac`; runtime paths didn't). Timing-
   dependent misbehavior under stress. Fixed at the single choke point.
2. **QEMU per-VA TLBI unreliability across recycled VAs** — stale entries
   survived `tlbi vaae1is` after descriptor rewrite+TLBI (observed:
   silent stale reads post-unmap). Workaround: full `tlbi vmalle1is`
   flush in `tlbi_range` (single-core/no-ASID makes this cheap);
   per-VA returns with ASIDs + hardware validation = `KNOWN LIMITATION`.
3. **Stale-slot shadow corruption** — protect/unmap/demand-fill commits
   used slot indices captured before hole-reusing inserts/removes;
   neighbouring pieces got wrong permissions/addresses (caught by stress
   iteration 1). Fixed with transactional value-keyed
   `MappingSet::replace_slots`.
4. **Lifecycle test mapped PA it did not own** (1 frame allocated, 3
   pages mapped) — wrote into a live page-table frame. Test fixed
   (`alloc_contiguous(3)`); ownership rule documented in-test.
5. **Lost `try_alloc_page_table_frame` override** (self-inflicted by a
   git checkout during debugging) — real fill path would panic-on-OOM
   instead of controlled failure. Restored; covered by OOM contract.

## KNOWN LIMITATIONS

- Per-VA TLBI semantics unproven under QEMU; full-flush strategy is
  correct-but-blunt. Hardware validation pending.
- Reverse-direction verifier (no leaf outside tracked pieces) deferred.
- EL0-originated demand fills not resolved (containment unchanged);
  requires syscall ABI work.
- M4/M9 behavioral depth as noted above.

## BACKLOG

- Heap-backed MappingSet (drop 64-slot limit) behind the new size guards.
- mmap/munmap/mprotect syscalls on top of ready primitives.
- COW/shared anonymous frames (extends PhysOwnership).
- Lazy executable mappings (instruction-abort IFSC handling).
- ELF loader / user processes (>8 ASes needs registry evolution).

## COMMITS

```text
branch: main
base:   2ceef95
docs(vm): specify fault resolution semantics (ADR-032)
refactor(vm): model mapping backing as explicit state
feat(vm): lazy reservation, demand-fill fault resolution, verifier
fix(vm): transactional value-keyed shadow commits + desc cache clean
test(mm): stress + mutation battery
docs(vm): mission report + status sync
(see git log for hashes)
```

## FINAL GIT STATUS

Working tree contains only this mission's committed work; foreign files
(`plan-next-phase` modification, untracked research docs) untouched.

---

## TEN REQUIRED ANSWERS

1. **MappingSet single source of truth?** Yes — INV-VM-001, mechanically
   verified per piece; backing metadata lives inside `Mapping`. `VERIFIED`
2. **Who may write page tables?** Only arch mechanism functions, invoked
   exclusively from VMM primitives; repo audit confirms no other writer.
   `VERIFIED`
3. **Fault classification?** ADR-032 §2: one resolvable tuple
   (EC=data-abort-same-EL ∧ DFSC∈{translation L1/L2/L3} ∧ access ⊆ perms ∧
   active-AS piece is LazyAnonymous); everything else fatal. `VERIFIED`
4. **Why is retry safe?** ELR restored unmodified; the instruction is made
   executable rather than skipped; effect-of-retry asserted in tests;
   M2 proves elr+=4 would fail loudly. `VERIFIED`
5. **Frame owner after lazy allocation?** The mapping (PhysOwnership::
   Anonymous); released on unmap; never aliased. `VERIFIED`
6. **OOM?** validate→allocate(fail⇒log+fatal/false)→zero→map→commit-last;
   mapping stays Lazy; no partial state; deterministic unit coverage.
   `VERIFIED` (contract) / `NOT TESTED` (true exhaustion)
7. **mprotect on lazy?** Metadata-only until materialization; fills use
   current permissions (leaf-bits asserted). `VERIFIED`
8. **Partial munmap?** Supported via transactional splitting; leftovers
   keep identity; fully-covered Anonymous frames released. `VERIFIED`
9. **MappingSet↔hardware divergence prevention?** Mechanical verifier +
   commit-last ordering + value-keyed transactions + stress with per-
   iteration verification. `VERIFIED` (forward direction)
10. **Exhaustion?** MappingSet-full → deterministic error before mutation
    (`MappingTableFull`); AS-registry-full → panic at registration;
    both documented limitations. `VERIFIED`
