# Platform Bringup Guide

## Boot Flows

### 1. QEMU AArch64 (`target-qemu-aarch64`) — Working

```
QEMU -kernel → ELF loaded at 0x4020_0000 → adapter_main → kernel_main ✓
```

- Load address `0x4020_0000` (`target-qemu-aarch64/linker.ld:9`), i.e. DRAM base
  `0x4000_0000` + `0x0200_0000`, not the DRAM base itself.
- UART: PL011 at 0x09000000 (QEMU virt)
- Memory map from FDT (`-m 512M` in `tools/qemu-gates.sh:72` → 0x40000000-0x60000000)
- Console discovered via FDT scanner (`pl011` compatible)
- GIC distributor at `0x0800_0000` is published to the kernel only as an MMIO
  `MmioKind::Device` window (`target-qemu-aarch64/src/main.rs:110-114`).
  `BootInfo.interrupt_controller` is **`None`** (`…/main.rs:128`) — the GIC is
  re-derived from the FDT inside the kernel by `irq_init`
  (`arch-aarch64/src/boot.rs:169-203`), which `.expect()`s the FDT
  `interrupt-controller` node and picks v2/v3 from its `compatible` string.
  (On the default `-M virt -cpu cortex-a53` configuration that node is GICv2:
  the boot log prints `GICv2: Distributor @ 0x8000000`.)
- `BootInfo.interrupt_controller` is written by `target-x96q` and
  `target-suma-q5`, stored in `HardwareState`
  (`kernel/src/state/hardware.rs:52`) and **never read** by anything.

Status: **Boots to `kernel_main`** — on QEMU only, under the advisory gate
(`docs/tooling/ci-cd.md`). No silicon has ever run this configuration.

Note that this target carries the same FP/SIMD defect as the other four
(`msr CPTR_EL2, #(0b11<<20)` at `…/src/main.rs:31` is an architecturally
no-op; the `b 7f` at `:32` jumps past the `msr CPACR_EL1` at `:35`, so
`CPACR_EL1.FPEN` stays at reset `0b00` and traps FP/SIMD to EL1) and the gate
**still passes** — it builds the `debug` profile, which is exactly the
configuration in which `write_volatile` emits NEON. QEMU therefore does not
expose the defect; it is a silicon blocker.

### 2. RK3568 (`target-rk3568`) — EL2 diagnostic, never a kernel

```
U-Boot booti 0x20500000 → boot_entry (EL2) → UART + FDT report + test_brk → spin ✗
```

- Load address `0x2050_0000` (`target-rk3568/linker.ld:9`); U-Boot also has
  `boot_flashkernel`, which loads 12 MiB to the same address.
- UART: NS16550 at `0xFE66_0000`, reg-shift 2 (`target-rk3568/src/main.rs:128`)
- **This crate does not link `vivanta-kernel`.** Its only dependency is
  `vivanta-boot-common` (`target-rk3568/Cargo.toml:8`) — commit `3789e76`
  ("snapshot before architecture migration", 2026-07-28) deleted the
  `vivanta-kernel`, `vivanta-boot-info`, `vivanta-arch-api`,
  `vivanta-arch-aarch64` and `vivanta-platform-rk3568` dependencies and replaced
  `main.rs` with a self-contained EL2 diagnostic: its own 16-entry EL2 exception
  vector table (`…/src/main.rs:40-116`, `…/src/exceptions.rs`), MMU disabled
  via `SCTLR_EL2`, `CPACR_EL1` set and `CPTR_EL2` cleared
  (`…/src/main.rs:190-191` — the only correct implementation of the FP/SIMD
  prescription in the tree; the other five targets get both halves wrong, see
  `docs/architecture/arm-exception-state.md`). **There is no
  `kernel_main` call path in this revision** — the entry prints a banner, scans
  the FDT for memory, runs `exceptions::test_brk()` (`…/src/main.rs:173`) and
  spins in `loop {}`.
- `platform-rk3568` (the *platform* crate) is a separate thing and is **linked by
  no target at all** — it is a workspace member (`vivanta-boot/Cargo.toml:20`)
  with zero dependents. Dead code.
- Hardware evidence: real UART output from a Vivanta binary on the board,
  validated 2026-07-19 — decoded signature
  `KC3FGZAB1CONSOLE OK231234WITH CONSOLE OK54X` (`docs/hardware/rk3568/board-info.md:359-380`).
  That is the only real serial output Vivanta has ever produced on silicon, and
  it never reached `kernel_main`. **The current revision of the crate is
  untested on the board**, and the board has since been observed in **Maskrom**
  after a mainline U-Boot flash, with boot unconfirmed
  (`docs/hardware/rk3568/README.md` §Status, 2026-09-18).

### 3. SDM660 / Lavender (`target-lavender`) — Banner then spin

```
ARM64 Image header (booti) → adapter_main → short banner → spin forever ✗
```

- UART: **BLSP1_UART2 @ `0x0C17_0000`**, hardcoded as
  `platform-sdm660/src/lib.rs:9` and printed by the banner
  (`target-lavender/src/main.rs:75`). The hardware log independently confirms
  the same address and part: `BLSP1_UART2 @ 0x0C170000, 115200 8N1, TP11=TX(gpio4),
  TP10=RX(gpio5)` (`docs/hardware/lavender/uart-xbl-log.md:43`). The driver is an
  MSM UART polled on `TX_READY` at offset `0x08` with 32-bit writes.
- Load address `0x4000_0000` (`target-lavender/linker.ld:3`).
- **The target never calls `kernel_main`.** `adapter_main` inits the console,
  prints a short banner — `Vivanta Boot v0.1` / `Arch: ARM64` / `SoC: SDM660` /
  `UART: 0xc170000`, bracketed by two blank lines (`…/src/main.rs:71-76`) — and
  then `loop { spin_loop() }` (`:78-80`). `vivanta-kernel`, `vivanta-arch-api`,
  `vivanta-arch-aarch64` and `vivanta-boot-info` are declared in the
  `Cargo.toml` but **none is referenced** from the source.
- Vivanta has never run on lavender (or on whyred/SDM636). What exists is an
  external survey: `/proc/device-tree` dumps, the XBL UART capture above, pmOS
  boots, `fastboot getvar all` (`docs/hardware/lavender/`). The loader is
  U-Boot `booti` with an ARM64 Image header (`…/src/main.rs:12-26`) — **not**
  ABL/UEFI.
- The EL2 path has both halves of the FP/SIMD defect: the `msr CPTR_EL2,
  #(0b11<<20)` at `…/src/main.rs:40` is an **architecturally no-op**
  (`CPTR_EL2.TFP` is bit 10; bits 20:21 are RES0), and the `b 7f` at `:41`
  jumps past the `msr CPACR_EL1` at `:44`, so `CPACR_EL1.FPEN` stays at reset
  `0b00` and traps FP/SIMD to EL1. See the X96Q note below.

### 4. Sumavision Q5 / Amlogic S905L (`target-suma-q5`) — Cannot print

```
BROM → BL2 → BL30 → BL31 → U-Boot → booti 0x01080000 → adapter_main → panic ✗
```

- Load address `0x0108_0000` (`target-suma-q5/linker.ld:9`).
- `kernel_main` is called at `target-suma-q5/src/main.rs:175`, but the target
  never gets there: **`platform-amlogic` never calls `set_console`**
  (`platform-amlogic/src/lib.rs:105-114`), so the first `println!` reaches
  `with_console()` → `.expect("console not initialized")`
  (`boot_common/src/lib.rs:225`) and panics. The Meson UART driver is a `TODO`
  at `platform-amlogic/src/lib.rs:106` — the only `TODO` comment in the tree.
  The target's own panic handler is `loop { spin_loop() }` (`:179-183`), so the
  failure is silent.
- Even with a console, the audit rates this board not bootable on silicon:
  16 KiB boot stack, no `spec-table-desc`, and incompletely mapped GIC regions
  (`docs/audit/2026-09-24-global-audit.md`, P0 §2).
- The EL2 path has both halves of the FP/SIMD defect: the `msr CPTR_EL2,
  #(0b11<<20)` at `…/src/main.rs:60` is an **architecturally no-op**
  (`CPTR_EL2.TFP` is bit 10; bits 20:21 are RES0), and the `b 7f` at `:61`
  jumps past the `msr CPACR_EL1` at `:64`, so `CPACR_EL1.FPEN` stays at reset
  `0b00` and traps FP/SIMD to EL1. See the X96Q note below.

### 5. X96Q / Allwinner H313 (`target-x96q`) — Compile-checked only

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

`kernel_main` is called at `target-x96q/src/main.rs:181` and the boot stack is
still only **16 KiB** (`target-x96q/linker.ld:34`) — see the boot-stack blocker
below. The EL2 path has **two independent FP/SIMD defects**. (1) It writes
`msr CPTR_EL2, #(0b11<<20)` (`…/src/main.rs:52-53`) under the comment "enable
FP/SIMD", but `0b11 << 20` is the encoding of `CPACR_EL1.FPEN[21:20]`, while
**`CPTR_EL2.TFP` is bit 10** and bits 20:21 of `CPTR_EL2` are RES0 — the store
is an **architecturally no-op**, so it does not set TFP and FP/SIMD control is
left at its reset state. (2) It never reaches `msr CPACR_EL1, x5` (`:68`): the
EL2 branch performs the EL2→EL1h drop and its `eret` (`:65`) returns to label
`7f` (`:69`), jumping past label `6:` (`:66`). So **`CPACR_EL1.FPEN` stays at
reset `0b00`, which traps FP/SIMD to EL1**. Both halves must be fixed; neither
is in effect here. This same pair of defects is why the QEMU gate stays green
even though it builds the `debug` profile — see §1.
`docs/architecture/arm-exception-state.md` is the canonical description, and
`target-rk3568/src/main.rs:190-191` the only correct implementation.
**Never run on silicon as it stands.**

**MMIO regions published to kernel:**
| Device | Base | Size | Access |
|--------|------|------|--------|
| UART0 | 0x05000000 | 0x1000 | User (UserDevice) |
| GIC-400 | 0x03000000 | 0x20000 | Kernel (Device) |

### 6. Raspberry Pi 3B+ (`target-rpi3b-plus`) — kernel_main wired, awaits silicon

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
is gitignored) — mind their licence if you redistribute the image. CI unpacks
them from the tech release tagged `assets` into `images/rpi3b-plus/firmware/`
(the script's cache); locally they are fetched from upstream on first build.
The FAT assembly uses `mkfs.vfat` + `mtools`, so the identical script runs on
macOS and Linux — install the tools first:
```sh
brew install mtools dosfstools          # macOS
apt-get install -y mtools dosfstools    # Debian/Ubuntu
```

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
- **Boot stack: still 16 KiB on four of the six targets.** The 256 KiB bump
  that this board *did* get (`target-rpi3b-plus/linker.ld:34-36`,
  `target-qemu-aarch64/linker.ld:33-42`) was forced by the ~82 KiB `-O0`
  `kernel_main` frame that overflowed into `.bss`. `target-x96q/linker.ld:34`,
  `target-suma-q5/linker.ld:34`, `target-rk3568/linker.ld:36` and
  `target-lavender/linker.ld:28` still reserve only `. += 16384` — and x96q and
  suma-q5 reach `kernel_main` with that same frame. This is a live silicon
  blocker, listed as such in `docs/audit/2026-09-24-global-audit.md` (P0 §2).
- No timer IRQ without a GIC: `sched_init` runs, preemption doesn't; the G4
  gate prints SKIP instead of asserting. Everything else is expected to run.
- Phase-10 `kread` probes a QEMU-virt kernel address (`0x4020…`, see
  `kernel/src/lib.rs:1162`): on 1 GiB RAM that VA is unmapped, so the fault
  class may differ from the asserted one — read the actual `EC/DFSC/FAR` from
  the UART log when triaging.
- `CNTFRQ_EL0` is never read (no `timer_init`); if a future change touches
  the generic timer, confirm the firmware programs CNTFRQ (19.2 MHz stock).
- `gpu_mem` other than default moves the usable-RAM ceiling: adjust
  `RPI_USABLE_END` in `target-rpi3b-plus/src/main.rs`.
- The EL2 path has both halves of the FP/SIMD defect described under X96Q: it
  writes `msr CPTR_EL2, #(0b11<<20)` (`target-rpi3b-plus/src/main.rs:41-42`)
  under an "enable FP/SIMD" comment — an **architecturally no-op**, because
  `CPTR_EL2.TFP` is bit 10 and bits 20:21 are RES0 — and this target contains
  **no `msr CPACR_EL1` instruction at all**, so `CPACR_EL1.FPEN` stays at reset
  `0b00` and traps FP/SIMD to EL1. Worst case of the 5 affected targets: there
  is no second code path to fall back on. See the x96q note above and
  `docs/architecture/assumption-register.md` A-050.

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

Write `platform-<soc>/Cargo.toml` — **every crate in the workspace is
`vivanta-`-prefixed; the package name is what goes in the dependency key, and it
is not always the directory name** (`boot-info/` is the package
`vivanta-boot-info`, `boot_common/` is `vivanta-boot-common`):
```toml
[dependencies]
vivanta-boot-common = { path = "../boot_common" }
```

Write `platform-<soc>/src/lib.rs`:
- Export `init_console_from_fdt(dtb: *const u8) -> bool`
- Export `build_memory_map(dtb: *const u8) -> (MemoryMap, usize)`
- Call `FdtScanner::console()` to find UART
- Use `set_console()` to register a Console impl — **without this call every
  `println!` in the target panics** (see `target-suma-q5` above)
- Call `FdtScanner::report()` to populate the memory map

### Step 2: Create the target crate

```
mkdir -p target-<board>/src
```

Write `target-<board>/Cargo.toml`:
```toml
[dependencies]
vivanta-boot-common = { path = "../boot_common" }
vivanta-boot-info   = { path = "../boot-info" }
vivanta-arch-api    = { path = "../arch-api" }
vivanta-arch-aarch64 = { path = "../arch-aarch64" }
vivanta-platform-<soc> = { path = "../platform-<soc>" }
vivanta-kernel      = { path = "../kernel" }
```

### Step 3: Write the linker script

Use `target-<board>/linker.ld`.  Set `.` to the load address:

| Boot method | Load address formula | Live example |
|-------------|---------------------|--------------|
| U-Boot `booti` | `DRAM_BASE + TEXT_OFFSET` | x96q `0x40280000`, rpi3b+ `0x80000`, rk3568 `0x20500000`, suma-q5 `0x01080000` |
| QEMU `-kernel` (ELF) | fixed; the ELF's own `.` — **not** the DRAM base | `target-qemu-aarch64/linker.ld:9` = `0x40200000` |
| Broadcom GPU firmware | firmware convention, `kernel8.img` | `target-rpi3b-plus/linker.ld:9` = `0x80000` |

There is **no UEFI row and no PE/COFF anywhere in this repository.** ACPI and
UEFI were deliberately excluded from the boot contract — see
`vivanta-boot/docs/adr/ADR-015-arch-boundary-contracts.md:170` ("No `acpi`,
`framebuffer`, `cmdline`, `initrd` (premature)") and
`vivanta-boot/docs/architecture/current-architecture.md` ("FDT — да; ACPI/UEFI —
нет (сознательно исключены, ADR-015)").

Also reserve the boot stack here, not later: the `-O0` `kernel_main` frame plus
the gate temporaries peak around 82 KiB, and 16 KiB overflows into `.bss` with a
silent console death. Every target should use `. += 262144` with the same
comment the QEMU and RPi3B+ scripts carry.

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
  `vivanta_kernel::kernel_main()`

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
   └─ vivanta_kernel::kernel_main(&boot_info)
```

The `InterruptControllerInfo` you fill in here is copied into `HardwareState`
by `SystemState::from_boot_info` (`kernel/src/state/hardware.rs:52`) and then
**never read** — the kernel re-derives the GIC from the FDT in
`arch-aarch64/src/boot.rs:169-203`. Setting it does no harm, but do not expect
it to configure anything.

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
    <board>) PACKAGE="vivanta-target-<board>" ;;
esac
cargo build -p "${PACKAGE}"
./tools/rust-objcopy.sh -O binary target/.../vivanta-target-<board> vivanta-<board>.bin
echo "U-Boot: booti <load_addr> - \${fdt_addr_r}"
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
5. If the console was *never registered*, this is not a wiring problem:
   `with_console()` panics on `.expect("console not initialized")`
   (`boot_common/src/lib.rs:225`) and most targets' panic handlers spin
   silently. Check that your platform crate calls `set_console()` — that is the
   `target-suma-q5` failure mode, and it is indistinguishable from dead UART
   over a serial cable.

### FDT not found
1. U-Boot passes DTB in `x0` — confirm by saving x0 early in entry ASM
2. Check `fdt_addr_r` in U-Boot environment: `echo ${fdt_addr_r}`
3. Scan FDT manually: `fdt list /` in U-Boot to validate the tree

### Memory map empty
1. Check `FdtScanner::report()` — it needs `/memory` or `/memory@...` node
2. Some U-Boot builds don't fix up the memory node.  Try `fdt memory 0x40000000 0x80000000`

### Linker errors
1. Ensure `vivanta-arch-aarch64` is listed in Cargo.toml dependencies
2. `BOOT_CONTEXT` symbol must resolve — it lives in `vivanta-boot-common`
   (`boot_common/src/lib.rs:125`)
3. `__stack_top` / `__bss_start` must be defined in linker.ld
4. The target's `build.rs` must emit
   `cargo:rustc-link-arg=-Ttarget-<board>/linker.ld`; editing
   `target-<board>/.cargo/config.toml` does nothing, because every script and
   workflow runs cargo from `vivanta-boot/` and cargo only reads config from the
   invocation directory.
