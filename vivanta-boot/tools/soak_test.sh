#!/usr/bin/env bash
# ---------------------------------------------------------------------------
# soak_test.sh — M5.0 G4+ reliability soak (60 minutes by default).
#
# Runs the QEMU kernel under the timer-driven preemption workload and checks
# the log for machine-readable invariants. The soak is NOT part of M5.0 exit
# criteria; it is a deferred reliability artifact per
# vivanta-boot/docs/milestones/M5.0-green-baseline.md (G4+).
#
# Pass criteria (all must hold):
#   1. No PANIC / EXCEPTION / "G4 FAIL" in the log.
#   2. Both preemption workers produced output ([PREEMPT] current=3/4 ...).
#   3. [G4] running_count=1 present.
#   4. Kernel reaches the G4 monitor (proves G1-G4 boot path intact).
#
# Usage:  tools/soak_test.sh [duration_seconds]
# Exit:   0 = PASS, 1 = FAIL.
# ---------------------------------------------------------------------------
set -euo pipefail
cd "$(dirname "$0")/.."

DURATION="${1:-3600}"
LOG_BASE="$(mktemp /tmp/vivanta-soak.XXXXXX)"
PIDFILE_BASE="$(mktemp /tmp/vivanta-soak.XXXXXX)"
LOG="${LOG_BASE}.log"
PIDFILE="${PIDFILE_BASE}.pid"

echo "=== Vivanta M5.0 G4+ soak test (${DURATION}s) ==="
echo "log: ${LOG}"

# Build the QEMU target first.
cargo build -p vivanta-target-qemu-aarch64 >/dev/null

# Launch QEMU.
qemu-system-aarch64 \
    -M virt -cpu cortex-a53 -m 512M -nographic \
    -kernel target/aarch64-unknown-none/debug/vivanta-target-qemu-aarch64 \
    -serial mon:stdio >"${LOG}" 2>&1 &
QEMU_PID=$!
echo "${QEMU_PID}" >"${PIDFILE}"

# Wait for the duration, then stop.
sleep "${DURATION}"
kill "${QEMU_PID}" 2>/dev/null || true
wait "${QEMU_PID}" 2>/dev/null || true

echo "=== soak finished, checking invariants ==="

FAIL=0
check() {
    if eval "$2"; then
        echo "PASS: $1"
    else
        echo "FAIL: $1"
        FAIL=1
    fi
}

check "no PANIC/EXCEPTION/G4-FAIL" \
    "! grep -qE 'PANIC|EXCEPTION|G4 FAIL' '${LOG}'"
check "preemption worker A produced output" \
    "grep -qE '\\[PREEMPT\\] current=[0-9]+ A=' '${LOG}'"
check "preemption worker B produced output" \
    "grep -qE '\\[PREEMPT\\] current=[0-9]+ B=' '${LOG}'"
check "Running invariant observed" \
    "grep -qE '\\[G4\\] running_count=1' '${LOG}'"
check "EL0 demo reached" \
    "grep -q 'Hello, Vivanta!' '${LOG}'"
check "fault containment exercised" \
    "grep -q 'survived the faulting task' '${LOG}'"

echo "=== soak log tail ==="
tail -5 "${LOG}"

if [ "${FAIL}" -eq 0 ]; then
    echo "=== M5.0 G4+ SOAK: PASS ==="
    exit 0
else
    echo "=== M5.0 G4+ SOAK: FAIL ==="
    exit 1
fi
