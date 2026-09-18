#!/usr/bin/env bash
# ---------------------------------------------------------------------------
# qemu-gates.sh — boot the AArch64 kernel under QEMU and verify the boot
# gate matrix from its serial log.
#
# This is the CI gate AND the local gate: TCG cortex-a53 by default, matching
# the proven local configuration. On an AArch64 host with /dev/kvm, set
# QEMU_ACCEL=kvm QEMU_CPU=host to run the guest natively instead.
# The kernel never exits, so we watch the log and stop as soon as the last
# terminal marker shows up (or bail early on a panic).
#
# Usage:  tools/qemu-gates.sh [kernel.elf] [timeout_seconds]
#         QEMU_TIMEOUT=600 tools/qemu-gates.sh path/to/kernel.elf
#         QEMU_ACCEL=kvm QEMU_CPU=host tools/qemu-gates.sh path/to/kernel.elf
# Exit:   0 = all required markers present, no panic; 1 = otherwise.
# ---------------------------------------------------------------------------
set -euo pipefail

cd "$(dirname "$0")/.."

KERNEL="${1:-}"
TIMEOUT="${2:-${QEMU_TIMEOUT:-600}}"
LOG_BASE="$(mktemp /tmp/vivanta-gates.XXXXXX)"
LOG="${LOG_BASE}.log"

# Required serial markers (substrings; see kernel/src/lib.rs). The last one
# is the terminal marker: its appearance means the whole gate matrix ran.
MARKERS=(
    'M7.12 concurrent ELF gate:'
    '[DUAL] two concurrent ELF tasks exit(42) PASS'
    'M10.4 EL0 fork gate:'
    '[FORK-EL0] fork/waitpid round-trip exit(42) PASS'
    '[FORK-EL0] child teardown PASS'
    'M10.3 EL0 execve gate:'
    '[EXECVE] EL0 execve("/init") -> exit(42) PASS'
    '[EXEC2] double load+stack in one AS PASS'
    '[PTABLE] 70 create/reap cycles'
    '[KILL] SIGKILL->Zombie + SIGCHLD PASS'
    '[OOM] full-table resolve refuses cleanly PASS'
    '[E2E] load/dup/teardown deltas all zero PASS'
    '[ELF] genuine ELF64 userland PASS (exit=42)'
    '[COW] break+isolation PASS'
    '[FORK] pid parent='
    '[SYS] mmap/store/mprotect/munmap/negatives PASS'
    'Hello, Vivanta!'
    '[G4] running_count=1'
)
TERMINAL='[G4] running_count=1'

# If no kernel was supplied, build the verified QEMU target (and the embedded
# EL0 image it needs at compile time).
if [ -z "${KERNEL}" ]; then
    [ -f user-init/user-init.elf ] || ./tools/build-user-init.sh
    echo "==> building vivanta-target-qemu-aarch64"
    cargo build -p vivanta-target-qemu-aarch64
    KERNEL="target/aarch64-unknown-none/debug/vivanta-target-qemu-aarch64"
fi
[ -f "${KERNEL}" ] || { echo "ERROR: kernel not found: ${KERNEL}" >&2; exit 1; }

echo "==> QEMU gate matrix (timeout ${TIMEOUT}s)"
echo "    kernel: ${KERNEL}"
echo "    log:    ${LOG}"

# Accelerator: TCG by default (portable, cross-arch). On an AArch64 host with
# /dev/kvm, run the gate natively via QEMU's KVM backend instead:
#   QEMU_ACCEL=kvm QEMU_CPU=host tools/qemu-gates.sh <kernel>
ACCEL="${QEMU_ACCEL:-tcg}"
CPU="${QEMU_CPU:-cortex-a53}"
echo "    accel:  ${ACCEL}  (cpu ${CPU})"

qemu-system-aarch64 \
    -M virt -cpu "${CPU}" -accel "${ACCEL}" -m 512M -nographic \
    -kernel "${KERNEL}" \
    -serial mon:stdio >"${LOG}" 2>&1 &
QEMU_PID=$!

finished=0
deadline=$(( SECONDS + TIMEOUT ))
while kill -0 "${QEMU_PID}" 2>/dev/null; do
    if grep -qE 'PANIC|EXCEPTION|G4 FAIL' "${LOG}"; then
        echo "==> FATAL marker in serial log" >&2
        break
    fi
    if grep -qF "${TERMINAL}" "${LOG}"; then
        finished=1
        break
    fi
    if [ "${SECONDS}" -ge "${deadline}" ]; then
        echo "==> timeout after ${TIMEOUT}s" >&2
        break
    fi
    sleep 1
done

kill "${QEMU_PID}" 2>/dev/null || true
wait "${QEMU_PID}" 2>/dev/null || true

FAIL=0
check() {
    if grep -qF "$1" "${LOG}"; then
        echo "PASS: $1"
    else
        echo "FAIL: $1"
        FAIL=1
    fi
}

echo "==> gate markers"
for m in "${MARKERS[@]}"; do check "$m"; done

if grep -qE 'PANIC|EXCEPTION|G4 FAIL' "${LOG}"; then
    echo "FAIL: panic/exception present"
    FAIL=1
fi
[ "${finished}" = 1 ] || echo "WARN: terminal marker not reached before stop"

total=${#MARKERS[@]}
found=0
for m in "${MARKERS[@]}"; do
    if grep -qF "$m" "${LOG}"; then found=$((found + 1)); fi
done
echo "==> markers: ${found}/${total}"

if [ "${FAIL}" -eq 0 ] && [ "${finished}" = 1 ]; then
    echo "==> QEMU GATES: PASS"
    exit 0
fi

echo "==> serial log tail (${LOG})"
tail -30 "${LOG}"
echo "==> QEMU GATES: FAIL"
exit 1
