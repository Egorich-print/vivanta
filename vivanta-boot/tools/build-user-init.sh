#!/usr/bin/env bash
# ---------------------------------------------------------------------------
# build-user-init.sh — build the EL0 userland program and place the artifact
# where the kernel expects it.
#
# Why this script exists:
#   `kernel/src/syscall/process.rs` embeds the EL0 image at compile time via
#   include_bytes!("../../../user-init/user-init.elf"), and `*.elf` is
#   gitignored — so a fresh clone CANNOT build the kernel until this runs.
#   CI calls this before any kernel/QEMU target build.
#
# Output: user-init/user-init.elf  (linked at the fixed user VA 0x01000000)
#
# Usage: tools/build-user-init.sh
# ---------------------------------------------------------------------------
set -euo pipefail

cd "$(dirname "$0")/.."

TARGET="aarch64-unknown-none"
SRC="user-init/target/${TARGET}/release/user-init"
DST="user-init/user-init.elf"
EXPECTED_ENTRY=0x1000000

echo "==> building user-init (${TARGET}/release)"
(
    cd user-init
    cargo build --release --target "${TARGET}"
)

[ -f "${SRC}" ] || { echo "ERROR: build produced no ${SRC}" >&2; exit 1; }

cp "${SRC}" "${DST}"
chmod +x "${DST}"

# Guard: the image must be an AArch64 ELF linked at the user VA base.
# A plain host build lands at 0x210180 and load_elf fails with OutOfVa.
python3 - "${DST}" "${EXPECTED_ENTRY}" <<'PY'
import struct, sys
path, want = sys.argv[1], int(sys.argv[2], 0)
with open(path, "rb") as f:
    hdr = f.read(64)
if hdr[:4] != b"\x7fELF":
    sys.exit(f"ERROR: {path} is not an ELF file")
machine = struct.unpack("<H", hdr[0x12:0x14])[0]
if machine != 0xB7:
    sys.exit(f"ERROR: {path} machine={machine:#x}, expected AArch64 (0xb7)")
entry = struct.unpack("<Q", hdr[0x18:0x20])[0]
if entry != want:
    sys.exit(f"ERROR: {path} entry={entry:#x}, expected {want:#x}")
PY

echo "    ${DST}: $(ls -lh "${DST}" | awk '{print $5}'), entry=0x1000000"
