#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")"

ADAPTER="${1:-vivanta-target-qemu-aarch64}"

# Map shorthands to full package names
case "${ADAPTER}" in
    rk3568) PACKAGE="vivanta-target-rk3568" ;;
    x96q)   PACKAGE="vivanta-target-x96q" ;;
    rpi3bp) PACKAGE="vivanta-target-rpi3b-plus" ;;
    *)      PACKAGE="${ADAPTER}" ;;
esac

echo "=== Building ${PACKAGE} ==="
cargo build -p "${PACKAGE}"

case "${ADAPTER}" in
    rk3568)
        PACKAGE="vivanta-target-rk3568"
        ELF="target/aarch64-unknown-none/debug/${PACKAGE}"
        BIN="images/vivanta-rk3568.bin"
        mkdir -p images
        echo "=== Converting to flat binary ==="
        rust-objcopy -O binary "${ELF}" "${BIN}"
        echo "=== Converting to flat binary ==="
        rust-objcopy -O binary "${ELF}" "${BIN}"
        ls -lh "${BIN}"

        # Verify ARM64 header
        python3 -c "
import struct
with open('${BIN}','rb') as f:
    f.seek(56)
    m = struct.unpack('<I', f.read(4))[0]
    assert m == 0x644d5241, f'Bad magic: {m:#x}'
    f.seek(8)
    to = struct.unpack('<Q', f.read(8))[0]
    assert to == 0, f'text_offset should be 0, got {to:#x}'
    print(f'Header OK: text_offset=0, magic=ARMd')
"

        echo ""
        echo "=== UART upload + boot: ==="
        echo "  python3 flash_rk3568.py"
        echo ""
        echo "=== Manual U-Boot commands: ==="
        echo "  mm.l 0x20500000    (then type hex words, . to quit)"
        echo "  booti 0x20500000 - -"
        echo ""
        echo "=== Flash write (SPI NAND at 0xF80000, after mm.l upload): ==="
        echo "  mtd erase spi-nand0 0xF80000 0xC00000"
        echo "  mtd write spi-nand0 0x20500000 0xF80000 \${filesize}"
        echo ""
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

    rpi3bp)
        ELF="target/aarch64-unknown-none/debug/vivanta-target-rpi3b-plus"
        BIN="kernel8.img"
        echo "=== Converting to flat binary ==="
        rust-objcopy -O binary "${ELF}" "${BIN}"
        ls -lh "${BIN}"
        file "${BIN}"
        echo "=== RPi3 GPU firmware boot: ==="
        echo "  1. Copy kernel8.img to SD card boot/ partition"
        echo "  2. Insert SD card, power on"
        echo "  3. Expected UART output: '.' (RP0 marker)"
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
