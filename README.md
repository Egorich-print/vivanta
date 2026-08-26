**[English](README.md)** · [Русский](README.ru.md)

---

# Vivanta

![Status: experimental](https://img.shields.io/badge/status-experimental-red)
![License: GPLv3](https://img.shields.io/badge/license-GPLv3-blue)
![Language: Rust](https://img.shields.io/badge/language-Rust-orange)
![Platform: ARM64 / ARMv7](https://img.shields.io/badge/platform-ARM64%20%2F%20ARMv7-lightgrey)

An experimental operating system exploring **identity continuity**, **resource-oriented
memory**, and a **portable boot architecture**.

Vivanta is designed from the start to run on heterogeneous hardware — ARM64 and
ARMv7 systems, from emulated QEMU machines to real boards and old smartphones
(RK3568, Raspberry Pi 3B, Qualcomm phone SoCs).

## What works today

| Area | State |
|------|-------|
| Kernel boot (QEMU AArch64) | ✅ |
| Physical memory manager (PMM, full usable RAM) | ✅ |
| Paging / VMM (address spaces, map/unmap) | ✅ |
| Memory Resource Manager (MRM) with reclamation | ✅ |
| Scheduler (priority, preemptive, sleep/wake) | ✅ |
| Process model (tasks, threads, process table) | ✅ |
| Syscalls (`read`, `write`, `exit`, `yield`, `mmap`, `munmap`, `mprotect`) | ✅ |
| First user-space program in EL0 | ✅ |
| User memory boundary (`access_ok`, copy, `-EFAULT`) | ✅ |
| EL0 fault containment | ✅ |
| Timer-driven preemption (100 Hz, two live threads) | ✅ |
| **Copy-on-Write for anonymous private memory** | ✅ **NEW** |
| Demand paging with lazy allocation | ✅ |
| `mmap` / `munmap` / `mprotect` syscalls | ✅ |
| ELF64 AArch64 loader | ✅ |
| First genuine ELF userland program in EL0 | ✅ |
| ELF64 AArch64 parser + kernel loader | ✅ |
| First genuine ELF64 userland program | ✅ |
| Process model (tasks, threads, process table) | ✅ |
| Generation-protected handles + deterministic capacity | ✅ |
| COW for anonymous private memory | ✅ **NEW** |

> **M9 COW COMPLETE** — Copy-on-Write for anonymous private memory is implemented and verified.
> First genuine ELF64 AArch64 userland program runs in EL0 via syscall ABI,
 *demand-fills on first access*, changes protection via `mprotect`, releases via `munmap`,
 *exercises COW fork semantics*, and exits cleanly. **All 9 QEMU gates pass.**

> **M9 COW COMPLETE** — Copy-on-Write for anonymous private memory is implemented and verified.
> First genuine ELF64 AArch64 userland program runs in EL0 via syscall ABI,
 *demand-fills on first access*, changes protection via `mprotect`, releases via `munmap`,
 *exercises COW fork semantics*, and exits cleanly. **All 9 QEMU gates pass.**

> **M9 COW COMPLETE** — Copy-on-Write for anonymous private memory is implemented and verified.
> First genuine ELF64 AArch64 userland program runs in EL0 via syscall ABI,
 *demand-fills on first access*, changes protection via `mprotect`, releases via `munmap`,
 *exercises COW fork semantics*, and exits cleanly. **All 9 QEMU gates pass.**

> **G-M7 CLOSED** — Syscall ABI + VM syscalls + Process model **COMPLETE**
> **M9 COW COMPLETE** — Copy-on-Write for anonymous private memory **COMPLETE**

> **M7 GREEN BASELINE — PASS** (2026-08-11). QEMU-correct baseline: all four gates verified on a clean clone.
> Honest status is "QEMU-correct", not "hardware-correct" — one deferred ARM MMU descriptor-encoding issue requires validation on physical hardware.

---

## Quick start (QEMU AArch64)

Prerequisites: Rust 1.98.0 stable (pinned via `vivanta-boot/rust-toolchain.toml`,
rustup installs it automatically) and QEMU.

```bash
rustup target add aarch64-unknown-none   # one-time

cd vivanta-boot
cargo build -p vivanta-target-qemu-aarch64

qemu-system-aarch64 -M virt -cpu cortex-a53 -m 512M -nographic \
  -kernel target/aarch64-unknown-none/debug/vivanta-target-qemu-aarch64 \
  -serial mon:stdio
```

At the end of the boot log, the first user-space program runs in EL0, prints via
the `write` syscall and exits cleanly:

```
Hello, Vivanta!
syscall: exit(0)
```

## Repository layout

The kernel source is a Cargo workspace of small `vivanta-*` crates in
[`vivanta-boot/`](vivanta-boot/):

```
vivanta-boot/
  arch-aarch64/    AArch64 support (MMU, exceptions, EL0 entry)
  arch-armv7a/     ARMv7 support (frozen, WIP)
  arch-api/        Architecture API contracts
  kernel/          Scheduler, syscalls, boot flow
  boot-info/       BootInfo contract passed by the bootloader
  boot_common/     Platform-shared helpers
  platform-*/      Board support crates (qemu, rk3568, rpi3b, sdm660, …)
  target-*/        Bootable binaries (qemu-aarch64, rk3568, rpi3b, sdm660, …)
  tools/           Reliability/soak test scripts
```

Architecture design documents (ADRs, RFCs, milestone checklists) live in
[`vivanta-boot/docs/`](vivanta-boot/docs/); project history and organizational
notes are in [`docs/`](docs/).

## Documentation

- [Master roadmap](docs/architecture/master-roadmap.md) — engineering plan
- [M5.0 GREEN BASELINE](vivanta-boot/docs/milestones/M5.0-green-baseline.md) — ratified recovery baseline (source of truth)
- [Architecture decision records](vivanta-boot/docs/adr/) — ADR-011 … ADR-034
- [Vision: network services & distributed OS](vivanta-boot/docs/rfc/network-services-vision.md)
- [Cluster research: budget smartphones as compute nodes](docs/research/cluster_research.md)

## Roadmap

Short version in [ROADMAP.md](ROADMAP.md). M5.0 (recovery baseline) is
**PASS/CLOSED**. Next milestone (M6) is being defined from actual repository
state, not the pre-M5 roadmap. See
[`vivanta-boot/docs/milestones/`](vivanta-boot/docs/milestones/).

## License

[GPLv3](LICENSE). Copyright (C) 2026 Egor Korostelev.