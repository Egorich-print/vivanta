#!/usr/bin/env bash
# ---------------------------------------------------------------------------
# host-test.sh — run the host unit tests for every crate that has them
# ---------------------------------------------------------------------------
# WHY THIS SCRIPT EXISTS
#
# vivanta-boot/.cargo/config.toml pins `build.target = "aarch64-unknown-none"`
# for the whole workspace, because the kernel is freestanding. A bare
# `cargo test` therefore tries to build the test harness for a target that has
# neither `std` nor the `test` crate, and dies with:
#
#     error[E0463]: can't find crate for `test`
#     error[E0463]: can't find crate for `std`
#
# A host test must therefore pass an explicit `--target <host-triple>`.
# This script does that automatically so that `cargo test` and CI agree.
#
# The test count is PARSED FROM CARGO'S OWN OUTPUT, never hardcoded: a script
# that prints "12 tests passed" without counting them is a gate that can
# report success without having tested anything.
#
# Usage:  tools/host-test.sh [--target <triple>]
# Exit:   0 = every crate passed, 1 = a crate failed
# ---------------------------------------------------------------------------
set -euo pipefail

cd "$(dirname "$0")/.."

target=""
if [ "$#" -ge 1 ] && [ "$1" = "--target" ]; then
    target="$2"
    shift 2
fi

if [ -z "$target" ]; then
    target="$(rustc -vV | sed -n 's/^host: //p')"
fi
if [ -z "$target" ]; then
    echo "host-test: cannot detect the host triple" >&2
    exit 1
fi

# Offline only when the registry cache is actually populated; a fresh CI
# runner has no cache and `--offline` would fail there. Kept as a plain
# string rather than an array: macOS ships bash 3.2, where expanding an
# empty array under `set -u` is an "unbound variable" error.
offline=""
if [ -d ".cargo/registry" ] && [ -n "$(ls -A .cargo/registry 2>/dev/null)" ]; then
    offline="--offline"
fi

CRATES=(
    vivanta-vm
    vivanta-exec
    vivanta-boot-info
    vivanta-boot-common
    vivanta-arch-api
)

total_passed=0
total_failed=0

for crate_name in "${CRATES[@]}"; do
    echo "==> [test] ${crate_name}"

    # Capture cargo's output while still streaming it, so a failure shows the
    # compiler diagnostics instead of only an exit code.
    out="$(cargo test ${offline} -p "${crate_name}" --target "${target}" 2>&1)" || {
        echo "${out}" >&2
        echo "==> [test] ${crate_name}: FAILED" >&2
        exit 1
    }
    echo "${out}"

    # Sum the "N passed" column over every test binary in the crate.
    passed="$(printf '%s\n' "${out}" \
        | sed -n 's/^test result: ok\. \([0-9][0-9]*\) passed.*/\1/p' \
        | awk '{ s += $1 } END { print s + 0 }')"
    failed="$(printf '%s\n' "${out}" \
        | sed -n 's/^test result: .* \([0-9][0-9]*\) failed.*/\1/p' \
        | awk '{ s += $1 } END { print s + 0 }')"

    total_passed=$(( total_passed + passed ))
    total_failed=$(( total_failed + failed ))
    echo "==> [test] ${crate_name}: ${passed} passed, ${failed} failed"
done

echo "==> [test] TOTAL: ${total_passed} passed, ${total_failed} failed"

if [ "${total_failed}" -ne 0 ]; then
    echo "host-test: failures reported by cargo" >&2
    exit 1
fi
if [ "${total_passed}" -eq 0 ]; then
    echo "host-test: no tests ran — refusing to report success" >&2
    exit 1
fi

echo "host-test: OK (host target ${target})"
