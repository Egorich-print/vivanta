# Stage 0 Migration Report

## Architecture Reset Complete

Date: 2026-07-13
ADR: ADR-011
Status: Verified

---

## Changed

### kernel/src/lib.rs

- `pub mod memory;` → commented out (frozen by ADR-011)
- Removed MemoryObject lifecycle demo (77 lines: MRM init, alloc, map, clone,
  share, revoke)
- `BootMemoryManager::new()` call wrapped in `unsafe {}` (safety contract)

### kernel/src/pmm.rs

- `PmmBitmap::init()` changed from `pub fn` to `pub unsafe fn` (takes raw ptr)
- `BootMemoryManager::new()` changed from `pub fn` to `pub unsafe fn` (takes raw ptr)
- Call to `PmmBitmap::init()` in `BootMemoryManager::new()` — inner `unsafe`
  removed (caller wraps in `unsafe`)

### kernel/src/vmm/mod.rs

- Removed: `AddrSpaceId`, `Translation`, `KERNEL_ADDR_SPACE`
- Removed: `map()` / `unmap()` / `protect()` / `translate()` stubs (all
  `unimplemented!()`)
- Removed: `pub mod address_space;` and `pub mod faults;` declarations
- Added: `AddressSpace` empty struct with `new()` and `Default`
- Kept: `pub use crate::mmu::{PageTableBuilder, PageTableGuard, PageFlags}`
  (required by kernel_main)

### kernel/src/vmm/address_space.rs

- Replaced full `KernelAddressSpace` struct + `from_guard()` + `root_addr()` +
  `PLACEHOLDER_GEOMETRY` with empty `AddressSpace` placeholder

### kernel/src/vmm/faults.rs

- Removed: `PageFaultInfo`, `FaultResolution`, `PageFaultHandler` trait,
  `PanicHandler` struct
- Added: standalone `fn handle_page_fault(virt, write, user, instr) -> !`

### kernel/src/memory/mod.rs

- Added: freeze header referencing ADR-011 and RFC-012/013/015/016
- No code changes (module is compiled out via `lib.rs`)

### kernel/src/mmu/armv7_impl.rs

- Added: freeze header ("Frozen target — ADR-011")

---

## Frozen (preserved but removed from active build)

| Path | RFC | Preconditions for revival |
|---|---|---|
| `kernel/src/memory/object.rs` | RFC-012 | VMM + page faults + userspace |
| `kernel/src/memory/capability.rs` | RFC-013 | Userspace isolation + MMU enforcement |
| `kernel/src/memory/manager.rs` | RFC-013 | 2+ backends exist |
| `kernel/src/memory/policy.rs` | RFC-016 | Multiple backends + real HW data |
| `kernel/src/memory/resource.rs` | RFC-015 | 2+ backend implementations |
| `kernel/src/memory/pmm_adapter.rs` | RFC-015 | MemoryObject revival |
| `kernel/src/mmu/armv7_impl.rs` | — | ARMv7 target reactivated |
| `boot/aarch32/qemu_virt/` | — | ARMv7 target reactivated |

---

## Removed (code paths eliminated)

- MemoryObject lifecycle demo from `kernel_main()` (~77 lines)
- VMM stubs: `map()`, `unmap()`, `protect()`, `translate()` — all
  `unimplemented!()` panic paths
- `PageFaultHandler` trait — premature abstraction without VMM
- `FaultResolution` enum — handled solely by PanicHandler
- `KernelAddressSpace` struct — replaced by empty `AddressSpace`

Nothing was deleted from disk. All frozen files preserve their source.

---

## Verified

### Build

```
cargo build -p boot-aarch64-qemu-kernel   → OK
cargo build --release -p rk3568           → OK
cargo build --release -p boot-aarch64-lavender → OK
```

### Clippy

```
cargo clippy --lib -p kernel -p boot-common -p boot-aarch64-qemu-kernel -p rk3568
→ 0 errors, 5 pre-existing warnings (div_ceil, missing_safety_doc, identity_op)
```

### Binary sizes (unchanged)

```
vivanta-rk3568.bin       2680 bytes
vivanta-qemu-kernel.bin  15100 bytes
```

### ARM64 Image header (RK3568)

```
Offset 0:  b +0x40         ✓
Offset 56: "ARMd" (magic)  ✓
```

---

## Active subsystems after Stage 0

```
BootContext     → boot_common/src/lib.rs
BootInfo        → boot_common/src/lib.rs
Console         → boot_common/src/lib.rs
MemoryMap       → boot_common/src/lib.rs
NS16550 UART    → boot_common/src/ns16550.rs
PMM (bitmap)    → kernel/src/pmm.rs
AArch64 MMU     → kernel/src/mmu/aarch64_impl.rs
RK3568 boot     → boot/platforms/rk3568/
QEMU aarch64    → boot/aarch64/qemu_kernel/
```

---

## Frozen subsystems (intent preserved in RFCs)

```
MemoryObject    → RFC-012  (ARCHITECTURAL EXPERIMENT)
Capability      → RFC-013  (ARCHITECTURAL EXPERIMENT)
Hardware Graph  → RFC-014  (Vision document)
Tiered Memory   → RFC-015  (Frozen)
Placement Policy→ RFC-016  (Frozen)
Identity Model  → RFC-001  (Design intent — active goal)
```

---

## Next

Stage 1 — RK3568 Physical Boot
