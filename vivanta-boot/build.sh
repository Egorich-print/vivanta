#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")"

ADAPTER="${1:-vivanta-target-qemu-aarch64}"

# Map shorthands to full package names
case "${ADAPTER}" in
    rk3568) PACKAGE="vivanta-target-rk3568" ;;
    x96q)   PACKAGE="vivanta-target-x96q" ;;
    *)      PACKAGE="${ADAPTER}" ;;
esac

echo "=== Building ${PACKAGE} ==="
cargo build -p "${PACKAGE}"

case "${ADAPTER}" in
    rk3568)
        ELF="target/aarch64-unknown-none/debug/vivanta-target-rk3568"
        BIN="vivanta-rk3568.bin"
        UIMAGE="vivanta-rk3568.uImage"
        echo "=== Converting to flat binary ==="
        rust-objcopy -O binary "${ELF}" "${BIN}"
        ls -lh "${BIN}"
        echo "=== Creating uImage for bootm ==="
        mkimage -A arm64 -O linux -T kernel -C none \
            -a 0x20500000 -e 0x20500000 \
            -d "${BIN}" "${UIMAGE}"
        ls -lh "${UIMAGE}"
        echo ""
        echo "=== Flash write via TFTP (SPI NAND at 0xF80000): ==="
        echo "  tftp 0x20500000 ${UIMAGE}"
        echo "  mtd erase spi-nand0 0xF80000 0xC00000"
        echo "  mtd write spi-nand0 0x20500000 0xF80000 \${filesize}"
        echo ""
        echo "=== Boot: ==="
        echo "  mtd read spi-nand0 0x20500000 0xF80000 0xC00000"
        echo "  bootm 0x20500000"
        ;;

    x96q)
        ELF="target/aarch64-unknown-none/debug/vivanta-target-x96q"
        BIN="vivanta-x96q.bin"
        echo "=== Converting to flat binary ==="
        rust-objcopy -O binary "${ELF}" "${BIN}"
        ls -lh "${BIN}"
        file "${BIN}"
        echo "=== U-Boot commands for X96Q (Allwinner H313): ==="
        echo "  load mmc 0:1 0x40280000 vivanta-x96q.bin"
        echo "  booti 0x40280000 - \${fdt_addr_r}"
        ;;

    vivanta-target-qemu-aarch64|vivanta-target-qemu-armv7a)
        KERNEL="target/aarch64-unknown-none/debug/${PACKAGE}"
        echo "=== Launching QEMU (ELF mode) ==="
        qemu-system-aarch64 \
            -M virt \
            -cpu cortex-a53 \
            -m 512M \
            -nographic \
            -kernel "$KERNEL" \
            -serial mon:stdio
        ;;

    *)
        echo "=== Target '${ADAPTER}' built successfully ==="
        ;;
esac
