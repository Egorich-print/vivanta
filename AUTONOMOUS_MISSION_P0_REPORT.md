# AUTONOMOUS MISSION P0 REPORT — corrective mission (§3.1/§4 of the 2026-09-08 audit)

**Date:** 2026-09-09
**Repository:** https://github.com/Egorich-print/vivanta (`main`)
**Base:** `9757f55` (M10.2) + uncommitted pre-mission refactor
**Result:** 4 commits (`f6ed78f`, `cb17b7a`, `5005542`, `795f119`), backup branch
`backup/pre-p0-mission-2026-09-09`

Tags: `FIXED` / `VERIFIED` / `NEW-FINDING` / `DEFERRED` / `NEEDS-SILICON`.

---

## BASELINE

`9757f55`: 13/13 QEMU gates PASS. Working tree carried an uncommitted
`sys_read` (reverted: violated ADR-033, wrong UART register, busy-wait),
a dead `exec/loader.rs` orphan (deleted), a doubled `include_bytes!` ELF
(deduped to the execve registry), and a `user-init.elf` rebuilt without its
linker script (entry `0x210180`, loader rejects it — fixed via new
`user-init/build.rs` emitting `-T user-init.ld`; entry back at `0x1000000`).

## P0 FIXES (all VERIFIED by new gates, 19/19 PASS, 0 panics)

1. **execve stack** (`kernel/src/syscall/process.rs`): stack reservation
   `0x5C01_0000` → `0x3FFF_F000` (top of the user VA domain; old value made
   every execve fail). `FIXED`.
2. **execve return path** (`process.rs`): `frame.sp` is the SVC epilogue's
   SP_EL1 restore slot — overwriting it with the user stack hung the return
   silently. User SP is now installed via `msr sp_el0` (legal at EL1, stable:
   context switch never touches SP_EL0). `NEW-FINDING`, `FIXED`.
3. **execve icache** (`process.rs` + new `mmu_flush_icache_range` arch-api):
   entry page flushed to PoU (spawn path gets `ic iallu` from the eret stub;
   the SVC path does not). `NEW-FINDING`, `FIXED`.
4. **unmap_all VA leak** (`vmm/address_space.rs`): `va.free` per removed
   piece (execve-into-live-AS leaked the domain). `FIXED`.
5. **unmap_all vs Lazy** (`vmm/address_space.rs`): skip HW unmap for pieces
   without a hardware image (walker panicked `MissingL2/MissingL3` on a
   never-touched Lazy stack at the first real teardown). `NEW-FINDING`, `FIXED`.
6. **Process table** (`scheduler/process_table.rs`): `live_count`/`count`
   exclude `Exited` tombstones; `create` reuses tombstone slots preserving
   bumped generations (was a 64-task lifetime cap AND unbounded Vec growth
   to kernel-heap OOM, found by the new gate itself). `FIXED`.
7. **Signals** (`signal.rs`, `process.rs`): async EL0 delivery documented as
   absent (scope fence); dead `take()` removed; self-SIGKILL terminates via
   `thread_exit` (was Zombie-with-Running-thread). Gate asserts
   SIGKILL→Zombie + parent SIGCHLD. `FIXED` (documented-absence branch).
8. **Fault capacity** (`vmm/address_space.rs`): MappingSet pre-check before
   HW mutation in both resolvers (was `.expect()` kernel panic on user
   faults at 63–64 slots). `FIXED`.
9. **Gate hygiene** (`kernel/src/lib.rs`): table-only gate tasks detach
   phantom tid-0 linkage (a kill gate terminated the live boot thread and
   broke the M6 gate downstream). `NEW-FINDING`, `FIXED`.

## NEW GATES (all PASS)

M10.3 genuine EL0 `execve("/init")`→exit(42); double execve/load+stack in one
AS; 70 create/reap cycles; kill/SIGCHLD; full-table resolve refusal; e2e
load/dup/teardown with zero PMM/table/COW deltas.

## INV-003: boot-stack overflow (NEW-FINDING, FIXED)

82 KiB `-O0` kernel_main frame + gate temporaries overflowed the 128 KiB
boot stack into `.bss`, zeroing the console pointer → silent console-lock
deadlock pre-dump, content-sensitive (a few KiB decide boot vs hang). Full
forensic trail: `vivanta-boot/docs/investigations/INV-003-boot-stack-overflow.md`.
Fix: stack 128→256 KiB + all six gates as `#[inline(never)]` fns (frame
0x14000→0xa000, measured in disassembly).

## P1 (done)

- `spec-table-desc` feature (arch-aarch64): 0b11→0b10 table emission behind
  the flag, default untouched. Feature build hangs at MMU enable exactly as
  the HW-validation doc predicts — recorded, `NEEDS-SILICON` (RK3568/RPi3B+).
- x96q EL2→EL1h drop (HCR_EL2.RW, CNTVOFF_EL2, SPSR/ELR eret; x0 preserved).
  Compile-checked only, `NEEDS-SILICON`. RK3568 intentionally untouched (its
  EL2-resident diagnostic flow is a separate bringup stage).
- Docs: STATUS.md refreshed (was stale since mission 2), README triplication
  + M7→M5.0 fix, INV-001 header Closed, ADR-035 embedded second draft removed.

## DEFERRED (with rationale)

`UserPtr` adoption in hot syscalls, `register()` panic→Result, kernel W^X,
MAIR/TCR unification — all touch proven paths for no P0 gain.

## VALIDATION

`cargo fmt --check` clean; `cargo check` 0 warnings (default + feature);
`cargo build --workspace` clean; QEMU (cortex-a53, 512 MiB): 19/19 PASS
(WX×2, Protect/TLBI, exec-nx, kread, unmapped, SYS, ELF, EXECVE, EXEC2,
PTABLE, KILL, OOM, E2E, COW, FORK, VM, STRESS-200, LAZY), 0 panics.
