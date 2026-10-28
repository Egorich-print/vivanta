# Master Roadmap

**Purpose:** This is the authoritative engineering *plan* for Vivanta — it maps
existing architectural components (BootInfo, PMM, VMM, MemoryObject) onto a
phased execution sequence. It does not introduce new specifications.

**Authority:** This document takes precedence over implementation plans, TODO
lists, sprint documents and experimental branches. It does **not** take
precedence over the code. Where this document and the repository disagree,
the code wins and this document is wrong. As of 2026-09-29 several sections
below are known to be stale; they are retained and marked rather than
rewritten, so that the reasoning behind past decisions stays readable.

**Authority for facts (not plans):**

| Question | Authoritative source |
|---|---|
| What actually runs | `vivanta-boot/tools/qemu-gates.sh` + `vivanta-boot/kernel/src/lib.rs` |
| Boot adapter ↔ kernel contract | `vivanta-boot/boot-info/src/lib.rs` + ADR-021 |
| Syscall ABI | `vivanta-boot/kernel/src/syscall.rs` + ADR-033 |
| Architecture decisions | `vivanta-boot/docs/adr/` — **18 files: ADR-011…015, 017…023, 030…035.** No ADR-016, ADR-024 or ADR-025 exists today (ADR-024/025 lived in the old `docs/adr/` set and were removed 2026-08-25 in `d6735c7`) |
| Open correctness backlog | [docs/audit/2026-09-24-global-audit.md](../audit/2026-09-24-global-audit.md) |
| What cleanup removed | [docs/audit/2026-09-25-cleanup.md](../audit/2026-09-25-cleanup.md) |
| RK3568 hardware truth | [docs/hardware/rk3568/README.md](../hardware/rk3568/README.md) |

> **Reading key.** Sections marked *(historical)* describe a superseded plan
> and are kept for context only. Sections marked *(aspirational)* are research
> directions, not commitments.

---

## Current Focus

*Last rewritten 2026-09-29 against the 2026-09-24 global audit and the
2026-09-25 cleanup. The previous version of this section had not been updated
since 2026-09-15 and predated the audit entirely.*

*   **Active track: correctness, not features.** The 2026-09-24 five-track
    audit produced the live backlog. One P0 defect dominates, plus one P0
    finding that has since been **disproven**:
    1. **Scheduler race.** A tick can land inside a context switch, losing a
       user thread's exit code (`VM-syscall program must exit with code 42
       (got None)`) or leaving a forked child unscheduled. The window between
       `CURRENT_THREAD.store(next)` and entry into the new thread is narrowed
       but not closed. Adding `println!` tracing makes the bug vanish
       (heisenbug) — it needs a deterministic failpoint (QEMU `-icount`), not
       a trace. This is real and unresolved, and it is **not** caused by
       unmasked interrupts.
    2. **~~IRQ masking is architecturally wrong.~~ DISPROVEN (2026-09-29).**
       `arch-aarch64/src/interrupts.rs` writes `DAIFSet/DAIFClr #2`. The `imm4`
       field order is bit3=D, bit2=A (SError), bit1=I (IRQ), bit0=F, so `#2` is
       the **IRQ** mask (I) and the code is correct; `#4` would set the SError
       mask (A). `PSTATE.I` (bit 7) is set/cleared via `DAIFSet/Clr #2`. The
       audit originally read the field in the wrong order; the project's `#4`
       experiment then failed to clear the IRQ mask, which is why the timer
       stopped — a defect in the experiment, not in GIC delivery. There is **no
       evidence of a GIC IRQ delivery defect**. See
       [docs/audit/2026-09-24-global-audit.md](../audit/2026-09-24-global-audit.md).
*   **Consequence for the gate:** the CI QEMU job is
    **`continue-on-error: true`** (advisory), with the reason written into
    `.github/workflows/ci.yml`. `main` has no branch protection, so a green run
    never blocked a merge anyway. The gate requires **18 serial markers**
    (`tools/qemu-gates.sh`); the retired framings "22/22", "19/19" and
    "18/18 PASS in CI" are all wrong.
*   **P0 silicon blockers** (from the audit): boot stack is still 16 KiB on
    `target-x96q` / `target-suma-q5` / `target-rk3568` / `target-lavender`
    while qemu and rpi3b-plus use 256 KiB — and x96q/suma-q5 *do* reach
    `kernel_main` with the same ~82 KiB frame that forced the bump. The
    `target-rpi3b-plus` and `target-x96q` EL2→EL1h paths set
    `CPTR_EL2 = 0b11 << 20` and never set `CPACR_EL1`, contradicting
    [docs/architecture/arm-exception-state.md](arm-exception-state.md), which
    prescribes `CPACR_EL1 = 0b11 << 20` at EL1 and `CPTR_EL2 = xzr`
    (TFP=0) at EL2. Neither x96q nor suma-q5 is bootable on silicon today.
*   **Current Engineering Objective:** close the scheduler race. No new
    subsystem work until that is done. (The audit's old order — "(1) GIC IRQ
    delivery, (2) `#4` instead of `#2`, (3) the switch window" — was retired
    with the disproven IA finding: IRQ masking is correct, so only the switch
    window remains.)
*   **Completed:** `M5.0 GREEN BASELINE` — CLOSED (see
    [vivanta-boot/docs/milestones/M5.0-green-baseline.md](../../vivanta-boot/docs/milestones/M5.0-green-baseline.md));
    `M6 Process Lifecycle` — CLOSED (see
    [vivanta-boot/docs/milestones/M6-process-lifecycle.md](../../vivanta-boot/docs/milestones/M6-process-lifecycle.md));
    `G-M7` syscall ABI / VM syscalls / process model — CLOSED; `G-M8` ELF64
    loader, `G-M9` COW, `G-M10` process lifecycle and `G-M11` signals all
    shipped (commits `f8039b0`, `661e22e`, `8d5f757`…`94fdfeb`, `b58adbe`).
*   **Architecture Version:** V1 architecture stabilized — ADR-021 ratified
    (BootInfo escape prevention), ADR-030 paging architecture accepted.
*   **Architecture Status:** 18 engineering ADRs canonical in
    `vivanta-boot/docs/adr/`. **ADR-035 is still `Proposed`.** A separate,
    unrelated set (ADR-031…039) lives in `docs/archive/research-track/`; the
    numbers collide — always cite the directory with the number.

> M5.0/M6 supersede the pre-M5 "V2/M5 Memory Resource Manager" roadmap. The
> next milestone is defined from actual repository state, not the old roadmap.
> Scope fence holds: IPC / storage / drivers / distributed AI / Ed25519 /
> BIP-39 / persistent identity / TTBR1 / ASID / signals / new syscalls.

---

## Engineering Principles

1.  **Hardware Before Abstraction:** New architectural concepts must be designed to solve validated hardware realities. We test on physical targets first, then generalize.
2.  **Prototype Before RFC:** A new RFC requires a working experiment or a reference implementation to prove viability before it can be accepted.
3.  **Mechanism Before Policy:** The kernel implements strict, low-level execution mechanisms (e.g., page tables, hardware capability checks). Resource orchestration and placement policies live at a higher layer.
4.  **One Active Milestone:** Only one milestone is active at any point. Parallel research on future tracks is encouraged, but must not distract from the active release goal.
5.  **No Feature Debt:** Do not write new features if the active milestone's exit criteria are unmet or if there are outstanding warnings/clippy issues.

---

## Out of Scope (Temporarily Postponed)

The following topics are intentionally out of scope for the near and mid-term:
- Symmetric Multiprocessing (SMP)
- Non-Uniform Memory Access (NUMA) balancing
- Physical CXL/HBM/GPU device programming
- Hardened security modules / TrustZone
- Filesystems & Network stacks
- General-purpose userspace POSIX translation layers

---

## Dependency Graph

### Historical track — V-epics *(superseded as a plan; kept for continuity)*

The V-epic chain below was the primary planning structure from 2026-07-19
until 2026-08-11. **It is no longer the active plan** and its `← ACTIVE`
marker is void: V2/M5 and V2/M6 were closed as M5.0 and M6.0, and V3…V7
have not started. Statuses are given in the V-Epic Roadmap section further
down.

```
P0  Rename + cleanup (TheseusOS → Vivanta) ✅
    ↓
P1  SystemState + Volatile Identity ✅ (V1.1)
    ↓
[M4.5.2 — RK3568 hardware stabilization] ⚠️ NOT COMPLETE (see below)
    ↓
P2  V2/M5 Memory Resource Manager (integrate existing MemoryObject) — closed as M5.0
    ↓
P2  V2/M6 Resource-backed Runtime (first isolated EL0 process) — closed as M6.0
    ↓
P3  V3 Device Graph + Minimal Driver Contract (ADR-022) — aspirational, not started
    ↓
P3.5  Storage driver (SPI NAND / eMMC) — aspirational, not started
    ↓
P4  V4 Kernel Task model (spawn/exit/yield) — aspirational, not started
    ↓
P5  V5 Persistent Identity + Storage — aspirational, not started
    ↓
P5  V5 Service Framework (LoggingService first) — aspirational, not started
    ↓
P6  V6 Recovery Manager — aspirational, not started
    ↓
P7  V7 Additional hardware targets (RPi3, SDM660) — aspirational, not started
```

### Historical — Phases R2–R7 (superseded by V-epics)

```

Phase R2 Reality Lock ──► ✅ Completed via ACS, M3-AB, M4, M4.4, M4.5

Phase R3 Kernel Foundation ──► K1 (Interrupts) ✅, K2 (Scheduler) ✅
                                  K3–K5 deferred to V-epic model

Phase R4 Resource System ──► Folded into V2/M5 (MemoryObject integration)

Phase R5 Runtime ──► Deferred post-V5

Phase R6 Native Runtime ──► Deferred post-V6

Phase R7 Desktop/Mobile ──► Deferred post-V7
```

---

## Phase R2 — Reality Lock (✅ COMPLETED)

**Goal:** Transition from research prototype to engineering platform.

R2 was completed via the following milestones, redirected from the original lavender/SDM660 target to QEMU + RK3568:

- **ACS** (Architecture Cleanup Sprint) — kernel/arch/platform/target split, `extern "Rust"` boundary
- **M3-AB** — AArch64 boot on QEMU with the shared BootInfo contract. *(This row
  originally read "AArch64 + ARMv7 boot on QEMU". That was never true:
  `arch-armv7a` is an 8-line stub kept per ADR-011, and `target-qemu-armv7a`
  was deleted 2026-09-25 because it was not a workspace member and
  `armv7a-none-eabi` is not in the pinned toolchain. The README's
  "AArch64 and ARMv7 systems" claim has the same defect.)*
- **M3-BC** — MemoryObject lifecycle validated on QEMU
- **M4** — Cooperative multi-threading, scheduler, IRQ, timer, thread lifecycle
- **M4.4** — Address spaces, multi-AS model with hardware isolation
- **M4.4.5** — Unified context switch (ADR-017)
- **M4.5.0** — EL0 transition preparation
- **M4.5.1** — First EL0 entry and SVC roundtrip on QEMU
- **RK3568 Stage 1** — UART output, EL2→EL1 transition, Rust entry on hardware

The original lavender/SDM660 specific targets (M1-B0 through M1-B4) were superseded by RK3568 as primary hardware validation target. *Corrected: RK3568 turned out to be an EL2 diagnostic only — `target-rk3568` does not link `vivanta-kernel` — so no kernel has run on any physical board. See M4.5.2 below.*

---

## Phase R3 — Kernel Foundation (PARTIALLY COMPLETED)

**Goal:** Build execution foundation over early boot code.

### K1 — Interrupt Subsystem ✅ COMPLETED (M4)
- Exception vector tables, GICv2/v3, ARM generic timer, timer IRQ.
- **Correction (2026-09-29):** the audit's 2026-09-24 claim that
  `DAIFSet/Clr #2` masks SError, not IRQ, is **disproven** — `#2` is the IRQ
  mask (I) and the code is correct (`#4` would set the SError mask A). See
  *Current Focus*.

### K2 — Scheduler ✅ COMPLETED (M4)
- Single-core round-robin, cooperative switching, 100 Hz timer, `NEED_RESCHEDULE`
  on every tick (no quantum accounting). `RunQueue` is a linear
  `Vec<Option<Thread>>` scan over a 5-level priority ladder; `MAX_TASKS=64`.
- True preemption is still deferred to physical hardware validation, and
  hardware validation is blocked by the two P0 defects above. The preemption
  claim is currently **not** reliable: see the scheduler race in *Current Focus*.

### K3–K5 — DEFERRED to V-epic model
- Capability IPC, VMM integration, Capability Manager are folded into V2/M5 (Memory Resource Manager) and V4 (Task abstraction).
- VMM foundations (address spaces) completed in M4.4.

---

## Phase R4 — Resource System

**Goal:** Implement the `ResourceObject` hierarchy.

-   `ResourceObject` becomes the common abstraction for memory, devices, storage, and future resources.
-   MemoryObject becomes a specific subclass of `ResourceObject`.
-   Introduce `DeviceObject` (MMIO/DMA ranges) and `StorageObject` (blocks/flash).
-   **Exit Criteria:** Devices are accessed strictly through capability-verified resource handles.

---

## Phase R5 — Runtime

**Goal:** Jump to userspace execution.

-   Implement `init` process, user space runtime library, and early continuity daemon.
-   **Exit Criteria:** A user process successfully executes an IPC transaction with a kernel-managed ResourceObject.

---

## Phase R6 — Native Theseus Runtime

**Goal:** Implement placement and migration.

-   Active `MemoryPlacementPolicy` engines.
-   Automatic background migration of VMOs between tiers (e.g., DDR and simulated persistent memory/CXL).
-   **Exit Criteria:** Live migration of active pages without process interruption.

---

## Phase R7 — Desktop/Mobile

**Goal:** High-level OS subsystems.

-   Add file system, networking, and early graphical framebuffer drivers.

---

## Engineering Tracks

### Track A: Architecture
-   Maintain RFCs under [specs/rfc/](../../specs/rfc/) (pre-R2 "Theseus" RFCs),
    [vivanta-boot/docs/rfc/](../../vivanta-boot/docs/rfc/) and
    [vivanta-boot/docs/rfcs/](../../vivanta-boot/docs/rfcs/), and ADRs under
    [vivanta-boot/docs/adr/](../../vivanta-boot/docs/adr/) — **not** `docs/adr/`,
    which does not exist.
-   Strict rule: speculative RFCs are frozen. RFC status must remain `Experimental` until proven on target.
-   `specs/rfc/README.md` claims RFC `0003` and `008` "cover the same ground" and
    are both "superseded by the code". Both claims are false: they are orthogonal
    (identity boot state machine vs `BootInfo` ABI), and the shipped `BootInfo`
    (5 fields) matches neither RFC-008 (9 fields) nor that README.

### Track B: Hardware Support
-   Current targets, in verified order of reality:
    - `QEMU AArch64` — the only configuration that boots to `kernel_main` with
      EL0 userland. Validation baseline, advisory-gated.
    - `RK3568` — **diagnostic adapter only; it does not link `vivanta-kernel`.**
      Real UART output from a Vivanta EL2 binary on silicon (July 2026) is the
      project's only silicon data point, and it stops short of the kernel
      (`MMU enable 🔲`, `Scheduler 🔲` in the bring-up log). The board is
      currently in **Maskrom** after a mainline U-Boot flash, boot unconfirmed.
    - `Raspberry Pi 3B+` — links the kernel and calls `kernel_main`; SD image
      builds. Never run. The verified Pi serial config came from booting
      another project's Buildroot image, not Vivanta.
    - `X96Q` (Allwinner H313) — links the kernel; compile-checked only, needs
      silicon. Blocked by the 16 KiB boot stack and the missing `CPACR_EL1`.
    - `Sumavision Q5` (Amlogic S905L) — cannot print: `platform-amlogic` never
      calls `set_console`.
    - `SDM660 (lavender)` — prints 5 lines and spins; platform deps declared but
      unused. Never ran, and is not scheduled to.
-   RK3568 is a **validation target**, not the architecture foundation. Vivanta
    must not become "RK3568 OS". If RK3568 bring-up is blocked by hardware
    issues, architecture work continues on QEMU.
-   Future expansion (post V3 Device Graph):
    `Raspberry Pi 3B+` ──► `Qualcomm SDM660 (lavender)` ──► `x86_64` / `RISC-V`

### Track C: Tooling & Automation
-   Build: `./build.sh rk3568`, `./build.sh qemu-aarch64` from `vivanta-boot/` — per-target
-   Gate: `tools/qemu-gates.sh` (18 markers); local replay: `tools/ci-local.sh`;
    soak: `tools/soak_test.sh`
-   Future: `cargo xtask` for standardised tasks (rename migration, flash, inspect-state)

### Track D: Verification
-   Host unit tests: **22**, in `vivanta-vm` (12) and `vivanta-exec` (10), run
    natively on x86_64 in CI. Six more host-runnable tests exist but CI never
    runs them (3 in `boot-info/src/mmap.rs`, 3 in
    `boot_common/src/memory_discovery.rs`). `kernel` and `arch-aarch64` set
    `[lib] test = false` by design — the QEMU gate is their verification.
-   QEMU CI for PRs: **shipped 2026-09-18** (`.github/workflows/ci.yml`), but the
    gate job is `continue-on-error: true` (advisory) while the scheduler race is
    open. There is no branch protection, so the job has never blocked a merge.
-   `clippy` is pinned in `rust-toolchain.toml` and **CI never runs it**.
-   No `#[ignore]`, no `#[should_panic]`, no integration tests anywhere.
-   Acknowledge that manual hardware testing on the target device is required before closing milestones.

### Track E: Documentation
-   Keep [STATUS.md](../../STATUS.md) synchronized.
-   Keep this roadmap (`master-roadmap.md`) synchronized.
-   Keep RFC and ADR status up to date.
-   Always document architectural decisions before starting implementation.
-   Watch the retired-claim pattern: point-in-time reports and milestone
    acceptance files were quoted as current long after they expired. Banner a
    document when its claims stop being true, instead of leaving it quotable.

---

## M1.0 — Early MMU Foundation (✅ COMPLETED 2026-07-27)

**Goal:** Establish minimal MMU infrastructure for all AArch64 targets — static identity map,
MAIR/TCR/TTBR0 configuration, MMU enable sequence, and memory discovery.

| Sub-step | Description | Status |
|----------|-------------|--------|
| M1.0.1 | Translation Tables — static L1+L2 page tables, descriptor constants | ✅ |
| M1.0.2 | Identity Mapping — 4 GB, 2 MB blocks, Normal WBWA | ✅ |
| M1.0.3 | MMU Enable — MAIR, TCR, TTBR0, SCTLR_EL1.M | ✅ |
| M1.0.4 | Validation — println! before/after MMU, all targets build | ✅ |
| M1.0.5 | Memory Discovery — subtract kernel/DTB/tables from BootInfo usable regions | ✅ |

UART refactoring (shared `Console`/`PL011`/`println!`, `platform-rpi3b`, GPIO init) was completed
as a prerequisite for M1.0.

**New files:**
- `arch-aarch64/src/early_mmu.rs` — static identity map, no allocator required
- `boot_common/src/pl011.rs` — shared PL011 driver
- `boot_common/src/memory_discovery.rs` — BootInfo → available regions
- `platform-rpi3b/` — GPIO init for RPi3 UART

## M1.1 — Physical Memory Manager (✅ COMPLETED 2026-07-27)

**Goal:** Self-contained bitmap PMM decoupled from BootInfo, with self-test and statistics.

| Sub-step | Description | Status |
|----------|-------------|--------|
| M1.1.1 | Bootstrap Bitmap — place bitmap in first AvailableRegion | ✅ |
| M1.1.2 | Page Allocation — allocate/free/reserve single pages, is_allocated | ✅ |
| M1.1.3 | Region Reservation — reserve_range(start, end) | ✅ |
| M1.1.4 | Self Tests — allocate→free→re-allocate smoke test in boot seq | ✅ |
| M1.1.5 | Statistics — total/reserved/allocated/free counters | ✅ |

**Contract:**
```
MemoryMap → discover() → AvailableRegion[] → PmmBitmap::new(region)
```
PMM has zero knowledge of BootInfo, DTB, or FDT.

## V-Epic Roadmap *(aspirational — not commitments)*

V-epics replaced the earlier R-phase model as the primary planning structure
from 2026-07-19 to 2026-08-11. **V3 and beyond have not started.** The tables
below are retained as the last written intent; treat every "not started" row as
a research direction, not a promise, and treat "Timing" columns as the author's
2026-07/08 guess rather than a schedule.

M-numbers (M1–M5) are preserved alongside V-epics for historical continuity.

### P0 — Rename and Cleanup ✅ done

| Sub-step | Description | Exit Criteria | Status |
|----------|-------------|---------------|--------|
| V0.1a | Cargo package rename (scripted, git tags) | `cargo check --workspace` + 2 build targets pass | ✅ |
| V0.1b | Documentation rename | Build clean | ✅ |
| V0.1c | Directory rename (`TheseusOS/` → `Vivanta/`) | Full rebuild clean | ✅ |
| V0.2 | Roadmap refresh (this document + STATUS.md) | All references consistent | ✅ |
| V0.3 | ~~`docs/vivanta/` scaffold~~ | ~~Stubs created, no dead links~~ | ❌ **Never done.** `docs/vivanta/` does not exist and never did; the row was written during the rename and kept only because its exit criterion demanded zero dead links. The scaffold was replaced by `specs/` + `vivanta-boot/docs/`. Marked cancelled rather than silently left as a dead path. |

**ADRs:** — (rename is execution, not architecture)

### P1 — System State and Identity ✅ (V1.1 completed 2026-07-23)

| Sub-step | Description | Exit Criteria | Status |
|----------|-------------|---------------|--------|
| V1.1 | `SystemState::from_boot_info()` + ownership structure per ADR-020 | SystemState constructed, BootInfo references escaped | ✅ Complete |
| V1.2a | Volatile IdentityState per ADR-023 | Identity generated per boot, enum match enforced | ⚠️ Complete as a *runtime* counter-based identity only — no crypto, no Ed25519 (scope fence) |
| V1.2b | Persistent Identity (blocked on storage driver P3.5) | — | ⏳ Blocked — no storage driver exists |

**ADRs:** ADR-020, ADR-021, ADR-023. (This section previously cited **ADR-024**.
A file of that number did exist — `docs/adr/ADR-024-identity-model-separation.md`
in the *old* pre-2026-08-25 ADR set — but that whole directory was removed in
`d6735c7` and the number was never reused. Nothing called ADR-024 exists today.)

### M4.5.2 — RK3568 Hardware Stabilisation ⚠️ NOT COMPLETE

**What was actually achieved (2026-07-19):** an EL2 **diagnostic** binary that
reaches `adapter_main`, executes Rust at EL2, and produces real UART text on
silicon. The bring-up log
([docs/hardware/rk3568/board-info.md](../hardware/rk3568/board-info.md))
shows `MMU enable 🔲` and `Scheduler 🔲`. **`target-rk3568` does not link
`vivanta-kernel` at all**, and neither `vivanta-kernel` nor `kernel_main` has
ever executed on this board.

**Why it is not ✅:** the board is currently in **Maskrom** after a mainline
U-Boot flash, with boot unconfirmed
([docs/hardware/rk3568/README.md](../hardware/rk3568/README.md), status
2026-09-18). macOS USB bulk transfers to the rockusb device also fail. The
milestone exit criterion — boot the kernel, receive `BootInfo`, execute — has
not been met and cannot be met with the current target crate.

| Objective | Fallback |
|-----------|----------|
| Boot kernel, receive BootInfo, execute diagnostic | If blocked by undocumented HW issue / missing firmware / board failure → continue architecture work on QEMU |
| Goal: `println!` + DTB on RK3568 | **Achieved at EL2 only** (`println!` + FDT report + `test_brk`). Hardware validation of the kernel remains pending. RK3568 is a **validation target**, not a foundation. |

### P2 — V2 / M5: Memory Resource Manager — closed as M5.0 / M6.0

| Sub-step | Description | Exit Criteria | Status |
|----------|-------------|---------------|--------|
| V2.0 | ADR-011 pre-flight: frozen component adaptation review | Change documented, regression pass on QEMU | ✅ |
| V2.1 | ~~ADR-025 MRM Integration Design~~ — *design note only; **ADR-025 does not exist**. MRM integration shipped without a ratified ADR; the only decision record is ADR-030/031 (paging, VA/page-table ownership).* | — | ⚠️ Design shipped, ADR never written |
| V2.2 | PmmBackend + MRM in SystemState (merge BootMemoryManager) | QEMU boots, MRM stats printed | ✅ |
| V2.3 | Runtime page table writer (arch-api + arch-aarch64) | `MemoryObject::map()` programs MMU | ✅ |
| V2.4 | QEMU smoke test — allocate, map, write, read, unmap | MemoryObject lifecycle validated on QEMU | ✅ (closed as M5.0 / M6.0) |
| V2.5 | PlacementPolicy formalization (Kernel, Device, Persistent, Fast) | Enum defined, wired to MRM | ✅ (policy.rs ported from frozen) |

**ADRs:** ADR-011 (Amendment: Frozen Component Unfreezing). *The MRM design
previously cited **ADR-025, which does not exist**.*

### P2 — V2 / M6: Resource-backed Runtime ✅ closed as M6.0

The first genuine isolated EL0 process. Not just a jump — a runtime environment.

| Sub-step | Description | Exit Criteria | Status |
|----------|-------------|---------------|--------|
| V2.6 | MemoryObject smoke test + L2 block splitting | Allocate→map→write→read→unmap via MRM+MMU | ✅ |
| V2.7 | KernelHeap as MRM consumer (not PMM bypass) | Heap backed by MemoryObject | ✅ — `KernelHeap` is a 64 KiB free-list with coalescing, `#[global_allocator]` at `kernel/src/lib.rs:9-10` |
| V2.8 | First EL0 process with allocated MemoryObjects | EL0 binary blob with MRM-allocated stack/heap | ✅ (QEMU) |

**Goal:** `Task { address_space, memory_objects, identity, capabilities }` — a real process, not just a thread.

### P3 — V3: Device Graph and Driver Model — 🚫 not started

| Sub-step | Description | ADRs | Status |
|----------|-------------|------|--------|
| V3.0 | DeviceDescriptor metadata (not Capability — name reserved) | ADR-022 | 🚫 Not started |
| V3.1 | Device Graph (devices/topology/capabilities only, NO driver state) | ADR-020 | 🚫 Not started |
| V3.2 | Driver lifecycle contract: `trait Driver { fn init(); fn shutdown(); }` | ADR-022 | 🚫 Not started |
| P3.5 | Storage driver (SPI NAND primary, eMMC secondary) | — | 🚫 Not started — **this blocks V1.2b** |
| V3.3 | DriverManager (owns driver instances, not SystemState) | ADR-020 | 🚫 Not started |

### P4 — V4: Execution Model Expansion — 🚫 not started

| Sub-step | Description | Status |
|----------|-------------|--------|
| V4.1 | Kernel Task object (Thread + Address Space + Resources) | 🚫 Not started. A partial analogue exists: `kernel/src/syscall/process.rs` implements fork/execve/waitpid over the M6 Task model, but not the Resource-object-carrying Task this row describes |
| V4.2 | Scheduler policy abstraction (concrete RoundRobin, deferred: priority/realtime) | 🚫 Not started |

### P5 — V5: Service Framework — 🚫 not started

| Sub-step | Description | Status |
|----------|-------------|--------|
| V5.0 | Service architecture definition (kernel-service boundary, IPC/communication) | 🚫 Not started — IPC is behind the scope fence |
| V5.1 | First service: **LoggingService** (UART, always needed, validates service boundary) | 🚫 Not started — `println!` over a spinlocked console is the current logging path |
| Future | DownloadService (post-R7 — requires networking + storage; aria2 as replaceable backend) | 🚫 Not started |

### P6 — V6: Recovery Manager — 🚫 not started

| Sub-step | Description | Status |
|----------|-------------|--------|
| V6.0 | Recovery architecture definition (detect/malfunction → restore → reboot) | 🚫 Not started |
| V6.1 | Volatile recovery prototype (QEMU, no storage dependency) | 🚫 Not started |
| V6.2 | Full recovery with persistent identity (blocked on V1.2b) | 🚫 Blocked |

### P7 — V7: Additional Hardware Targets

| Target | Timing | Verified state (2026-09-29) |
|--------|--------|------------------------------|
| QEMU AArch64 | ✅ Baseline (always active) | **The only working configuration**; advisory-gated (18 markers) |
| RK3568 | ✅ Active (primary) — *corrected* | EL2 diagnostic only, does not link the kernel; board in Maskrom, boot unconfirmed |
| Raspberry Pi 3B+ | Post-V3 (Device Graph + Driver model) — *corrected* | Already links the kernel and builds an SD image; **code written, never run**. Awaiting the V3 Device Graph, not a V7 item |
| Qualcomm SDM660 (lavender) | Post-V3 | Prints 5 lines and spins; never ran. Not silicon-ready |
| X96Q / Sumavision Q5 | *(not in the 2026-07 list)* | Link the kernel but are not bootable on silicon today (16 KiB boot stack; missing `CPACR_EL1`; no console on Amlogic) |
| x86_64, RISC-V | Post-V7 | 🚫 Not started; `arch-armv7a` is an 8-line stub (ADR-011) |

---

## Design Watchlist (Architectural Lessons)

### Vivanta (Academic Research)
-   **Study:** Safe live component replacement, Rust typing as a safety boundary, module loading without ELFs.
-   **Do Not Copy:** Process-less monolithic single-address-space design, custom non-standard object format.

### seL4
-   **Study:** Capability derivation tree, authoritative revocation propagation model, formal safety bounds.
-   **Do Not Copy:** Pure microkernel decomposition where memory management is completely externalized to user space (too complex for early bringup).

### Zircon (Fuchsia)
-   **Study:** Virtual Memory Objects (VMO) structure, thread/process handle semantics.
-   **Do Not Copy:** Massive object-oriented C++ kernel codebase (maintain Rust-idiomatic functional approach).

### Barrelfish
-   **Study:** Heterogeneous systems support, hardware topology representation, distributed kernel concepts.
-   **Do Not Copy:** Purely distributed multi-kernel model where message-passing is mandatory even for local intra-SoC memory allocation.

### Redox
-   **Study:** Rust-first kernel design, scheme namespaces.
-   **Do Not Copy:** Unix/POSIX-first compatibility as a primary structural driver.

### Linux
-   **Study:** Memory allocators (buddy/slab), DMA API constraints.
-   **Do Not Copy:** Heavy global Virtual File System (VFS) and POSIX-first subsystem assumptions.

---

## Long-term Vision (R8+)

*The following goals represent long-term research directions rather than committed implementation milestones.*

-   **Hardware Independence:** The operating system shall preserve user identity, state, and execution semantics across fundamentally different processor architectures (e.g., transparently bridging ARM64 and RISC-V) by representing processes as architecture-agnostic object graphs (Thread, MemoryObject, Capability).
-   **Atomic State Migration:** Support for serializing and signing the entire state of an execution context (including MemoryObjects) to allow migration across non-ECC consumer hardware with verified integrity.
-   **Distributed Resource Topology:** The operating system continuously measures hardware parameters (coherence, power, latency) and adapts its own memory/execution layout dynamically across a pool of heterogeneous cluster nodes.

---

## Changelog

*Every roadmap modification must update this section.* Newest first.

> **Gate-count note.** Counts quoted below ("19/19", "22/22") are what was
> measured on the day. None of them is the current figure: the gate now
> requires **18 serial markers** and the CI job running it is advisory
> (`continue-on-error: true`). The point-in-time mission reports those entries
> link to have since been moved to `docs/history/missions/`; the durable record
> is [docs/audit/2026-09-24-global-audit.md](../audit/2026-09-24-global-audit.md)
> and [docs/audit/2026-09-25-cleanup.md](../audit/2026-09-25-cleanup.md).

### 2026-09-25 — cleanup, advisory gate, and a retraction

- `d083f8e` **fix(scheduler)**: a thread could be scheduled before its `Task`
  record existed, so `thread_exit` silently lost the exit code. Threads are no
  longer schedulable before the process record lands.
- `82a2b94` **fix(vm)**: `VaAllocator::reserve()` must be covered by a single
  free interval and must not mutate state before it succeeds; `alloc()` no
  longer overflows when rounding to page size. 4 new host tests.
- `4ddbbfe` **fix(scheduler)**: `InterruptGuard` outlived the context switch and
  restored DAIF on the *new* thread.
- `465cbf4` **perf(scheduler)**: `CURRENT_THREAD` is published immediately before
  the switch; the TLB flush was moved out of the race window.
- `602fe7b` **test(gates)**: `[FORK-EL0] child teardown` no longer prints PASS
  when teardown did not run; `waitpid` now returns `ECHILD`.
- `358c692` **docs(audit)**: the night "YOLO" mission findings on the scheduler
  race — measured evidence that IRQs are effectively never masked, that the
  architecturally correct `DAIFSet #4` exposes a GIC delivery stall, that the
  `CURRENT_THREAD` window is the residual race, and that tracing hides the bug
  (heisenbug). Records why the re-entrancy-flag attempt was reverted: it would
  have silently disabled preemption for EL0 threads. *(Correction 2026-09-29:
  the "IRQs never masked" / "GIC delivery stall" reading was wrong — see the
  entry below. The heisenbug and the reverted-flag record stand.)*
- `029057d` **ci**: the QEMU gate job becomes **`continue-on-error: true`**
  (advisory) while the scheduler race is open. Measured flake rate on
  `ubuntu-24.04-arm`: ~1 run in 2–5.
- `6e78a11` **docs(ci-cd)**: records the advisory status and the reason.
- `b4b30a0` **chore**: working tree 118 MB → 13 MB. Removed the vendored
  `rkdeveloptool` Mach-O, **`target-qemu-armv7a/`** (not a workspace member,
  `armv7a-none-eabi` not in the pinned toolchain), `arch-test-stub/` +
  `target-test/` (the stub lacked 13 `arch-api` symbols and `target-test`
  never referenced `kernel_main`, so the "ISA-independence proof" proved
  nothing), `target-mmu-test/`, and 10 unreferenced scheduler items.
- `65eff7f` **docs+ci**: M4 acceptance C11 RETIRED with the dependency-direction
  rule named as what actually holds; the wrong `repository-layout.md` corrected
  (the root one is the stale copy); CI firmware fallback un-broken.
- `273f9bc` **docs**: retractions propagated everywhere the `target-test` claim
  survived, including `current-architecture.md` (target crate count 9 → 6).

### 2026-09-24 — global audit and clean-clone bootstrap

- `99a1ec9` **fix(kernel)**, found by the five-track audit, verified 18/18
  markers with 0 panics: `context_fork` gave the child `SP_EL0` = the parent's
  saved `SP_EL1`; `wake_waiters` resurrected SIGKILLed threads; `sys_kill`
  could kill a Zombie and could alias `kill(pid, 265)` to SIGKILL;
  `rt_sigreturn` returned 0 with no delivery machinery; `unmap`/`protect`
  only programmed hardware for `Backing::Present`, so COW leaves survived
  teardown and `mprotect` left COW pages executable. The `interrupts.rs` claim
  of a "non-existent QEMU DAIF quirk" was replaced with a new but **incorrect**
  reading that `#2` masks SError, not IRQ — corrected on 2026-09-29 (`#2` is
  the IRQ mask; the old "quirk" framing was itself wrong, but so was its
  replacement). See the correction note in the audit.
- `16c7134` **fix(build)**: `build.sh`, `make-rpi3b-image.sh` and `soak_test.sh`
  now build the gitignored `user-init.elf` themselves, so a fresh clone can
  build `vivanta-kernel`; `rust-objcopy` resolved through a wrapper; CI
  artifacts keyed by `run_attempt`; release jobs pinned to `inputs.tag`;
  `contents:write` scoped to the publish job only.
- `a1ed7d7` **docs**: public status claims corrected (syscalls, gate counts,
  RK3568 scope, `execve` scope), and
  [docs/audit/2026-09-24-global-audit.md](../audit/2026-09-24-global-audit.md)
  recorded — this is the document that produced the current backlog.

### 2026-09-18 — CI/CD landed, then went advisory, then blocking again

- `3011c39` **ci**: GitHub Actions pipeline (lint, native host tests, kernel
  build, QEMU gate, RPi3B+ SD image) and `release.yml` publishing artifacts
  with checksums; portable `tools/qemu-gates.sh`, `tools/build-user-init.sh`,
  `tools/ci-local.sh`, `tools/make-rpi3b-image.sh` (mkfs.vfat + mtools instead
  of macOS-only `hdiutil`); `docs/tooling/ci-cd.md`.
- `00a22f6`, `5a8d86b`, `0da636d`, `564926f` **fix(ci/release)**: objcopy from
  the toolchain sysroot, un-break the package step, `gh release create -R`,
  unique checksum asset names.
- `1256baa` **docs**: README/ROADMAP/STATUS refreshed for CI and badges.
- `50f7909` **ci**: gate demoted to advisory — the flake was cross-ISA TCG
  hang on an x86_64 runner.
- `50fecb7`, `c8a14c9` **ci**: run the gate natively on `ubuntu-24.04-arm`
  (host and guest share an architecture, so no cross-ISA TCG) and drop
  `continue-on-error` again. The flake was TCG, not the gate.
- `ff1ca2c`, `2319f5d`, `a2fae75` **docs/chore**: `docs/hardware/rk3568/`
  promoted to the canonical U-Boot/eMMC runbook; AIBOX-1684X reconnaissance
  moved into the canonical tree; `tools/log/` ignored.
- *Hardware, non-code:* board observed in **Maskrom** after the mainline U-Boot
  flash; boot unconfirmed (`docs/hardware/rk3568/README.md`, status 2026-09-18).

### 2026-09-15 — P0 mission follow-ups, Pi 3B+ wired to `kernel_main`

- `f6ed78f` **fix(P0)**: execve stack/return/icache, VA reclaim, process-table
  lifetime, signal-delivery honesty, fault capacity.
- `cb17b7a` **test(P0)**: six regression gates moved into `#[inline(never)]` fns.
- `5005542` **feat(P1)**: the `spec-table-desc` feature (L1/L2 table descriptors
  `0b11` → `0b10`; default unchanged because QEMU hangs with `0b10`), and the
  guarded x96q EL2→EL1h drop — **"Compile-checked only, needs silicon"**, which
  is the exact wording this roadmap previously contradicted.
- `795f119` **docs**: P0 audit sync (STATUS/README/INV-001/ADR-035).
- `5eb7936` **fix(YOLO)**: signal-mask hardening, kill-path cleanup.
- `aef1fcb` **feat(YOLO)**: EL0 fork round-trip + dual-ELF gates, 64K thread
  stacks, child `x30` = `eret_to_user_stub`.
- `b84d58f` **fix(YOLO)**: the same `context_fork` `x30` bug.
- `b8b26c3` **docs**: P0 report and `INV-003` forensics (+ stack addendum).
- `251b981` **feat(rpi3b)**: `target-rpi3b-plus` brought to `kernel_main` —
  **untested on silicon**. Its 16 KiB-era `early_mmu` claim in STATUS was wrong
  from this commit onward: the target links `vivanta-kernel`.
- `958af2c` **feat(rpi3b)**: one-command SD image (MBR + FAT32, EL2 drop,
  `miniuart-bt` overlay — without it the Pi 3 routes PL011 to Bluetooth).

### 2026-09-09 — P0 corrective mission closed *(point-in-time record)*

- All audit §3.1-class defects fixed (execve stack/return/icache, VA reclaim,
  process-table lifetime, signal-delivery honesty, fault capacity), 6 new
  regression gates, 0 panics. Gate count reported as 19/19 that day; a later
  same-day follow-up reported "22/22 milestone gate groups". Both framings are
  retired — the gate requires **18 markers** and is advisory. The mission log
  itself now lives in `docs/history/missions/`; the durable record is the
  2026-09-24 audit.
- **INV-003 (closed)**: the `-O0` `kernel_main` frame (≈82 KiB) overflowed the
  128 KiB boot stack into `.bss`, killing the console silently before the dump;
  the stack is now 256 KiB and the gates are `#[inline(never)]`. Forensics in
  `vivanta-boot/docs/investigations/INV-003-boot-stack-overflow.md`.
- **Follow-up (same day)**: thread stacks 16 → 64 KiB (the fork path measured
  18 KiB past the bottom — a second INV-003 instance).
- `spec-table-desc` and the x96q EL2→EL1h drop landed (see 2026-09-15).
- Next: functional work without silicon (signal-mask hardening, MAIR/TCR audit);
  scope fence unchanged.

### 2026-09-05 — M10.2 real EL0 fork

- `9757f55` **feat(M10.2)**: `mmu_clone_kernel_half()` gives a forked child a
  real root (identity RAM + MMIO) instead of a zero-filled L1 that killed it on
  first schedule; `write_desc` cleans descriptors to PoC; `duplicate_as` keeps
  the *original* permissions in shadow and forces hardware to RO so a child
  cannot inherit a parent's mappings; `KernelHeap` free list gained adjacent
  coalescing; a refcount-zero COW frame is returned to the backend; stale AS
  lookup is panic-free.

### 2026-08-28 — G-M10 process syscalls, G-M11 signals

- `551a6b2` … `94fdfeb` **G-M10**: `fork` with COW address-space duplication,
  `waitpid` with blocking + parent notification, `munmap`/`mprotect`, `execve`
  over a compile-time built-in program registry, PID domain unified to
  `TaskId`; fork SP bug and `ExceptionFrame` unification fixed.
- `b58adbe` **G-M11 signals**: `sigaction` (13), `sigprocmask` (14, reserved),
  `sigreturn` (15, reserved), 32 dispositions, blocked mask inherited across
  `fork`, `SIGKILL` unblockable. `pending` is still write-only — there is no
  async EL0 delivery path (scope fence, `kernel/src/signal.rs`).
- `1e4c2a4` **test**: PID-domain gate. `e48b52e` **fix**: that gate had to be
  disabled — it left AS 8/9 registered and poisoned the next VM test.
- `eacfe20` **revert**: `G-M10.1 true COW` was reverted to eager copy to keep the
  QEMU COW gate green.
- `3b71227` **docs**: autonomous mission plan for G-M10 hardening.

### 2026-08-27 — G-M9 Copy-on-Write

- `661e22e` **feat(mm)**: `Backing::CoW` + `PhysOwnership::CoWShared`
  refcounting, `resolve_cow_fault()` (allocate, copy, remap writable,
  decrement; last owner keeps the frame), EL0/EL1 write-permission faults
  routed through the COW resolver. QEMU gate `[COW] break+isolation`.
- Known remaining defect (2026-09-24 audit): the `Present` → `CoWShared`
  conversion in `kernel/src/vmm/address_space.rs` ignores prior `External`
  ownership, so the last owner can free a frame the VMM never owned.

### 2026-08-26 — G-M8 ELF64 loader and the first genuine userland

- `f8039b0` **feat(exec)**: the `vivanta-exec` ELF64 parser (magic/class/
  endianness/machine/type, PH bounds, truncation, page congruence, W^X
  rejection, segment overlap, entry inside an executable segment) with 10 host
  tests; a kernel loader that applies a validated `LoadPlan` through proven VMM
  primitives only; `materialize_with()`; `AddressSpace::unmap_all()`; and
  `user-init` — 111 lines of naked assembly, the project's only real user
  program. QEMU gate `[ELF] genuine ELF64 userland PASS (exit=42)`.
- `6fd73d8` **fix(exec)**: host-test `std` imports and `vivanta-vm` warnings.

### 2026-08-25 — G-M7 syscall ABI frozen; research track archived

- `72eb73b` **feat(abi)**: the syscall ABI frozen (ADR-033) and
  `mmap`/`munmap`/`mprotect` implemented.
- `08859f1` **feat(process)**: generation-protected handles + deterministic
  process-table capacity (M7.4).
- `09dcc99` **docs(adr)**: ADR-032 §2.3 amendment — EL0 lazy-fill resolution
  policy.
- `a35bb41` **docs**: G-M7 final acceptance report.
- `d6735c7` **chore(repo)**: the quarantined research track (ADR-031…039 plus a
  root copy of ADR-018) was moved to `docs/archive/research-track/`, and the
  whole old `docs/adr/` directory was removed — including **ADR-024 and
  ADR-025, which no longer exist anywhere in the tree**. The engineering ADRs
  that cited them live in `vivanta-boot/docs/adr/`. Note the **number
  collision**: research ADR-031…039 and engineering ADR-031…035 are unrelated.

### 2026-08-24 — reverse-direction INV-VM-001 verifier

- `69f556b` **feat(vm)**: ghost-leaf detection (hardware → shadow, not only
  shadow → hardware).

### 2026-08-23 — M6.0 user-VM mission closed

- `d69a516` **docs(vm)**: ADR-032 fault-resolution semantics.
- `bd3d46b`, `62328fa` **feat(vm)**: lazy reservation, demand-fill fault
  resolution, hardware verifier.
- `02da886` **test(mm)**: QEMU lazy-VM lifecycle, stress and mutation coverage.
- `f1eb6c9`, `5fec9e6` **docs**: M6.0 mission report; M6.0 recorded as closure of
  fault/recovery + transactional paging semantics.

### 2026-08-21 (mission 4 — M6.0)
-   **User VM foundation**: ADR-032 (fault classification/retry), Mapping
    state machine (`Present/LazyAnonymous/Reserved` + `PhysOwnership`),
    EL1 resumable fault vector (`el1h_sync` may now return for approved
    demand-fills; retry = same instruction, ELR untouched).
-   **INV-VM-001 enforced mechanically**: MappingSet⇔hardware verifier
    (per-piece descriptor readback incl. permission-bit compare).
-   **Transactional value-keyed shadow commits** replace slot-index
    arithmetic in protect/unmap/demand-fill (stale-slot corruption found
    by stress testing).
-   **FIXED**: runtime descriptor writes missing cache clean-to-PoC
    (`walker::write_desc` now does `dc civac`); QEMU per-VA TLBI
    unreliability worked around via full flush (single-core/no-ASID);
    lifecycle test mapped PA it did not own (test bug, caught by stress).

### 2026-08-21 (mission 3 — M5.1/M5.2)
-   **VA allocator** added (`vivanta-vm`, host-proven): first-fit interval
    allocator, overflow-safe, canonical merged free list, user/kernel
    domain separation.
-   **Page-table ownership** (ADR-031): install-time registry via
    `PageTableAllocator::table_installed`; reclamation only after
    hardware-proven emptiness + IRQ-guarded parent unlink; roots and
    boot-era frames leak by explicit rule.
-   **Range protect/unmap**: partial ranges supported; shadow mappings
    split transactionally; `mmu_map_object` now creates missing
    intermediate tables (allocator-backed).
-   QEMU VM lifecycle test added (map/protect/unmap/reclaim/alias/
    block-split/remap); mutation battery M1–M4, M6, M8 caught; M5 (TLBI
    widening) remains a QEMU limitation.

### 2026-08-21 (mission 2)
-   **Toolchain:** whole workspace migrated to Rust 1.98.0 stable +
    edition 2024 (`rust-toolchain.toml` pin, resolver 3). unsafe-extern /
    unsafe(no_mangle) / static-mut-ref rework across all members.
-   **Protection audit (post-M5):** full permission matrix asserted on all
    three encoders; W^X rejection assert at every encoder; kernel-AS
    RW->RO->RW protect + TLBI transition test; three hardware-visible EL0
    fault scenarios (exec-NX, kernel-read, unmapped) with ESR/FAR asserts;
    five-mutation battery run.
-   **FIXED (critical):** `MrmPageTableAllocator` freed page-table frames
    while the MMU walked them (MemoryObject dropped -> PMM deallocate ->
    frame reused as data). Latent since M5 runtime mapper; frames now leak
    deliberately like boot tables until refcounted table teardown exists.
-   Known limitation: a removed TLBI after permission *widening* is not
    observable under QEMU (re-walk on permission miss); narrowing-side
    staleness is bounded by tlbi_all_sync on every address-space switch.

### 2026-08-21
-   **W^X enforced for AArch64 user pages** (WX-001): user read-only pages
    previously encoded AP=01 (EL0 *writable*), making user code RWX in
    practice despite the M5.0 G3 "user code = RX" claim. Fixed via single
    `ap_bits()` encoder + boot-time `[WX]` descriptor readback + EL0
    store-to-code-page negative test
    (`vivanta-boot/docs/investigations/WX-user-code-ap-encoding.md`).
-   **VMM protect() implemented** (was `todo!()`): arch-api `mmu_protect`
    contract + arch-aarch64 implementation (page-granular permission rewrite,
    2 MiB block splitting, TLBI) + `AddressSpace::protect` wiring
    (whole-mapping granularity). ADR-019 amended accordingly.
-   INV-002 fix (console-lock IRQ guard + ThreadContext at stack bottom)
    committed to main after re-verification.

### 2026-08-14 — RPi3B+ serial bring-up verified *(foreign image)*

- `f2af711` **docs(hardware)**: the Pi 3B+ reached a login prompt — booting
  **another project's Buildroot image**, not Vivanta. Root cause of the earlier
  bare-metal failure: the firmware routes PL011 to Bluetooth by default, so
  `dtoverlay=miniuart-bt` is mandatory. This is a hardware/configuration
  result; no Vivanta code ran on the board.

### 2026-08-11
-   **M5.0 GREEN BASELINE — CLOSED** (`vivanta-boot/docs/milestones/M5.0-green-baseline.md`).
-   **M6 Process Lifecycle — CLOSED** (`vivanta-boot/docs/milestones/M6-process-lifecycle.md`).
-   Current Focus: post-M6 deferred reliability (60-min soak, MMU descriptor HW validation), next milestone from actual state.
-   ADR registry reconciled: canonical engineering ADRs live in `vivanta-boot/docs/adr/`;
     the distributed/AI research track was quarantined. *(The original wording
     said "ADR-031…039 quarantined" — which reads today as if it referred to
     the *engineering* ADRs. It referred to a separate research-track set that
     now lives in `docs/archive/research-track/`, whose numbers collide with
     engineering ADR-031…035.)*

### 2026-07-24
-   **V2/M5 Phases A–E completed:** PmmBackend created, MRM integrated into SystemState, BootMemoryManager removed.
-   A design note for Memory Resource Manager integration was proposed with 5 implementation phases. *It was written as `docs/adr/ADR-025` and removed with the rest of the old `docs/adr/` on 2026-08-25 (`d6735c7`). **No ADR-025 exists today**; the MRM integration shipped without a ratified ADR.*
-   Runtime page table writer added: `mmu_map_object` / `mmu_unmap` in arch-api + aarch64 impl.
-   `MemoryObject::map()` now programs real MMU via arch-api, not just software-recorded slots.
-   `kernel/src/memory/` module created: resource, capability, object, policy, manager, pmm_backend.
-   Full workspace build passes on QEMU AArch64 and RK3568.
-   V2/M5 substeps V2.0–V2.3 and V2.5 marked complete; V2.4 (QEMU smoke test) pending.

### 2026-07-23
-   **V1.1 completed:** Runtime Identity, ADR-021 ratified. *(The entry also
    read "ADR-024 ratified". That referred to the pre-2026-08-25 ADR set at
    `docs/adr/` — note the old `docs/adr/ADR-021-system-state-encapsulation.md`
    is a *different* document from today's
    `vivanta-boot/docs/adr/ADR-021-bootinfo-escape-prevention.md`. ADR-024 does
    not exist today.)*
-   RK3568 boot flow unified with QEMU (adapter_main → FDT → BootInfo → kernel_main).
    *Superseded in practice: `target-rk3568` does not link the kernel, so the
    unified flow was never exercised on the board.*
-   Current Focus updated to P2 V2/M5 Memory Resource Manager.

### 2026-07-19
-   **Roadmap v1.1:** Replaced R-phase structure with V-epics (V0–V8).
-   Current Focus updated to M4.5.x/RK3568 + V0 rename.
-   Added V-epic dependency graph; R2 marked ✅, R3 K1/K2 ✅, K3–K5 deferred.
-   Hardware target list updated (lavender → RK3568).
-   RK3568 fallback policy documented.
-   V2.1 MemoryObject redefined as "hardware adaptation" (not redesign).
-   V5 restructured: LoggingService first, DownloadService deferred.
-   References new ADRs 020–023.

### 2026-07-12
-   Initial release of the Master Roadmap.
-   R2.0 and R2.1 marked as active focus.
