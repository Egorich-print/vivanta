# Vivanta Status

> Last updated: 2026-08-11

## Current milestone

M5.0 — GREEN BASELINE (recovery milestone, in progress)
Ratified spec: `vivanta-boot/docs/milestones/M5.0-green-baseline.md`

> M5.0 is NOT a feature milestone. It restores the kernel to a provable
> baseline: workspace integrity (G1), physical memory ownership + reclamation
> (G2), user memory boundary + fault containment (G3), scheduler state +
> preemption correctness (G4).

## G1 — Workspace integrity (PASS as of 2026-08-11)

- `cargo build --workspace` — PASS (clean clone)
- `cargo clippy --workspace` — PASS (0 errors)
- `cargo fmt --check` — PASS
- `cargo test --workspace --target aarch64-apple-darwin` — PASS (13 unit tests
  in boot-info / boot_common / kernel-memory-frozen; bare-metal crates are
  `test = false` and QEMU-verified)
- `cargo build -p vivanta-target-qemu-aarch64` — PASS
- QEMU boot + EL0 demo — PASS (`Hello, Vivanta!` → `exit(0)`)

## Kernel

- PMM (Physical Memory Manager) — ⚠️ G2 pending (currently 1 MiB of usable RAM)
- Early MMU (aarch64) — ✅
- Paging API — ✅ (ADR-030: mechanism/policy split)
- Memory Resource Manager — ⚠️ G2 pending (no reclamation)
- Scheduler — ⚠️ G4 pending (index-based current, preemption unproven)
- VMM (AddressSpace) — ⚠️ G2 pending (MappingSet capacity bug, no VA allocator)
- Identity — ⚠️ nominal only (counter-based UUID; no crypto/Ed25519)
- Process Model — ⚠️ lifecycle incomplete (task state never updated on exit)
- Signals — ⚠️ enum only, no delivery path
- Syscalls — ⚠️ G3 pending (access_ok implemented, copy primitives + fault
  containment pending)
- User threads — ✅ EL0 demo works end-to-end

## Known blockers (M5.0)

- G2: full-RAM PMM, MemoryObject Drop→deallocate, contiguous allocation
  contract, MappingSet slot reuse, boot_alloc_frame OOM semantics
- G3: copy_from_user/copy_to_user, `elr += 4` fault masking removal, W^X
- G4: ThreadId-based current, Running invariant, preemption proof on QEMU

## Platforms

| Platform | Status |
|----------|--------|
| qemu-aarch64 | Active, boots to kernel_main, EL0 demo works |
| rk3568 | Diagnostic only (does not link vivanta-kernel) |
| rpi3b+ | Standalone diagnostic (early_mmu identity map) |
| qemu-armv7a | Frozen (arch-armv7a is an empty stub; removed from workspace members) |
| allwinner-h616 / amlogic / sdm660 | Stalled / planned |

## Scope fence (until M5.0 PASS)

IPC · storage · drivers · distributed AI (ADR-031…039) · Ed25519 · BIP-39 ·
persistent identity · TTBR1/ASID · signal delivery · new syscalls · new
hardware targets · new architectures
