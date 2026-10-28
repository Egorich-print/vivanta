# Platform Validation Matrix

**Last verified: 2026-09-29** against HEAD `273f9bc`, by reading each
`target-*/Cargo.toml`, each `target-*/src/main.rs` and
`tools/qemu-gates.sh`. This matrix previously listed three boards and marked
four shipped subsystems as "not yet implemented".

## The headline fact

**Vivanta has never run a kernel to userspace on silicon.**

- QEMU `virt` is the only configuration that reaches `kernel_main` with EL0
  userland. It is gated by an **advisory** CI job (`continue-on-error: true`).
- `target-rpi3b-plus` and `target-x96q` link `vivanta-kernel` and call
  `kernel_main`. **Neither has been run.**
- `target-suma-q5` links the kernel but panics before reaching it —
  `platform-amlogic` never calls `set_console`, so the first `println!` hits
  `with_console()`'s `.expect("console not initialized")`.
- `target-lavender` prints 5 lines and spins; its platform dependencies are
  declared but never used.
- `target-rk3568` **does not link `vivanta-kernel` at all** — it is an EL2
  diagnostic adapter.

## Target summary

| Target | SoC / board | Links kernel? | Reaches `kernel_main`? | Verified on silicon? |
|---|---|:--:|:--:|---|
| `target-qemu-aarch64` | QEMU virt, Cortex-A53 | ✅ | ✅ | **NO** — QEMU only, advisory gate |
| `target-rk3568` | RK3568 NVR | ❌ **no `vivanta-kernel` dependency** | ❌ (EL2 diagnostic: UART + FDT report + `test_brk`) | **YES** — Stage 1–2 diagnostic UART output, July 2026 |
| `target-rpi3b-plus` | BCM2837B0 Pi 3B+ | ✅ (`Cargo.toml:17`) | ✅ (`src/main.rs:132`) | **NO** — wired, never run |
| `target-x96q` | Allwinner H313 (X96Q) | ✅ | ✅ (`src/main.rs:181`) | **NO** — "compile-checked only, needs silicon" |
| `target-suma-q5` | Amlogic S905L (Sumavision Q5) | ✅ | call present (`src/main.rs:175`) but **panics first** | **NO** |
| `target-lavender` | SDM660 (Redmi Note 7) | deps declared, **unused** | ❌ — 5 `println!` then spin forever | **NO** — Vivanta never ran |

## The one silicon data point

**RK3568, July 2026** — a Vivanta **diagnostic** binary produced real UART
output on the board. Decoded string:
`KC3FGZAB1CONSOLE OK231234WITH CONSOLE OK54X`.

What that does and does not prove:

- ✅ Rust executes at EL2, the NS16550 driver works at the real baud, BSS
  zeroing and stack setup work, the FDT parses.
- ❌ It never reached `kernel_main`: the bring-up log leaves **MMU enable 🔲**
  and **Scheduler 🔲**, and `target-rk3568` has no dependency on
  `vivanta-kernel`.
- ⚠️ The board was later observed in **Maskrom** after a mainline U-Boot flash;
  boot is unconfirmed. See
  [docs/hardware/rk3568/README.md](../../../docs/hardware/rk3568/README.md) and
  [board-info.md](../../../docs/hardware/rk3568/board-info.md).

## Hardware evidence tiers

Stated plainly so the tiers never blur:

1. **RK3568** — real UART output from a Vivanta binary on silicon (above).
   Never `kernel_main`. Board now in Maskrom.
2. **lavender (SDM660) / whyred (SDM636)** — thoroughly surveyed with real
   evidence (`/proc/device-tree`, XBL UART capture, pmOS boots,
   `fastboot getvar all`). **Vivanta itself never ran on either.**
3. **AIBOX-1684X (BM1684X)** — 912-line raw boot log with an
   `[OBSERVED]/[INFERRED]/[UNAVAILABLE]` evidence ledger. **Research only — no
   code, not on the roadmap.**
4. **RPi3B+** — the working UART configuration came from booting **another
   project's Buildroot image**, not Vivanta. The mandatory
   `dtoverlay=miniuart-bt` finding (the Pi 3 routes PL011 to Bluetooth by
   default) is a real hardware fact and is now encoded in
   `tools/make-rpi3b-image.sh`.
5. **QEMU** — the only genuinely working configuration, advisory-gated.

## Subsystem matrix

`✅` verified working in that tier · `⬜` not yet exercised in that tier ·
`—` not applicable (the target does not run a kernel)

| Feature | QEMU (virt) | Pi 3B+ | X96Q | Sumavision Q5 | RK3568 (EL2 diag) | lavender |
|---|:--:|:--:|:--:|:--:|:--:|:--:|
| UART | ✅ | ⬜ | ⬜ | ⬜ | ✅ | ⬜ |
| FDT parsing | ✅ | ⬜ | ⬜ | ⬜ | ✅ | ⬜ |
| Memory map | ✅ | ⬜ | ⬜ | ⬜ | ✅ (FDT report) | ⬜ |
| PMM (bitmap) | ✅ | ⬜ | ⬜ | ⬜ | — | — |
| MMU (AArch64 4-level) | ✅ | ⬜ | ⬜ | ⬜ | — | — |
| L1/L2 descriptor `0b10` (spec) | ❌ hangs | ⬜ | ⬜ | ⬜ | — | — |
| Exception vectors | ✅ | ⬜ | ⬜ | ⬜ | — | — |
| Crash dump | ✅ | ⬜ | ⬜ | ⬜ | — | — |
| GIC discovery | ✅ | ⬜ | ⬜ | ⬜ | — | — |
| GIC init (v2/v3) | ✅ | ⬜ | ⬜ | ⬜ | — | — |
| SGI self-test | ✅ | ⬜ | ⬜ | ⬜ | — | — |
| Barrier API | ✅ | ⬜ | ⬜ | ⬜ | — | — |
| MMIO API | ✅ | ⬜ | ⬜ | ⬜ | — | — |
| IrqGuard | ⚠️ see below | ⬜ | ⬜ | ⬜ | — | — |
| SpinLock | ✅ | ⬜ | ⬜ | ⬜ | — | — |
| Generic Timer | ✅ | ⬜ | ⬜ | ⬜ | — | — |
| Context switch (coop) | ✅ | ⬜ | ⬜ | ⬜ | — | — |
| Thread lifecycle | ✅ | ⬜ | ⬜ | ⬜ | — | — |
| Thread exit + cleanup | ✅ | ⬜ | ⬜ | ⬜ | — | — |
| Arch API (extern Rust) | ✅ | ⬜ | ⬜ | ⬜ | — | — |
| BootInfo contract | ✅ | ⬜ | ⬜ | ⬜ | — | — |
| MmioRegion purged | ✅ | ⬜ | ⬜ | ⬜ | — | — |
| **VMM** | ✅ | ⬜ | ⬜ | ⬜ | — | — |
| **Userspace (EL0)** | ✅ | ⬜ | ⬜ | ⬜ | — | — |
| **Syscalls** | ✅ 13/16 live | ⬜ | ⬜ | ⬜ | — | — |
| **ELF loader** | ✅ | ⬜ | ⬜ | ⬜ | — | — |
| Scheduler (preemptive) | ❌ flaky | ⬜ | ⬜ | ⬜ | — | — |
| RK3568 `test_brk` | — | — | — | — | ✅ | — |

**Legend**

- ✅ — verified working at this tier
- ❌ — known broken or unreliable at this tier (documented limitation)
- ⬜ — code written, **never run** at this tier
- — — not applicable: the target does not link the kernel

### Notes on the ✅ cells

- **VMM / Userspace / Syscalls / ELF loader are all implemented and gated.**
  The previous version of this matrix marked all four "not yet implemented".
  Evidence: `tools/qemu-gates.sh` requires markers `[ELF] genuine ELF64
  userland PASS (exit=42)`, `[SYS] mmap/store/mprotect/munmap/negatives PASS`,
  `[COW] break+isolation PASS`, `[FORK-EL0] fork/waitpid round-trip exit(42)
  PASS`, `[EXECVE] EL0 execve("/init") -> exit(42) PASS`.
- **Syscalls: 13 live of 16 numbers** — `read(0)`, `rt_sigprocmask(14)` and
  `rt_sigreturn(15)` return `-ENOSYS` by design (ADR-033, scope fence).
- **IrqGuard** is ✅: `arch-aarch64/src/interrupts.rs` writes
  `DAIFSet/DAIFClr #2`, and `#2` is the **IRQ** mask (I) — the `imm4` field
  order is bit3=D, bit2=A (SError), bit1=I (IRQ), bit0=F, and `PSTATE.I` (bit 7)
  is set/cleared this way. `#4` would set the SError mask (A). (The earlier
  ⚠️ finding — "`#2` masks SError, `PSTATE.I` never manipulated, `#4` stalls GIC
  timer delivery" — was **disproven** on 2026-09-29; the `#4` experiment failed
  because it never cleared the IRQ mask. There is no GIC delivery defect.)
- **Scheduler (preemptive)** is ❌ flaky, not ❌ absent: cooperative `yield_now()`
  is solid and gated, but a timer tick can land inside a context switch and lose
  a user thread's exit code or leave a forked child unscheduled. The QEMU gate
  job is `continue-on-error: true` for exactly this reason.
- **L1/L2 descriptor `0b10`** is ❌ on QEMU: QEMU hangs at MMU enable with the
  spec-correct encoding, so the default stays `0b11` and the `spec-table-desc`
  feature exists for silicon. **Never `cargo build --workspace`** — feature
  unification would flip the QEMU build too.

## QEMU invocation

```sh
cd vivanta-boot
./tools/build-user-init.sh                 # gitignored ELF the kernel embeds
cargo build -p vivanta-target-qemu-aarch64
./tools/qemu-gates.sh target/aarch64-unknown-none/debug/vivanta-target-qemu-aarch64
```

Raw boot, for reference:

```sh
qemu-system-aarch64 \
    -M virt \
    -m 512M \
    -cpu max \
    -kernel target/aarch64-unknown-none/debug/vivanta-target-qemu-aarch64 \
    -nographic \
    -serial mon:stdio
```

The gate requires **18 serial markers** and fails on `PANIC|EXCEPTION|G4 FAIL`,
with `[G4] running_count=1` as the terminal marker. It is **advisory**: the CI
job running it is `continue-on-error: true`, and `main` has no branch
protection, so a green run has never blocked a merge.
