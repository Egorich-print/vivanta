#!/usr/bin/env bash
# ---------------------------------------------------------------------------
# race-harness.sh — deterministic reproducer for the open scheduler race
# ---------------------------------------------------------------------------
# Mission 2026-10, wave 1.1.
#
# The scheduler race (mission charter R2: "no causative fix before a failpoint
# actually fires") is intermittent in the field: a timer tick has to land
# inside a window a handful of instructions wide — between the resume-side
# `enable_interrupts()` in the scheduler and the vector epilogue that retires
# the exception frame. On CI it showed up in roughly 1 run out of 2–5.
#
# This harness builds the kernel with the failpoint feature, which injects a
# nested scheduler entry at exactly that point. The condition is then produced
# on every run instead of being waited for.
#
# TWO MODES, and the control matters:
#
#   control  Harness compiled in, nothing armed. The kernel must boot
#            exactly like a release build and reach the terminal marker.
#            This proves the harness code itself does not perturb the
#            kernel — without it, a reproduction proves nothing.
#
#   fire     F3 additionally armed. The nested entry is injected and the
#            kernel is expected to lose the affected thread's saved
#            context.
#
# Usage:
#   tools/race-harness.sh control [iterations]   # default 5
#   tools/race-harness.sh fire    [iterations]   # default 10
#
# Exit status:
#   0  the expected outcome for this mode (control clean / fire did not
#      reproduce)
#   1  the other outcome — the failpoint reproduced the race (fire mode), or
#      the control build misbehaved (control mode, which is a harness bug)
#   2  harness error (build failure, QEMU unavailable, ...)
# ---------------------------------------------------------------------------
set -euo pipefail

MODE="${1:-fire}"
ITERATIONS="${2:-}"

case "${MODE}" in
    control) FEATURE="vivanta-kernel/sched-failpoint";       DEFAULT_ITERS=5 ;;
    fire)    FEATURE="vivanta-kernel/sched-failpoint-fire";  DEFAULT_ITERS=10 ;;
    *) echo "usage: $0 {control|fire} [iterations]" >&2; exit 2 ;;
esac
ITERATIONS="${ITERATIONS:-${DEFAULT_ITERS}}"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE="$(cd "${SCRIPT_DIR}/.." && pwd)"
KERNEL_PKG="vivanta-target-qemu-aarch64"

cd "${WORKSPACE}"

if ! command -v qemu-system-aarch64 >/dev/null 2>&1; then
    echo "race-harness: qemu-system-aarch64 not found in PATH" >&2
    exit 2
fi

echo "==> mode:      ${MODE}"
echo "==> feature:   ${FEATURE}"
echo "==> iterations:${ITERATIONS}"
echo "==> building kernel"
cargo build --offline -p "${KERNEL_PKG}" --features "${FEATURE}"

KERNEL="target/aarch64-unknown-none/debug/${KERNEL_PKG}"
if [[ ! -f "${KERNEL}" ]]; then
    echo "race-harness: kernel image not found at ${KERNEL}" >&2
    exit 2
fi

LOG="$(mktemp -t vivanta-race.XXXXXX.log)"
trap 'rm -f "${LOG}"' EXIT

# The marker the gate matrix keys on. If the race reproduces, the kernel
# stops before reaching it (or faults), and this marker is absent.
TERMINAL='[G4] running_count=1'
FAULT_RE='PANIC|EXCEPTION|Data Abort|CPU halted'

reproduced=0
clean=0
nested_total=0

for i in $(seq 1 "${ITERATIONS}"); do
    # Same invocation shape as tools/qemu-gates.sh: the guest has no disk,
    # the kernel is the only image, and the serial port carries the log.
    # The kernel never exits (it ends in a cooperative idle loop), so QEMU
    # is started in the background and killed once the terminal marker
    # appears — or once the timeout expires.
    : >"${LOG}"
    qemu-system-aarch64 \
        -M virt -cpu cortex-a53 -accel tcg -m 512M -nographic \
        -kernel "${KERNEL}" \
        -serial mon:stdio >"${LOG}" 2>&1 &
    QEMU_PID=$!

    deadline=$(( SECONDS + 300 ))
    while kill -0 "${QEMU_PID}" 2>/dev/null; do
        if grep -qE "${TERMINAL}|${FAULT_RE}" "${LOG}"; then
            break
        fi
        if (( SECONDS >= deadline )); then
            break
        fi
        sleep 0.5
    done
    kill "${QEMU_PID}" 2>/dev/null || true
    wait "${QEMU_PID}" 2>/dev/null || true

    hits="$(grep -oE 'nested_entries=[0-9]+' "${LOG}" | head -1 | cut -d= -f2 || true)"
    hits="${hits:-0}"
    nested_total=$(( nested_total + hits ))

    reached=0
    grep -qF "${TERMINAL}" "${LOG}" && reached=1
    faulted=0
    grep -qE "${FAULT_RE}" "${LOG}" && faulted=1

    if [[ "${reached}" -eq 1 && "${faulted}" -eq 0 ]]; then
        echo "run ${i}: clean (reached terminal marker, no fault)"
        clean=$(( clean + 1 ))
        continue
    fi

    echo "run ${i}: FAULT — reached_terminal=${reached} faulted=${faulted}"
    grep -nE "${FAULT_RE}" "${LOG}" | head -5 || true
    echo "--- context before the fault ---"
    grep -nE 'x30:|ESR_EL1|Class:' "${LOG}" | head -6 || true
    reproduced=$(( reproduced + 1 ))
    if [[ "${MODE}" == "control" ]]; then
        echo
        echo "RESULT: the CONTROL build faulted. The harness code itself is"
        echo "        perturbing the kernel — rework it before drawing any"
        echo "        conclusion from the fire mode (mission charter G6)."
        exit 1
    fi
    break
done

echo
echo "==> clean runs:             ${clean}"
echo "==> faulting runs:         ${reproduced}"
# The counter is printed at the end of boot, so a run that FAULTED never
# reports it — it is only meaningful for clean runs.
echo "==> nested entries fired:  ${nested_total} (reported by clean runs only)"

if [[ "${MODE}" == "control" ]]; then
    echo
    if [[ "${clean}" -eq "${ITERATIONS}" ]]; then
        echo "RESULT: the control build is clean in ${ITERATIONS}/${ITERATIONS} runs."
        echo "        The harness does not perturb the kernel — the fire mode is"
        echo "        a valid experiment."
        exit 0
    fi
    echo "RESULT: the control build is NOT clean. See above."
    exit 1
fi

echo
if [[ "${reproduced}" -gt 0 ]]; then
    echo "RESULT: the failpoint reproduces the race deterministically."
    echo "        A causative fix may now be developed against this reproducer"
    echo "        (mission charter R2 satisfied)."
    exit 1
fi

echo "RESULT: the failpoint did NOT reproduce the race in ${ITERATIONS} runs."
echo "        The window is narrower than the failpoint reaches — refine the"
echo "        injection point before attempting a fix."
exit 0
