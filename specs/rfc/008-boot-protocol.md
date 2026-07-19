# RFC-008 — Vivanta Protocol (TBP)

| Field       | Value                                   |
|-------------|-----------------------------------------|
| Status      | Draft                                   |
| Type        | Interface & Architecture                |
| Layer       | Boot                                    |
| Author      | [open]                                  |
| Created     | 2026-07-11                              |

## Abstract

This RFC defines the **Vivanta Protocol (TBP)** — a stable contract between
boot adapters and the kernel.  Every boot path (UEFI, U-Boot, OpenSBI, QEMU
`-kernel`, BIOS, …) must produce a single `BootInfo` structure and hand it to
`kernel_main`.  The kernel never calls any boot-adapter code after entry.

## Motivation

Without a formal boot protocol the kernel must understand the details of every
boot environment.  This makes:

- adding a new architecture or boot method expensive (touches kernel code);
- testing different boot paths on the same platform impossible;
- the boot pipeline fragile — a change in one adapter risks breaking the kernel.

TBP decouples boot from kernel: adapters implement one interface, the kernel
consumes one interface, and both evolve independently.

## Specification

### 1. `BootInfo` — the single handoff structure

```rust
/// Every field is valid for the lifetime of `kernel_main`.
/// The adapter must not modify or free any memory after calling kernel_main.
pub struct BootInfo {
    pub arch:          Architecture,
    pub memory_map:    MemoryMap,
    pub boot_source:   BootSource,
    pub dtb:           Option<*const u8>,
    pub acpi:          Option<AcpiInfo>,
    pub framebuffer:   Option<FramebufferInfo>,
    pub cmdline:       Option<&'static str>,
    pub initrd:        Option<InitrdInfo>,
    pub cpu_count:     usize,
}

pub struct AcpiInfo {
    pub rsdp: *const u8,
}

pub struct FramebufferInfo {
    pub addr:  *mut u8,
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    pub bpp:   u8,
}

pub struct InitrdInfo {
    pub addr: *const u8,
    pub size: usize,
}
```

### 2. `Architecture`

```rust
#[non_exhaustive]
pub enum Architecture {
    AArch64,
    AArch32(LpaeMode),
    X86_64,
    Riscv64,
    Riscv32,
}

pub enum LpaeMode {
    /// Short-descriptor (no LPAE), 32-bit phys
    Short,
    /// LPAE, up to 40-bit phys
    Lpae,
}
```

### 3. `MemoryMap`

Reuse the existing structure: a fixed-capacity array of `MemoryRegion` entries
with a `MemoryRegionKind` (`Usable`, `Reserved`, `BootloaderReclaimable`,
`KernelImage`, `DeviceMemory`, `Framebuffer`).  The adapter guarantees:

- at least one `Usable` region;
- regions are sorted and non-overlapping;
- the region containing the current CPU stack is marked either `KernelImage`
  or `Usable`.

### 4. `BootSource`

```rust
pub enum BootSource {
    Uefi,
    Bios,
    Uboot,
    OpenSbi,
    QemuKernel,
    Raw,
    #[allow(non_camel_case_types)]
    ArmTrustedFirmware,
}
```

### 5. Adapter contract

Every boot adapter must:

1. **Set up a console.**  Before calling `kernel_main` at least one console
   must be active.  The `Console` trait lives in `boot_common`; the adapter
   provides its implementation.

2. **Discover physical memory.**  The adapter populates `memory_map` from
   whatever source is available (FDT, UEFI memory map, ACPI, SMBIOS, …).

3. **Reserve its own footprint.**  Any memory used by the adapter (stack,
   page tables, DTB copy, …) must be marked as `BootloaderReclaimable` or
   `Reserved` so the kernel does not use it.

4. **Prepare CPU state.**  The CPU must be in the architecturally-defined
   **kernel-entry state** (see §6).

5. **Call `kernel_main(&BootInfo)`.**  The call never returns — the kernel
   takes over completely.

### 6. Kernel-entry CPU state

| Architecture | Entry state                                                    |
|--------------|----------------------------------------------------------------|
| AArch64      | EL1, MMU off (or on with identity map), SP = stack top,        |
|              | DTB pointer in x0, `kernel_main(x0: usize)`                    |
| AArch32      | SVC mode, MMU off (or on with identity map), SP = stack top,   |
|              | DTB pointer in r0, `kernel_main(r0: usize)`                    |
| x86-64       | Long mode, page tables loaded (identity), GDT set,             |
|              | RSP = stack top, `kernel_main()` via C ABI (rdi, rsi, …)       |
| RISC-V 64    | Supervisor mode, MMU off (or on identity), SP = stack top,     |
|              | DTB pointer in a1, `kernel_main(a0: usize, a1: usize)`        |

Rationale: the MMU may be either off or on with an identity mapping because
some firmware (e.g. UEFI) may leave it on.  The adapter must ensure the
identity map covers the entry code + stack if it enables the MMU.

### 7. Adapter lifecycle

```
Platform power-on
        │
        ▼
  Firmware / existing bootloader
        │
        ▼
  ┌───────────────────────────┐
  │  Vivanta Adapter     │
  │  - arch-specific setup    │
  │  - console                │
  │  - memory discovery       │
  │  - BootInfo construction  │
  │  - call kernel_main       │
  └───────────┬───────────────┘
              │
              ▼
  ┌───────────────────────────┐
  │  kernel_main(&BootInfo)   │
  │  - never returns          │
  └───────────────────────────┘
```

### 8. Extension rules

- New fields in `BootInfo` must be `Option` (backward compatible).
- A new `Architecture` variant requires adding the corresponding adapter.
- Adapters may define additional `MemoryRegionKind` variants (the kernel must
  treat unknown kinds as `Reserved`).

## Implementation plan

1. Create `boot_common/` crate with `BootInfo`, `MemoryMap`, `Architecture`,
   `BootSource`, and the `Console` trait.
2. Create `kernel/` crate with `kernel_main(&BootInfo)`.
3. Refactor existing code into `boot/aarch64/qemu_kernel/` as an adapter.
4. Add adapters for ARMv7, x86-64, and RISC-V QEMU virt.
5. Enable MMU (M3-B) on the AArch64 path.
6. Write integration tests that run each adapter under QEMU with a minimal
   kernel that validates `BootInfo`.

## Security considerations

- `BootInfo` contains raw pointers; the kernel must validate addresses before
  dereferencing.
- An adapter could lie about the memory map; there is no protection against a
  malicious adapter.

## Alternatives considered

- **Single monolithic binary**: rejected because it ties every architecture to
  one build system and makes reuse of `boot_common` harder.
- **DTB-only handoff**: rejected because not all platforms have a device tree
  (x86 ACPI does not map to DTB naturally).
- **Separate kernel image per arch**: rejected — `kernel_main(&BootInfo)` is
  generic; only the adapter is arch-specific.

## Open questions

- Should `Console` be part of `BootInfo` or a global static set up by the
  adapter before kernel_main?  Current design: global static (via
  `boot_common::set_console`), because passing a trait object through
  `BootInfo` adds complexity for little benefit.
