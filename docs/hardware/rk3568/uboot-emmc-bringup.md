# RK3568 — Mainline U-Boot / eMMC bring-up runbook

> Sub-project runbook. Hardware reference lives in [`board-info.md`](board-info.md).

## Why

The stock vendor U-Boot (`2017.09-svn246980`) has **no eMMC (SDHCI) driver** for
RK3568, so the 8 GB eMMC is invisible. Mainline U-Boot `v2026.07`
(`evb-rk3568_defconfig`) was rebuilt with SPL + external TPL + BL31 to enable it.

## Build

Source, toolchain and blobs:

```
U-Boot:    v2026.07        https://source.denx.de/u-boot/u-boot.git
Defconfig: evb-rk3568_defconfig
Toolchain: aarch64-linux-gnu-gcc 13.3.0 (BuildRoot aarch64, Lima VM)
BL31:      rkbin/bin/rk35/rk3568_bl31_v1.46.elf            (ATF)
TPL:       rkbin/bin/rk35/rk3568_ddr_1560MHz_v1.26.bin     (DDR4 1560 MHz init)
SPL:       enabled (CONFIG_SPL=y from defconfig)
ROCKCHIP_EXTERNAL_TPL: enabled
```

Automated: [`scripts/build-mainline-uboot.sh`](scripts/build-mainline-uboot.sh).
Manual equivalent:

```bash
cd ~/uboot-rk3568/u-boot
make mrproper
make evb-rk3568_defconfig
make CROSS_COMPILE=aarch64-linux-gnu- \
     BL31=~/rkbin/bin/rk35/rk3568_bl31_v1.46.elf \
     ROCKCHIP_TPL=~/rkbin/bin/rk35/rk3568_ddr_1560MHz_v1.26.bin \
     -j10
```

### Output artifacts

| File | Size | Purpose |
|------|------|---------|
| `idbloader.img` | 178,176 B (178 KB) | TPL + SPL + BL31 — image for the BootROM |
| `u-boot.itb` | 1,080,832 B (1 MB) | FIT image: U-Boot + DTB + ATF |
| `u-boot.bin` | 883,808 B (884 KB) | Plain U-Boot binary (no SPL) |
| `spl/u-boot-spl.bin` | 115,539 B (116 KB) | SPL only |

Only `idbloader.img` (and the FIT `u-boot.itb` for non-SPL boot) is flashed.

**Benign warning:**
```
Image 'simple-bin' is missing optional external blobs but is still functional: tee-os
```
OP-TEE is optional; the idbloader works without it.

### Verified in binary

eMMC/SDHCI support is present:

- `/mmc@fe310000` (eMMC), `/mmc@fe2b0000` (SD card)
- `rk3568_sdhci_set_clock`, `CONFIG_CMD_MMC`
- HS200/HS400 modes, RPMB, `cap-mmc-highspeed`

### Boot chain

```
BootROM → TPL (DDR init) → SPL (load U-Boot from flash) → BL31 (ATF) → U-Boot proper
```

## Flashing

### A. Serial `mm.l` upload (vendor U-Boot console, proven)

The vendor U-Boot has **no** `loadx/loady/loadb` and no `go`. The only reliable
upload path is interactive `mm.l`. Use
[`scripts/flash-serial-mm.py`](scripts/flash-serial-mm.py).

Proven timing (no flow control, NS16550 64-byte FIFO):

```python
for i in range(0, len(words), 64):
    chunk = words[i:i+64]
    base = ADDR + i * 4
    s.write(f'mm.l 0x{base:08x}\n'.encode())
    time.sleep(0.1)              # mm.l startup echo
    for v in chunk:
        s.write(f'0x{v:08x}\n'.encode())
        time.sleep(0.06)         # allow FIFO drain
    s.write(b'.\n')
    time.sleep(0.15)             # mm.l exit + prompt sync
    s.reset_input_buffer()
```

Rules:
- ≤ 64 words per chunk (NS16550 FIFO limit)
- ≥ 0.06 s per word; bursts corrupt data
- `reset_input_buffer()` to discard echo without per-word reads
- Load address `0x20500000`; throughput ≈ 26 words/s

Then from the console:

```
mtd erase spi-nand0 0x0 0x100000
mtd write spi-nand0 0x20500000 0x0 0x2B800
reset
```

### B. USB OTG / Maskrom + rockusb

If nothing valid boots, the BootROM enters **Maskrom** mode: USB VID `0x2207`,
PID `0x350a`, waiting for the rockusb protocol.

Maskrom symptoms:
- Long continuous buzzer tone (vs. short beeps when U-Boot runs)
- No UART output
- USB device "Unnamed Device", VID `0x2207` / PID `0x350a`

macOS host tooling (Apple Silicon, Homebrew):

```bash
brew install autoconf automake libusb pkg-config rkflashtool openocd libftdi
cd /tmp && git clone --depth 1 https://github.com/rockchip-linux/rkdeveloptool.git
cd rkdeveloptool && ./autogen.sh
LIBUSB1_CFLAGS="$(pkg-config --cflags libusb-1.0)" \
LIBUSB1_LIBS="$(pkg-config --libs libusb-1.0)" ./configure
make CXXFLAGS="-O2 -g -Wno-vla-cxx-extension" -j4   # → ./rkdeveloptool

./rkdeveloptool ld
#   DevNo=1 Vid=0x2207,Pid=0x350a,LocationID=1  Maskrom
```

Procedure: build idbloader → enter maskrom → `rkdeveloptool db <idbloader>` to load
it into SRAM → write `u-boot.itb` to eMMC/SPI NAND → reset.

> **⚠️ macOS USB bulk-transfer bug.** `rkdeveloptool` enumerates the device and
> can send control transfers, but **bulk transfers fail** with
> `darwin_transfer_status` / `kIOReturnNoDevice` (pipe stall). `rkflashtool` 6.1
> also lacks PID `0x350a` in its device table. Workarounds:
> - run `rkdeveloptool` inside a Linux VM with USB passthrough;
> - run it on a Linux host;
> - fall back to JTAG (below).

### C. Recovery via JTAG (FT232H + OpenOCD)

TP1/TP4 near U28/R452 are the likely JTAG pads. FT232H wiring:

```
FT232H        RK3568 JTAG
-------       -----------
AD0 (TCK)  →  TCK
AD1 (TMS)  →  TMS
AD2 (TDO)  →  TDO
AD3 (TDI)  →  TDI
GND        →  GND
```

```bash
openocd -f interface/ftdi/ft232h_module_swd.cfg -f target/rk3568.cfg
# halt CPU → load idbloader to 0x20500000 → set PC → resume
```

## Findings / caveats

- The board **enumerates an SPI NAND** (`spi-nand0`, 128 MB) in the vendor U-Boot
  even though no discrete SPI NAND chip is visible (eMMC + SPI NAND likely share
  an MCP package). See `board-info.md` → "SPI NAND flash note".
- UART debug console runs at **1,500,000 baud** (Rockchip standard), not 115200.
- `bootm` in the vendor U-Boot has no FDT/ATAGS support; use `booti` for kernels.
- After flashing the mainline idbloader the board was observed in Maskrom mode;
  full boot verification is still pending.
