# RFC-009: Platform Capability Model & BootInfo Contract

| Field | Value |
|-------|-------|
| **Status** | Draft |
| **Objective** | Formalize the contract between boot adapters and the kernel |
| **Replaces** | RFC-008 (BootInfo) — supersedes with richer structure |

---

## 1. Motivation

RFC-008 defined `BootInfo` as a flat bundle of fields. The current implementation works, but two architectures (AArch64, ARMv7) have already revealed that:

- The kernel needs structured platform data (page size, MMU model, timer model), not just a raw DTB pointer and an enum.
- `#[cfg(target_arch)]` guards in kernel code are already proliferating (see `kernel/src/mmu.rs`).
- The boundary between boot adapter responsibility and kernel responsibility is implicit.

This RFC freezes the interface so that VMM, heap, scheduler, and later subsystems build against a stable contract — not against `cfg(target_arch)`.

---

## 2. Components

### 2.1 Architecture

```rust
pub struct Architecture {
    pub isa: IsaKind,
    pub pointer_width: u8,    // 32 or 64
    pub endian: Endianness,
}

pub enum IsaKind {
    AArch64,
    AArch32,
    X86,
    X86_64,
    Riscv32,
    Riscv64,
}

pub enum Endianness {
    Little,
    Big,
}
```

MMU-specific details (page table format, levels) are in `MmuModel`, not here.

### 2.2 MmuModel

```rust
pub enum MmuModel {
    /// ARMv7-A short-descriptor (L1 section mapping, 16 KB page tables)
    ArmShortDescriptor,
    /// ARMv7-A LPAE (similar to AArch64 stage 1)
    ArmLpae,
    /// AArch64 VMSAv8-64 (4 translation levels with 4 KiB pages)
    AArch64,
    /// RISC-V Sv39 (39-bit virtual address, 3 levels)
    Sv39,
    /// RISC-V Sv48 (48-bit virtual address, 4 levels)
    Sv48,
    /// x86-64 4-level paging
    X86_64_4Level,
    /// x86-64 5-level paging
    X86_64_5Level,
    // non_exhaustive
}
```

### 2.3 MemoryGeometry

```rust
pub struct MemoryGeometry {
    pub page_size: usize,
    pub page_shift: u8,
    pub page_mask: usize,
    pub table_levels: u8,
    pub supported_block_sizes: &'static [usize],
}
```

**Immutable after kernel entry.** The kernel never modifies `MemoryGeometry`. It describes the page granularity the MMU will use for the entire session.

### 2.4 PlatformCapabilities

What the hardware can do (capability, not configuration).

```rust
pub struct PlatformCapabilities {
    pub page_sizes_supported: &'static [usize],
    pub cache_line_size: usize,
    pub has_fdt: bool,
    pub has_acpi: bool,
    pub supports_smp: bool,
    pub supports_iommu: bool,
    pub dma_coherent: bool,
}
```

### 2.5 PlatformConfiguration

What is actually in use on this particular machine (configuration, not capability).

```rust
pub struct PlatformConfiguration {
    pub mmu: MmuModel,
    pub interrupt_model: InterruptModel,
    pub timer_model: TimerModel,
}

pub enum InterruptModel {
    None,
    GicV2,
    GicV3,
    Apic,
    PLic,
    CLint,
}

pub enum TimerModel {
    None,
    ArmGenericTimer,
    LocalApic,
    CLint,
}
```

### 2.6 FirmwareInfo

```rust
pub struct FirmwareInfo {
    pub dtb: Option<*const u8>,
    pub acpi: Option<AcpiInfo>,
    pub cmdline: Option<&'static str>,
}
```

### 2.7 CpuInfo

```rust
pub struct CpuInfo {
    pub boot_cpu_id: u64,
    pub cpu_count: usize,
}
```

### 2.8 BootInfo (revised)

```rust
pub struct BootInfo {
    pub arch: Architecture,
    pub boot_source: BootSource,
    pub memory_map: &'static MemoryMap,
    pub geometry: &'static MemoryGeometry,
    pub capabilities: &'static PlatformCapabilities,
    pub config: &'static PlatformConfiguration,
    pub firmware: FirmwareInfo,
    pub cpu: CpuInfo,
    pub framebuffer: Option<FramebufferInfo>,
    pub initrd: Option<InitrdInfo>,
    pub early_console: Option<&'static dyn Console>,
}
```

**BootInfo is read-only after hand-off.** All references are `&'static`. The kernel must never mutate it.

---

## 3. EarlyConsole

Boot adapters provide an `EarlyConsole` (trait identical to the current `Console`). It exists only until the driver subsystem initialises a real console (TTY, framebuffer, serial, USB CDC, VirtIO console). After that, `early_console` is replaced by the driver-managed console.

---

## 4. Contract: Who Owns What

### Boot adapter MUST

- Put the CPU in a known state (SVC mode / EL1, stack, BSS zeroed)
- Discover memory (FDT, UEFI, ATAGs, board-specific)
- Build `MemoryMap`
- Build `PlatformCapabilities`, `PlatformConfiguration`, `MemoryGeometry`
- Provide an `EarlyConsole`
- Call `kernel::kernel_main(&boot_info)`

### Boot adapter MUST NOT

- Touch the PMM
- Touch page tables
- Allocate kernel heap memory
- Initialise kernel subsystems (scheduler, IPC, drivers)

### Kernel MUST

- Initialise PMM from `memory_map` and `geometry`
- Build page tables using `geometry` and `config.mmu`
- Provide VMM, heap, scheduler, IPC, drivers

### Kernel MUST NOT

- Hardcode `page_size`, `table_levels`, or any `MemoryGeometry` field
- Depend on platform specifics beyond what `BootInfo` provides
- Use `#[cfg(target_arch)]` in generic subsystems (PMM, VMM, heap, scheduler, IPC)

`#[cfg(target_arch)]` is permitted **only inside architecture-specific modules** (`kernel/src/mmu/aarch64_impl.rs`, `kernel/src/mmu/armv7_impl.rs`, etc.). Generic subsystems dispatch through `config.mmu`, `geometry.page_size`, etc.

---

## 5. BootInfo Stability

`BootInfo` is a **stable ABI between all boot adapters and the kernel**.

Any future architecture (LoongArch, PowerPC, MIPS, SPARC) must implement the same contract. Adding a new architecture means writing a new boot adapter; the kernel interface does not change.

Fields may be added to `BootInfo` over time, but existing fields must retain their semantic meaning. Backward-incompatible changes require a new RFC.

---

## 6. Extensibility Principle

> Adding a new architecture must require implementing a new boot adapter and platform layer. It must NOT require modifying existing kernel subsystems (PMM, VMM, heap, scheduler, IPC).

This is the primary quality metric for the architecture. If adding MIPS or RISC-V later requires changing `kernel/src/mmu.rs` dispatch logic, the architecture is broken.

---

## 7. Migration Path

1. Define the new types in `boot_common` alongside the old `BootInfo`
2. Update both boot adapters to fill the new fields
3. Update `kernel_main` to accept the new `BootInfo`
4. Remove the old `BootInfo` and `Architecture` enum
5. Remove `#[cfg(target_arch)]` dispatch from generic code — use `config.mmu` and `geometry`

Each step is a separate commit; the tree must remain buildable at every step.

---

## 8. Non-Goals

- This RFC does NOT require implementing `PlatformCapabilities` parsing in boot adapters now.
- This RFC does NOT prescribe any specific PMM, VMM, or scheduler design.
- Fields are minimal by design — add only when a concrete subsystem needs them.