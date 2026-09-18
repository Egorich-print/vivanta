#!/usr/bin/env bash
# ---------------------------------------------------------------------------
# rust-objcopy.sh — run `rust-objcopy` from the active toolchain's llvm-tools.
#
# Why: `rust-objcopy` ships inside the `llvm-tools` component but rustup does
# not create a PATH shim for it, and cargo-binutils may not be installed.
# This resolves the binary from the toolchain sysroot instead, so the image and
# release builds work on a bare CI runner and on a dev machine alike.
#
# Usage: tools/rust-objcopy.sh -O binary <in.elf> <out.bin>
# ---------------------------------------------------------------------------
set -euo pipefail

host="$(rustc -vV | sed -n 's/^host: //p')"
bin="$(rustc --print sysroot)/lib/rustlib/${host}/bin/rust-objcopy"

if [ ! -x "${bin}" ]; then
    echo "ERROR: rust-objcopy not found at ${bin}" >&2
    echo "       run: rustup component add llvm-tools" >&2
    exit 1
fi

exec "${bin}" "$@"
