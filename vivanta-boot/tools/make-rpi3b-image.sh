#!/usr/bin/env bash
# ---------------------------------------------------------------------------
# Build the Raspberry Pi 3B+ SD-card image for Vivanta.
#
# Output: images/rpi3b-plus/vivanta-rpi3b-plus.img
#   MBR + one 64 MiB FAT32 partition (type 0x0c, starts at LBA 8192) with:
#     bootcode.bin start.elf fixup.dat bcm2710-rpi-3-b-plus.dtb
#     overlays/miniuart-bt.dtbo
#     config.txt kernel8.img
#   Vivanta is a bare-metal kernel: there is no rootfs and no second
#   partition (contrast BalanSir, whose A/B rootfs layout is irrelevant here).
#
# Requirements: cargo + rust-objcopy (rustup component add llvm-tools),
# curl, python3, and macOS hdiutil. On Linux use mkfs.vfat+mtools or
# buildroot genimage with the same file set (see PLATFORM_BRINGUP.md §5).
#
# The Broadcom firmware blobs are downloaded, NOT committed (images/ is
# gitignored) — redistribute the image accordingly (firmware LICENCE terms).
# ---------------------------------------------------------------------------
set -euo pipefail

cd "$(dirname "$0")/.."

TARGET="vivanta-target-rpi3b-plus"
ELF="target/aarch64-unknown-none/debug/${TARGET}"
OUT_DIR="images/rpi3b-plus"
FW_DIR="${OUT_DIR}/firmware"
FW_OVERLAYS="${FW_DIR}/overlays"
KERNEL="${OUT_DIR}/kernel8.img"
IMG="${OUT_DIR}/vivanta-rpi3b-plus.img"
FW_BASE="https://raw.githubusercontent.com/raspberrypi/firmware/master/boot"
FAT_MB=64
PART_START=8192   # LBA — Raspberry Pi OS convention

# 1. Kernel ------------------------------------------------------------------
mkdir -p "${OUT_DIR}"
echo "==> building ${TARGET}"
cargo build -p "${TARGET}" --target aarch64-unknown-none
rust-objcopy -O binary "${ELF}" "${KERNEL}"
echo "    kernel8.img: $(ls -lh "${KERNEL}" | awk '{print $5}')"

# 2. Firmware (cached under images/) ----------------------------------------
mkdir -p "${FW_OVERLAYS}"
fetch() { # url dest
    [ -s "$2" ] && return 0
    echo "    fetching $(basename "$2")"
    curl -fsSL --max-time 180 -o "$2" "$1"
}
for f in bootcode.bin start.elf fixup.dat bcm2710-rpi-3-b-plus.dtb; do
    fetch "${FW_BASE}/${f}" "${FW_DIR}/${f}"
done
for o in miniuart-bt.dtbo; do
    fetch "${FW_BASE}/overlays/${o}" "${FW_OVERLAYS}/${o}"
done

# 3. config.txt --------------------------------------------------------------
cat > "${OUT_DIR}/config.txt" <<'EOF'
# Vivanta bare-metal kernel — Raspberry Pi 3B+ boot configuration.
#
# 64-bit physical kernel entry (Vivanta drops EL2->EL1h itself).
arm_64bit=1
kernel=kernel8.img

# Serial console: Vivanta drives the PL011 (UART0) at 0x3F201000 on
# GPIO 14/15. On the Pi 3 the PL011 is routed to Bluetooth by default, so
# miniuart-bt must move Bluetooth to the mini UART and free PL011; without
# this overlay (and its .dtbo) the kernel's serial output goes nowhere.
# enable_uart=1 pins the VPU core clock, which is also the PL011 clock
# Vivanta assumes (250 MHz in target-rpi3b-plus/src/main.rs).
enable_uart=1
dtoverlay=miniuart-bt

# Keep VideoCore RAM at the default 64 MiB: Vivanta's memory map caps usable
# DRAM at 0x3C000000 (960 of 1024 MiB). A larger gpu_mem shrinks usable RAM
# below that and a smaller one is not modelled.
gpu_mem=64

disable_overscan=1
EOF

# 4. Assemble the FAT32 boot partition --------------------------------------
command -v hdiutil >/dev/null 2>&1 || {
    echo "ERROR: hdiutil not found (macOS). On Linux build the image with" >&2
    echo "       mkfs.vfat + mtools, same file set — see PLATFORM_BRINGUP.md §5." >&2
    exit 1
}

BOOT_DMG="${OUT_DIR}/boot.vfat.dmg"
MNT="$(mktemp -d)/vivanta-boot"
rm -f "${BOOT_DMG}"
# macOS: strip xattrs and prevent AppleDouble ._ sidecars on the FAT volume
# (they are junk the firmware ignores, but they must not be shipped).
export COPYFILE_DISABLE=1
if command -v xattr >/dev/null 2>&1; then
    xattr -c "${FW_DIR}"/* "${FW_OVERLAYS}"/* "${KERNEL}" "${OUT_DIR}/config.txt" 2>/dev/null || true
fi
hdiutil create -size "${FAT_MB}m" -fs MS-DOS -volname VIVANTA "${BOOT_DMG}" >/dev/null
hdiutil attach -nobrowse -mountpoint "${MNT}" "${BOOT_DMG}" >/dev/null
mkdir -p "${MNT}/overlays"
cp "${FW_DIR}/bootcode.bin" "${FW_DIR}/start.elf" "${FW_DIR}/fixup.dat" \
   "${FW_DIR}/bcm2710-rpi-3-b-plus.dtb" "${MNT}/"
cp "${FW_OVERLAYS}/miniuart-bt.dtbo" "${MNT}/overlays/"
cp "${KERNEL}" "${MNT}/kernel8.img"
cp "${OUT_DIR}/config.txt" "${MNT}/config.txt"
if command -v dot_clean >/dev/null 2>&1; then
    dot_clean -m "${MNT}" 2>/dev/null || true
fi
sync
hdiutil detach "${MNT}" >/dev/null

# 5. Wrap into a partition table (MBR, partition at LBA 8192) ----------------
python3 - "${BOOT_DMG}" "${IMG}" "${PART_START}" <<'PY'
import os, struct, shutil, sys
boot, out, start = sys.argv[1], sys.argv[2], int(sys.argv[3])
total = os.path.getsize(boot)
fat_sectors = (total - 512) // 512            # drop hdiutil's own MBR
with open(out, "wb") as f:
    f.truncate((start + fat_sectors) * 512)
with open(boot, "rb") as i, open(out, "r+b") as o:
    i.seek(512)                                # FAT VBR must land on `start`
    o.seek(start * 512)
    shutil.copyfileobj(i, o)
with open(out, "r+b") as f:
    for vbr in (0, 6):                         # VBR + its backup: hidden sectors
        f.seek((start + vbr) * 512 + 0x1C)
        f.write(struct.pack("<I", start))
    mbr = bytearray(512)
    e = bytearray(16)
    e[0] = 0x80; e[4] = 0x0C                   # bootable, FAT32 LBA
    e[5:8] = b"\xfe\xff\xff"
    e[8:12] = struct.pack("<I", start)
    e[12:16] = struct.pack("<I", fat_sectors)
    mbr[446:462] = e
    mbr[510], mbr[511] = 0x55, 0xAA
    f.seek(0); f.write(mbr)
print(f"    image: {fat_sectors} FAT sectors, partition at LBA {start}")
PY
rm -f "${BOOT_DMG}"

# 6. Verify ------------------------------------------------------------------
echo "==> verifying image contents"
hdiutil attach -nobrowse -mountpoint "${MNT}" "${IMG}" >/dev/null
( cd "${MNT}" && find . -type f | sort )
hdiutil detach "${MNT}" >/dev/null

echo
echo "==> done: ${IMG} ($(ls -lh "${IMG}" | awk '{print $5}'))"
shasum -a 256 "${IMG}" || true
echo
echo "Flash (replace N with the SD disk, e.g. disk4 shown by 'diskutil list'):"
echo "  diskutil unmountDisk /dev/diskN"
echo "  sudo dd if=${IMG} of=/dev/rdiskN bs=4m"
echo "  sync && diskutil eject /dev/diskN"
echo "Serial console: GPIO 14/15 (header pins 8/10), 115200 8N1."
