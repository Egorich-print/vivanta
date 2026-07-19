# Session Summary — 2026-07-17

## Milestone Status

| Milestone | Status | Tag |
|-----------|--------|-----|
| M4 — Execution Foundation | ✅ | `M4` |
| M4.4 — Address Spaces | ✅ | — |
| M4.4.5 — Execution Contract Freeze | ✅ | — |
| M4.5.0 — EL0 Transition Preparation | ✅ | `M4.5.0-el0-preparation` |
| M4.5.1 — First EL0 Entry & SVC Roundtrip | ✅ | `M4.5.1-el0-execution` |
| M4.5.1 — RK3568 Bring-up Stage 1 (UART) | ✅ | `8fa7da0` |
| M4.5.2 — Bring-up Infrastructure (println, DTB) | 🔲 | — |
| M4.6 — User Isolation & Syscall Boundary | 🔲 | — |

## Git Log (top of tree)

```
8fa7da0 feat(rk3568): first UART byte — Stage 1 bring-up complete
294a4d8 fix(rk3568): link at loadaddr 0x20500000, generate uImage for bootm
d32586e chore(M4.5.1): add milestone review document and roadmap update
5fbb4b3 feat(M4.5.1): first EL0 entry and SVC roundtrip
2bb1b59 fix(M4.5.1): add PXN bit to PageFlags
4694a10 chore(M4.5.1): remove obsolete UserBootstrap::enter()
e5bcc87 docs: add ADR-019 user page permissions
25301eb feat(M4.5.0): prepare execution context for EL0 transition
47f2e07 chore(M4.5.0.1): add EL0 transition audit checklist
2b0b320 feat(M4.4.5): execution contract freeze (ADR-017)
915846e feat(M4.4): address space isolation
```

## Architecture (Vivanta)

### Project structure

```
vivanta-boot/
├── arch-api/              # ISA contract declarations (extern "Rust")
├── arch-aarch64/          # AArch64 implementation (MMU, context, vectors, user)
├── arch-armv7a/           # Frozen stub
├── arch-test-stub/        # Test stub for build-time ISA-independence proof
├── boot-common/           # Console, println!, FDT scanner, NS16550, BootContext
├── boot-info/             # MemoryMap, BootInfo, MmioRegion
├── platform-rk3568/       # RK3568 platform init (UART from FDT, memory map)
├── platform-qemu/         # QEMU virt platform
├── kernel/                # Arch-independent: PMM, scheduler, VMM
├── target-qemu-aarch64/   # QEMU AArch64 target binary
├── target-rk3568/         # RK3568 target binary
├── target-test/           # Build-time proof (kernel + stub)
└── boot_common/           # Shared boot protocol types
```

### Execution model (ADR-017, ADR-018, ADR-019)

- Unified `context_switch()` — single mechanism for all switching
- `ExceptionFrame` never copied between thread stacks
- `ExecutionLevel` enum (Kernel/User) determines SPSR
- `InterruptGuard` in arch-api (RAII, saves/restores exact DAIF)
- `eret_to_user_stub` — ONLY EL1→EL0 transition path
- PXN/UXN enforced for user pages
- Kernel contains zero inline DAIF asm

## RK3568 Bring-up Results

### Stage 1: Boot chain proven

```
ROM → SPL → U-Boot → Vivanta → Rust entry → UART output
```

| Component | Status | Details |
|-----------|--------|---------|
| U-Boot accepts image | ✅ | `booti 0x20500000 - 0xa100000` with ARM64 Image header |
| Load address | ✅ | 0x20500000 (loadaddr=0x20500000 from U-Boot env) |
| ARM64 Image header | ✅ | PIE, text_offset=0 (patched post-build) |
| MMU disable | ✅ | Both SCTLR_EL2 and SCTLR_EL1 cleared |
| EL2→EL1 transition | ✅ | via eret with HCR_EL2.RW, SPSR=0x3c5 |
| EL1 vector table | ✅ | Minimal VBAR_EL1 (all entries: spin) |
| BSS zeroing | ✅ | |
| Stack setup | ✅ | |
| Rust entry | ✅ | `adapter_main` reached and executes |
| UART output (direct) | ✅ | `uart_putc()` via `strb` asm works |
| UART output (local fn) | ✅ | `write_uart()` via `strb` asm works |
| `set_console()` | ✅ | Reference stored |
| `with_console()` / `println!()` | ❌ | Hangs — lock issue persists even with non-atomic fix |
| DTB parsing | ❌ | Cache coherency issue after MMU disable |
| Memory map (hardcoded) | ✅ | 4 GB RAM, 4 CPUs |
| Panic handler | ✅ | spin loop |

### Known Issues

1. **`println!` hangs** — `GlobalConsole` with `UnsafeCell<bool>` lock still hangs.
   Likely the compiler generates load-exclusive/store-exclusive pair even for plain
   `bool` access on ARM64, which requires cache coherency. Fix: use a single-core
   no-lock approach or ensure boot_common crate is recompiled with fix.

2. **DTB cache coherency** — DTB loaded via mm.l while MMU was ON, cache has stale
   data after MMU disable. Need clean+invalidate D-cache for DTB region.
   Workaround: hardcoded memory map.

3. **`core::ptr::write_volatile` UART write** — writing a byte via `*mut u8`
   `write_volatile` does not produce output on real hardware. Using `asm!("strb ...")`
   works. Likely a code generation issue or bus access width issue on RK3565.

4. **Power cycle required after each boot** — No watchdog or auto-reset.
   DTR on serial adapter is not connected to board reset.

## Board Information: RK3568 NVR

See `docs/hardware/rk3568/board-info.md` for full details.

Key facts collected during this session:
- U-Boot 2017.09-svn246980 (Rockchip vendor fork), 4 GiB DDR4
- SPI NAND, kernel at 0xF80000, 12 MiB partition, 128 KiB erase block
- UART: NS16550 @ 0xFE660000, reg-shift=2, 115200 8N1
- Ethernet: JL2101 PHY on eth0 (needs cable connected)
- DTB: available as rk3568.dtb (~58 KiB), loads to fdt_addr_r=0x0a100000
- Boot with: `booti 0x20500000 - 0xa100000` (requires DTB in RAM)
- Serial transfer: `mm.l` interactive at ~290 words/sec (72 KiB kernel in 65 sec)
- TFTP: available but requires network cable

## Key Changes Made This Session

### Build system
- `build.sh rk3568`: now generates both flat binary and uImage
- Linked at 0x20500000 (matching U-Boot loadaddr)

### Entry code (`target-rk3568/src/main.rs`)
- ARM64 Image header with PIE=1, text_offset=0
- MMU disable for both EL2 and EL1
- Full EL2→EL1 transition with HCR_EL2, SPSR_EL2, eret
- Minimal VBAR_EL1 vector table
- BSS zeroing, stack init, BOOT_CONTEXT store
- Direct UART output before Rust entry (asm debug)

### Rust entry
- Hardcoded UART init (NS16550 @ 0xFE660000, reg-shift=2)
- Hardcoded memory map (4 GB, 4 CPUs)
- Working direct UART output via `strb` asm
- `write_direct()` and `with_console()` in boot_common
- Non-atomic lock attempted (not yet verified)

### ADRs added
- ADR-018: User Entry Transition Model
- ADR-019: User Page Permissions and EL0 Memory Model

### Documentation
- `docs/hardware/rk3568/board-info.md` — complete board reference
- `docs/architecture/milestones/M4.5.1-el0-execution.md` — milestone review

## RK3568 Bring-up Checklist

| Stage | Component | Status |
|-------|-----------|--------|
| 1 | U-Boot → kernel | ✅ |
| 2 | First UART byte | ✅ |
| 3 | Console (println!) | 🔲 |
| 4 | DTB parsing | 🔲 |
| 5 | MMU enable | 🔲 |
| 6 | Timer interrupt | 🔲 |
| 7 | Scheduler | 🔲 |
| 8 | Address space switch | 🔲 |
| 9 | EL0 execution | 🔲 |

## Next Steps (M4.5.2 — Bring-up Infrastructure)

Priority order:
1. **Fix `println!`** — P0: make console output work reliably
2. **Fix DTB cache coherency** — P1: clean/invalidate D-cache for DTB region
3. **Update roadmap** — P2: record M4.5.2 in evolution-plan.md
