# Platform Validation Matrix

| Feature                 | QEMU (virt) | RK3568 | Lavender (SDM660) |
|-------------------------|:-----------:|:------:|:-----------------:|
| UART                    | ✅           | ⏳      | ⏸                 |
| FDT parsing             | ✅           | ⏳      | ⏸                 |
| Memory map              | ✅           | ⏳      | ⏸                 |
| PMM (bitmap)            | ✅           | ⏳      | ⏸                 |
| MMU (AArch64 4-level)   | ✅           | ⏳      | ⏸                 |
| Exception vectors       | ✅           | ⏳      | ⏸                 |
| Crash dump              | ✅           | ⏳      | ⏸                 |
| GIC discovery           | ✅           | ⏳      | ⏸                 |
| GIC init (v2/v3)        | ✅           | ⏳      | ⏸                 |
| SGI self-test           | ✅           | ⏳      | ⏸                 |
| Barrier API             | ✅           | ⏳      | ⏸                 |
| MMIO API                | ✅           | ⏳      | ⏸                 |
| IrqGuard                | ✅           | ⏳      | ⏸                 |
| SpinLock                | ✅           | ⏳      | ⏸                 |
| Generic Timer           | ✅           | ⬜      | ⏸                 |
| Scheduler (preemptive)  | ❌           | ⬜      | ⏸                 |
| Context switch (coop)   | ✅           | ⬜      | ⏸                 |
| Thread lifecycle        | ✅           | ⬜      | ⏸                 |
| Thread exit + cleanup   | ✅           | ⬜      | ⏸                 |
| Arch API (extern Rust)  | ✅           | ⬜      | ⏸                 |
| BootInfo contract       | ✅           | ⬜      | ⏸                 |
| MmioRegion purged       | ✅           | ⬜      | ⏸                 |
| VMM                     | ⬜           | ⬜      | ⏸                 |
| Userspace (EL0)         | ⬜           | ⬜      | ⏸                 |
| Syscalls                | ⬜           | ⬜      | ⏸                 |
| ELF loader              | ⬜           | ⬜      | ⏸                 |

**Legend**

- ✅ — verified working on this platform
- ❌ — known broken on this platform (documented limitation)
- ⏳ — not yet tested (hardware not available)
- ⏸ — postponed (blocking issue)
- ⬜ — not yet implemented

**QEMU Invocation**

```sh
qemu-system-aarch64 \
    -M virt \
    -m 512M \
    -cpu max \
    -kernel target/aarch64-unknown-none/debug/target-qemu-aarch64 \
    -nographic \
    -serial mon:stdio
```
