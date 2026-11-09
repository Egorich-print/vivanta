# ADR-036: The portable base supports ARMv7-A as a build target

## Status
Accepted

## Date
2026-11-09

## Context

Vivanta is an AArch64 kernel: `vivanta-boot/arch-armv7a` is a frozen eight-line stub
(ADR-011), and the kernel's exception model, MMU, EL0 userland and ELF loader are
AArch64 by construction.

The **portable** part of the tree is a different matter. Four crates carry no ISA
assumption at all:

| crate | what it is |
|---|---|
| `vivanta-arch-api` | the ISA contract — traits for the MMU, exceptions, interrupts, the PMM, the scheduler, the syscall ABI |
| `vivanta-boot-info` | the boot-time contract between platform and kernel — the memory map, the MMIO regions, the interrupt controller, the DTB |
| `vivanta-boot-common` | the shared boot helpers |
| `vivanta-vm` | the architecture-neutral VA allocator and range arithmetic |

A device family such as the **teXet TB-176FL** (Rockchip **RK2906**, a **Cortex-A8** —
ARMv7-A, `E-002b` in that project's evidence index) needs exactly this part on a
32-bit ARMv7-A core: the boot contract and the ISA contract are what a *platform*
contributes, and the platform is per-board.

Until now that need was a **belief**: the crates were described as
architecture-neutral, and nothing measured it. A single AArch64 assumption in
`arch-api` or `boot-info` — a register width, a page-size constant, an `u64` where
`u32` was meant — would surface only at the end of a port, when it is most expensive
to find.

**Measured on 2026-11-09:** all four crates build for `armv7a-none-eabihf` **unchanged**,
with the pinned 1.98.0 toolchain. The belief was correct; the question is whether it
stays correct.

## Decision

**`armv7a-none-eabihf` is a supported build target of the portable base, and that is a
checked fact rather than a claim.**

* `rust-toolchain.toml` lists `armv7a-none-eabihf` beside `aarch64-unknown-none`, so
  the pinned toolchain installs the target and every developer and CI runner gets it.
* **`tools/armv7-portability.sh`** builds those four crates for the target and fails if
  one of them does not compile. It is wired into `tools/ci-local.sh` and into the
  `build` job of `.github/workflows/ci.yml`, so the check runs on every push.
* A crate joins that list only by an explicit decision: the list *is* the statement
  about which crates must stay architecture-independent.

**What this decision does not do.** It does **not** claim that Vivanta boots on
ARMv7-A. It does not: the kernel, the exception model and the MMU are still AArch64,
and `arch-armv7a` is still a stub. The claim is narrower and precisely stated: *the
portable base compiles for the ISA of a 32-bit ARMv7-A device, unchanged.*

## Consequences

* The portable core can no longer grow an AArch64 assumption **silently** — the CI
  says so on the push that introduces it, not on the port that trips over it.
* The next steps are named and unblocked, but they are **not** done here:
  1. unfreeze `arch-armv7a` and port the ISA contract to ARMv7-A (the exception model
     and the MMU are the hard part — the portable base is the easy part, and this ADR
     is what makes the easy part stay easy);
  2. a `platform-tb176fl` crate that fills the boot contract with the board's own
     measured facts — System RAM `0x60000000`, the EBC window `0x6BE00000`, the
     `win1 buf` window `0x6BF00000`, the EBC register block `0x10114000` at IRQ 47,
     the LCDC at `0x1010C000`, the NAND controller at `0x10500000`, UART1 at
     `0x20060000`;
  3. the E-Ink driver of that project — the panel driver, the waveform codec and the
     compositor — is already `no_std`, dependency-free and hardware-independent, so
     hosting it in Vivanta is packaging, not porting.
* The toolchain pin grows one target. The `aarch64-unknown-none` kernel targets are
  unaffected: nothing in the kernel path is rebuilt for ARMv7-A.

## Links

* ADR-011 (in this repository) — ARMv7 frozen.
* `tools/armv7-portability.sh`, `tools/ci-local.sh`, `.github/workflows/ci.yml`.
* `vivanta-boot/arch-api`, `vivanta-boot/boot-info`, `vivanta-boot/boot_common`,
  `vivanta-boot/vm`.
* texet-tb176fl: `docs/adr/0013-vivanta-as-the-base.md` (the other side of this
  decision), `research/EVIDENCE-INDEX.md` (`E-002b`, `E-181`, `E-210`).
