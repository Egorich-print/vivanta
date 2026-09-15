# Platform Bringup Guide

## Boot Flows

### 1. QEMU AArch64 (`target-qemu-aarch64`) — Working

```
QEMU -kernel → ELF loaded at 0x40000000 → adapter_main → kernel_main ✓
```

- UART: PL011 at 0x09000000 (QEMU virt)
- Memory map from FDT
- Console discovered via FDT scanner (`pl011` compatible)
- GICv3 at 0x08000000
- DRAM: 0x40000000 - 0x60000000 (512M)

Status: **Boots to kernel_main**

### 2. RK3568 (`target-rk3568`) — Stuck at Stage 1

```
U-Boot booti → 0x00280000 → adapter_main → println → spin loop ✗
```

- UART: NS16550 at 0xFE660000
- FDT scanned, usable memory regions detected
- **Progress**: Console works, FDT parses, memory map builds
- **Blockage**: Never calls kernel_main (Stage 1 only)

### 3. SDM660 / Lavender (`target-lavender`) — Early bringup

```
ABL (UEFI) → adapter_main → …
```

- UART: TBD (BLSP UART via GENI)
- **Status**: Platform crate exists, target binary in progress

### 4. X96Q / Allwinner H313 (`target-x96q`) — In development

```
BROM → SPL → U-Boot → booti 0x40280000 → adapter_main → kernel_main
```

| Parameter | Value |
|-----------|-------|
| SoC | Allwinner H313 (H616 family) |
| UART0 | 0x05000000, reg-shift=2, NS16550 |
| GIC | GIC-400 at 0x03000000 |
| DRAM | 0x40000000 |
| Load address | 0x40280000 (DRAM + text_offset) |
| Bootloader | U-Boot (`sun50i_h616_defconfig`) |

Console init falls back to hardcoded NS16550 at 0x05000000 if FDT console
node is not found.  This ensures output even with a missing or malformed DTB.

**MMIO regions published to kernel:**
| Device | Base | Size | Access |
|--------|------|------|--------|
| UART0 | 0x05000000 | 0x1000 | User (UserDevice) |
| GIC-400 | 0x03000000 | 0x20000 | Kernel (Device) |

### 5. Raspberry Pi 3B+ (`target-rpi3b-plus`) — kernel_main wired, awaits silicon

```
GPU firmware → kernel8.img @ 0x80000 (EL2) → EL2→EL1h drop → adapter_main → kernel_main
```

| Parameter | Value |
|-----------|-------|
| SoC | BCM2837B0 (4× Cortex-A53) |
| UART | PL011 at 0x3F201000, 250 MHz, 115200 baud (GPIO 14/15 ALT0) |
| GIC | none on this SoC — kernel runs cooperatively (no preemption) |
| DRAM | 1 GiB @ 0x0; top 64 MiB reserved for VideoCore (`RPI_USABLE_END`, assumes default `gpu_mem`) |
| Load address | 0x80000 (GPU firmware convention) |
| Descriptors | `spec-table-desc` feature (0b10 — mandatory on silicon) |

**Build the SD image (one command):**
```sh
./build.sh rpi3bp          # == tools/make-rpi3b-image.sh
```
Produces `images/rpi3b-plus/vivanta-rpi3b-plus.img` — MBR + one 64 MiB
FAT32 partition (type 0x0c, LBA 8192) holding the firmware, `config.txt`,
`overlays/miniuart-bt.dtbo` and `kernel8.img`. Flash it whole:
```sh
diskutil unmountDisk /dev/diskN
sudo dd if=images/rpi3b-plus/vivanta-rpi3b-plus.img of=/dev/rdiskN bs=4m
sync && diskutil eject /dev/diskN
```
The Broadcom firmware blobs are **downloaded, not committed** (`images/`
is gitignored) — mind their licence if you redistribute the image.
Linux has no `hdiutil`: build the same file set with `mkfs.vfat` + `mtools`
(or a Buildroot `genimage` config, as BalanSir does).

`config.txt` shipped by the script (all lines matter):
```text
arm_64bit=1
kernel=kernel8.img
enable_uart=1
dtoverlay=miniuart-bt
gpu_mem=64
disable_overscan=1
```
- `dtoverlay=miniuart-bt` is **required for serial output**: on the Pi 3 the
  PL011 (UART0) is wired to Bluetooth by default, so without it Vivanta's
  writes to `0x3F201000` go to the BT radio and GPIO 14/15 carry only the
  mini UART. The overlay's `.dtbo` must be present under `overlays/` or the
  firmware silently ignores the directive.
- `enable_uart=1` pins the VPU core clock, which is the PL011 clock source
  Vivanta assumes (250 MHz in `target-rpi3b-plus/src/main.rs`). If serial
  output is garbled, this assumption is the first suspect (`init_uart_clock`
  / actual PL011 clock), not the wire.
- `gpu_mem=64` (default) keeps usable DRAM at 960 MiB, matching
  `RPI_USABLE_END`; a larger split shrinks RAM below the model.

UART output on GPIO 14/15 (header pins 8/10) at 115200 8N1. First lines to
expect: `Vivanta Boot Adapter (RPi3B+/BCM2837)`, `Enabling MMU...` with
`L1/L2 table encoding 0b10 (spec-correct)`, then the standard gate log.

**⚠️ Feature-unification hazard:** this target enables
`vivanta-arch-aarch64/spec-table-desc` via its Cargo.toml. A bare
`cargo build --workspace` unifies that feature into *every* target of the
build — including QEMU, which hangs on 0b10. Never flash from workspace
builds; always build single-target (`-p` / `build.sh`) and rebuild QEMU
separately before validating on the emulator.

**Known gaps on this board (no silicon here to close them):**
- No timer IRQ without a GIC: `sched_init` runs, preemption doesn't; the G4
  gate prints SKIP instead of asserting. Everything else is expected to run.
- Phase-10 `kread` probes a QEMU-virt kernel address (`0x4020…`): on 1 GiB
  RAM that VA is unmapped, so the fault class may differ from the asserted
  one — read the actual `EC/DFSC/FAR` from the UART log when triaging.
- `CNTFRQ_EL0` is never read (no `timer_init`); if a future change touches
  the generic timer, confirm the firmware programs CNTFRQ (19.2 MHz stock).
- `gpu_mem` other than default moves the usable-RAM ceiling: adjust
  `RPI_USABLE_END` in `target-rpi3b-plus/src/main.rs`.

**MMIO regions published to kernel:**
| Device | Base | Size | Access |
|--------|------|------|--------|
| PL011 UART | 0x3F201000 | 0x1000 | User (UserDevice) |

---

## How to Add a New Platform

### Step 1: Create the platform crate

```
mkdir -p platform-<soc>/src
```

Write `platform-<soc>/Cargo.toml`:
```toml
[dependencies]
boot-common = { path = "../boot_common" }
```

Write `platform-<soc>/src/lib.rs`:
- Export `init_console_from_fdt(dtb: *const u8) -> bool`
- Export `build_memory_map(dtb: *const u8) -> (MemoryMap, usize)`
- Call `FdtScanner::console()` to find UART
- Use `set_console()` to register a Console impl
- Call `FdtScanner::report()` to populate the memory map

### Step 2: Create the target crate

```
mkdir -p target-<board>/src
```

Write `target-<board>/Cargo.toml`:
```toml
[dependencies]
boot-common = { path = "../boot_common" }
boot-info = { path = "../boot-info" }
arch-api = { path = "../arch-api" }
arch-aarch64 = { path = "../arch-aarch64" }
platform-<soc> = { path = "../platform-<soc>" }
kernel = { path = "../kernel" }
```

### Step 3: Write the linker script

Use `target-<board>/linker.ld`.  Set `.` to the load address:

| Boot method | Load address formula |
|-------------|---------------------|
| U-Boot `booti` | `DRAM_BASE + TEXT_OFFSET` |
| QEMU `-kernel` | `0x40000000` (DRAM base) |
| UEFI | Entry point via PE/COFF header |

### Step 4: Write the entry point

Create `target-<board>/build.rs`:
```rust
fn main() {
    println!("cargo:rustc-link-arg=-Ttarget-<board>/linker.ld");
}
```

Write `target-<board>/src/main.rs`:
- `global_asm!` with platform-specific entry (ARM64 Image header for `booti`,
  plain entry for QEMU)
- `adapter_main()` calling platform init, building BootInfo, calling
  `kernel::kernel_main()`

### Step 5: Assembly line checklist

When writing `adapter_main()`:

```
1. Platform init (console)
   └─ FDT scan → set_console() → println works
2. Memory discovery
   └─ FdtScanner::report() → MemoryMap with Usable/Reserved regions
3. MMIO regions
   └─ Static array of MmioRegion (UART, GIC, timers…)
4. Interrupt controller
   └─ InterruptControllerInfo (distributor base, optional redistributor)
5. BootInfo assembly
   └─ MemoryMap (leaked/static), MmioRegions, InterruptController
6. Handoff
   └─ kernel::kernel_main(&boot_info)
```

### Step 6: Register in workspace

Edit `Cargo.toml` (workspace root):
```toml
members = [
    …
    "platform-<soc>",
    "target-<board>",
]
```

Edit `build.sh`:
```bash
case "${ADAPTER}" in
    <board>)
        PACKAGE="target-<board>"
        cargo build -p "${PACKAGE}"
        rust-objcopy -O binary target/.../target-<board> vivanta-<board>.bin
        echo "U-Boot: booti <load_addr> - \${fdt_addr_r}"
        ;;
esac
```

### Step 7: Build and deploy

```bash
# Build
./build.sh <board>

# Copy binary to SD card / TFTP
cp vivanta-<board>.bin /path/to/boot/

# On U-Boot:
load mmc 0:1 <load_addr> vivanta-<board>.bin
booti <load_addr> - ${fdt_addr_r}
```

---

## Debugging Tips

### No output on UART
1. Verify UART base address in the SoC manual or schematic
2. Check `reg-shift` (2 for NS16550 on Allwinner/Rockchip, 0 for 8250)
3. If FDT path fails, the fallback address kicks in — verify it matches
4. For U-Boot, check `bdinfo` to confirm DRAM layout

### FDT not found
1. U-Boot passes DTB in `x0` — confirm by saving x0 early in entry ASM
2. Check `fdt_addr_r` in U-Boot environment: `echo ${fdt_addr_r}`
3. Scan FDT manually: `fdt list /` in U-Boot to validate the tree

### Memory map empty
1. Check `FdtScanner::report()` — it needs `/memory` or `/memory@...` node
2. Some U-Boot builds don't fix up the memory node.  Try `fdt memory 0x40000000 0x80000000`

### Linker errors
1. Ensure `arch-aarch64` is listed in Cargo.toml dependencies
2. `BOOT_CONTEXT` symbol must resolve — depends on `boot-common`
3. `__stack_top` / `__bss_start` must be defined in linker.ld
