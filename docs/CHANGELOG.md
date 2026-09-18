# Changelog

## 2026-07-11

### R2 Repository Reorganization

Added:
- `docs/architecture/repository-layout.md` — target directory structure
- `.gitignore` — exclude `target/`, `*.bin`, `.DS_Store`
- `docs/hardware/lavender/` — initial hardware notes for SDM660 target
- `docs/adr/ADR-001-rfc-freeze.md` — placeholder
- `docs/adr/ADR-002-state-versioning.md` — placeholder
- `specs/schemas/` — placeholder for state/environment schemas
- `archive/README.md` — policy for archived documents

Changed (git mv):
- Milestone documents moved under `docs/milestones/`
- RFCs moved under `specs/rfc/`
- ADRs moved under `docs/adr/`
- R0/R1 reviews archived under `archive/milestones/pre-r2/`
- `Goals/`, `MindMap/`, `research/`, `OPEN_QUESTIONS.md` archived
- `theseus-m1/` archived under `archive/experiments/m1/`

## 2026-07-14

### ACS — Architecture Cleanup Sprint

- Repository restructuring: boot/ → archive/boot_legacy/, kernel/src/memory/ → kernel-memory-frozen/
- Arch-api: extern "Rust" bidirectional contract layer
- Kernel/arch/platform/target split with strict dependency direction
- target-test: build-time arch independence proof
- ADR-011 through ADR-015

## 2026-07-16

### M4 — Execution Foundation (Complete)

- Cooperative round-robin scheduling (3 threads)
- Thread lifecycle: create, exit, trampoline, cleanup, idle
- Timer infrastructure (CNTP, ~79 Hz on QEMU)
- Thread exit and cleanup verified
- Repository layout finalized
- Tag: M4

Next:
- M4.4 Address Spaces — fill AddressSpace, mmap/munmap/mprotect

## 2026-09-18

### CI/CD + repository hygiene

Added:
- `.github/workflows/ci.yml` — lint, host tests, kernel build, QEMU gate matrix,
  and the Raspberry Pi 3B+ SD image on every push/PR.
- `.github/workflows/release.yml` — `workflow_dispatch` release that publishes
  the SD image, `kernel8.img`, the QEMU kernel ELF, a flat RK3568 binary and the
  EL0 image with checksums.
- `vivanta-boot/tools/build-user-init.sh` — builds the embedded EL0 image
  (`user-init.elf` is gitignored, so this must run before any kernel build).
- `vivanta-boot/tools/qemu-gates.sh` — boots the kernel under QEMU and verifies
  18 serial-log markers (22/22 gate matrix, 0 panics), locally and in CI.
- `vivanta-boot/tools/ci-local.sh` — one-command local replay of the pipeline.
- `docs/tooling/ci-cd.md` — pipeline, release flow, local ritual, known traps.

Changed:
- `tools/make-rpi3b-image.sh` is now portable: FAT32 via `mkfs.vfat` + `mtools`
  instead of macOS-only `hdiutil`, so the same script builds the image locally
  and on the CI runner.
- `vivanta-boot/rust-toolchain.toml` pins `rustfmt`, `clippy` and `llvm-tools`
  alongside the toolchain and target.
- `docs/hardware/rk3568/` promoted to the canonical U-Boot/eMMC reference
  (README, runbook, scripts, evidence); the old `vivanta-boot/` copy is marked
  superseded.
- README/README.ru/ROADMAP refreshed: live CI/release badges, working quick
  start, status aligned with `STATUS.md`.
