# Vivanta Roadmap

The authoritative engineering roadmap is
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
| M5 | Memory Resource Manager (ADR-025) | in progress |
| M6+ | Userspace services, IPC, drivers, networking | planned |

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

| Platform | Status |
|----------|--------|
| qemu-aarch64 | Active — boots to `kernel_main`, user-space EL0 works |
| rk3568 | Active (bring-up) |
| rpi3b+ | Active |
| qemu-armv7a | Active |
| allwinner-h616 / amlogic / sdm660 | Stalled / planned |

## Long-term vision

Vivanta aims to become a distributed operating system where execution state and resources are represented as architecture-agnostic objects. This enables:
- **Identity & State Continuity** — process state and memory objects can be migrated across heterogeneous hardware (e.g., PC to mobile).
- **Resource-Oriented Memory** — strict separation of mechanism (kernel capabilities) and policy (orchestration).
- **Heterogeneous Compute Clustering** — utilizing budget smartphone SoCs as secure, coordinated cluster nodes.

See the [network services vision](vivanta-boot/docs/rfc/network-services-vision.md) and [cluster research](docs/research/cluster_research.md) for architectural explorations.
