# Vivanta OS Maturity Assessment

> **Rewritten:** 2026-09-29 (previous version dated 2026-07-24)
> **Verified against:** HEAD `273f9bc`, `vivanta-boot/tools/qemu-gates.sh`,
> `docs/audit/2026-09-24-global-audit.md`
> **Reference:** [STATUS.md](../STATUS.md),
> [master-roadmap.md](architecture/master-roadmap.md),
> [global audit](audit/2026-09-24-global-audit.md)

---

## 1. Why this document has no headline percentage

The previous version of this file opened with **"~35-40% done"**. That number
is gone, and it is not being replaced with a new one.

A single percentage for "OS maturity" is not measurable from inside this
repository, and it has already been wrong twice: at 2026-07-24 it counted three
subsystems as pending that had since shipped, and the reasoning under it
(`§4`, "do not build a heap allocator yet, `StubAllocator` is ugly") described a
symbol that no longer exists anywhere in the tree. A number that is quotable
out of context and cheap to produce is a liability in a document like this one.

What follows instead is the same information, decomposable and falsifiable:
what exists, at what evidence level, and what has not been started. Every
claim below is anchored to a file you can open.

### How to read the tiers

| Tier | Meaning |
|------|---------|
| **A** | Runs and is asserted by the 18-marker QEMU gate |
| **B** | Implemented and gate-covered, but some assertions are weak (see below) |
| **C** | Implemented and compile-verified; never executed on any machine |
| **D** | Diagnostic scaffolding; does not reach `kernel_main` |
| **E** | Not started. No code. |

**Caveat that applies to all of Tier A and B:** the QEMU gate job in
`.github/workflows/ci.yml:87-99` is `continue-on-error: true` and `main` has no
branch protection. A green run has never blocked a merge. The `qemu` job
concluded `failure` in **9 of the 21 non-cancelled `main` runs** of the `ci`
workflow to date (**≈1 in 2.3**); the most recent 4 are green, which 4 samples
at that rate does not establish as a fix. That is a scheduler race described in
`docs/audit/2026-09-24-global-audit.md`. Never read "CI green" as "verified".

## 2. What exists, by tier

### Tier A — running under the QEMU gate

The gate requires 18 serial markers (`vivanta-boot/tools/qemu-gates.sh:28-47`),
terminated by `[G4] running_count=1` (`vivanta-boot/tools/qemu-gates.sh:48`).
In the table below, `lib.rs:N` means `vivanta-boot/kernel/src/lib.rs:N`.

| # | Subsystem | Evidence in the gate (marker → what it prints from) |
|---|-----------|------------------------------------------------------|
| 1 | Boot chain ROM → `BootInfo` → `kernel_main` | `Hello, Vivanta!` — an assembly blob inside the kernel at `vivanta-boot/arch-aarch64/src/user.rs:107` |
| 2 | MMU: 4 KiB L1→L2→L3 walk, 39-bit VA, `mair_el1=0x44_FF` | implied by every later marker |
| 3 | Process table lifecycle (create / remove / live count) | `[PTABLE] 70 create/reap cycles, live=N PASS` — `vivanta-boot/kernel/src/lib.rs:223` |
| 4 | VMM: demand paging, `LazyAnonymous`, COW (`CoWShared` refcounts) | `[COW] break+isolation PASS` (`:1353`), `[OOM] full-table resolve refuses cleanly PASS` (`:326`) |
| 5 | Address-space dup + teardown returning frames to the PMM | `[E2E] load/dup/teardown deltas all zero PASS` (`:557`) — `register_child`, `unmap_all`, `unregister`, `pmm.free_frame` |
| 6 | GICv2/v3 + 100 Hz timer, round-robin scheduler | `[G4] running_count=1` (`:2061`) |
| 7 | EL0 entry, SVC roundtrip, per-address-space isolation | `[ELF] genuine ELF64 userland PASS (exit=42)` (`:1265`), `[EXEC2] double load+stack in one AS PASS` (`:189`) |
| 8 | Syscall ABI (ADR-033), 16 numbers / 13 live / 3 `-ENOSYS` | `[SYS] mmap/store/mprotect/munmap/negatives PASS` (`lib.rs:1208`) |
| 9 | Process model: `fork`, `execve`, `waitpid` | `[FORK-EL0] fork/waitpid round-trip exit(42) PASS` (`lib.rs:454`), `[EXECVE] EL0 execve("/init") -> exit(42) PASS` (`lib.rs:127`) |
| 10 | `mmap`/`munmap`/`mprotect` with MMU enforcement | `[SYS]` (`lib.rs:1208`), `[OOM]` (`lib.rs:326`) |
| 11 | `kill` → `SIGKILL` → zombie + `SIGCHLD` | `[KILL] SIGKILL->Zombie + SIGCHLD PASS` (around `lib.rs:260`) |
| 12 | Concurrent multi-process execution | `[DUAL] two concurrent ELF tasks exit(42) PASS` (`lib.rs:404`) |

Also Tier A: 22 host unit tests (12 in `vivanta-boot/vm/src/va_tests.rs`, 10 in
`vivanta-boot/exec/src/elf_tests.rs`), run in CI. Six more exist and CI never
runs them (3 in `vivanta-boot/boot-info/src/mmap.rs`, 3 in
`vivanta-boot/boot_common/src/memory_discovery.rs`). `kernel` and
`arch-aarch64` set `[lib] test = false` by design — the gate is their test.

### Tier B — real, but under-asserted

| Subsystem | Why not Tier A |
|-----------|----------------|
| MRM / `MemoryObject` / `MemoryCapability` / `PlacementPolicy` (`vivanta-boot/kernel/src/memory/`) | Exercised through the memory gates, not asserted directly. `MemoryCapability::check()` is a declared placeholder that always returns `true` |
| Kernel heap: free-list with coalescing, fixed 64 KiB, MRM-backed (`vivanta-boot/kernel/src/memory/kernel_heap.rs`, `#[global_allocator]` at `vivanta-boot/kernel/src/lib.rs:9-10`) | 64 KiB is the whole heap; the audit notes remaining `Vec` uses on user and fault paths where OOM panics |
| `signal.rs` dispositions | `pending` is write-only by explicit scope fence (`vivanta-boot/kernel/src/signal.rs:134-140`); `MAX_SIG=32`, `SIGKILL` only |
| Gate assertions themselves | Several gates compare `after >= before` instead of exact values, or sit inside `if let`, so a branch that never ran still prints PASS |

### Tier C — implemented, never executed

| Target | SoC | Why not higher |
|--------|-----|----------------|
| `vivanta-boot/target-rpi3b-plus/` | BCM2837B0, Cortex-A53 | Links the kernel and calls `kernel_main` (`vivanta-boot/target-rpi3b-plus/src/main.rs:132`). Never run. Also has the FP/SIMD defect in `docs/architecture/arm-exception-state.md` |
| `vivanta-boot/target-x96q/` | Allwinner H313 | Calls `kernel_main` (`vivanta-boot/target-x96q/src/main.rs:181`). "Compile-checked only, needs silicon". 16 KiB boot stack |
| `vivanta-boot/target-suma-q5/` | Amlogic S905L | Calls `kernel_main` (`vivanta-boot/target-suma-q5/src/main.rs:175`) but **panics first** — `platform-amlogic` never calls `set_console`, so `with_console()`'s `.expect(...)` fires (`vivanta-boot/platform-amlogic/src/lib.rs:105-114`). Cannot print a character |
| `vivanta-boot/target-lavender/` | SDM660, Redmi Note 7 | Declares `vivanta-kernel` but never calls it; prints five lines, then spins (`vivanta-boot/target-lavender/src/main.rs:71-87`) |

The 16 KiB boot stack in `vivanta-boot/target-x96q/linker.ld:34` (vs
256 KiB in `vivanta-boot/target-qemu-aarch64/linker.ld:41`) is
the reason x96q and suma-q5 are not bootable on silicon today.

### Tier D — diagnostic only

| Target | What it does |
|--------|--------------|
| `vivanta-boot/target-rk3568/` | EL2 stage: UART banner, FDT report, `test_brk` (`vivanta-boot/target-rk3568/src/main.rs:173`). No `vivanta-kernel` dependency at all |
| `vivanta-boot/platform-rk3568/` | A workspace member that **no target links** |

RK3568 is nonetheless the only board where anything Vivanta-related has run on
silicon: a committed UART capture from 2026-07-19 decoding to
`KC3FGZAB1CONSOLE OK231234WITH CONSOLE OK54X`
(`docs/hardware/rk3568/board-info.md:409-437`). That run never reached
`kernel_main`. The board was later seen in Maskrom after a mainline U-Boot
flash; boot unconfirmed.

### Tier E — not started

No code exists for any of these. Verified by search, not by omission:

- Device Graph and driver model
- Storage, filesystem, block layer
- Networking stack
- Service framework, IPC
- Recovery manager, persistent identity
- SMP, PSCI, secondary CPUs (`vivanta-boot/arch-armv7a/` is an 8-line stub;
  `target-qemu-armv7a` was deleted 2026-09-25)
- AArch32 userspace — `lower_aarch32_sync` halts the CPU
  (`vivanta-boot/arch-aarch64/src/vectors.rs:262`)
- A second userspace program. `vivanta-boot/user-init/` (111 lines) is the only
  one, and it is 100% `global_asm!` — no Rust, no stack
- EL0 FP/SIMD context save (`d8-d15`, FPSR, FPCR are not preserved anywhere)

### Exception model, stated precisely

Of the 16 AArch64 exception vectors
(`vivanta-boot/arch-aarch64/src/vectors.rs:253-269`): **6 return, 10 halt.**

**Return (6):**

| Vector | Line | Behaviour |
|--------|------|-----------|
| `el1t_irq` | `:266` | `save_and_aret` |
| `el1h_irq` | `:267` | `save_and_aret` |
| `lower_aarch64_irq` | `:268` | `save_and_aret` |
| `lower_aarch32_irq` | `:269` | `save_and_aret` — unreachable in practice, because a faulting AArch32 EL0 task takes `lower_aarch32_sync` first, which halts |
| `el1h_sync` | `:256` | `save_and_eret_sync_el1` — the only resumable EL1 fault class (demand-fill + COW resolution, ADR-032) |
| `lower_aarch64_sync` | `:259` | `save_and_eret_sync` — dispatches SVC and EL0 faults |

**Halt (10):** `el1t_sync`, `el1t_fiq`, `el1t_serror`, `el1h_fiq`,
`el1h_serror`, `lower_aarch64_fiq`, `lower_aarch64_serror`,
`lower_aarch32_sync`, `lower_aarch32_fiq`, `lower_aarch32_serror`.

Consequence: **no AArch32 userspace** — `lower_aarch32_sync` halts the CPU
rather than dispatching.

**IRQ masking is correct — earlier finding disproven.** `vivanta-boot/arch-aarch64/src/interrupts.rs:13-28`
uses `DAIFSet/DAIFClr #2`, and `#2` is the **IRQ** mask (I): the `imm4` field
order is bit3=D, bit2=A (SError), bit1=I (IRQ), bit0=F, and `PSTATE.I` (bit 7) is
set/cleared this way. `#4` would set the SError mask (A). Verified on QEMU by
reading DAIF after each instruction. The earlier claim that `#2` was SError, that
`PSTATE.I` was never manipulated and that a GIC delivery defect blocked
preemption was **wrong**. The real open defect is the scheduler race; see
`docs/audit/2026-09-24-global-audit.md`.

## 3. What the previous version got wrong

Kept as a record, because "it was wrong" is only useful if the error is named.

| Previous claim | Reality |
|----------------|---------|
| "~35-40% done" | Not measurable; retired. See §1 |
| "Do not build a real heap allocator — `StubAllocator` is ugly" | `StubAllocator` does not exist anywhere in the tree except in two documents. Replaced by the MRM-backed free-list heap at `vivanta-boot/kernel/src/memory/kernel_heap.rs` |
| Remaining: "MemoryObject smoke test + L2 block splitting" | Shipped. L2→L3 split is `split_l2_block` (`vivanta-boot/arch-aarch64/src/paging/walker.rs:102-124`); MRM churn/verification is `vivanta-boot/kernel/src/verify.rs` |
| Remaining: "Isolated EL0 runtime environment" | Shipped. ROADMAP lists M6.0 CLOSED and M10 gates present; the gate runs two concurrent ELF tasks in separate address spaces |
| Remaining: "Kernel Task model (first real process)" | Shipped. `vivanta-boot/kernel/src/scheduler/task.rs`,
`vivanta-boot/kernel/src/scheduler/task_manager.rs`,
`vivanta-boot/kernel/src/scheduler/process_table.rs` and
`vivanta-boot/kernel/src/syscall/process.rs` |
| "V2/M5 Phase A–E complete" as current baseline | Superseded. Active milestones are M10 validation on declared hardware and M11+ behind a scope fence (`ROADMAP.md`) |
| "The heap should be an MRM consumer, not an MRM bypass" | Still the right principle — and it is now the implemented design |

The document's architectural argument (§3 lineage, §5 development order, §6
"every resource must have an owner, a lifetime, and an authority boundary",
§7 V3–V7) is unaffected by the staleness and is retained below.

## 4. Architectural lineage

Measured by architectural philosophy, not code volume:

```
Minix 3        ───┐
                   │
seL4           ───┼── Vivanta direction
                   │
QNX            ───┘

Linux          ─── separate lineage
```

Why:
- **MemoryObject** ≈ microkernel object model (resources, not functions)
- **Capability layer** (`MemoryCapability`) ≈ seL4/QNX capability derivation
- **Device Graph** → explicit resource topology, not a flat device list
- **SystemState** → controlled kernel context, not global mutable state

## 5. What still is not worth doing now

### SMP

Tempting but wrong. SMP requires per-CPU data, locking, IPI, TLB shootdown and
scheduler migration. There is no PSCI code in the repository, no secondary-CPU
path, and no storage or device discovery. SMP provides zero user value at this
stage, and the single-core design is what makes the current gate reproducible
at all.

### Shell / filesystem / drivers

These come after the runtime foundation, and the runtime foundation is now
Tier A. The binding constraint has moved: what is missing is not the ability to
create an isolated EL0 runtime, but **a way to run more than one program** —
`execve` resolves a single compile-time table
(`vivanta-boot/kernel/src/syscall/process.rs:544-545`) and ignores `argv` and
`envp` (`:582-583`). A filesystem is a precondition for expanding that table,
not a follow-on nicety.

### A second percentage

Do not reintroduce one. If a status number is needed, derive it from the tiers
above, which are each checkable against a file.

## 6. Architectural principle

Vivanta operates under a unified principle:

> **Every resource must have an owner, a lifetime, and an authority boundary.**

| Domain    | Object         | Owner   | Rights         | Lifetime         |
|-----------|----------------|---------|----------------|------------------|
| Memory    | MemoryObject   | Task/MRM| MemRights      | allocate→revoke  |
| Device    | DeviceObject   | System  | Capability     | discover→release |
| Task      | TaskObject     | System  | Resources      | spawn→exit       |
| Storage   | StorageObject  | Task    | Identity       | persist→recover  |
| Identity  | IdentityState  | System  | Root Keypair   | boot→generations |

If a new feature cannot answer "who owns this, how long does it live, who can
access it?", it does not belong in the kernel.

## 7. What the next milestone has to clear

Per `ROADMAP.md`, the current goal is to **validate M10 on the declared
hardware target**, with new subsystems held behind the scope fence. Concretely,
in dependency order:

1. **Close the scheduler race** — a tick can land inside a context switch. IRQ
   masking is already correct (`DAIFSet/Clr #2` sets/clears the IRQ mask, I), so
   the earlier claim of a GIC delivery defect behind it is retired. Until the
   race is closed the CI gate stays advisory. Diagnosis:
   `docs/audit/2026-09-24-global-audit.md`.
2. **Fix the EL2 FP/SIMD init** on the five targets whose EL2 path writes
   `0b11 << 20` to `CPTR_EL2` — an architectural no-op, since `TFP` is bit 10 and
   bits 21:20 are RES0 — and whose `msr CPACR_EL1` sits behind a label the EL2
   branch jumps past, leaving `CPACR_EL1.FPEN` at its reset `0b00` and trapping
   FP/SIMD to EL1. Both halves are needed: `msr CPTR_EL2, xzr` **and** a
   `msr CPACR_EL1, #(0b11<<20)` on every entry path. See
   `docs/architecture/arm-exception-state.md`.
3. **Raise the boot stack** to 256 KiB for x96q and suma-q5.
4. **Fix `platform-amlogic`** so suma-q5 can print at all.
5. **Correct COW ownership accounting** — at
   `vivanta-boot/kernel/src/vmm/address_space.rs:617-619` the `Backing::Present`
   fork branch overwrites `parent_piece.phys` with
   `PhysOwnership::CoWShared { refcount: 2 }` unconditionally, including when the
   prior value was `PhysOwnership::External`, whose contract is "VMM never frees
   it" (`vivanta-boot/kernel/src/vmm/mapping.rs:40`). The last owner to unmap
   therefore receives a frame the VMM never owned. Separately, the nested-fork
   branch at `:645-648` falls back to `refcount = 2` for any `PhysOwnership`
   that is not already `CoWShared`, discarding the true owner count.
6. **Make `waitpid` release** the child's address space, mappings and thread.
   It currently does not, and gate M10.4 papers over this by performing the
   teardown by hand.

Once those clear, the honest next feature is a **real filesystem-backed
`execve`**, which is what unblocks more than one userspace program and turns
Tier A into something an OS user would recognise.

## 8. Summary

Vivanta has a working kernel runtime on one configuration: QEMU, under an
advisory gate. The foundation subsystems — boot, MMU, PMM, VMM, GIC, timer,
scheduler, EL0 entry, syscalls, process model — are real and gate-asserted.

What is not real: hardware. No board has run `kernel_main` from this tree.
RK3568 has run an EL2 diagnostic; the RPi3B+ UART evidence came from booting
BalanSir's Buildroot, not Vivanta; the board-info for lavender, whyred and
AIBOX-1684X is genuine research with excellent methodology and no Vivanta code
attached.

The transition from *kernel foundation* to *operating system* is not
"another subsystem done". It is the moment when more than one program can run,
and when at least one board that is not QEMU executes `kernel_main` and
survives a timer interrupt.
