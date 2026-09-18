#!/usr/bin/env bash
# Build mainline U-Boot for the RK3568 NVR board with eMMC support.
#
# Reproduces the artifacts described in ../uboot-emmc-bringup.md:
#   idbloader.img   TPL + SPL + BL31 (BootROM image)
#   u-boot.itb      FIT: U-Boot + DTB + ATF
#   u-boot.bin      plain U-Boot (no SPL)
#
# Runs on a Linux host (aarch64 or x86_64) with an
# aarch64-linux-gnu cross toolchain. The original build used the BuildRoot
# aarch64 toolchain inside the Lima VM `br2`.
#
# Usage:
#   CROSS_COMPILE=aarch64-linux-gnu- ./build-mainline-uboot.sh
#
set -euo pipefail

UBOOT_VER="${UBOOT_VER:-v2026.07}"
WORK="${WORK:-$HOME/uboot-rk3568}"
CROSS_COMPILE="${CROSS_COMPILE:-aarch64-linux-gnu-}"
JOBS="${JOBS:-$(nproc 2>/dev/null || echo 4)}"

RKBIN="$WORK/rkbin"
UBOOT="$WORK/u-boot"
BL31="$RKBIN/bin/rk35/rk3568_bl31_v1.46.elf"
TPL="$RKBIN/bin/rk35/rk3568_ddr_1560MHz_v1.26.bin"

mkdir -p "$WORK"

if [ ! -d "$RKBIN" ]; then
    echo ">> cloning rkbin"
    git clone --depth 1 https://github.com/rockchip-linux/rkbin.git "$RKBIN"
fi

if [ ! -d "$UBOOT" ]; then
    echo ">> cloning U-Boot $UBOOT_VER"
    git clone --depth 1 --branch "$UBOOT_VER" \
        https://source.denx.de/u-boot/u-boot.git "$UBOOT"
fi

for f in "$BL31" "$TPL"; do
    [ -f "$f" ] || { echo "missing blob: $f" >&2; exit 1; }
done

echo ">> building U-Boot"
make -C "$UBOOT" mrproper
make -C "$UBOOT" evb-rk3568_defconfig
make -C "$UBOOT" CROSS_COMPILE="$CROSS_COMPILE" \
     BL31="$BL31" ROCKCHIP_TPL="$TPL" -j"$JOBS"

echo
echo ">> artifacts"
ls -l "$UBOOT/idbloader.img" "$UBOOT/u-boot.itb" "$UBOOT/u-boot.bin"
echo
echo "Install into Vivanta (git-ignored binaries):"
echo "  cp $UBOOT/idbloader.img           <vivanta>/vivanta-boot/images/uboot-rk3568-idbloader-new.img"
echo "  cp $UBOOT/u-boot.itb              <vivanta>/vivanta-boot/images/u-boot.itb"
echo "  cp $UBOOT/u-boot.bin              <vivanta>/vivanta-boot/images/uboot-rk3568-new-mainline.bin"
