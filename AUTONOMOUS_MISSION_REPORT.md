# AUTONOMOUS MISSION REPORT — Vivanta Long-Run Engineering Session

**Date:** 2026-08-21
**Repository:** https://github.com/Egorich-print/vivanta (`main`)
**Workspace root:** `/Users/egorich/ai-workstation/Projects/Vivanta`

---

## 1. Executive Summary

The session found and fixed a **proven security/correctness defect in the
kernel's page-permission model**: all three AArch64 descriptor encoders mapped
every user page as EL0-*writable* (AP=01) regardless of the requested
writability, so user code pages declared "read-only + executable" were in fact
**RWX from EL0** — directly violating the ratified M5.0 G3 acceptance criteria
("user code = RX; no RWX") that had been declared PASS. The defect was fixed at
its root (single-source-of-truth AP encoder), the missing `mmu_protect`
mechanism was implemented end-to-end (arch-api contract → AArch64 policy →
paging mechanism → VMM wiring), the `todo!()` landmine in
`AddressSpace::protect()` was removed, and three independent verification
layers were added (boot-time descriptor readback, encoder regression matrix,
EL0 behavioral negative test). A pre-existing but uncommitted INV-002 fix was
re-verified and committed first to establish a clean baseline.

## 2. Selected Task

**W^X enforcement for AArch64 user pages + mmu_protect/VMM protect mechanism.**

Why this task over the alternatives (SYS_READ stub, VA allocator, signals):

1. **Proven doc/code divergence.** STATUS.md claimed "User memory boundary ✅ /
   W^X" while the encoding could not express user read-only at all. This is
   exactly the class of latent defect the project's own gate process exists to
   prevent — and it sat at the heart of the memory-permission model.
2. **Kernel landmine.** `AddressSpace::protect()` was `todo!()`: any caller
   would panic the kernel.
3. **Architectural leverage.** Permission rewriting is a prerequisite for COW,
   mprotect-style syscalls and hardened W^X; implementing it per ADR-030's
   mechanism/policy split strengthens the whole VMM stack.
4. **Fully validatable here.** QEMU boot evidence is achievable; no physical
   hardware required.

## 3. Initial State

- Baseline commit `f2af711` (docs: RPi3B+ UART bring-up).
- M5.0 PASS/CLOSED, M6 PASS/CLOSED per docs; INV-002 marked "Closed" in its
  investigation doc — **but the actual fix was sitting uncommitted in the
  working tree** (12 modified files). Last commits were docs-only.
- Working tree also contained pre-existing untracked research docs
  (ADR-031…039, distributed/, current-architecture.md) and one uncommitted
  doc modification (`plan-next-phase-2026-08-01.md`, Pixel target decision).
- Validation baseline: workspace build ✅, host tests 6/6 ✅, clippy warnings
  pre-existing (~197, mostly `static_mut_refs` style), QEMU boot + M6 demo +
  G4 preemption ✅.

## 4. Implementation

| Layer | Change |
|-------|--------|
| `arch-aarch64/src/paging/descriptor.rs` | `DESC_AP_MASK`; `ap_bits(user, writable)` const fn — single source of truth (EL1 RW=00, EL1 RO=10, EL0 RW=01, EL0 RO=11) |
| `arch-aarch64/src/paging/mod.rs` | `MappingFlags::to_descriptor_bits` routes through `ap_bits` (was lossy if/else) |
| `arch-aarch64/src/mmu.rs` | `block_or_page_desc` + `flags_to_desc_bits` route through `ap_bits`; new `mmu_protect` runtime entry point (walk → split L2 blocks when needed → rewrite leaf permission bits → TLBI range) |
| `arch-aarch64/src/paging/walker.rs` | `leaf_with_permissions()` pure mechanism helper: rewrites AP/XN/PXN preserving PA/type/AF/SH/ATTRIDX |
| `arch-api/src/mmu.rs` | `mmu_protect` contract with safety docs |
| `arch-api/src/boot/mmu.rs` | `wx_verify_user_as` contract |
| `arch-test-stub/src/lib.rs` | no-op `mmu_protect`, `wx_verify_user_as` |
| `kernel/src/vmm/address_space.rs` | `AddressSpace::protect()` implemented: exact-match mapping validation → hardware rewrite → software shadow commit; returns `NotMapped` otherwise |
| `kernel/src/vmm/mapping.rs` | `MappingSet::get_mut` |
| `kernel/src/lib.rs` | `[WX]` verification of both user address spaces during boot |
| `arch-aarch64/src/paging/self_test.rs` | `test_wx_encoding` regression matrix (all three encoders + `ap_bits` truth table + rewrite preservation); `wx_verify_user_as` implementation |
| `arch-aarch64/src/user.rs` | G3 fault task now stores to its **own code page** first (W^X negative test); store-to-VA-0 kept as fallback marker |

Docs: new investigation `vivanta-boot/docs/investigations/WX-user-code-ap-encoding.md`;
ADR-019 amended (user-RO encoding row, `USER_READ_EXEC` as user-code flag,
W^X amendment section); STATUS.md and master-roadmap changelog synced.

## 5. Architecture Fit

- Follows ADR-030's mechanism/policy split exactly: bit-level descriptor
  transformation lives in `paging/` (pure, allocation-free), allocation-aware
  orchestration in `mmu.rs`, kernel-side bookkeeping in `vmm/`.
- No architectural decision was overturned. ADR-019 was *amended* (it was
  status "Proposed" and its AP table predated the M5.0 W^X policy that
  superseded it — the code had followed the stale table).
- Transactional ordering mirrors `map_pages`: validate → program MMU → commit
  shadow; OOM during block split panics (same boot/runtime-fatal policy as
  existing page-table allocation — verified `MrmPageTableAllocator` panics).

## 6. Invariants

- **W^X:** no user mapping is simultaneously writable and executable; user
  code is EL0 RO+X with PXN=1 (EL1 cannot fetch it either).
- **Shadow consistency:** `Mapping.permissions` always reflects programmed
  hardware after `protect()` returns; validation failure mutates nothing.
- **Rewrite purity:** permission rewrites never move a physical address nor
  alter type/AF/shareability/ATTRIDX; applied to an invalid descriptor the
  result stays invalid (no accidental mapping creation).
- **Encoding regression immunity:** any future encoder bypassing `ap_bits` is
  caught by the boot smoke test before the kernel proceeds.

## 7. Tests

| Check | Result |
|-------|--------|
| `cargo build --workspace` | ✅ 0 errors |
| All 6 bootable targets build (qemu-aarch64, rk3568, rpi3b-plus, lavender, suma-q5, x96q) | ✅ |
| `cargo test --workspace --target aarch64-apple-darwin` | ✅ 6/6 pass |
| `cargo clippy --workspace` | ✅ no new warnings from changed code |
| `cargo fmt -- --check` | ✅ clean |
| `git diff --check` | ✅ clean |
| QEMU boot: `[WX]` readback both user ASes | ✅ code AP=0b11/XN=0/PXN=1; stack AP=0b01/XN=1 |
| QEMU boot: EL0 store to own code page | ✅ aborts, ESR DFSC=0b001111 (permission fault L3), FAR=0x5f000000 |
| QEMU boot: demo/M6 lifecycle/fault containment/G4 preemption | ✅ unchanged, exit codes correct |
| 95 s soak | ✅ 37 K log lines, zero panics, both preempt workers ~7×10⁹ iterations |
| **Mutation test** (reintroduced bug in one encoder) | ✅ caught — `test_wx_encoding` panicked at the exact assertion; live-mapping readback correctly unaffected (different path) |

## 8. QEMU / Hardware

**Verified in QEMU (cortex-a53):** full boot path, W^X descriptor readback,
behavioral permission-fault evidence, M6 lifecycle, fault containment,
preemption stability, soak.

**Not verifiable without physical hardware:** absolute descriptor semantics on
real silicon. Note: AP-bit semantics are architecturally defined (VMSAv8-64)
and orthogonal to the deferred L1/L2 table-descriptor encoding question
(`table_desc` 0b11 vs spec 0b10), which remains tracked in
`MMU-descriptor-encoding-hardware-validation.md`.

## 9. Regression Analysis

- **All descriptor writers audited:** every AP-bit write routes through the
  three fixed encoders; no other writer exists (`grep '<< 6'` across arch
  crates).
- **Kernel mappings bit-identical:** only the user+RO case changed encoding;
  kernel RW/RO and user RW are unchanged. `MemoryObject::map` uses kernel
  flags — unaffected (smoke test passes).
- **access_ok consistency:** `descriptor_allows(Write)` already keyed off AP
  bit 7 — now correctly denies writes to code pages (previously allowed).
- **Indirect consumers checked:** usercopy, faults.rs, scheduler, task_manager,
  early_mmu (identity map = kernel RWX, unchanged), split_l2_block attr
  inheritance, all platform/target crates link.

## 10. Remaining Risks

1. `mmu_protect` splits even fully-covered 2 MiB blocks (correct, wasteful) —
   optimization follow-up.
2. Partial-range protect requires software mapping splitting → blocked on the
   post-M5 VA allocator; API deliberately rejects non-exact ranges.
3. Pre-existing clippy debt (~197 warnings, mostly `static_mut_refs`) and
   `static mut` addressing-space registry remain untouched (out of scope).
4. Real-hardware MMU validation still pending (pre-existing deferral).

## 11. Follow-up Work

1. VA allocator (post-M5 backlog) → unlocks partial-range protect + mmap.
2. SYS_READ implementation (UART RX + blocking semantics).
3. mprotect-style syscall once the syscall-number scope fence lifts.
4. 60-minute soak run as a standing reliability gate.

## 12. Files Changed

Code: `arch-aarch64/src/{mmu.rs, boot.rs, user.rs, paging/descriptor.rs,
paging/mod.rs, paging/walker.rs, paging/self_test.rs}`,
`arch-api/src/{mmu.rs, boot/mmu.rs}`, `arch-test-stub/src/lib.rs`,
`kernel/src/lib.rs`, `kernel/src/vmm/{address_space.rs, mapping.rs}`.
Docs: `docs/investigations/WX-user-code-ap-encoding.md` (new),
`docs/adr/ADR-019-user-page-permissions.md`, `STATUS.md`,
`docs/architecture/master-roadmap.md`.

## 13. Git

```text
branch:       main
base commit:  f2af711  (docs: RPi3B+ UART bring-up)
final commit: <see git log> 
history:
  4a46ceb fix: close INV-002 — console lock held with IRQs disabled,
          ThreadContext at stack bottom
          (pre-existing verified-but-uncommitted work; re-verified:
          QEMU smoke 25 s, counters >2B, zero panics — then committed)
  67d5b87 fix: enforce W^X for AArch64 user pages; implement mmu_protect
          + VMM protect()
```

Left untouched (pre-existing, not mine): uncommitted edit to
`vivanta-boot/docs/plan-next-phase-2026-08-01.md` (Pixel target decision) and
untracked research docs (ADR-031…039, `docs/architecture/current-architecture.md`,
`docs/distributed/`, evolution-plan/execution-context).

---

## Definition of Done

- [x] Repository researched deeply enough (boot→PMM→MMU→VMM→IRQ→sched→syscalls)
- [x] Self-selected technically significant task, rationale recorded
- [x] Working solution implemented (not a skeleton)
- [x] Integrated into existing architecture (ADR-030 split respected, ADR-019 amended)
- [x] Tests added/extended (encoder matrix, boot readback, EL0 negative test)
- [x] QEMU-side validation executed (incl. mutation test + soak)
- [x] Adversarial self-review performed (12 scenarios analyzed; 1 mutation test)
- [x] Regression search beyond edited files
- [x] Documentation synchronized
- [x] Working tree clean of session artifacts
- [x] Final report created
- [x] Git history coherent (two purposeful commits)
