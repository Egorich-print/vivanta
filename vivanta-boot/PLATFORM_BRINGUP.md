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
