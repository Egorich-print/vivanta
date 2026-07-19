# RFC-014: Hardware Graph

## Status

**Vision document** (per ADR-011).

Not frozen — actively discussed as long-term architectural direction. No
implementation planned until Device Architecture stage (Stage 8+).

## Motivation

Device Tree provides a static snapshot of hardware. Hardware Graph provides a
runtime model with:

- Device topology (bus hierarchy, power domains, clock trees)
- Device lifecycle (discovered → authorized → active → suspended)
- Dynamic relationships (PCIe hotplug, power gating, clock parenting)
- Driver binding (compatible → driver component)
- Capability-gated access (MMIO ranges, IRQ lines, DMA channels)

## Relationship with Hardware Descriptor IR

```
FDT
 │
 ▼
Hardware Descriptor IR      ← Stage 1 (flat array of nodes)
 │
 ▼
Hardware Graph              ← Stage 9+ (graph with edges, lifecycle, auth)
 │
 ▼
DeviceObject → DriverObject → Capability
```

The Hardware Descriptor IR (Stage 1) is the flat, no-lifecycle precursor.
Hardware Graph adds edges, states, and authorization — only when they are
needed.

## Requirements for revival

1. 5+ hardware device types are enumerated via Hardware IR
2. Driver API exists (Stage 8)
3. At least one dynamic bus (PCIe or USB) needs topology management
4. Power management or device hotplug is required by a use-case

## Design questions

1. Should Hardware Graph be a runtime overlay on Hardware IR, or a separate
   structure?
2. Should graph edges represent physical topology (bus) or logical topology
   (power, clock, interrupt)?
3. Should DeviceObject implement a common trait (like KernelObject) or be
   standalone?
4. Is DeviceObject identity derived from DTB path or assigned by the kernel?

## Related RFCs

- RFC-012 (Memory Object) — DeviceObject extends the object pattern to devices
- RFC-013 (Capability System) — device access gated by capabilities
- RFC-015 (Tiered Memory) — memory nodes in the hardware topology
