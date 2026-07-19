# M1-B Acceptance Criteria: Hardware Port — Xiaomi Redmi Note 7 (lavender)

**Objective:** Prove that the Theseus Continuity Protocol runs on real ARM hardware (Qualcomm SDM660) and can discover, adapt, and survive storage replacement using dynamic Device Tree.

**Method:** Port the QEMU-validated protocol to the Xiaomi Redmi Note 7. Replace hardcoded hardware inventory with DTB-parsed inventory.

**Prerequisite:** M3-A is complete and accepted. RFC-007 (Dynamic Device Tree) is accepted.

---

## 1. The Core Question

M1-A through M3-A proved in simulation:

> "I am the same system with the same data — including incremental changes."

M1-B must prove:

> "I am the same system with the same data — **on real hardware discovered dynamically**."

The simulation used hardcoded hardware inventories. Real hardware requires:
- Discovery: parse Device Tree at boot to learn what hardware is present
- Adaptation: record the actual hardware in State Documents
- Verification: compare old and new inventories during recovery

---

## 2. Experimental Setup

### 2.1 Hardware

| Component | Detail | Role |
|-----------|--------|------|
| Xiaomi Redmi Note 7 | lavender, SDM660 | Target platform |
| UART adapter | 3.3V USB-TTL (CP2102) | Recovery seed entry + debug output |
| SD card | Class 10, 16GB+ | Secondary boot / recovery data |
| Storage | eMMC (internal) | Primary storage for State Documents |

### 2.2 Boot Chain

```
Qualcomm PBL → SBL → ABL
    │
    ▼ loads theseus_boot.img from boot partition
Vivanta Stub
    │  validates, parses DTB, extracts inventory
    ▼
Continuity Check
    │  Genesis / Normal / Recovery
    ▼
System Boot (or halt)
```

### 2.3 Communications

- **UART only** for M1-B. No display, no touch.
- Baud rate: 115200 8N1
- Pinout: BLSP1 UART2 (test points on motherboard)

---

## 3. Experiment Sequence

### Phase 1: Boot Stub Loads + DTB Parsing

```
1. Power on device (flashed with theseus_boot.img via fastboot)
2. ABL loads boot.img from boot partition
3. Boot stub starts execution at kernel entry point
4. Boot stub locates DTB:
   - Check appended DTB in boot.img (r2 register)
   - Fallback: check dtbo partition
   - Fallback: hardcoded default
5. Parse DTB:
   - Validate magic (0xD00DFEED), size, version
   - Walk structure block
   - Extract: model, storage nodes, serial numbers
6. Print discovered hardware inventory via UART
▶ Verify: DTB parsed correctly, inventory matches actual hardware
```

### Phase 2: Identity Check (Genesis)

```
7. No State Document found on eMMC → Genesis mode
8. Generate identity seed (BIP-39)
9. Display recovery seed via UART
10. Record hardware inventory from DTB
11. Create Genesis State Document:
    - Inventory from DTB (not hardcoded)
    - Signature by Root Private Key
12. Write State Document to eMMC (dedicated partition)
▶ Verify: Genesis State Document written, can be read back, signature valid
```

### Phase 3: Normal Boot

```
13. Power cycle
14. Boot stub loads, parses DTB
15. Identity Check: State Document found on eMMC
16. Compare DTB inventory vs State Document inventory:
    - Same storage → Normal boot
    - Different storage → Recovery mode
17. Load Environment Manifest (from eMMC)
18. Verify data integrity (hash check)
▶ Verify: Normal boot proceeds on unchanged hardware
```

### Phase 4: Storage Replacement (Simulated)

```
19. Power off
20. Remove eMMC (or reprogram CID to simulate replacement)
    Alternative: use SD card as "replacement storage"
    - Copy State Document / Environment Manifest to SD card
    - Remove eMMC, boot from SD card
21. Power on
▶ Verify: No State Document found on new storage → Recovery mode
```

### Phase 5: Recovery + Continuity Proof

```
22. UART prompts: "Enter recovery seed"
23. User enters 12-word BIP-39 seed via UART
24. Keypair regenerated from seed
25. Verify: public key matches Genesis State Document
26. Discover new storage via DTB
27. Create Migration State Document:
    - New inventory from DTB
    - Links to Genesis via previous_state_hash
28. Perform continuity check:
    - State chain valid? (Genesis → Migration)
    - Keypair match?
    - Data integrity?
▶ Verify: Continuity proven on real hardware
```

---

## 4. Acceptance Criteria

### 4.1 Must Prove ( ✅ Mandatory )

| # | Criterion | Verification Method |
|---|-----------|-------------------|
| C1 | DTB is parsed correctly at boot | UART output shows discovered hardware |
| C2 | Hardware inventory is extracted from DTB | At least storage component discovered |
| C3 | Genesis State Document is created with DTB-derived inventory | Signed, written to eMMC, read back |
| C4 | Normal boot detects and verifies existing State Document | No recovery prompt on unchanged hardware |
| C5 | Storage replacement is detected (inventory mismatch) | Recovery mode triggered |
| C6 | Recovery seed can be entered via UART | 12 words accepted, keypair regenerated |
| C7 | Continuity is proven on real hardware | Exit code 0, UART shows "CONTINUITY: PROVEN" |
| C8 | Boot stub runs within 4KB of pre-allocated memory | Memory usage verified |
| C9 | DTB parsing succeeds within 1 second of power-on | Measured via UART timestamps |

### 4.2 Should Prove ( ✅ Recommended )

| # | Criterion | Importance |
|---|-----------|-----------|
| C10 | State Document persists across power cycles | Read back after reboot |
| C11 | Environment Manifest can be created and verified | M2 protocol on hardware |
| C12 | Boot stub handles missing DTB gracefully | Falls back to hardcoded default |
| C13 | UART output is printable and unambiguous | Human-readable debug |

### 4.3 Must NOT Prove ( ❌ Excluded )

| # | Non-Goal | Why Excluded |
|---|----------|-------------|
| N1 | Display or graphics output | UART-only |
| N2 | Touchscreen or input drivers | UART-only for seed entry |
| N3 | Full OS kernel | Continuity layer boot stub only |
| N4 | Filesystem | Raw block I/O for State Document storage |
| N5 | Network connectivity | N/A |
| N6 | Encryption at rest | Deferred |
| N7 | Secure boot / verified boot chain | Deferred |
| N8 | Android application compatibility | N/A |
| N9 | Power management | Full power-on only |
| N10 | Multiple boot source support | eMMC only |

---

## 5. Implementation Plan

### 5.1 New Files

| File | Purpose |
|------|---------|
| `src/dt/` | Device Tree parsing module |
| `src/dt/parser.rs` | Minimal FDT parser |
| `src/dt/types.rs` | FDT data structures |
| `src/hardware/` | Hardware discovery module |
| `src/hardware/inventory.rs` | DTB to HardwareComponent conversion |
| `src/boot/` | Boot stub (ARM64 entry) |
| `src/boot/start.S` | ARM64 assembly entry point |
| `src/boot/uart.rs` | Minimal 16550 UART driver |
| `src/boot/stub.rs` | Boot stub main logic |
| `flash.sh` | Script to flash via fastboot |
| `build/boot.img` | boot.img generation configuration |

### 5.2 Modified Files

| File | Change |
|------|--------|
| `src/state.rs` | Accept HardwareComponent from DTB parser instead of hardcoded values |
| `src/simulator.rs` | Add DTB injection mode for testing |
| `Cargo.toml` | Add ARM64 target, optional `no_std` features |
| `PROJECT_STATE.md` | Update milestone status |

### 5.3 DTB Parser Design (No External Dependencies)

```rust
// src/dt/types.rs
pub struct FdtHeader {
    pub magic: u32,         // 0xD00DFEED
    pub totalsize: u32,
    pub off_dt_struct: u32,
    pub off_dt_strings: u32,
    pub off_mem_rsvmap: u32,
    pub version: u32,
    pub last_comp_version: u32,
    pub boot_cpuid_phys: u32,
    pub size_dt_strings: u32,
    pub size_dt_struct: u32,
}

pub struct FdtNode {
    pub name: String,
    pub properties: Vec<FdtProperty>,
    pub children: Vec<FdtNode>,
}

pub struct FdtProperty {
    pub name: String,
    pub value: Vec<u8>,
}
```

The parser walks the FDT structure block in place (no allocation beyond the output inventory). This keeps memory usage minimal (target: < 4KB heap for the entire parsing operation).

### 5.4 DTB → HardwareComponent Mapping

```rust
fn dtb_to_hardware_components(fdt: &Fdt) -> Vec<HardwareComponent> {
    let mut components = Vec::new();

    // Storage: look for mmc, sdhci, sdmmc nodes
    for node in find_nodes_by_compatible(fdt, &["mmc", "sdhci", "sdmmc"]) {
        if let Some(serial) = get_property_as_string(node, "serial-number") {
            components.push(HardwareComponent {
                component_class: "storage".into(),
                vendor_id: get_property_as_string(node, "vendor").unwrap_or("unknown"),
                model_id: get_property_as_string(node, "model").unwrap_or("unknown"),
                serial_number: serial,
            });
        }
    }

    // Memory: parse /memory node reg property
    if let Some(memory_node) = find_node_by_path(fdt, "/memory") {
        // Extract size from reg property
    }

    components
}
```

### 5.5 Build Configuration

```makefile
# target: aarch64-unknown-none (or aarch64-unknown-linux-gnu for userspace)
TARGET = aarch64-unknown-none

# boot.img layout:
# +------------------+
# | kernel header    |  (Android boot image header v2)
# +------------------+
# | kernel binary    |  (Theseus boot stub, ELF → raw binary)
# +------------------+
# | ramdisk          |  (optional, unused for M1-B)
# +------------------+
# | dtb              |  (SDM660 lavender DTB)
# +------------------+
# | dtbo             |  (device tree overlay, unused for M1-B)
# +------------------+
```

Use `mkbootimg` from AOSP to package `theseus_boot.img`.

---

## 6. Success Definition

M1-B is successful if and only if:

1. All mandatory criteria (C1-C9) pass on the Xiaomi Redmi Note 7 hardware.
2. The full Genesis → Storage Death → Recovery → Continuity Proven cycle works over UART.
3. Hardware inventory is discovered from DTB, not hardcoded.
4. The boot stub operates within the resource constraints (< 4KB heap, < 1 second parsing).
5. `cargo build --target aarch64-unknown-none` produces a valid boot stub binary.

---

## 7. After M1-B

If M1-B succeeds, the next questions become:

> How does the boot stub evolve into a minimal runtime?
> Can the continuity layer support network-based recovery?

This leads to:
- **M4-A**: Minimal runtime environment (memory management, process model)
- **M4-B**: Runtime on hardware

---

## 8. Risk Mitigation

| Risk | Mitigation |
|------|-----------|
| ABL cannot be easily modified | Add U-Boot as intermediate bootloader; package in boot.img |
| eMMC CID cannot be read without kernel drivers | Use fixed serial number from DT `serial-number` property |
| UART pins not accessible with test points | Use I2C-over-USB (Fastboot) for initial debug |
| DTB parsing is too slow or memory-heavy | Minimize parser scope: only storage + memory nodes |
| `no_std` Rust complications | Minimize `no_std` surface: parser is pure data walking, no alloc |

---

*End of M1-B Acceptance Criteria*
