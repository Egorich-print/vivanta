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
