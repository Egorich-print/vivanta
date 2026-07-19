# Memory Geometry & Multiple Page Size Support

## Motivation

Some AArch64 platforms (e.g. Apple Silicon via Asahi Linux) use 16 KiB hardware pages. Many applications (Steam, Proton, Wine, JIT runtimes, emulators) assume 4 KiB granularity. Asahi currently solves this by running such software inside a microVM with a 4 KiB guest page size. Vivanta should be designed from the start so that page size is a platform property, not a kernel-wide constant.

## MemoryGeometry Abstraction (M3-E)

Introduce a new architecture concept:

```rust
pub struct MemoryGeometry {
    /// Size of the smallest hardware page (e.g. 4096, 16384, 65536)
    pub page_size: u64,
    /// log2(page_size)
    pub page_shift: u8,
    /// page_size - 1 (alignment mask)
    pub page_mask: u64,
    /// Number of levels in the page table walk
    pub page_table_levels: u8,
    /// Supported block sizes for large pages (e.g. [2MiB, 1GiB] on AArch64)
    pub supported_block_sizes: &'static [u64],
}
```

The MMU layer must obtain page information from MemoryGeometry instead of hardcoded constants.

## Hardcoded Page Constants Inventory (M3-F)

All occurrences of 4 KiB assumptions in the codebase:

### kernel/src/pmm.rs
| Line | Code | Status |
|------|------|--------|
| 7 | `const FRAME_SIZE: u64 = 4096;` | **Arch-dependent** — replace with geometry.page_size |
| 146 | `(self.bitmap_bytes as u64 + 0xFFF) / 0x1000` | **Arch-dependent** — page rounding |
| 147 | `pages * 0x1000` | **Arch-dependent** — page-size multiply |

### kernel/src/mmu/aarch64_impl.rs
| Line | Code | Status |
|------|------|--------|
| 54 | `write_bytes(l1, 0, 4096)` | **Arch-dependent** — 4 KiB table zeroing |
| 68 | `va & 0x1F_FFFF == 0` | 2 MiB block check — **arch-specific** (AArch64 block size) |
| 73 | `offset += 0x20_0000` | 2 MiB block step — **arch-specific** |
| 81 | `offset += 0x1000` | **Arch-dependent** — page step |
| 104 | `write_bytes(frame, 0, 4096)` | **Arch-dependent** — 4 KiB table zeroing |

### kernel/src/mmu/armv7_impl.rs
| Line | Code | Status |
|------|------|--------|
| 48 | `f1 == f0 + 0x1000` | **Arch-dependent** — frame contiguity check |
| 49 | `f0 & 0x3FFF == 0` | 16 KiB alignment — **armv7-specific** (short-desc L1) |
| 59-60 | `off = f0 & 0x3FFF; to_skip = (0x4000 - off) / 0x1000` | 16 KiB alignment — **armv7-specific** |
| 65 | `write_bytes(root, 0, 0x4000)` | 16 KiB table — **armv7-specific** |
| 74 | `step_by(0x10_0000)` | 1 MiB section step — **armv7-specific** |
| 77 | `if idx < 4096` | L1 index range — **armv7-specific** (4096 entries in L1) |

### kernel/src/lib.rs
| Line | Code | Status |
|------|------|--------|
| 50 | `(kernel_end + 0xFFF) / 0x1000) * 0x1000` | **Arch-dependent** — page rounding |
| 81 | `pt.map(..., 0x1000, ...)` | Map size for UART — correct (1 page of MMIO) |

## Separate Concepts (M3-G)

Explicitly distinguish:

- **Physical Frame** — unit of physical memory allocation (may equal page_size)
- **Virtual Page** — unit of virtual memory mapping (may equal page_size)
- **Application Allocation Unit** — granularity visible to user-space

These three concepts must not be treated as identical. On a 16 KiB page system, a 4 KiB allocation request may need sub-page tracking.

## Future Compatibility (M4+)

The kernel should eventually support:
- 4 KiB pages (AArch64, ARMv7, x86-64)
- 16 KiB pages (AArch64 Apple Silicon)
- 64 KiB pages (AArch64 server, some RISC-V)

without requiring redesign. Runtime support is NOT required now.

## Future Application ABI (M5-X)

Long-term goal: applications should not need to know the hardware page size. Layering:

```
Hardware Page Size
    ↓
Virtual Memory Manager (architecture-aware)
    ↓
Application-visible allocation granularity (abstracted)
```

This does NOT guarantee full 4 KiB compatibility on a 16 KiB MMU; it only states that hardware page geometry should be isolated from higher layers wherever technically feasible.

## Roadmap

| ID | Item | Description |
|----|------|-------------|
| M3-E | MemoryGeometry abstraction | Define the struct, integrate into MMU layer |
| M3-F | Replace hardcoded page constants | Audit and replace all `4096`/`0x1000` occurrences |
| M3-G | Page-size independent PMM/VMM | Separate Physical Frame, Virtual Page, App Unit |
| M4-X | Investigation: 4 KiB compat layer | How to handle apps assuming 4 KiB pages |
| M4-Y | Research: Sub-page allocation | Internal tracking for smaller-than-page allocations |
| M5-X | Research: MicroVM compat | Optional microVM for legacy 4 KiB apps |