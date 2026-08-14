# Raspberry Pi 3B+ — UART bring-up: VERIFIED (2026-08-14)

## Result

**UART serial console on Raspberry Pi 3B+ WORKS** — demonstrated by booting a
Buildroot Linux image (BalanSir) and reaching the login prompt over the GPIO
UART (pins 8/10, GND pin 6) at 115200 8N1 via a USB-TTL adapter to a Mac.

This closes the previously open item (STATUS.md / `docs/session-summary-2026-07-17.md`:
"RPi3 UART firmware | No serial output on real hardware", Phase 0.3 blocker).

## Root cause of the earlier failure (Vivanta bare-metal attempt)

The prior attempt programmed the PL011 registers (UART0 @ 0x3F201000) directly,
expecting PL011 on the GPIO header — **without any firmware routing**. On
RPi3B+ the GPU firmware routes by default:

- **PL011 (UART0, `/dev/ttyAMA0`)** → **Bluetooth** module
- **mini-UART (UART1, `/dev/ttyS0`)** → **GPIO 14/15** (the header)

So the earlier code drove a PL011 that was not connected to the console pins.
The PL011 driver code itself was never the problem — the **board/firmware
routing** was.

## Working configuration (Buildroot/Linux, verified on hardware)

`config.txt` (boot partition):
```
enable_uart=1
dtoverlay=miniuart-bt
```
- `miniuart-bt` moves UART0 (PL011) to GPIO 14/15; Bluetooth takes the mini
  UART.
- `enable_uart=1` forces a fixed core clock (stable baud) and is required for
  the mini-UART path.

`cmdline.txt`:
```
console=tty1 console=serial0,115200 loglevel=8
```
- `console=serial0,115200` is the portable alias (firmware maps it to the
  primary UART = PL011 after the overlay).
- Last `console=` becomes `/dev/console` (getty target).

getty: `ttyAMA0 @115200` (root autologin in the dev image).

## Lesson for Vivanta

For a Vivanta bare-metal RPi3B+ UART bring-up, the GPIO UART requires the
firmware to route PL011 to the header (or use the mini-UART, which needs a
fixed `core_freq`). Two paths:

1. **Boot via GPU firmware with `config.txt` overlay** (as above) — firmware
   sets the routing before the kernel/OS runs.
2. **Bare-metal without firmware config**: drive the **mini-UART (UART1)** on
   GPIO 14/15 (its registers/clock differ from PL011), OR configure the
   BCM2837 pin mux + a fixed core clock yourself. Driving PL011 registers
   alone will not reach the header by default.
