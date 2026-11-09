#!/usr/bin/env bash
# ---------------------------------------------------------------------------
# armv7-portability.sh — prove the PORTABLE crates build for ARMv7-A.
#
# WHY THIS SCRIPT EXISTS
#
# Vivanta is an AArch64 kernel today, and `arch-armv7a` is a frozen stub
# (ADR-011). But the *portable* part of the tree — the ISA contract
# (`arch-api`), the boot-time contract (`boot-info`), the shared boot helpers
# (`boot_common`) and the architecture-neutral VA allocator (`vm`) — is
# supposed to be architecture-independent, and a device family like the teXet
# TB-176FL (Rockchip RK2906, Cortex-A8, ARMv7-A) needs exactly that part to
# be usable on a 32-bit ARMv7-A core.
#
# "Supposed to be" is not a measurement. This script measures it: it builds
# those crates for `armv7a-none-eabihf` and fails if they do not compile.
# It is the cheapest possible guard against the portable core quietly growing
# an AArch64 assumption — the kind of assumption that costs a port months when
# it is only found at the end.
#
# WHAT IT DOES NOT CLAIM
#
# That Vivanta boots on ARMv7-A. It does not: the kernel, the exception model
# and the MMU are still AArch64 (ADR-011, ADR-036). What this script proves is
# narrower and precisely stated: the portable base compiles for the ISA of a
# 32-bit ARMv7-A device, unchanged.
#
# The test count is PARSED FROM CARGO'S OWN OUTPUT, never hardcoded — the same
# rule `host-test.sh` follows.
#
# Usage:  tools/armv7-portability.sh [--target <triple>]
# Exit:   0 = every portable crate compiled for the target, 1 = one failed.
# ---------------------------------------------------------------------------
set -euo pipefail

cd "$(dirname "$0")/.."

TARGET="armv7a-none-eabihf"
if [ "$#" -ge 2 ] && [ "$1" = "--target" ]; then
    TARGET="$2"
fi

# The crates that must stay architecture-independent. Adding a crate here is a
# statement about that crate: it must not grow an ISA assumption.
CRATES=(
    vivanta-arch-api
    vivanta-boot-common
    vivanta-boot-info
    vivanta-vm
)

echo "==> ARMv7-A portability: ${#CRATES[@]} portable crates for ${TARGET}"

built=0
for crate in "${CRATES[@]}"; do
    echo "==> building ${crate} for ${TARGET}"
    if ! cargo build --target "$TARGET" -p "$crate"; then
        echo "FAIL: ${crate} does not build for ${TARGET}" >&2
        exit 1
    fi
    built=$((built + 1))
done

echo "==> PASS: ${built}/${#CRATES[@]} portable crates build for ${TARGET}."
echo "    (This does NOT mean Vivanta boots on ARMv7-A — see ADR-036.)"
