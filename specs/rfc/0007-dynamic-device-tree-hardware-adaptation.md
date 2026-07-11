# RFC-007: Dynamic Device Tree and Hardware Adaptation Strategy

| Field | Value |
|-------|-------|
| **Status** | Draft |
| **Replaces** | — |
| **Depends on** | RFC-001 (Identity Model), RFC-003 (Boot Protocol), RFC-005 (State Document Format), RFC-006 (Environment Continuity Model) |
| **Validated by** | M1-B (pending) |

---

## 1. Summary

RFC-007 defines how a Theseus system discovers and adapts to its hardware at boot time. The core insight: the bootloader-provided Device Tree Blob (DTB) contains a complete hardware inventory. Theseus extracts this inventory, records it in a State Document, and uses it for continuity verification during hardware replacement.

This is the bridge between the simulation (M1-A, M2-A, M3-A) and real hardware (M1-B: Xiaomi Redmi Note 7 / lavender).

---

## 2. Motivation

The continuity protocol requires a hardware inventory at every state transition:

```rust
StateDocument {
    hardware_inventory: Vec<HardwareComponent>,
    // ...
}

HardwareComponent {
    component_class: String,   // "storage", "memory", "cpu", etc.
    vendor_id: String,
    model_id: String,
    serial_number: String,
}
```

In QEMU simulation, this inventory is hardcoded:
```rust
vec![HardwareComponent {
    component_class: "storage".to_string(),
    vendor_id: "qemu".to_string(),
    model_id: "virtio-blk".to_string(),
    serial_number: "eMMC-0001-QEMU".to_string(),
}]
```

On real hardware, the inventory must be **discovered dynamically** because:
- The system may boot on different hardware (e.g., storage replaced with a different model)
- The State Document must record the actual hardware, not a hardcoded stub
- Continuity verification requires comparing old and new hardware inventories

Device Tree is the natural source of this information.

---

## 3. Architectural Approach

### 3.1 Layer Boundaries

```
Bootloader (U-Boot / ABL)
    │
    ▼  passes DTB to kernel
Early Boot (Theseus boot stub)
    │  parses DTB, extracts hardware inventory
    ▼
Identity Check (RFC-003 Stage 1)
    │  compares inventory against State Document
    ▼
Identity Resolution (RFC-003 Stage 2)
    │
    ▼
Boot Decision (RFC-003 Stage 3)
    │
    ▼
System Boot (RFC-003 Stage 4)
```

The DTB parsing happens at the Early Boot stage, BEFORE the Identity Check. This ensures the hardware inventory is available when identity is resolved.

### 3.2 DTB Parsing Scope

Theseus does NOT need to understand the full Device Tree (clocks, regulators, pins, etc.). It only extracts what it needs for continuity:

| DT Property | State Document Field | Required For |
|-------------|---------------------|--------------|
| `/model` | — | Platform identification |
| `/soc/*/mmc@*` or `/sdhci@*` | `HardwareComponent` (storage) | Storage tracking |
| `/chosen` | — | Boot arguments |
| `/memory@*` | `HardwareComponent` (memory) | Memory tracking |
| `serial-number` (any node) | `HardwareComponent.serial_number` | Component identity |

### 3.3 Hardware Adaptation Strategy

The adaptation happens at three levels:

**Level 1: Bootloader Fixups (U-Boot)**
- U-Boot's `board_fix_fdt()` hook modifies the DTB before passing it to the kernel
- Theseus can use this to add platform-specific information
- Example: inject `theseus,system-serial` property into the DTB

**Level 2: Runtime DTB Parsing (Theseus Boot Stub)**
- Minimal DTB parser reads the fixed-up DTB
- Extracts hardware inventory
- Records in State Document

**Level 3: Runtime Overlay (DTBO)**
- If hardware changes after boot (e.g., hotplug storage), a DT overlay can be applied
- Triggers a new State Document entry
- Out of scope for M1-B (cold replacement only)

---

## 4. Data Structures

### 4.1 DTB Parser Interface

```rust
pub struct DtbHardwareInventory {
    pub platform_model: String,
    pub components: Vec<HardwareComponent>,
    pub boot_arguments: String,
}

pub trait DtbParser {
    fn parse(dtb_addr: *const u8) -> Result<DtbHardwareInventory, DtbError>;
    fn extract_storage(&self) -> Vec<HardwareComponent>;
    fn extract_memory(&self) -> Vec<HardwareComponent>;
}
```

### 4.2 FDT (Flattened Device Tree) Node

```rust
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

### 4.3 Hardware Inventory to State Document Mapping

```rust
impl From<DtbHardwareInventory> for Vec<HardwareComponent> {
    fn from(inv: DtbHardwareInventory) -> Self {
        inv.components
    }
}
```

---

## 5. M1-B (Xiaomi Redmi Note 7) Specifics

### 5.1 Target Hardware Profile

| Component | Detail |
|-----------|--------|
| SoC | Qualcomm SDM660 (Snapdragon 660) |
| Storage | eMMC 5.1 (64GB) |
| Memory | LPDDR4x (4GB) |
| Bootloader | ABL (Android Bootloader) → U-Boot |
| Boot chain | PBL → SBL → ABL → boot.img → kernel |
| DTB location | Embedded in `boot.img` at `dtb` offset, or `dtbo` partition |
| UART | BLSP1 UART2 (pins 4, 5) on the test point |
| Display | MIPI DSI (not needed for M1-B — UART-only interaction) |

### 5.2 Boot Flow for Theseus

```
1. PBL → SBL → ABL (Qualcomm standard chain)
2. ABL loads `theseus_boot.img` from boot partition
3. `theseus_boot.img` contains:
   - Theseus boot stub (DTB parser + identity check)
   - Theseus continuity layer (State Document, Environment Manifest)
4. Boot stub parses DTB from `dtbo` partition (or appended)
5. DTB fixup: ABL or U-Boot adds `theseus,system-serial` property
6. Identity Check: locate State Document on eMMC
   - If found: verify against DTB inventory
   - If not found: Genesis mode
   - If mismatch: Recovery mode
7. Boot Decision → load remaining system
```

### 5.3 Dynamic Device Tree Overlay

For M1-B, the DT overlay is handled at the **bootloader level** (ABL or U-Boot), not at the kernel level:

```
ABL fixup (before boot.img is loaded):
  1. Read eMMC CID (Card Identification) → serial number
  2. Add to DTB: /chosen/theseus,storage-serial = <CID>
  3. Add to DTB: /chosen/theseus,storage-vendor = "QCOM"
  4. Add to DTB: /chosen/theseus,storage-model = "eMMC-5.1"
  5. Load boot.img with modified DTB
```

This requires modifying ABL (or adding a U-Boot shim), but ensures the DTB always contains current hardware information.

---

## 6. Implementation Plan

### 6.1 Phase 1: DTB Parser (M1-B, Rust)

Write a minimal FDT parser as a `no_std` Rust library:

```rust
// src/dt/parser.rs
pub fn parse_fdt(dtb_addr: *const u8) -> Result<Fdt, DtbError>
pub fn fdt_get_model(fdt: &Fdt) -> Option<String>
pub fn fdt_get_storage_nodes(fdt: &Fdt) -> Vec<FdtNode>
pub fn fdt_get_serial(fdt: &Fdt, node: &FdtNode) -> Option<String>
```

The parser does NOT need to understand all FDT structures. It only needs:
- Header validation (magic number, size, version)
- Structure block traversal (node/property tree)
- String block access for property names
- Value extraction (reg, status, compatible, serial-number)

### 6.2 Phase 2: Hardware Inventory Module (M1-B, Rust)

```rust
// src/hardware/inventory.rs
pub fn discover_hardware(dtb: &Fdt) -> DtbHardwareInventory
pub fn is_storage_replaced(old: &[HardwareComponent], new: &[HardwareComponent]) -> bool
pub fn serialize_inventory(inv: &[HardwareComponent]) -> String
```

### 6.3 Phase 3: Bootloader Integration (M1-B, C/assembly)

Minimal U-Boot board file or ABL patch:
```
Add DTB fixup callback that injects Theseus-specific properties.
```

### 6.4 Phase 4: Integration Test (M1-B, QEMU + simulated DTB)

Extend the simulator to inject a fake DTB and test the parsing pipeline:
```
QEMU: -dtb theseus-test.dtb
→ Theseus boot stub parses DTB
→ Extracts storage component: "virtio-blk", serial "qemu-001"
→ Creates State Document with real inventory
→ Verified against chain
```

---

## 7. Non-Goals (M1-B Exclusions)

| # | Non-Goal | Rationale |
|---|----------|-----------|
| N1 | Full OS kernel | Continuity layer only; no process management, filesystem, or drivers |
| N2 | Display / GPU | M1-B operates over UART only |
| N3 | Touch / input | No user interaction beyond seed entry via UART |
| N4 | Power management | No battery or charging management |
| N5 | Secure boot / chain of trust | Deferred to post-M1 |
| N6 | Runtime DT overlay application | Cold replacement only |
| N7 | UEFI compatibility | Qualcomm uses Android boot chain, not UEFI |
| N8 | Multiple boot source support | Boot from eMMC only |

---

## 8. Open Questions

| Question | Status |
|----------|--------|
| Should the DTB parser be `no_std` for boot stub use? | Yes — boot stub runs before Rust runtime init |
| How does the boot stub allocate memory for DTB parsing? | Use a fixed-size static buffer (4KB) |
| What if no DTB is available (legacy boot)? | Fall back to hardcoded QEMU-style inventory |
| How does recovery seed entry work via UART? | Serial console prompts for 12 words |
| Where is the State Document stored on eMMC? | Dedicated partition or file in boot partition |
| Does ABL need modification or can U-Boot be added? | Research ongoing — ABL may be patchable via boot.img header |

---

## 9. Dependencies

| Crate | Purpose | Required |
|-------|---------|----------|
| `fdt` (or custom) | Flattened Device Tree parsing | M1-B |
| `uart_16550` (or custom) | Serial I/O for recovery | M1-B |
| `crc32` | DTB header checksum verification | M1-B |
| None of these are external dependencies — implement as minimal custom code. | | |

---

## 10. References

- RFC-001: Identity Model
- RFC-003: Boot Protocol (5-stage boot sequence)
- RFC-005: State Document Format (hardware inventory)
- RFC-006: Environment Continuity Model
- M1-B Acceptance Criteria (hardware port)
- Device Tree Specification v0.4 (devicetree.org)
- Qualcomm SDM660 Device Tree bindings (Linux kernel)

---

*End of RFC-007*
