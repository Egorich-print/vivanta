# Vivanta

An experimental operating system exploring identity continuity, resource-oriented
memory, and a portable boot architecture.

Vivanta is designed from the start to run on heterogeneous hardware — ARM64 and
ARMv7 systems, from emulated QEMU machines to real boards and smartphones
(RK3568, Raspberry Pi 3B, and old Qualcomm phone SoCs).

## Status

| Area | State |
|------|-------|
| Kernel boot (QEMU aarch64) | ✅ |
| PMM / MRM / VMM (paging) | ✅ |
| Scheduler (priority, preemptive, sleep/wake) | ✅ |
| Process model (tasks, threads, process table) | ✅ |
| Syscalls (read, write, exit, yield, mmap) | ✅ |
| First user-space program (EL0 hello world) | ✅ milestone M4.5 |

See [STATUS.md](STATUS.md), [docs/OS_MATURITY.md](docs/OS_MATURITY.md), and
[docs/architecture/master-roadmap.md](docs/architecture/master-roadmap.md).

## Repository layout

The boot and kernel source lives in [`vivanta-boot/`](vivanta-boot/), a Cargo
workspace of small `vivanta-*` crates:

```
vivanta-boot/
  arch-aarch64/       AArch64 support (MMU, exceptions, EL0 entry)
  arch-armv7a/        ARMv7 support
  kernel/             Scheduler, syscalls, boot flow
  boot-info/          BootInfo contract passed by the bootloader
  boot_common/        Platform-shared helpers
  user/               Minimal user-space libc and programs
  target-qemu-aarch64/ QEMU virt machine (AArch64)
  target-qemu-armv7a/  QEMU virt machine (ARMv7)
  target-rk3568/      Rockchip RK3568 board
  target-rpi3b/       Raspberry Pi 3B
  ...
```

Higher-level design documentation (ADRs, RFCs, architecture notes) is under
[`vivanta-boot/docs/`](vivanta-boot/docs/). Project history and organization
notes are under [`docs/`](docs/).

## Quick start (QEMU AArch64)

Prerequisites: a Rust toolchain with `aarch64-unknown-none` and
`armv7a-none-eabi` targets installed, and QEMU.

```bash
cd vivanta-boot
cargo build -p vivanta-target-qemu-aarch64 --target aarch64-unknown-none
qemu-system-aarch64 -M virt -cpu cortex-a53 -m 512M -nographic \
  -kernel target/aarch64-unknown-none/debug/vivanta-target-qemu-aarch64 \
  -serial mon:stdio
```

The boot output ends with the first user-space program running in EL0:

```
SHello, Vivanta!
  syscall: exit(0)
```

## Building for other targets

```bash
# ARMv7 QEMU
cargo build -p vivanta-target-qemu-armv7a --target armv7a-none-eabi

# RK3568 board
cargo build -p vivanta-platform-rk3568 --target aarch64-unknown-none
```

## Testing

The workspace includes `arch-test-stub` and `target-test` crates used to run
kernel logic self-tests on the host. See
[`vivanta-boot/tests/`](vivanta-boot/tests/) and the
[milestone checklists](vivanta-boot/docs/architecture/milestones/).

## License

[GPLv3](LICENSE). Copyright (C) 2026 Egor Korostelev.
