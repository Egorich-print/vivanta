# Assumption Register

Every engineering assumption in the project is tracked here with its supporting evidence,
validation method, and verification status. The goal is to explicitly separate **verified
knowledge** from **working hypotheses**.

## Status Legend

| Symbol | Meaning |
|--------|---------|
| 🔶 Unverified | No evidence collected yet. Working hypothesis. |
| 🟡 Partially Verified | Some evidence supports it, but not conclusive. |
| 🟢 Verified | Strong evidence confirms the assumption. |
| 🔴 Rejected | Evidence disproves the assumption. Alternative adopted. |

## Register

### Platform — Xiaomi Redmi Note 7 (lavender, SDM660)

| ID | Assumption | Evidence | Validation Method | Status |
|----|-----------|----------|-------------------|--------|
| A-001 | UART base address = `0x0C1B0000` (BLSP1 UART0) | ❌ **REJECTED**: `earlycon=msm_serial_dm,0xc170000`, DTB alias `serial0` → `/soc/serial@0c170000` | EXP-001: DTB extraction, recovery kernel `cmdline` | 🔴 Rejected |
| | **Corrected**: UART base = `0x0C170000` | Confirmed by DTB reg `0x0C170000` size `0x100000`, compatible `qcom,msm-uartdm-v1.4` | | 🟢 Verified |
| A-002 | UART is initialized by ABL before kernel entry | `earlycon=msm_serial_dm` works at boot without kernel UART initialization | EXP-001: observe kernel cmdline earlycon behaviour | 🟡 Partially Verified |
| A-003 | Kernel load address = `0x80000000` | boot.img header reports `kernel_load_addr = 0x00008000`; ABL ignores this and loads at `0x40000000` (start of DDR on SDM660). Confirmed from `/proc/iomem`: `Kernel code` at `0x40080000 = 0x40000000 + 0x80000`. Stub is PIC, so any address works. | EXP-002: decompress kernel Image, parse header, cross-reference iomem | 🟢 Verified |
| A-004 | Entry point = load address (no offset) | ARM64 boot protocol: bootloader jumps to load address (offset 0 of Image). ABL jumps to `0x40000000` → `_start` → `b _real_start` branches to real entry. Stub uses PC-relative code. | EXP-002: ARM64 Image header analysis + boot protocol spec | 🟢 Verified |
| A-005 | ABL loads the kernel via ARM64 Linux boot protocol | boot.img format: Android v1, gzip kernel, page_size=4096, uses DTBO | EXP-001: boot.img header + partition inspection | 🟡 Partially Verified |
| A-006 | DTB is appended to the kernel image or in a separate dtbo partition | DTBO partition exists (`mmcblk0p52`); `androidboot.dtb_idx=11` selects DTB from table | EXP-001: `ls /dev/block/by-name/dtbo`, inspect `/proc/device-tree` | 🟢 Verified |
| A-007 | MSM UART DM register layout (TX at +0x00, SR at +0x08, bit 2 = TX ready) | Compatible string `qcom,msm-uartdm-v1.4` matches MSM UART DM register layout | EXP-001: DTB compatible string | 🟡 Partially Verified |
| A-008 | Exception level at kernel entry = EL1 | dmesg: `CPU: All CPU(s) started at EL1` | EXP-001: dmesg | 🟢 Verified |
| A-009 | MMU state at kernel entry = disabled | Standard ARM64 Linux boot protocol; kernel enables MMU in `__primary_switch` | EXP-001: kernel boot flow analysis | 🟢 Verified (by ARM spec) |

### M3 Memory Model

| ID | Assumption | Evidence | Validation Method | Status |
|----|-----------|----------|-------------------|--------|
| A-010 | Page table walk on SDM660 uses standard ARMv8 MMU | ARMv8 architecture reference | Hardware bringup: observe MMU enable + page walk | 🟢 Verified (by ARM spec) |
| A-011 | MemoryObject lifecycle (Created→Allocated→Mapped→Shared→Revoked) is complete for M1-B use cases | M3-C QEMU validation | Hardware bringup: verify on lavender | 🟡 Partially Verified (QEMU only) |

## Process

1. When introducing a new assumption in code or architecture, add a row to this register.
2. Assign a validation method and tag the experiment that will validate it.
3. When an experiment produces evidence, update the status.
4. If an assumption is rejected, document the alternative and keep the row for audit trail.

This register is reviewed as part of every phase gate (M1-B0 exit, M1-B1 entry, etc.).
