# EXP-001 — Lavender Boot Survey

## Objective

Perform a complete hardware and boot-chain survey of the physical Xiaomi Redmi Note 7 (lavender, SDM660) before continuing M1-B0.

The goal is to replace assumptions with verified hardware information.

Do **not** modify boot partitions or flash any images.
Operate in read-only mode wherever possible.

---

## Discipline

> **Do not write or modify kernel code during EXP-001 unless the survey uncovers a verified hardware mismatch that requires an immediate fix.**

Facts first, then code. This is a Reality Lock phase requirement.

---

## Phase 1 — Host Environment Validation

Verify the development host (macOS) is ready.

Collect:

- `fastboot version`
- `adb version`
- `fastboot devices`
- `adb devices`

Report:

- whether fastboot works
- whether recovery exposes adb
- USB transport mode

---

## Phase 2 — Bootloader Survey

Collect every supported bootloader variable.

Attempt:

```
fastboot getvar all
fastboot getvar product
fastboot getvar unlocked
fastboot getvar secure
fastboot getvar current-slot
fastboot getvar slot-count
fastboot oem device-info
```

Determine:

- bootloader state
- unlock status
- slot layout
- secure boot state
- supported commands

Produce:

```
docs/hardware/lavender/bootloader.md
```

---

## Phase 3 — Recovery Survey

If ADB shell is available:

Collect:

```
/proc/cpuinfo
/proc/cmdline
/proc/meminfo
/proc/iomem
/proc/device-tree
```

Also gather:

```
dmesg
mount
cat /default.prop
getprop
```

Determine:

- kernel command line
- memory layout
- DTB availability
- UART configuration
- boot arguments
- console parameters

---

## Phase 4 — Device Tree Survey

Inspect Device Tree.

Determine:

- memory node
- cpus
- chosen
- reserved-memory
- uart
- gic
- timer

Extract:

- UART base address
- interrupt numbers
- RAM regions

Produce:

```
docs/hardware/lavender/device-tree.md
```

---

## Phase 5 — Boot Image Investigation

Without modifying the phone:

Determine:

- boot.img format
- kernel load address
- DTB placement
- entry point
- page size
- header version

Identify:

- Android Boot Image version
- vendor_boot usage
- dtbo usage

---

## Phase 6 — UART Validation

Compare current driver assumption (`0x0C1B0000`) against:

- Device Tree
- Qualcomm documentation
- recovery kernel

Determine whether the address is confirmed.

---

## Phase 7 — Engineering Report

Produce:

```
docs/hardware/lavender/EXP-001.md
```

### Assumption Register Update

For every assumption (A-001 through A-007) in the [Assumption Register](../architecture/assumption-register.md):

- Update the Status field with the evidence collected.
- If rejected, document the corrected value and re-link to the register.

### Code Audit

If any assumption is rejected:

- Identify every file that depends on that assumption.
- Decide whether to fix immediately (blocking bug) or defer to M1-B0 proper.
- Record the decision.

containing:

### Hardware

- CPU
- RAM
- MMU state
- Exception level (if determinable)

### Boot Chain

```
BootROM
  ↓
ABL
  ↓
boot.img
  ↓
Kernel entry
  ↓
Theseus stub (future)
```

### UART

- address
- interrupt
- baud assumptions

### Device Tree

- location
- loading method
- chosen node

### Memory

- physical layout
- reserved regions

### Risks

List every uncertainty preventing M1-B0 completion.

---

## Deliverables

- `docs/hardware/lavender/bootloader.md`
- `docs/hardware/lavender/device-tree.md`
- `docs/hardware/lavender/EXP-001.md`

---

## Success Criteria

- Boot chain understood.
- UART address verified.
- DTB loading method confirmed.
- Memory layout documented.
- No assumptions remain regarding early bring-up.
- Sufficient information available to continue M1-B0 on physical hardware.
