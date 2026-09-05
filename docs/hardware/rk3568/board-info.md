# RK3568 NVR Board — Hardware Reference

## Board Identification

U-Boot reports:
```
Model: Rockchip RK3568 NVR DEMO DDR4 Linux Board
Board Type: NVR304-32E2
Revision: RK3568/4H/4G/512M-12/2022
```

Also known as:
- Barebone RK68/4H/4G/8G (05/2024)
- Board model: **VS01NMRH VER.A** (per visual identification)
- Rockchip RK3568 Evaluation Board

## Hardware (per physical inspection)

- **SoC:** Rockchip RK3568 (4× Cortex-A55, NPU, PCIe/SATA)
- **RAM:** 4 GiB DDR4 — 2× SK hynix H5ANAG6NCJ-RXNC (16 Gbit each, 2 GB per chip)
- **eMMC:** FORESEE FEMDNN008G-58A39 (8 GB, eMMC 5.1, BGA package)
- **SATA:** 4-port PCIe-to-SATA bridge (likely JMB585 or ASM1064) with 25 MHz crystal
- **Ethernet:** 2× Gigabit with RJ45 + magnetics + 2× PHY (likely RTL8211F)
- **RTC:** 8-pin RTC/calibration EEPROM near CR2032 battery
- **Unpopulated U7:** SOP-8 footprint near SATA controller (reserved for SPI NOR/EEPROM, not installed)
- **24.000 MHz crystal:** X2 near SoC (base clock for RK3568)
- **Connectors:** Micro-USB (OTG), HDMI, 2× RJ45, 4× SATA power, FRONT panel header
- **UART header:** 4-pin white connector (GND, TX, RX, 3.3V) — silk "UART" near button

**Test points visible:** TP1, TP4 marked near U28/R452; matrix grid (A-T × 1-9) around RAM/CPU.

## SoC: Rockchip RK3568

- CPU: 4× Cortex-A55 (ARMv8.2-A, AArch64)
- RAM: 4 GiB (DDR4)
  - Region 1: 0x00200000 – 0xF0000000 (3.75 GiB)
  - Region 2: 0x1F0000000 – 0x200000000 (256 MiB)
- Boot storage: SPI NAND (128 MB)
  - Chip: sfc_nand (vendor 0xa1, device 0xe4, density 0x7f)
  - Erase block: 128 KiB (0x20000)
  - Page: 2048 bytes

### SPI NAND flash note

Even though the board visually shows only an eMMC chip (FORESEE) and the
schematic online describes "no SPI NAND", the vendor U-Boot enumerates a
`spi-nand0` device with 128 MB capacity. The SPI NAND may be:
- Hidden inside an MCP (Multi-Chip Package) on the same BGA die as eMMC
- Accessible via the same physical pads but not separately populated

In practice, the vendor U-Boot SPL lives on SPI NAND, and eMMC holds
the kernel + rootfs. The board boots from SPI NAND first; eMMC is read
later by U-Boot.

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

### Vendor U-Boot (pre-installed)

- Version: `U-Boot 2017.09-svn246980 (Nov 10 2023 - 16:11:09 +0800)`
- Vendor: Rockchip (vendor fork)
- Boot flow: SPL → U-Boot → kernel
- Script: `boot_flashkernel`
- Storage: lives on SPI NAND (offset 0x0)
- eMMC: not visible to this U-Boot (no SDHCI/eMMC driver for RK3568 in the vendor fork)

### Mainline U-Boot (rebuild 2026-09-04)

To enable eMMC support, mainline U-Boot v2026.07 was rebuilt with:

```
Source: u-boot v2026.07 (https://source.denx.de/u-boot/u-boot.git)
Defconfig: evb-rk3568_defconfig
Toolchain: aarch64-linux-gnu-gcc 13.3.0 (BuildRoot aarch64 in Lima VM)
BL31: rkbin/bin/rk35/rk3568_bl31_v1.46.elf (ATF, rockchip binary)
TPL: rkbin/bin/rk35/rk3568_ddr_1560MHz_v1.26.bin (DDR4 1560 MHz init)
SPL: enabled (CONFIG_SPL=y from defconfig)
ROCKCHIP_EXTERNAL_TPL: enabled
```

**Build commands (validated):**
```bash
cd ~/uboot-rk3568/u-boot
make mrproper
make evb-rk3568_defconfig
make CROSS_COMPILE=aarch64-linux-gnu- \
     BL31=~/rkbin/bin/rk35/rk3568_bl31_v1.46.elf \
     ROCKCHIP_TPL=~/rkbin/bin/rk35/rk3568_ddr_1560MHz_v1.26.bin \
     -j10
```

**Output artifacts:**
| File | Size | Purpose |
|------|------|---------|
| `idbloader.img` | 178,176 bytes (178 KB) | TPL + SPL + BL31, for BootROM |
| `u-boot.itb` | 1,080,832 bytes (1 MB) | FIT image: U-Boot + DTB + ATF |
| `u-boot.bin` | 883,808 bytes (884 KB) | Plain U-Boot binary (no SPL) |
| `spl/u-boot-spl.bin` | 115,539 bytes (116 KB) | SPL only |

**Known idbloader warning (benign):**
```
Image 'simple-bin' is missing optional external blobs but is still functional: tee-os
```
TEE/OS (OP-TEE) is optional; idbloader works without it.

**MMC/SDHCI/eMMC strings verified in binary:**
- `/mmc@fe310000` (eMMC)
- `/mmc@fe2b0000` (SD card)
- `rk3568_sdhci_set_clock`
- `CONFIG_CMD_MMC`
- HS200/HS400 modes, RPMB, cap-mmc-highspeed

**Boot chain on RK3568 with this build:**
```
BootROM → TPL (DDR init) → SPL (load U-Boot from flash) → BL31 (ATF) → U-Boot proper
```

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
- MMIO base: `0xFE660000` (UART0)
- Reg shift: 2 (32-bit access: `*(volatile u32*)(base + (reg << 2))`)
- **Baud rate: 1,500,000 (1.5 Mbaud) — Rockchip standard**, 8N1
  - **NOT 115200** — earlier docs were wrong. The 4-pin white "UART" header
    on the board (GND/TX/RX/3.3V) is the debug console at 1.5 Mbaud.
- Console: serial (also has HDMI, eDP video outputs)
- Host serial device: `/dev/cu.wchusbserial110` (CH340 USB-Serial)

### UART caveat: BootROM / SPL / TPL output

- **Maskrom mode:** no UART output. BootROM only listens on USB OTG.
- **SPL (DDR init):** outputs at 1.5 Mbaud (TPL/SPL configured for 1.5 Mbaud
  in the mainline build).
- **Vendor U-Boot:** outputs at 1.5 Mbaud.
- **U-Boot prompt:** `UBOOT #` (vendor fork), `=>` (mainline).

## U-Boot Boot Protocol

### Option A: uImage + bootm (default boot_flashkernel)

U-Boot has `bootm` compiled WITHOUT FDT/ATAGS support:
```
FDT and ATAGS support not compiled in - hanging
```

This means `bootm` cannot pass a device tree or ATAGs to the kernel.
**This is a blocker for bootm-based boot.**

### Option B: ARM64 Image + booti (working, validated)

U-Boot has `booti` available. The kernel must:
1. Have a valid 64-byte ARM64 Image header (magic `0x644d5241` at offset 56)
2. Be PIE-capable (bit 3 of flags = 1) or use `text_offset = 0`
3. Accept a DTB address in x0 (or handle x0 = 0)

Two working boot commands (validated 2026-07-19):

```
# With U-Boot's internal control FDT (no external DTB needed):
booti 0x20500000 - 0xebd753c0

# With external DTB loaded to fdt_addr_r:
booti 0x20500000 - 0x0a100000
```

**U-Boot control FDT** at `0xebd753c0` is always available and contains the board's
device tree (memory banks, CPUs, UART, GIC). This is the simplest boot path —
no external DTB loading required.

The DTB at `fdt_addr_r` (0x0a100000) must be written to RAM before booting.
U-Boot does NOT automatically load a DTB for `booti` on this board.

### DTB availability

U-Boot reports: `DTB: dts/kern.dtb` but then `Failed to load DTB, ret=-19`.
The board has a DTB header in flash but the data partition is missing/corrupt.

A working DTB for this board: `rk3568-nvr-demo-v12.dtb` (58 KiB).
When loaded to `fdt_addr_r`, U-Boot parses it correctly (adds memory banks, configs video).

### Option C: Serial download (fallback, validated)

No serial download commands (`loadx`, `loady`, `loadb`) are available in this U-Boot.
No `go` command either. To load data over serial, use `mm.l` (interactive memory modify)
with a Python script.

**Reliable mm.l parameters (validated 2026-07-19):**

| Parameter | Value | Notes |
|-----------|-------|-------|
| Words per chunk | 64 | Larger chunks cause UART FIFO overflow |
| Delay per word | 0.06s | Required for U-Boot to drain NS16550 64-byte FIFO |
| Delay per chunk start | 0.1s | For mm.l command echo |
| Delay per chunk end | 0.15s | For mm.l exit + U-Boot prompt sync |
| Buffer drain | `s.reset_input_buffer()` | Discard echo, no per-word reads needed |
| Total for 4.5K | ~90s | 1141 words at proven timing |
| Total for 8.5K | ~3 min | 2173 words at proven timing |

**Known failure modes:**
- **Burst writes** (>64 words without sync) → data corruption (FIFO overflow)
- **Per-word delay < 0.03s** → echo buffering causes address/value misalignment
- **TFTP** → hangs if Ethernet cable not connected (PHY auto-negotiation timeout)

**NOT available:**
- `go` command (not compiled in)
- `loadx`, `loady`, `loadb` (not compiled in)
- Fast serial download protocols

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
rust-objcopy -O binary target/aarch64-unknown-none/debug/target-rk3568 theseus-rk3568.bin

# Patch ARM64 Image header: text_offset = 0 for booti compatibility
python3 -c "
import struct
with open('theseus-rk3568.bin','r+b') as f:
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
- File served: `uImage_rk3568` (or `theseus-rk3568.uImage`)
- Runs on port 69 (requires no sudo)

### mm.l serial uploader

Located at `/tmp/flash_booti2.py`.
Protocol: sends kernel binary + DTB via interactive `mm.l` U-Boot commands.
Each word (32-bit) is sent with a 3-byte hex value + CR, synchronized on `?` prompt.
Achieves ~280 words/second at 115200 baud.

## Bring-up Status (2026-07-19 — M4.5.2 diagnostic complete)

### Stage 1 ✅ — Kernel boots, UART outputs text

| Step | Status | Details |
|------|--------|---------|
| U-Boot accepts image | ✅ | `booti 0x20500000 - 0xebd753c0` |
| Kernel entry reached | ✅ | Entry code executes at EL2 |
| MMU disabled | ✅ | Both SCTLR_EL2 and SCTLR_EL1 cleared before memory access |
| EL2→EL1 transition | ✅ | via `eret` with SPSR_EL2=0x3c5, HCR_EL2.RW=1 |
| BSS zeroed | ✅ | |
| Stack set up | ✅ | |
| Rust code executes | ✅ | `adapter_main` reached |
| UART outputs text | ✅ | Direct register writes via `strb` to 0xFE660000 |
| Hardcoded memory map | ✅ | |

### Stage 2 ✅ — Full initialization (M4.5.2 complete)

| Step | Status | Notes |
|------|--------|-------|
| CPACR_EL1 fix (EL2 path) | ✅ | FPEN=3, CPTR_EL2.TFP=0 (was bug: CPTR_EL2 set instead of CPACR_EL1) |
| `println!` / `with_console` | ✅ | Works through GlobalConsole via Ns16550 driver |
| Console trait (direct call) | ✅ | `c.write_str()` via `write_volatile` |
| FP/SIMD instructions | ✅ | `fmov d0, x6` executes without trap |
| MMU enable | 🔲 | |
| Scheduler | 🔲 | |
| FDT console init | 🔲 | Blocked on DTB cache coherency |
| FDT memory map | 🔲 | Blocked on DTB cache coherency |
| EL0 transition | 🔲 | | |

### Known Issues

1. ~~**println! hangs** — the `GlobalConsole` implementation in `boot_common` hangs when~~
   ~~`println!()` is called.~~ ✅ **RESOLVED 2026-07-19**. Root cause: CPACR_EL1.FPEN=0
   (not set in EL2 entry path) + CPTR_EL2.TFP=1 (trap all FP/SIMD to EL2). FP/SIMD
   instructions used by `write_volatile` alignment checks in debug builds caused
   silent trap → hang. Fix: `msr CPACR_EL1, x5` + `msr CPTR_EL2, xzr` in EL2 entry path.

2. **DTB cache coherency** — DTB loaded via `mm.l` while MMU was on. After MMU disable,
   the data cache has stale entries. DTB at physical address 0x0A100000 is not reliably
   readable. Fix: clean/invalidate D-cache for the DTB region after MMU disable.
   Workaround: hardcode UART init and memory map.

### RAM Persistence

- **U-Boot `reset` command**: RAM contents preserved (DDR in self-refresh)
- **Power cycle** (unplug/replug): RAM cleared
- **boot_flashkernel** reads 12 MiB from SPI NAND to 0x20500000 on each boot, overwriting
  anything previously loaded via mm.l

### Diagnostic Boot Output (validated 2026-07-19)

Minimal boot signature (with diagnostic markers):

```
Starting kernel ...

KC3FGZAB1CONSOLE OK231234WITH CONSOLE OK54X
```

| Marker | Location | Meaning |
|--------|----------|---------|
| `K` | After BSS zeroing | asm entry alive |
| `C` + digit | After TX wait | CPACR_EL1.FPEN bits 21:20 (3 = full FP access) |
| `F` | Before fmov | FP/SIMD test start |
| `G` | After fmov | FP/SIMD test passed (no trap) |
| `Z` | Before bl adapter_main | Rust function will be called |
| `A` | First line of adapter_main | Rust code executing |
| `B` | After set_console() | Console reference stored |
| `1` | Before Console trait call | Direct write_str test |
| `CONSOLE OK` | | Console trait output |
| `2` | After write_str | Direct test complete |
| `3` | Before with_console | GlobalConsole lock test |
| `12345` | Inside with_console | Lock steps (enter, get, expect, call, exit) |
| `WITH CONSOLE OK` | | with_console() output |
| `4` | After with_console | GlobalConsole test complete |
| `X` | Spin loop | Kernel alive, idle |

## Debugging Notes (2026-07-19 Session)

### CPACR_EL1 / CPTR_EL2 Fix (Root cause of all early hangs)

**Problem**: EL2 entry path (`10:` in entry code) had:

```asm
mov x5, #(0b11 << 20)
msr CPTR_EL2, x5       // BUG: sets TFP=1 (trap ALL FP/SIMD to EL2)
```

And did NOT set CPACR_EL1. After eret to EL1:
- CPTR_EL2.TFP=1 → any FP/SIMD instruction traps to EL2 (no vector table → hang)
- CPACR_EL1=0 (reset value) → FPEN=0 → would trap to EL1 anyway

**Fix** (applied 2026-07-19):

```asm
mov x5, #(0b11 << 20)
msr CPACR_EL1, x5       // FPEN=3: enable FP at EL1 and EL0
msr CPTR_EL2, xzr       // clear TFP — no FP/SIMD traps to EL2
```

**Symptom**: `write_volatile` in debug builds calls `precondition_check` →
`is_aligned_to` which uses NEON (fmov, cnt, addv) → silent trap → hang.
Release builds or `-C opt-level=z` omit these checks, which explains why QEMU
with different optimisation settings sometimes worked.

### Kernel entry debug markers

For early bring-up, kernel entry code writes debug characters to UART:

| Marker | Location | Meaning |
|--------|----------|---------|
| `K` | After BSS zeroing, before BOOT_CONTEXT | asm entry alive |
| `C` + digit | After K, CPACR_EL1 read | CPACR_EL1.FPEN bits 21:20 (0-3) |
| `F` | Before fmov d0, x6 | FP/SIMD test marker |
| `G` | After fmov d0, x6 | FP/SIMD works (no trap) |
| `Z` | Before bl adapter_main | Rust function will be called |
| `A` | First line of adapter_main | Rust code executing |
| `B` | After set_console() | Console reference stored |
| `1`–`5` | Inside with_console | Lock steps debug |
| `3` | Before with_console test | GlobalConsole lock test |
| `4` | After with_console | Test complete |
| `X` | Spin loop | Kernel alive, idle |

### UART byte write methods (validated on RK3568)

- ✅ `asm!("strb {val:w}, [{base}]")` — works (16-bit strh also works)
- ✅ `core::ptr::write_volatile(base as *mut u32, val)` — works NOW (FP/SIMD fixed)
- ❌ `core::ptr::write_volatile(base, b)` where base is `*mut u8` — 8-bit STRB not
   supported on all NS16550 implementations with reg-shift=2; 32-bit write required.
- ✅ Direct asm from entry code (`mov w5, #0x4b; str w5, [x4]`) — always works

## Flash Programming & Recovery

### USB OTG / Maskrom recovery

The board has a Micro-USB OTG port. In normal operation the BootROM tries SPI NAND
first, then falls through other boot sources. If nothing is valid it enters
**Maskrom mode** (USB VID `0x2207`, PID `0x350a`) and waits for rockusb protocol
commands on USB OTG.

**Symptoms of maskrom mode:**
- Long continuous buzzer tone (vs. short rapid beeps when U-Boot is running)
- No UART output
- USB device enumerates as "Unnamed Device" with VID 0x2207 / PID 0x350a

**Verified on macOS (Apple Silicon, Homebrew):**

```bash
# Build rkdeveloptool from source (1.32 is upstream, no RK3568 in rkflashtool)
brew install autoconf automake libusb pkg-config
cd /tmp && git clone --depth 1 https://github.com/rockchip-linux/rkdeveloptool.git
cd rkdeveloptool && ./autogen.sh
LIBUSB1_CFLAGS="$(pkg-config --cflags libusb-1.0)" \
LIBUSB1_LIBS="$(pkg-config --libs libusb-1.0)" ./configure
make CXXFLAGS="-O2 -g -Wno-vla-cxx-extension" -j4
# Binary: /tmp/rkdeveloptool/rkdeveloptool

# Verify board is detected:
./rkdeveloptool ld
#   DevNo=1 Vid=0x2207,Pid=0x350a,LocationID=1  Maskrom
```

**Mac-native recovery tools installed via Homebrew:**

```bash
brew install rkflashtool    # rkflashtool 6.1 (no RK3568 PID in device list)
brew install rkdeveloptool  # built from source above (preferred for RK3568)
brew install openocd        # for JTAG via FT232H (uses libftdi)
brew install libftdi        # FT232H userspace library
```

### Flash procedure (validated partial 2026-09-04)

1. Build mainline U-Boot + idbloader (see "Mainline U-Boot" section above).
2. Enter maskrom mode: erase SPI NAND or hold maskrom key.
3. Connect Micro-USB OTG; board enumerates as `0x2207:0x350a`.
4. Upload idbloader to board's SRAM via `db` command.
5. From there, write `u-boot.itb` to eMMC or SPI NAND via `wl` command.

**Caveat — macOS USB bulk transfer issue:** on macOS, libusb bulk transfers to
the rockusb device fail with `kIOReturnNoDevice` / pipe stalls even though
control transfers and device enumeration work. Workarounds:
- Use Linux VM (Lima) with USB passthrough
- Use a different host machine
- Use JTAG via FT232H + OpenOCD (slower but reliable)

### Recovery via JTAG (FT232H + OpenOCD)

The board has TP1/TP4 test points near U28/R452 (likely JTAG). The
FT232H module (USB-UART/I2C/SPI/JTAG) can connect to these:

```
FT232H       RK3568 JTAG
-------      -----------
AD0 (TCK) →  TCK
AD1 (TMS) →  TMS
AD2 (TDO) →  TDO
AD3 (TDI) →  TDI
GND      →  GND
```

Then OpenOCD:
```bash
openocd -f interface/ftdi/ft232h_module_swd.cfg \
        -f target/rk3568.cfg
# Halt CPU, load idbloader to 0x20500000, set PC, resume
```

The proven flash approach for this U-Boot (no flow control, no serial download protocol):

```python
for i in range(0, len(words), 64):
    chunk = words[i:i+64]
    base = ADDR + i * 4
    s.write(f'mm.l 0x{base:08x}\n'.encode())
    time.sleep(0.1)
    for v in chunk:
        s.write(f'0x{v:08x}\n'.encode())
        time.sleep(0.06)
    s.write(b'.\n')
    time.sleep(0.15)
    s.reset_input_buffer()
```

**Rules**:
- 64 words per chunk max (NS16550 64-byte FIFO limit)
- 0.06s minimum per-word delay (allows U-Boot to drain FIFO)
- 0.1s delay before chunk (mm.l startup echo)
- 0.15s delay after chunk (mm.l exit + prompt sync)
- `reset_input_buffer()` to discard echo without per-word reads
- Burst mode (>1 word without inter-word delay) causes data corruption

### Autoboot behavior

The board's `boot_flashkernel` reads 12 MiB from flash and tries `bootm` at five
addresses (0x20500000, 0x20520000, 0x20540000, 0x20560000, 0x20580000).
After all 5 attempts fail, returns to U-Boot prompt (~30 seconds total).

The written kernel binary (5-20 KiB) fits easily in the 12 MiB partition.

---

## M4.5.2 Validation (2026-07-19)

**Status:** PASS

### Validated

| Requirement | Status | Evidence |
|-------------|--------|----------|
| ARMv8 EL1 entry (EL2→EL1 via eret) | ✅ | Boots via `booti 0x20500000 - 0xebd753c0` |
| CPACR_EL1.FPEN configuration | ✅ | FPEN=3 verified by diagnostic readback |
| CPTR_EL2.TFP clearing | ✅ | fmov executes without trap on EL1 |
| GlobalConsole initialization | ✅ | `set_console()` + `with_console()` path works |
| println! output | ✅ | Boot log printed via println! |
| UART (NS16550) via write_volatile | ✅ | `*mut u32` 32-bit writes work (requires FP) |
| Console trait dispatch | ✅ | Direct and via GlobalConsole both work |
| Boot chain: asm → EL1 → Rust → Console | ✅ | Full path validated on real hardware |

### Not Yet Validated (V0/V1 territory)

| Requirement | Status | Blocked By |
|-------------|--------|------------|
| BootInfo lifecycle (ADR-021) | ❌ Pending | V0/V1 implementation |
| SystemState ownership (ADR-020) | ❌ Pending | V0/V1 implementation |
| FDT-based console init | ❌ Blocked | DTB cache coherency (needs D-cache maintenance) |
| FDT-based memory map | ❌ Blocked | Same DTB issue |
| MMU enable | ❌ Pending | Post FDT init |
| Scheduler | ❌ Pending | Post MMU + IRQ init |

### Boot Output (clean, post-cleanup)

Expected output after marker removal:
```
[VIVANTA] Boot start
[ARCH]  FP/SIMD enabled (CPACR_EL1.FPEN=3)
[OK]    M4.5.2 diagnostic complete
```
