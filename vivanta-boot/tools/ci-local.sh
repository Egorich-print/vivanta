#!/usr/bin/env bash
# ---------------------------------------------------------------------------
# ci-local.sh — run the whole CI pipeline locally, one command.
#
# CI calls the same scripts (build-user-init.sh, qemu-gates.sh,
# make-rpi3b-image.sh); this orchestrator exists so the "prove it locally
# before pushing" ritual (§7 of docs/tooling/ci-cd.md) is a single command.
#
# Usage:  tools/ci-local.sh
#   SKIP_QEMU=1   skip the QEMU gate (fast sanity pass)
#   BUILD_IMAGE=1 also build the RPi3B+ SD image (needs mtools + dosfstools)
# Exit: 0 = green, 1 = first failing step.
# ---------------------------------------------------------------------------
set -euo pipefail

cd "$(dirname "$0")/.."

HOST="$(rustc -vV | sed -n 's/^host: //p')"
echo "==> ci-local on ${HOST}"

echo "==> [lint] rustfmt"
cargo fmt --all -- --check
( cd user-init && cargo fmt --check )

echo "==> [lint] shell syntax"
while IFS= read -r f; do bash -n "$f" || { echo "syntax error: $f" >&2; exit 1; }; done < <(git ls-files '*.sh')

echo "==> [lint] python syntax"
while IFS= read -r f; do python3 -m py_compile "$f"; done < <(git ls-files '*.py')

echo "==> [lint] executable bits"
bad="$(git ls-files -s | awk '$1=="100644" && $4 ~ /\.sh$/ {print $4}')"
if [ -n "${bad}" ]; then echo "these must be 755:" >&2; echo "${bad}" >&2; exit 1; fi

echo "==> [test] host unit tests (${HOST})"
cargo test -p vivanta-vm -p vivanta-exec --target "${HOST}"

echo "==> [build] EL0 userland + workspace check + QEMU target"
./tools/build-user-init.sh
cargo check --workspace
cargo build -p vivanta-target-qemu-aarch64

if [ "${SKIP_QEMU:-0}" = 1 ]; then
    echo "==> [qemu] skipped (SKIP_QEMU=1)"
else
    ./tools/qemu-gates.sh
fi

if [ "${BUILD_IMAGE:-0}" = 1 ]; then
    echo "==> [image] Raspberry Pi 3B+ SD image"
    ./tools/make-rpi3b-image.sh
fi

echo "==> ci-local: GREEN"
