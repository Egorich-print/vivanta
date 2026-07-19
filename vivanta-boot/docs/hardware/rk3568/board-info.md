# RK3568 NVR Board — Hardware Reference

## Board Identification

U-Boot reports:
```
Model: Rockchip RK3568 NVR DEMO DDR4 V12 Linux Board
Board Type: NVR304-32E2
Revision: RK3568/4H/4G/512M-12/2022
```

Also known as: Rockchip RK3568 Evaluation Board.

## SoC: Rockchip RK3568

- CPU: 4× Cortex-A55 (ARMv8.2-A, AArch64)
- RAM: 4 GiB (DDR4)
  - Region 1: 0x00200000 – 0xF0000000 (3.75 GiB)
  - Region 2: 0x1F0000000 – 0x200000000 (256 MiB)
- Boot storage: SPI NAND (128 MB)
  - Chip: sfc_nand (vendor 0xa1, device 0xe4, density 0x7f)
  - Erase block: 128 KiB (0x20000)
  - Page: 2048 bytes

## SPI NAND Flash Layout

| Partition       | Offset     | Size       | Notes                    |
|-----------------|------------|------------|--------------------------|
| (? bootrom)     | 0x00000000 |            |                          |
| (? trust)       |            |            |                          |
| U-Boot          | 0x?????    | ~1.3 MiB   |                          |
| kernel          | 0x00F80000 | 0xC000000  | 12 MiB                   |
| (resource/logo) | 0x00700000 | 0x80000    | 512 KiB (logo partition) |
| (environment)   |            | 131068 B   |                          |

Locations verified:

- kernel offset: `0x00F80000` (from `kernel_addr=0xf80000`)
- kernel size: `0xC00000` (12 MiB, from `kernel_size=0xc00000`)
- erase block: `0x20000` (128 KiB)

## U-Boot

- Version: `U-Boot 2017.09-svn246980 (Nov 10 2023 - 16:11:09 +0800)`
- Vendor: Rockchip (vendor fork)
- Boot flow: SPL → U-Boot → kernel
- Script: `boot_flashkernel`

### Boot script

```
bootcmd=run boot_flashkernel

boot_flashkernel=mtd read spi-nand0 ${loadaddr} ${kernel_addr} ${kernel_size}; bootm ${loadaddr};bootm 0x20520000;bootm 0x20540000; bootm 0x20560000; bootm 0x20580000
```

The script reads 12 MiB from flash offset 0xf80000 to RAM at ${loadaddr} (0x20500000),
then tries `bootm` at five addresses (primary + 4 backup slots at 0x20000 intervals).

### Key environment variables

| Variable          | Value          | Description                     |
|-------------------|----------------|---------------------------------|
| `loadaddr`        | `0x20500000`   | Kernel load address (~541 MiB)  |
| `kernel_addr`     | `0xf80000`     | Flash offset of kernel partition|
| `kernel_size`     | `0xc00000`     | Size of kernel partition (12 MiB)|
| `fdt_addr_r`      | `0x0a100000`   | FDT address in RAM (~161 MiB)   |
| `kernel_addr_r`   | `0x00280000`   | Reserved for booti (when used)   |
| `ramdisk_addr_r`  | `0x04000000`   | Ramdisk address                  |
| `serverip`        | `192.168.0.105`| TFTP server IP                   |
| `ipaddr`          | `192.168.0.110`| Board IP address                 |
| `bootfile`        | `uImage_rk3568`| Expected TFTP filename           |
| `baudrate`        | `115200`       | Console baud rate                |

### Available U-Boot commands (relevant subset)

| Command      | Description                          |
|--------------|--------------------------------------|
| `booti`      | Boot ARM64 Linux Image from memory   |
| `bootm`      | Boot legacy uImage from memory       |
| `tftp`       | Download image via TFTP protocol     |
| `dhcp`       | Get IP via DHCP + TFTP download      |
| `mtd`        | MTD utils (read/write/erase)         |
| `mw`         | Memory write (fill)                  |
| `mm`         | Memory modify (interactive)          |
| `md`         | Memory display                       |
| `cp`         | Memory copy                          |
| `crc32`      | CRC32 checksum                       |
| `usb`        | USB subsystem                        |
| `ums`        | USB Mass Storage                     |
| `rockusb`    | Rockchip USB protocol                |
| `download`   | Rockusb/bootrom download mode        |
| `fatload`    | Load file from FAT filesystem        |
| `fdt`        | Flattened Device Tree commands       |
| `mmc`        | MMC/SD subsystem                     |
| `reset`      | Reset CPU                            |

### Networking

The board has two Ethernet controllers:
- `eth0: ethernet@fe2a0000` — primary (connected to RJ45)
- `eth1: ethernet@fe010000` — secondary

PHY: JL2101 (detected, patch applied). Auto-negotiation: works when cable is connected.

**Known issue**: When no cable connected, Ethernet commands hang at `Waiting for PHY auto negotiation to complete`.

## UART

- Controller: NS16550 compatible
- MMIO base: `0xFE660000`
- Reg shift: 2 (32-bit access: `*(volatile u32*)(base + (reg << 2))`)
- Baud rate: 115200, 8N1
- Console: serial (also has HDMI, eDP video outputs)
- Host serial device: `/dev/cu.wchusbserial110`

## U-Boot Boot Protocol

### Option A: uImage + bootm (default boot_flashkernel)

U-Boot has `bootm` compiled WITHOUT FDT/ATAGS support:
```
FDT and ATAGS support not compiled in - hanging
```

This means `bootm` cannot pass a device tree or ATAGs to the kernel.
**This is a blocker for bootm-based boot.**

### Option B: ARM64 Image + booti (working)

U-Boot has `booti` available. The kernel must:
1. Have a valid 64-byte ARM64 Image header (magic `0x644d5241` at offset 56)
2. Be PIE-capable (bit 3 of flags = 1) or use `text_offset = 0`
3. Accept a DTB address in x0 (or handle x0 = 0)

For `booti` to work, pass a DTB (either loaded from flash or written to RAM):
```
booti 0x20500000 - 0x0a100000
```

The DTB at `fdt_addr_r` (0x0a100000) must be written to RAM before booting.
U-Boot does NOT automatically load a DTB for `booti` on this board.

### DTB availability

U-Boot reports: `DTB: dts/kern.dtb` but then `Failed to load DTB, ret=-19`.
The board has a DTB header in flash but the data partition is missing/corrupt.

A working DTB for this board: `rk3568-nvr-demo-v12.dtb` (58 KiB).
When loaded to `fdt_addr_r`, U-Boot parses it correctly (adds memory banks, configs video).

### Option C: Serial download (fallback)

No serial download commands (`loadx`, `loady`, `loadb`) are available in this U-Boot.
To load data over serial, use `mm.l` (interactive memory modify) with a Python script.
At 115200 baud, ~280 words/second is achievable; a 72 KiB kernel loads in ~65 seconds.

## Kernel Requirements

### Entry point

The kernel must boot at the exception level it's entered (EL2 or EL1).
U-Boot on this board enters at EL2.

**Critical**: U-Boot leaves the MMU enabled when jumping to the kernel via `booti`.
The kernel must disable the MMU in its entry code before accessing any memory-mapped data:

```asm
mrs x6, sctlr_el2
bic x6, x6, #1       // clear M bit
msr sctlr_el2, x6
dsb sy
isb
tlbi alle2
dsb sy
isb
```

After disabling MMU at EL2, the kernel should drop to EL1:
```asm
mov x5, #(1 << 31)   // RW bit: AArch64 for EL1
msr hcr_el2, x5
mov x5, #0x3c5       // SPSR: EL1h, DAIF masked
msr spsr_el2, x5
adr x5, 1f
msr elr_el2, x5
eret                  // → EL1
1:
```

### Hardware address map (verified)

| Peripheral | Address    | Size   | Description               |
|------------|------------|--------|---------------------------|
| UART0      | 0xFE660000 | 64K    | NS16550, reg-shift=2      |
| UART1      | 0xFE650000 | 64K    | NS16550 (console?)        |
| UART2      | 0xFE640000 | 64K    | NS16550                   |
| GICv3      | 0xFD400000 | 2M     | Distributor               |
| GICv3      | 0xFD460000 | 2M     | Redistributor              |
| Ethernet0  | 0xFE2A0000 | 64K    | DWC EQOS                  |
| Ethernet1  | 0xFE010000 | 64K    |                           |
| eDP        | 0xFE0C0000 | 64K    |                           |
| HDMI       | 0xFE0A0000 | 64K    |                           |
| Timer      | 0xFE1F0000 | 64K    | ARM Generic Timer (CNTP)  |

## Building for RK3568

### Build and deploy workflow (proven)

```bash
# Build the kernel
cargo build -p target-rk3568

# Create flat binary
rust-objcopy -O binary target/aarch64-unknown-none/debug/target-rk3568 vivanta-rk3568.bin

# Patch ARM64 Image header: text_offset = 0 for booti compatibility
python3 -c "
import struct
with open('vivanta-rk3568.bin','r+b') as f:
    f.seek(8); f.write(b'\x00'*8)  # text_offset = 0
    f.seek(24); f.write(struct.pack('<Q', 0x0a))  # flags: PIE, LE
"

# Transfer via mm.l serial protocol (~65 seconds for 72K)
python3 /tmp/flash_booti2.py

# On board (after mm.l writes kernel + DTB to RAM):
#   mtd erase spi-nand0 0xF80000 0x20000
#   mtd write spi-nand0 0x20500000 0xF80000 0x20000
#   booti 0x20500000 - 0x0a100000
```

### Build for Linux (reference)

```bash
# Cross-compiler
export CROSS_COMPILE=aarch64-linux-gnu-
export ARCH=arm64

# Kernel config
make defconfig
# or for Rockchip:
make rockchip_linux_defconfig

# Build
make -j$(nproc) Image dtbs

# Package as uImage (not suitable — U-Boot has no FDT support)
# Use booti instead:
cp arch/arm64/boot/Image .
cp arch/arm64/boot/dts/rockchip/rk3568-nvr-demo-v12.dtb .

# Write Image to SPI NAND
# Flash kernel at 0xF80000
# Load DTB to 0x0a100000 before booti
```

## Shell Scripts for Automation

### TFTP server (macOS)

A Python TFTP server for sending files to the board:
- Serves from `/tmp/tftp/`
- File served: `uImage_rk3568` (or `vivanta-rk3568.uImage`)
- Runs on port 69 (requires no sudo)

### mm.l serial uploader

Located at `/tmp/flash_booti2.py`.
Protocol: sends kernel binary + DTB via interactive `mm.l` U-Boot commands.
Each word (32-bit) is sent with a 3-byte hex value + CR, synchronized on `?` prompt.
Achieves ~280 words/second at 115200 baud.

## Bring-up Status (2026-07-17)

### Stage 1 ✅ — Kernel boots, UART outputs text

| Step | Status | Details |
|------|--------|---------|
| U-Boot accepts image | ✅ | `booti` with uImage header (PIE=1, text_offset=0) |
| Kernel entry reached | ✅ | Entry code executes at EL2 |
| MMU disabled | ✅ | Both SCTLR_EL2 and SCTLR_EL1 cleared before memory access |
| EL2→EL1 transition | ✅ | via `eret` with SPSR_EL2=0x3c5, HCR_EL2.RW=1 |
| BSS zeroed | ✅ | |
| Stack set up | ✅ | |
| Rust code executes | ✅ | `adapter_main` reached |
| UART outputs text | ✅ | Direct register writes via `strb` to 0xFE660000 |
| Hardcoded memory map | ✅ | |

### Stage 2 🔲 — Full initialization

| Step | Status | Notes |
|------|--------|-------|
| `println!` macro | ❌ | Hangs — GlobalConsole lock or Write impl issue |
| FDT console init | ❌ | Hangs on DTB memory access (cache coherency) |
| FDT memory map | ❌ | Same DTB access issue |
| MMU enable | 🔲 | |
| Scheduler | 🔲 | |
| EL0 transition | 🔲 | |

### Known Issues

1. **println! hangs** — the `GlobalConsole` implementation in `boot_common` hangs when
   `println!()` is called. Likely a lock issue or borrow conflict with `static`
   `GLOBAL_CONSOLE`. Workaround: use direct UART writes.

2. **DTB cache coherency** — DTB loaded via `mm.l` while MMU was on. After MMU disable,
   the data cache has stale entries. DTB at physical address 0x0A100000 is not reliably
   readable. Fix: clean/invalidate D-cache for the DTB region after MMU disable.
   Workaround: hardcode UART init and memory map.

### Kernel Boot Signature (for verification)

After `booti 0x20500000 - 0xA100000`, expected output:
```
Starting kernel ...

─── Vivanta v0.1 ────
  Arch:      AArch64
  Platform:  Rockchip RK3568
  Memory:    3824 MiB across 1 region(s)
  CPUs:      4 core(s)
  Status:    Stage 1 ✓
```

## Debugging Notes (2026-07-17 Session)

### Kernel entry debug markers

For early bring-up, kernel entry code writes debug characters to UART:

| Marker | Location | Meaning |
|--------|----------|---------|
| `K` | After BSS zeroing | Entry code alive |
| `Z` | Before `bl adapter_main` | Rust function will be called |
| `A` | First line of `adapter_main` | Rust code executing |
| `B` | After `set_console()` | Console reference stored |

### UART byte write methods

- ✅ `asm!("strb {val:w}, [{base}]")` — works for both char-by-char and strings
- ❌ `core::ptr::write_volatile(base, b)` where base is `*mut u8` — does NOT produce output on RK3568
- ✅ Direct asm write from entry code (`mov w5, #0x4b; str w5, [x4]`) — works

### `println!` debug

`with_console` lock replaced from `AtomicBool` to `UnsafeCell<bool>` to avoid
`ldaxr`/`stlxr` exclusive access instructions that require cache coherency.
**Not yet verified** — requires clean rebuild of `boot-common` crate.

### Autoboot behavior

The board's `boot_flashkernel` reads 12 MiB from flash and tries `bootm` at five
addresses (0x20500000, 0x20520000, 0x20540000, 0x20560000, 0x20580000).
After all attempts fail, returns to U-Boot prompt.

The written kernel binary (5-20 KiB) fits easily in the 12 MiB partition.
