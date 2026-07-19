# Virtual Address Space Layout

## Principles

1. **Higher-half kernel.** The kernel occupies the upper half of the virtual address space. User space occupies the lower half. This matches the convention used by Linux, x86-64, and AArch64, and simplifies page-table sharing on context switches.

2. **Direct physical map.** A direct map of all physical memory is present in the kernel address space. This allows the kernel to access any physical page by a fixed offset without creating temporary mappings. The direct map window covers the full physical address range reported in `BootInfo.memory_map`.

3. **Identity map during boot.** The boot adapter and early kernel code run identity-mapped. After VMM initialisation, the identity map may be destroyed or retained as part of the direct map.

4. **MMIO regions.** Device MMIO ranges are mapped into a dedicated kernel window, separate from the direct map. This prevents accidental execution from device memory and keeps caching attributes correct (nGnRnE on AArch64, uncacheable on x86).

5. **Guard pages.** Accessible regions are separated by unmapped guard pages where feasible. This catches out-of-bounds accesses and stack overflows early.

6. **Heap region.** The kernel heap occupies a contiguous virtual region sized at boot. The heap grows by committing additional physical pages via the VMM.

---

## Provisional Layout (AArch64, 4 KiB pages)

```
0x0000_0000_0000_0000 – 0x0000_7FFF_FFFF_FFFF   User space     (128 TiB)
0x0000_8000_0000_0000 – 0xFFFF_7FFF_FFFF_FFFF   Gap / non-canonical
0xFFFF_8000_0000_0000 – 0xFFFF_8FFF_FFFF_FFFF   Direct phys map  (64 TiB)
0xFFFF_9000_0000_0000 – 0xFFFF_9FFF_FFFF_FFFF   Kernel text/data
0xFFFF_A000_0000_0000 – 0xFFFF_AFFF_FFFF_FFFF   MMIO window
0xFFFF_B000_0000_0000 – 0xFFFF_BFFF_FFFF_FFFF   Kernel heap
0xFFFF_C000_0000_0000 – 0xFFFF_FFFF_FFFF_FFFF   Reserved
```

This layout is provisional and will be finalised when VMM implementation begins.

---

## ARMv7 (32-bit, short descriptor)

```
0x4000_0000 – 0x5FFF_FFFF   Direct phys map / kernel (identity)
0x0000_0000 – 0x3FFF_FFFF   User space (when added)
0x6000_0000 – 0xFFFF_FFFF   MMIO, heap, reserved
```

On 32-bit ARM the kernel runs identity-mapped or in a 1:1 section-mapped region. The layout is simpler because of the limited virtual address space (4 GiB total).

---

## Non-Goals (for now)

- ASLR for kernel or user space.
- Per-process address spaces (single kernel address space only).
- Managing the user half of the address space (userspace is not yet implemented).