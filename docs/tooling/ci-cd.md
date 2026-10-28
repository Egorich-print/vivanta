# CI/CD

GitHub Actions pipelines live in `.github/workflows/`:

```
push/PR → lint (ubuntu-latest) ─┐
        test (ubuntu-latest) ───┤
                                ├→ build (ubuntu-latest) → qemu (ubuntu-24.04-arm,
                                │                         KVM if /dev/kvm is writable,
                                │                         else same-arch TCG)
        └→ image (ubuntu-latest, mkfs.vfat + mtools → RPi3B+ SD image)

workflow_dispatch → release → attach artifacts to a GitHub Release
```

- **lint** — `cargo fmt --check` (workspace + `user-init`), `bash -n` on tracked
  `*.sh`, `py_compile` on tracked `*.py`, and a 755 exec-bit check.
  `runs-on: ubuntu-latest`. **clippy is pinned in `vivanta-boot/rust-toolchain.toml:12`
  but no workflow and no script ever runs it** — `grep -rn clippy .github/ vivanta-boot/tools/`
  returns nothing. The lint gate is fmt + syntax + exec-bits only.
- **test** — `cargo test -p vivanta-vm -p vivanta-exec --target
  x86_64-unknown-linux-gnu` on `ubuntu-latest` (native x86_64, no emulation),
  **22 tests**: 12 in `vm/src/va_tests.rs` + 10 in `exec/src/elf_tests.rs`.
  The workspace `.cargo/config.toml` defaults to `aarch64-unknown-none`, hence
  the explicit `--target`. Six more host-runnable tests exist (3 in
  `boot-info/src/mmap.rs`, 3 in `boot_common/src/memory_discovery.rs`) and CI
  never runs them; `kernel` and `arch-aarch64` set `[lib] test = false` by
  design, because their verification *is* the QEMU gate.
- **build** — `runs-on: ubuntu-latest`, needs `lint`+`test`. Builds the EL0
  `user-init.elf`, `cargo check --workspace`, then the
  QEMU target standalone; uploads the kernel ELF as an artifact.
- **qemu** — `runs-on: ubuntu-24.04-arm`, needs `build`. Boots the artifact and
  checks the serial log with `tools/qemu-gates.sh` (18 required markers, no
  panic). The job picks its accelerator itself: if `/dev/kvm` is writable it
  exports `QEMU_ACCEL=kvm` / `QEMU_CPU=host`, otherwise it falls back to
  `QEMU_ACCEL=tcg` / `QEMU_CPU=cortex-a53` (`.github/workflows/ci.yml:108-120`).
  Either way host and guest are the same architecture, so there is no cross-ISA
  translation. GitHub does not expose `/dev/kvm` on hosted runners today.
  **Advisory (`continue-on-error: true`, `.github/workflows/ci.yml:87-99`)** while
  a known scheduler race is open: long local runs stay green, but the runner hits
  the race often enough to matter (the workflow's own comment *estimates* roughly
  1 run out of 2–5; the measurement below is 9 failures in 21 non-cancelled
  `main` runs, ≈ **1 in 2.3**) — a lost user exit code, or a forked child that
  never gets scheduled. A tick can land inside a context switch.
  (IRQ masking itself is correct: `DAIFSet/Clr #2` sets/clears the IRQ mask, I.
  An earlier claim that `#2` serialised SError and that a GIC IRQ delivery
  defect blocked preemption was **disproven** — see the dated correction in
  `docs/audit/2026-09-24-global-audit.md`.)
  Measurements and the fix order:
  `docs/audit/2026-09-24-global-audit.md`. Restore blocking once the switch
  window is closed.
  (History: an earlier cross-ISA TCG stall on x86_64 runners was a separate
  issue; the AArch64 runner removed that one.)
- **image** — `runs-on: ubuntu-latest`, needs `lint`. Builds the RPi3B+ SD image
  portably (`mkfs.vfat` + `mtools`).
- **release** — `workflow_dispatch` with a tag (default `v0.1.0-alpha`); builds
  the kernel binaries and the SD image, then `gh release create` with
  `permissions: contents: write`.

## Local ritual (run before pushing)

```sh
cd vivanta-boot
tools/ci-local.sh              # fmt + lint + host tests + workspace check + QEMU gates
BUILD_IMAGE=1 tools/ci-local.sh   # also build the SD image (needs mtools + dosfstools)
```

Everything CI does *is* an ordinary host script (`tools/build-user-init.sh`,
`tools/qemu-gates.sh`, `tools/make-rpi3b-image.sh`) — the workflows only call
them, so the same steps run on both sides. But **a local green run does not mean
a CI green run.** The `qemu` job executes on a *different runner*
(`ubuntu-24.04-arm`, same-arch TCG), and that configuration hits the open
scheduler race in roughly one run out of 2.3 (measured below), where a local
macOS host does not reproduce it — the gate is green 100% of the time locally
over long runs, and **why** CI sees it far more often is still unexplained. The
The three failure modes below are timing-dependent: the same ELF, the same
script, the same
18 markers — different interleavings.

## Measured QEMU-gate failure record

From `gh api repos/Egorich-print/vivanta/actions/runs`, over every non-cancelled
`main` run of the `ci` workflow to date (the runner switch was commit `50fecb7`,
2026-09-18; the counting below is per-job, not per-run):

- **21 non-cancelled `main` runs** of `ci.yml`; **9** of them had the `qemu` job
  conclude `failure` — **≈ 1 in 2.3**.
- Note this is a *job* tally, not a run tally: the run-level conclusions differ,
  because `continue-on-error: true` lets a failed `qemu` job sit inside a
  run-level `success`, and because one run failed in `image` with `qemu` green.
- The workflow's own comment at `.github/workflows/ci.yml:87-99` ("roughly 1 run
  out of 2-5") is the repo author's **estimate** of local-vs-CI variance. The
  9/21 figure is the **measurement**.

Three failure modes, read out of the job logs of those 9 runs:

1. **Lost exit code — VM-syscall task** (2 runs: `36082832333`, `36087720860`) —
   `panicked at kernel/src/lib.rs:1202:13: assertion 'left == right' failed:
   VM-syscall program must exit with code 42 (got None)`, `left: None`. The gate
   reported `markers: 1/18` before dying.
2. **Lost exit code — M6 demo task** (2 runs: `35361045739`, `35363760708`) —
   `panicked at kernel/src/lib.rs:1956:13: assertion 'left == right' failed:
   M6 FAIL: demo task exit code != 0`, `left: None, right: Some(0)`. Both reached
   `markers: 17/18` first, so these look far more "successful" than they are.
3. **600 s hang** (5 runs: `35361383631`, `36084754426`, `36085594920`,
   `36086395114`, `36093613699`) — `==> timeout after 600s` with no panic. Three
   stopped at `markers: 0/18`, last serial line
   `G3 fault-containment test: … fault task 2 spawned, yielding to it`; two
   reached `markers: 6/18` and their last serial line is
   `[DUAL] two concurrent ELF tasks exit(42) PASS` — a forked child that never
   got scheduled.

All three are the same underlying scheduler race: a tick lands inside a context
switch and a task's exit code is lost or a
forked child is never scheduled. (IRQ masking is correct; the race is the
switch window, not unmasked interrupts.) Modes 1 and 2 are the *same* bug
hitting two
different assertions — one task's exit code instead of two.

**Four consecutive green runs is not evidence of a fix.** At ~1-in-2.3, four
samples have a ~(0.57)⁴ ≈ **11%** chance of all landing green by luck. The last
four runs (`36094467933`, `36138214686`, `36139486140`, `36140534933`) are green;
that is all that can be said.

Three further reasons a green run proves less than it looks:

- `continue-on-error: true` means the `qemu` job is allowed to fail without
  turning the run red: from commit `029057d` (2026-09-25T04:26:29Z) onwards, a
  green `ci` run is no longer evidence about the gate at all. No gate failure
  has been observed since the flag landed, so this has never actually been
  exercised — the four green runs after it are the only post-advisory data, and
  they happened to pass.
- `main` has **no branch protection**, so even before the advisory flag a green
  run never blocked a merge. (Verified against the GitHub API, not assumed.)
- The one configuration CI runs cannot see a **P0 silicon blocker** at all:
  FP/SIMD enablement on the QEMU target's EL2 path. The store at
  `vivanta-boot/target-qemu-aarch64/src/main.rs:31`,
  `msr CPTR_EL2, #(0b11<<20)`, is an **architecturally no-op** (`0b11 << 20`
  is `CPACR_EL1.FPEN[21:20]`; `CPTR_EL2.TFP` is bit 10 and bits 20:21 are
  RES0), and the `b 7f` at `:32` jumps past the `msr CPACR_EL1` at `:35`, so
  `CPACR_EL1.FPEN` stays at reset `0b00` and traps FP/SIMD to EL1. The gate
  builds the **debug** profile (`vivanta-boot/tools/qemu-gates.sh:55-56`) —
  the very profile in which `write_volatile` emits NEON — and is green anyway.
  Five of six targets have this defect; only `target-rk3568/src/main.rs:190-191`
  is correct, and it is the one target that never links the kernel. Full
  description and per-target table:
  `docs/architecture/arm-exception-state.md`. **18/18 is not evidence that
  FP/SIMD is enabled.**

Never write "gates green", "18/18 PASS in CI" or "22/22" for this job.

## Never run a bare `cargo build --workspace`

It compiles, and the QEMU binary it leaves in
`target/aarch64-unknown-none/debug/vivanta-target-qemu-aarch64` **hangs at MMU
enable**. `target-rpi3b-plus/Cargo.toml:16` enables the
`vivanta-arch-aarch64/spec-table-desc` feature, and in a `--workspace` build
cargo unifies that feature into the single shared `arch-aarch64` every target
links. `spec-table-desc` switches the L1/L2 table descriptor from `0b11` to the
spec-correct `0b10` (`arch-aarch64/src/paging/descriptor.rs:16-27`); QEMU's
cortex-a53 model does not boot with `0b10` and hangs at MMU enable instead
(`arch-aarch64/src/mmu.rs:83-91`).
CI avoids this by building the QEMU target on its own, and says so in a comment
at `.github/workflows/ci.yml:68-73`:

```
# Note: build the QEMU target on its own. A bare `--workspace` build would
# unify target-rpi3b-plus's `spec-table-desc` feature into arch-aarch64 and
# produce a QEMU kernel that hangs at MMU enable.
cargo build -p vivanta-target-qemu-aarch64
```

`cargo check --workspace` is fine — the bug is in the *linked* artifact, not in
compilation. Build every runnable target standalone (`-p …`, or `build.sh
<board>`), and note that the per-target `target-*/.cargo/config.toml` files are
dead config: cargo reads config from the invocation directory, and every script
and workflow runs from `vivanta-boot/`. The effective mechanism is each crate's
`build.rs`.

Reproduced locally, A/B, same tree, same script:

| Build | Encoding string in the binary | Gate |
|---|---|---|
| `cargo build --workspace` | `0b10 (spec-correct)` | hangs at `Enabling MMU...`, killed at the timeout, `markers: 0/18` |
| `cargo build -p vivanta-target-qemu-aarch64` | `0b11 (QEMU-compatible)` | `markers: 18/18` — `QEMU GATES: PASS` |

## Traps (all actually hit)

| Symptom | Cause | Fix |
|---|---|---|
| Fresh clone: kernel build fails on `include_bytes!` | `user-init.elf` is gitignored but embedded at compile time | run `tools/build-user-init.sh` first (CI build job does) |
| QEMU kernel hangs at MMU enable | bare `cargo build --workspace` unified `target-rpi3b-plus`'s `spec-table-desc` feature into `arch-aarch64` (0b10 descriptors) | build runnable targets standalone: `-p vivanta-target-qemu-aarch64`, `tools/ci-local.sh`, or `build.sh <board>` — see above |
| `cargo test` tries to build for `aarch64-unknown-none` | `.cargo/config.toml` sets `[build] target` | pass `--target <host triple>` |
| `mkstemp failed ... File exists` | BSD `mktemp` needs the template to end in `XXXXXX` | `mktemp /tmp/x.XXXXXX` then append the suffix |
| `mkfs.vfat: command not found` | `mtools`/`dosfstools` missing | `brew install mtools dosfstools` / `apt-get install -y mtools dosfstools` |
| `gh release create` → 403 | workflow lacks `contents: write` | only `release.yml` gets write; `ci.yml` stays read-only |
| `rust-objcopy: command not found` on the runner | it lives in the `llvm-tools` component, which rustup does not put on PATH (locally it came from cargo-binutils) | call `tools/rust-objcopy.sh`, which resolves it from the toolchain sysroot |
| Edited `target-*/.cargo/config.toml` and nothing changed | cargo only reads config from the invocation directory, and every script runs from `vivanta-boot/` | put link args in the crate's `build.rs` |

## Build assets (`assets` release)

Heavy or moving inputs are not in git. The Broadcom firmware blobs
(`bootcode.bin`, `start.elf`, `fixup.dat`, the board DTB, `miniuart-bt.dtbo`)
are published once in a prerelease tagged `assets` as
`rpi3b-firmware-*.tar.gz`; CI unpacks them into `images/rpi3b-plus/firmware/`
(the build script's cache) and falls back to upstream only if the asset is
missing. This keeps builds reproducible and independent of
`raspberrypi/firmware@master`.
