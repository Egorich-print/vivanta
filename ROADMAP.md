# Vivanta Roadmap

The detailed engineering roadmap is
[docs/architecture/master-roadmap.md](docs/architecture/master-roadmap.md).
This page is a short public-facing summary.

## Milestones

| Milestone | Goal | Status |
|-----------|------|--------|
| M1 | Boot protocol and BootInfo on QEMU aarch64 | ✅ |
| M2 | Virtual memory: paging, PMM, VMM | ✅ |
| M3 | Process model: tasks, threads, scheduler, syscalls | ✅ |
| M4 | Execution foundation: privilege transitions, EL0 | ✅ |
| M4.5 | First user-space program (hello world via syscalls) | ✅ |
| M5.0 | QEMU-correct recovery baseline (green-baseline) | ✅ CLOSED |
| M6.0 | User VM: fault-driven mapping, transactional paging semantics | ✅ CLOSED |
| G-M7 | Syscall ABI + VM syscalls + process model | ✅ CLOSED |
| G-M8 | ELF64 loader + first genuine ELF userland (`user-init`) | ✅ SHIPPED |
| G-M9 | Copy-on-Write for anonymous private memory | ✅ SHIPPED |
| G-M10 | Process lifecycle: `fork`/`execve`/`waitpid`, EL0 round-trips | ✅ SHIPPED |
| G-M11 | Signals: `sigaction`, blocked mask, unblockable `SIGKILL` | ✅ SHIPPED (no async delivery) |
| M12+ | Userspace services, IPC, drivers, networking | planned (scope fence) |

[vivanta-boot/docs/milestones/](vivanta-boot/docs/milestones/) holds
close-out records for M5.0, M6.0 and G-M10 — and the G-M10 one is still headed
`Status: PLANNING`, predating the work it describes, so read it as a plan rather
than a record. G-M7, G-M8, G-M9 and G-M11 have no milestone document at all;
they are evidenced by their commits and by the QEMU gate markers that cover
them (`[ELF]`, `[COW]`, `[FORK-EL0]`, `[KILL]`).

## Current objective

Not "validate M10" — that objective was retired by the 2026-09-24/25 audit.
The current blockers, in order:

1. **Scheduler race.** A timer tick can land inside a context switch, losing a
   user thread's exit code or leaving a forked child unscheduled. Printing a
   trace makes it disappear (heisenbug), so it needs a deterministic failpoint.
   Until it is fixed the QEMU gate is **advisory**, not blocking. (An earlier
   audit item claimed IRQ masking was architecturally wrong — that finding was
   **disproven**; `DAIFSet/Clr #2` masks IRQ correctly. See
   [docs/audit/2026-09-24-global-audit.md](docs/audit/2026-09-24-global-audit.md).)
2. **Silicon blockers:** 16 KiB boot stack on `target-x96q` and
   `target-suma-q5` (256 KiB elsewhere) even though both reach `kernel_main`
   with the same ~82 KiB frame that forced the bump; FP/SIMD left at reset on
   the EL2 path of 5 of 6 targets — `msr CPTR_EL2, #(0b11<<20)` is an
   architecturally **no-op** (`CPTR_EL2.TFP` is bit 10; bits 20:21 are RES0)
   and the `msr CPACR_EL1` that would set `FPEN` sits behind a label the EL2
   branch jumps past, so `FPEN` stays at reset `0b00` and traps FP/SIMD to
   EL1; and neither board is silicon-ready at all.

Because of (1) and (2), new subsystems stay behind the scope fence. See the
[2026-09-24 global audit](docs/audit/2026-09-24-global-audit.md),
[STATUS.md](STATUS.md) and the
[master roadmap](docs/architecture/master-roadmap.md).

## Next mission (2026-10): correctness, honest gates, programmable userland

The full executable plan lives in
[`docs/planning/mission-2026-10/00-mission-charter.md`](docs/planning/mission-2026-10/00-mission-charter.md)
with six deep appendices (scheduler race, memory, process lifecycle, silicon,
functionality, CI/repo). In short: deterministic race reproduction first, then
a single-serialized teardown + frame-ownership core, then honest gates and a
blocking CI job; userland becomes programmable (`execve` transactions,
`argv`/`envp`, signal delivery). Five hard serialisation points
(`address_space.rs`, `scheduler/mod.rs`, `kernel/src/lib.rs`,
`arch-aarch64/src/user.rs`, `syscall/process.rs`) — one owner per file per wave.

## Design principles

1. **Hardware before abstraction** — concepts are designed against validated
   hardware realities, then generalized.
2. **Prototype before RFC** — a new RFC needs a working experiment first.
3. **Mechanism before policy** — the kernel provides low-level mechanisms;
   resource orchestration lives at a higher layer.
4. **One active milestone** — parallel research is fine, but focus stays on the
   active release goal.
5. **No feature debt** — no new features while the active milestone's exit
   criteria are unmet or warnings/clippy issues remain.

## Platforms

Verified states, not intentions. No Vivanta kernel has ever reached
`kernel_main` on physical hardware.

| Platform | Verified state |
|----------|----------------|
| `qemu-aarch64` | **The only working configuration.** Boots to `kernel_main` with EL0 userland. The gate checks 18 required serial markers (`tools/qemu-gates.sh`); the CI job that runs it is `continue-on-error: true`, so it is advisory, not a merge gate |
| `rpi3b+` | `target-rpi3b-plus` links `vivanta-kernel` and calls `kernel_main`; SD image builds (`tools/make-rpi3b-image.sh`). **Code written, never run.** The verified Pi UART config came from booting another project's Buildroot image, not Vivanta |
| `x96q` (Allwinner H313) | Links the kernel, calls `kernel_main` at `src/main.rs:181`. **Compile-checked only, needs silicon** (commit `5005542`) |
| `suma-q5` (Amlogic S905L) | Links the kernel, but `platform-amlogic` never calls `set_console`, so `with_console()` panics — it cannot print a character, let alone boot |
| `rk3568` | EL2 diagnostic adapter only — **does not link `vivanta-kernel`**. Real UART output from a Vivanta binary on silicon (July 2026) is the project's only silicon data point. The board is currently in **Maskrom**, post-mainline-U-Boot-flash, boot unconfirmed |
| `lavender` (SDM660) | Prints 5 lines and spins. Its platform deps are declared but unused. Vivanta never ran on it (nor on `whyred`); both were surveyed with external evidence only |
| `arch-armv7a` | An 8-line stub kept per ADR-011. The `target-qemu-armv7a` crate was **deleted 2026-09-25** (not a workspace member; `armv7a-none-eabi` is not in the pinned toolchain) |

## Long-term vision

Vivanta aims to become a distributed operating system where execution state and resources are represented as architecture-agnostic objects. This enables:
- **Identity & State Continuity** — process state and memory objects can be migrated across heterogeneous hardware (e.g., PC to mobile).
- **Resource-Oriented Memory** — strict separation of mechanism (kernel capabilities) and policy (orchestration).
- **Heterogeneous Compute Clustering** — utilizing budget smartphone SoCs as secure, coordinated cluster nodes.

These are research directions, not commitments. See the
[network services vision](vivanta-boot/docs/rfc/network-services-vision.md).
`docs/research/cluster_research.md` is a **used-phone buying survey** (July
2026 prices, GeekBench figures) — useful as a node-selection input, but it
contains no measured hardware and is not an architectural exploration.
