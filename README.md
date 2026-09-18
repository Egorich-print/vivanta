**[English](README.md)** · [Русский](README.ru.md)

---

# Vivanta

[![CI](https://github.com/Egorich-print/vivanta/actions/workflows/ci.yml/badge.svg)](https://github.com/Egorich-print/vivanta/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/Egorich-print/vivanta?include_prereleases&label=release&color=blue)](https://github.com/Egorich-print/vivanta/releases)
![License: GPLv3](https://img.shields.io/badge/license-GPLv3-blue)
![Rust 1.98](https://img.shields.io/badge/rust-1.98.0-orange)
![Platform: AArch64](https://img.shields.io/badge/platform-AArch64-lightgrey)

An experimental operating system exploring **identity continuity**,
**resource-oriented memory**, and a **portable boot architecture**.

Vivanta is built bare-metal for heterogeneous hardware — AArch64 and ARMv7
systems, from emulated QEMU machines to real boards and old phones (RK3568,
Raspberry Pi 3B+, Qualcomm SoCs). The kernel is a `no_std` Rust workspace.

## Status

**QEMU-correct, not yet hardware-correct.** On QEMU `virt` the AArch64 kernel
boots to `kernel_main`, runs the full gate matrix (**22/22 PASS, 0 panics**)
and runs genuine EL0 userland. There is no on-silicon validation yet.

| Area | State |
|------|-------|
| Boot (QEMU AArch64) | ✅ |
| Physical memory manager (PMM, all usable RAM) | ✅ |
| Virtual memory (address spaces, map/unmap, table ownership + reclamation) | ✅ |
| Demand paging (lazy anonymous) | ✅ |
| Copy-on-Write for anonymous private memory | ✅ |
| Scheduler (priority, 100 Hz preemptive, sleep/wake) | ✅ |
| Process model (tasks, threads, process table, lifecycle) | ✅ |
| ELF64 AArch64 loader (validated, W^X) | ✅ |
| User memory boundary (`access_ok`, `-EFAULT`) + EL0 fault containment | ✅ |
| Syscalls (`read`, `write`, `exit`, `yield`, `mmap`, `munmap`, `mprotect`, `fork`, `waitpid`, `execve`) | ✅ |
| EL0 userland: hello via `write`, `execve`, `fork`/`waitpid` round-trip | ✅ |
| Signals: `SIGKILL`/`SIGCHLD` on synchronous paths | ✅ |

Scope fence (deliberately not implemented yet): IPC · storage · drivers ·
networking · Ed25519/BIP-39 identity · TTBR1/ASID · asynchronous EL0 signal
delivery. See [STATUS.md](STATUS.md).

## Quick start (QEMU AArch64)

Prerequisites: [Rust](https://rustup.rs) (1.98.0, pinned in
`vivanta-boot/rust-toolchain.toml`) and QEMU.

```bash
git clone https://github.com/Egorich-print/vivanta.git
cd vivanta/vivanta-boot

rustup target add aarch64-unknown-none   # one-time
./tools/build-user-init.sh               # EL0 image; the kernel embeds it
cargo build -p vivanta-target-qemu-aarch64

qemu-system-aarch64 -M virt -cpu cortex-a53 -m 512M -nographic \
  -kernel target/aarch64-unknown-none/debug/vivanta-target-qemu-aarch64 \
  -serial mon:stdio
```

At the end of the boot log the first user-space program runs in EL0 through
the syscall ABI and exits cleanly:

```
Hello, Vivanta!
syscall: exit(0)
```

To check the boot automatically (same gate CI runs):

```bash
./tools/qemu-gates.sh          # 18 markers, PASS/FAIL, non-zero on failure
./tools/ci-local.sh            # the whole pipeline: fmt, lint, host tests, QEMU
```

## Continuous integration

Every push and pull request runs [`.github/workflows/ci.yml`](.github/workflows/ci.yml):

- **lint** — `rustfmt`, shell/python syntax, executable bits;
- **test** — host unit tests (`vivanta-vm`, `vivanta-exec`) natively on x86_64;
- **build** — builds the embedded EL0 image and the QEMU kernel;
- **qemu** — boots the kernel under `cortex-a53` TCG and verifies the gate matrix;
- **image** — builds the flashable Raspberry Pi 3B+ SD image.

Tagged builds are published from
[`.github/workflows/release.yml`](.github/workflows/release.yml) to
[Releases](https://github.com/Egorich-print/vivanta/releases): the SD image,
`kernel8.img`, the QEMU kernel ELF, a flat RK3568 binary and the EL0 image.
Details and the local ritual: [`docs/tooling/ci-cd.md`](docs/tooling/ci-cd.md).

## Running on a Raspberry Pi 3B+

`tools/make-rpi3b-image.sh` builds a flashable SD-card image (MBR + FAT32 boot
partition with the firmware, `config.txt` and `kernel8.img`) for the primary
hardware target. The kernel drops EL2→EL1 itself and drives the PL011 on
GPIO 14/15. Not validated on silicon yet — see
[`vivanta-boot/PLATFORM_BRINGUP.md`](vivanta-boot/PLATFORM_BRINGUP.md).

## Repository layout

The kernel is a Cargo workspace of small `vivanta-*` crates in
[`vivanta-boot/`](vivanta-boot/):

```
vivanta-boot/
  arch-aarch64/    AArch64 support (MMU, exceptions, EL0 entry)
  arch-armv7a/     ARMv7 support (frozen stub)
  arch-api/        Architecture API contracts
  kernel/          Scheduler, syscalls, VMM, boot flow
  boot-info/       BootInfo contract passed by the bootloader
  boot_common/     Platform-shared helpers
  platform-*/      Board support crates (qemu, rk3568, rpi3b, sdm660, …)
  target-*/        Bootable binaries (qemu-aarch64, rpi3b-plus, rk3568, …)
  user-init/       EL0 userland program, embedded by the kernel
  tools/           CI, QEMU gate and image-build scripts
```

Architecture documents (ADRs, RFCs, milestone checklists) live in
[`vivanta-boot/docs/`](vivanta-boot/docs/); project history and the master
roadmap are in [`docs/`](docs/).

## Documentation

- [Master roadmap](docs/architecture/master-roadmap.md) — the engineering plan
- [ROADMAP.md](ROADMAP.md) — short, public-facing milestone summary
- [STATUS.md](STATUS.md) — honest verified/unverified status
- [Architecture decision records](vivanta-boot/docs/adr/) — ADR-011 … ADR-035
- [CI/CD](docs/tooling/ci-cd.md) — pipelines, release flow, local ritual
- [Vision: network services & distributed OS](vivanta-boot/docs/rfc/network-services-vision.md)
- [Cluster research: budget smartphones as compute nodes](docs/research/cluster_research.md)

## License

[GPLv3](LICENSE). Copyright (C) 2026 Egor Korostelev.
