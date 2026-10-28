**[English](README.md)** · [Русский](README.ru.md)

---

# Vivanta

[![CI](https://github.com/Egorich-print/vivanta/actions/workflows/ci.yml/badge.svg)](https://github.com/Egorich-print/vivanta/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/Egorich-print/vivanta?include_prereleases&label=release&color=blue)](https://github.com/Egorich-print/vivanta/releases)
![License: GPLv3](https://img.shields.io/badge/license-GPLv3-blue)
![Rust 1.98.0](https://img.shields.io/badge/rust-1.98.0-orange)
![Platform: AArch64](https://img.shields.io/badge/platform-AArch64-lightgrey)

An experimental operating system exploring **identity continuity**,
**resource-oriented memory**, and a **portable boot architecture**.

Vivanta is a `no_std` Rust kernel, bare-metal, built for **AArch64** only. It
targets QEMU `virt` and several real boards (RK3568, Raspberry Pi 3B+,
Allwinner H313, SDM660, Amlogic S905L); the hardware targets are early —
see the tiered table below. `vivanta-boot/arch-armv7a` is an 8-line frozen stub
kept for reference (ADR-011) and contains no ARMv7 code.

## Status

**QEMU-correct, not hardware-correct.** The AArch64 kernel boots to
`kernel_main` on QEMU `virt`, runs the boot gate matrix — **18 required serial
markers** (`tools/qemu-gates.sh`), no panics — and exercises real EL0 userland
through the syscall ABI. That gate is verified on local runs; in CI the job is
**advisory** because of an open scheduler race (see [CI](#continuous-integration)).
There is no hardware validation of the kernel itself.

| Area | State |
|------|-------|
| Boot to `kernel_main` (QEMU AArch64) | ✅ |
| Physical memory manager (PMM over all usable RAM) | ✅ |
| Virtual memory: address spaces, map/unmap/protect, table ownership + reclamation | ✅ |
| Demand paging (lazy anonymous), one resumable EL1 fault class (ADR-032) | ✅ |
| Copy-on-Write for anonymous private memory | ✅ |
| Kernel heap (free-list with coalescing) | ✅ |
| Scheduler: priority ladder, 100 Hz timer, cooperative `yield` is the primary path | ✅ |
| Process model: tasks, threads, process table, lifecycle | ✅ |
| ELF64 AArch64 load planner — magic/class/machine, W^X, overlap and bounds checks | ✅ |
| User memory boundary (`access_ok`, `-EFAULT`) and EL0 fault containment | ✅ |
| Syscalls live (13): `write`, `exit`, `yield`, `mmap`, `munmap`, `mprotect`, `fork`, `waitpid`, `kill`, `getpid`, `getppid`, `execve`, `rt_sigaction` | ✅ |
| Syscalls reserved (3): `read`, `rt_sigprocmask`, `rt_sigreturn` → `-ENOSYS` | — |
| EL0 userland: write, `execve`, `fork`/`waitpid` round-trip, fault kill | ✅ |
| Signals | ⚠️ dispositions only — see below |
| Identity | ⚠️ nominal only — a counter, not a keypair |

`execve` resolves **only** the built-in `/init` image (a compile-time
`include_bytes!` table). There is no filesystem, and `argv`/`envp` are ignored.

### What is NOT implemented

No filesystem · `execve` serves only the built-in `/init` ELF and ignores
`argv`/`envp` · signals are **dispositions only** — `pending` is write-only, there
is no asynchronous EL0 delivery, and only `SIGKILL` acts (`rt_sigreturn` is
`-ENOSYS`) · `waitpid` does not release the child's address space, mappings or
thread · no SMP, no PSCI, no secondary CPUs · no drivers, no storage, no
networking, no IPC · no crypto identity (the UUID is filled from a monotonic
counter) · no EL0 FP/SIMD context save · TTBR1/ASID not implemented.

No `todo!()` and no `unimplemented!()` anywhere in the tree. The gaps live in
prose, in `expect()` calls, and in
[`docs/audit/2026-09-24-global-audit.md`](docs/audit/2026-09-24-global-audit.md).

## Hardware status, tiered

| Tier | Target | What is true |
|---|---|---|
| Works | `target-qemu-aarch64` | Boots to `kernel_main`, gate matrix green locally. Advisory-gated in CI. |
| Never run on silicon | `target-rpi3b-plus` | Links the real kernel and calls `kernel_main`. The working RPi UART capture came from booting **BalanSir's** Buildroot image, not Vivanta. |
| Never run on silicon | `target-x96q` (Allwinner H313) | Compiles and reaches the `kernel_main` call. Compile-checked only. |
| Diagnostic only | `target-rk3568` | **Does not link the kernel.** The only board where a Vivanta binary produced real UART output on silicon (Stage-1/2 diagnostic, July 2026). The board has since been observed in **Maskrom**; boot unconfirmed. |
| Cannot boot | `target-lavender` (SDM660) | Prints 5 lines and spins forever. Platform deps are declared but unused. |
| Cannot boot | `target-suma-q5` (Amlogic S905L) | `platform-amlogic` never registers a console, so the first `println!` panics. |
| Research only | AIBOX-1684X (BM1684X) | Reconnaissance with an `[OBSERVED]/[INFERRED]/[UNAVAILABLE]` evidence ledger. No code, not on the roadmap. |

Board-level evidence lives in [`docs/hardware/`](docs/hardware/); the
platform-bringup notes are in
[`vivanta-boot/PLATFORM_BRINGUP.md`](vivanta-boot/PLATFORM_BRINGUP.md).

## Quick start (QEMU AArch64)

Prerequisites: [Rust](https://rustup.rs) 1.98.0 (pinned in
`vivanta-boot/rust-toolchain.toml`) and QEMU.

```bash
git clone https://github.com/Egorich-print/vivanta.git
cd vivanta/vivanta-boot

rustup target add aarch64-unknown-none   # one-time
./tools/build-user-init.sh               # MANDATORY: the kernel include_bytes!s this ELF
cargo build -p vivanta-target-qemu-aarch64

qemu-system-aarch64 -M virt -cpu cortex-a53 -m 512M -nographic \
  -kernel target/aarch64-unknown-none/debug/vivanta-target-qemu-aarch64 \
  -serial mon:stdio
```

`user-init/user-init.elf` is gitignored and embedded into the kernel at compile
time, so a fresh clone **cannot** build `vivanta-kernel` until
`./tools/build-user-init.sh` has run.

> **Do not run `cargo build --workspace`.** Feature unification pulls
> `target-rpi3b-plus`'s `spec-table-desc` (the silicon-correct `0b10` L1/L2 table
> descriptor) into the QEMU build, and the resulting kernel **hangs at MMU
> enable**. QEMU needs the default `0b11`. Build the target explicitly, as above.
> (`.github/workflows/ci.yml:68-73`)

The boot log ends with the built-in EL0 demo thread — a `write` of
`Hello, Vivanta!` followed by a deliberately out-of-bounds write that must return
`-EFAULT`, then `exit(0)`:

```
Hello, Vivanta!
  syscall: exit(0)
```

Automated check (the same gate CI runs):

```bash
./tools/qemu-gates.sh          # 18 markers, PASS/FAIL, non-zero exit on failure
./tools/ci-local.sh            # the whole pipeline: fmt, shell/py syntax, host tests, QEMU
```

## Continuous integration

Pushes to `main` and PRs targeting `main` run
[`.github/workflows/ci.yml`](.github/workflows/ci.yml):

- **lint** — `rustfmt`, shell/python syntax, executable bits;
- **test** — 22 host unit tests (`vivanta-vm`, `vivanta-exec`) natively on x86_64;
- **build** — builds the embedded EL0 image, `cargo check --workspace`, then the
  QEMU kernel standalone (for the reason above);
- **qemu** — boots the artifact on the `ubuntu-24.04-arm` runner and checks the
  18 markers. **`continue-on-error: true` — advisory, not blocking.** An open
  scheduler race (a tick can land inside a context switch; IRQ masking itself is
  correct) costs a lost exit code or an unscheduled forked child on
  roughly 1 run in 2.5 on that runner; long local runs stay green. `main` has no
  branch protection, so a green run never blocked a merge anyway. Diagnosis:
  [`docs/audit/2026-09-24-global-audit.md`](docs/audit/2026-09-24-global-audit.md).
- **image** — builds the flashable Raspberry Pi 3B+ SD image.

Releases are published by manually dispatching
[`.github/workflows/release.yml`](.github/workflows/release.yml) with a tag, to
[Releases](https://github.com/Egorich-print/vivanta/releases): the SD image,
`kernel8.img`, the QEMU kernel ELF, a flat RK3568 **diagnostic adapter** binary
(never a kernel) and the EL0 image. Details and the local ritual:
[`docs/tooling/ci-cd.md`](docs/tooling/ci-cd.md).

## Raspberry Pi 3B+

`tools/make-rpi3b-image.sh` builds a flashable SD-card image (MBR + one 64 MiB
FAT32 boot partition with the firmware, `config.txt`, `kernel8.img`). The kernel
drops EL2→EL1 itself and drives the PL011; `dtoverlay=miniuart-bt` is required or
the writes go to the Bluetooth radio. **Never run on silicon.** The image builds
in CI every push; that is as far as the evidence goes.

## Repository layout

The kernel is a 20-member Cargo workspace of small `vivanta-*` crates in
[`vivanta-boot/`](vivanta-boot/):

```
vivanta-boot/
  arch-aarch64/    AArch64 support (MMU/paging, exceptions, GIC, EL0 entry)
  arch-api/        Architecture API contracts
  arch-armv7a/     8-line frozen stub (ADR-011) — no ARMv7 code
  kernel/          Scheduler, syscalls, VMM (`kernel/src/vmm/`), process model, boot flow
  exec/            Pure ELF64 image parser + load-plan builder (no page tables)
  vm/              Virtual-address range allocator — NOT a VM (the VMM is kernel/src/vmm/)
  boot-info/       BootInfo contract passed by the bootloader
  boot_common/     Platform-shared boot helpers
  platform-*/      Board support crates (qemu, rk3568, rpi3b, sdm660, amlogic, allwinner-h616)
  target-*/        Bootable binaries (qemu-aarch64, rpi3b-plus, rk3568, x96q, lavender, suma-q5)
  user-init/       The one real EL0 program, embedded by the kernel
  tools/           CI, QEMU gate, image-build and soak scripts
```

`user-init/` is a separate workspace built by `tools/build-user-init.sh`.
Architecture documents (ADRs, RFCs, milestone specs) live in
[`vivanta-boot/docs/`](vivanta-boot/docs/); audits, hardware notes and the master
roadmap live in [`docs/`](docs/).

## Documentation

- [STATUS.md](STATUS.md) — verified/unverified status per subsystem (contains
  known stale rows; the table above supersedes it where they disagree)
- [Open correctness backlog](docs/audit/2026-09-24-global-audit.md) — the P0/P1/P2 list this README summarises
- [Architecture decision records](vivanta-boot/docs/adr/) — 18 ADRs, ADR-011 … ADR-035 (016, 024, 025 were never written)
- [CI/CD](docs/tooling/ci-cd.md) — pipelines, release flow, local ritual
- [Platform bring-up notes](vivanta-boot/PLATFORM_BRINGUP.md) — per-board boot state
- [Hardware dossiers](docs/hardware/) — RK3568 board truth, SDM660/whyred surveys, AIBOX-1684X recon
- [ROADMAP.md](ROADMAP.md) and [master roadmap](docs/architecture/master-roadmap.md) — engineering plan
- [Vision: network services & distributed OS](vivanta-boot/docs/rfc/network-services-vision.md)

## License

[GPLv3](LICENSE). Copyright (C) 2026 Egor Korostelev.
