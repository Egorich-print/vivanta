# Architecture Principle — Hardware Transparency

Hardware-specific properties (page size, cache line size, MMU geometry, interrupt controller details, etc.) should be isolated inside platform and architecture layers. Higher kernel subsystems and user applications should interact with abstract interfaces and remain unaware of hardware-specific parameters whenever this is technically possible.

## Rationale

If applied consistently, this principle enables:

- Supporting AArch64 (4/16/64 KiB), x86-64, RISC-V, and other platforms without spreading architecture-dependent assumptions throughout the kernel.
- Changing hardware parameters (e.g., switching from 4 KiB to 16 KiB pages) without modifying code above the platform/architecture layer.
- Porting to new architectures with minimal changes to core subsystems.

## Scope

This principle covers, but is not limited to:

| Property | Affected subsystems |
|----------|-------------------|
| Page size | PMM, VMM, file system cache, IPC |
| Cache line size | DMA, synchronisation primitives, data structure layout |
| MMU geometry (page table levels, block sizes) | VMM, boot code |
| Interrupt controller type (GIC, APIC, PLIC) | IRQ management, scheduler |
| Timer frequency | Scheduler, timekeeping |
| Cache coherence protocol | SMP synchronisation |

## Implementation Strategy

1. Define abstract traits or structs per hardware domain (MemoryGeometry, InterruptController, TimerInfo)
2. Implement these in architecture/platform crates
3. Core kernel code uses only the abstract interfaces
4. New architectures implement the interfaces; core code does not change