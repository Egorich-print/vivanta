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
| A-001 | UART base address = `0x0C1B0000` (BLSP1 UART0) | Downstream kernel DTS references, common SDM660 UART mapping | EXP-001: DTB extraction, recovery kernel `iomem` | 🔶 Unverified |
| A-002 | UART is initialized by ABL before kernel entry | ABL is known to use UART for debug output | EXP-001: observe UART output before driver init | 🔶 Unverified |
| A-003 | Kernel load address = `0x80000000` | Android boot.img defaults for ARM64, common Qualcomm base | EXP-001: inspect boot.img header | 🔶 Unverified |
| A-004 | Entry point = load address (no offset) | Linker script assumes `.text` at `0x80000000` | EXP-001: boot.img header entry point field | 🔶 Unverified |
| A-005 | ABL loads the kernel via ARM64 Linux boot protocol | Standard Android boot chain | EXP-001: verify with boot.img header version + format | 🔶 Unverified |
| A-006 | DTB is appended to the kernel image or in a separate dtbo partition | Common Qualcomm implementation | EXP-001: inspect boot.img, recovery `/proc/device-tree` | 🔶 Unverified |
| A-007 | MSM UART DM register layout (TX at +0x00, SR at +0x08, bit 2 = TX ready) | Qualcomm UART DM documentation | EXP-001: verify against DTS compatible string | 🔶 Unverified |

### M3 Memory Model

| ID | Assumption | Evidence | Validation Method | Status |
|----|-----------|----------|-------------------|--------|
| A-008 | Page table walk on SDM660 uses standard ARMv8 MMU | ARMv8 architecture reference | Hardware bringup: observe MMU enable + page walk | 🟢 Verified (by ARM spec) |
| A-009 | MemoryObject lifecycle (Created→Allocated→Mapped→Shared→Revoked) is complete for M1-B use cases | M3-C QEMU validation | Hardware bringup: verify on lavender | 🟡 Partially Verified (QEMU only) |

## Process

1. When introducing a new assumption in code or architecture, add a row to this register.
2. Assign a validation method and tag the experiment that will validate it.
3. When an experiment produces evidence, update the status.
4. If an assumption is rejected, document the alternative and keep the row for audit trail.

This register is reviewed as part of every phase gate (M1-B0 exit, M1-B1 entry, etc.).
