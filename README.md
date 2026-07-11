# Theseus OS

Experimental operating system exploring identity continuity, resource-oriented memory, and portable boot architecture.

## Repository structure

See `docs/architecture/repository-layout.md` for directory organization and naming conventions.

## Quick start

```bash
cd theseus-boot
cargo build -p boot-aarch64-qemu-kernel --target aarch64-unknown-none
cargo build -p boot-aarch32-qemu-virt --target armv7a-none-eabi
```

## Status

See `PROJECT_STATE.md` and `docs/milestones/`.