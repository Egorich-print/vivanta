# CI/CD

GitHub Actions pipelines live in `.github/workflows/`:

```
push/PR → lint ─┐
                ├→ build (user-init + QEMU ELF) → qemu (gate matrix, TCG)
        test ───┘
        └→ image (mkfs.vfat + mtools → RPi3B+ SD image)

workflow_dispatch → release → attach artifacts to a GitHub Release
```

- **lint** — `cargo fmt --check` (workspace + `user-init`), `bash -n` on tracked
  `*.sh`, `py_compile` on tracked `*.py`, and a 755 exec-bit check.
- **test** — `cargo test -p vivanta-vm -p vivanta-exec` on the native x86_64
  runner (no emulation). The workspace `.cargo/config.toml` defaults to
  `aarch64-unknown-none`, so host tests pass `--target x86_64-unknown-linux-gnu`.
- **build** — builds the EL0 `user-init.elf`, `cargo check --workspace`, then the
  QEMU target standalone; uploads the kernel ELF as an artifact.
- **qemu** — boots the artifact under `-M virt -cpu cortex-a53` (TCG) and checks
  the serial log with `tools/qemu-gates.sh` (18 required markers, no panic).
  Blocking; `QEMU_TIMEOUT=600` because x86_64 emulation is slower than native.
  If the runner ever proves non-reproducible, demote the job to
  `continue-on-error: true` **with a comment** instead of deleting it.
- **image** — builds the RPi3B+ SD image portably (`mkfs.vfat` + `mtools`).
- **release** — `workflow_dispatch` with a tag (default `v0.1.0-alpha`); builds
  the kernel binaries and the SD image, then `gh release create` with
  `permissions: contents: write`.

## Local ritual (run before pushing)

```sh
cd vivanta-boot
tools/ci-local.sh              # fmt + lint + host tests + workspace check + QEMU gates
BUILD_IMAGE=1 tools/ci-local.sh   # also build the SD image (needs mtools + dosfstools)
```

Everything CI does is an ordinary host script (`tools/build-user-init.sh`,
`tools/qemu-gates.sh`, `tools/make-rpi3b-image.sh`) — the workflows only call
them, so a local green run means a CI green run modulo emulation speed.

## Traps (all actually hit)

| Symptom | Cause | Fix |
|---|---|---|
| Fresh clone: kernel build fails on `include_bytes!` | `user-init.elf` is gitignored but embedded at compile time | run `tools/build-user-init.sh` first (CI build job does) |
| QEMU kernel hangs at MMU enable | bare `cargo build --workspace` unified `target-rpi3b-plus`'s `spec-table-desc` feature into `arch-aarch64` (0b10 descriptors) | build runnable targets standalone: `-p vivanta-target-qemu-aarch64` / `build.sh rpi3bp` |
| `cargo test` tries to build for `aarch64-unknown-none` | `.cargo/config.toml` sets `[build] target` | pass `--target <host triple>` |
| `mkstemp failed ... File exists` | BSD `mktemp` needs the template to end in `XXXXXX` | `mktemp /tmp/x.XXXXXX` then append the suffix |
| `mkfs.vfat: command not found` | `mtools`/`dosfstools` missing | `brew install mtools dosfstools` / `apt-get install -y mtools dosfstools` |
| `gh release create` → 403 | workflow lacks `contents: write` | only `release.yml` gets write; `ci.yml` stays read-only |
| `rust-objcopy: command not found` on the runner | it lives in the `llvm-tools` component, which rustup does not put on PATH (locally it came from cargo-binutils) | call `tools/rust-objcopy.sh`, which resolves it from the toolchain sysroot |

## Build assets (`assets` release)

Heavy or moving inputs are not in git. The Broadcom firmware blobs
(`bootcode.bin`, `start.elf`, `fixup.dat`, the board DTB, `miniuart-bt.dtbo`)
are published once in a prerelease tagged `assets` as
`rpi3b-firmware-*.tar.gz`; CI unpacks them into `images/rpi3b-plus/firmware/`
(the build script's cache) and falls back to upstream only if the asset is
missing. This keeps builds reproducible and independent of
`raspberrypi/firmware@master`.
