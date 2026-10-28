# Assumption Register

Every engineering assumption in the project is tracked here with its supporting evidence,
validation method, and verification status. The goal is to explicitly separate **verified
knowledge** from **working hypotheses**.

> **Scope note (2026-09-29).** Sections A (lavender / SDM660) and B (M3 memory
> model) are retained verbatim from 2026-07. They describe a **superseded target**
> and are kept for audit trail, not as current guidance. Section C covers the
> **currently active** boards and is the section to update. No board in section C
> has run `kernel_main`.

## Status Legend

| Symbol | Meaning |
|--------|---------|
| 🔶 Unverified | No evidence collected yet. Working hypothesis. |
| 🟡 Partially Verified | Some evidence supports it, but not conclusive. |
| 🟢 Verified | **A measurement on real hardware confirms it.** |
| 🔴 Rejected | Evidence disproves the assumption. Alternative adopted. |
| 📄 Spec-derived | An architecture specification or protocol document states it. **Not a measurement.** |

### ⚠️ The legend and its use did not match

The legend previously defined 🟢 as "Strong evidence confirms the assumption", and
four rows used it on the strength of an ARM specification rather than a
measurement. **A specification is not an observation.** Those rows are re-tagged
📄 below, and the legend now separates the two. This is a deliberate split: an
assumption that is architecturally sound but has never been executed is a
genuinely different risk from one confirmed on a board, and merging the two is
how a document like this drifts.

A-009 and A-010 previously read "🟢 Verified (by ARM spec)" — a status that
contradicted its own definition. They are now 📄 Spec-derived, and the
distinction is recorded rather than quietly tidied.

---

## Section A — Platform: Xiaomi Redmi Note 7 (lavender, SDM660)

> **Superseded target.** Vivanta has never run on this board. The assumptions
> below are about the *stock Android boot chain* on the device, gathered from
> real evidence (DTB extraction, XBL UART capture, pmOS boots,
> `fastboot getvar all`). They remain useful if lavender is ever reactivated;
> they are not evidence about Vivanta's own boot path.

| ID | Assumption | Evidence | Validation Method | Status |
|----|-----------|----------|-------------------|--------|
| A-001 | UART base address = `0x0C1B0000` (BLSP1 UART0) | ❌ **REJECTED**: `earlycon=msm_serial_dm,0xc170000`, DTB alias `serial0` → `/soc/serial@0c170000` | EXP-001: DTB extraction, recovery kernel `cmdline` | 🔴 Rejected |
| | **Corrected**: UART base = `0x0C170000` | Confirmed by DTB reg `0x0C170000` size `0x100000`, compatible `qcom,msm-uartdm-v1.4` | | 🟢 Verified |
| A-002 | UART is initialized by ABL before kernel entry | `earlycon=msm_serial_dm` works at boot without kernel UART initialization | EXP-001: observe kernel cmdline earlycon behaviour | 🟡 Partially Verified |
| A-003 | Kernel load address = `0x80000000` | boot.img header reports `kernel_load_addr = 0x00008000`; ABL ignores this and loads at `0x40000000` (start of DDR on SDM660). Confirmed from `/proc/iomem`: `Kernel code` at `0x40080000 = 0x40000000 + 0x80000`. Stub is PIC, so any address works. | EXP-002: decompress kernel Image, parse header, cross-reference iomem | 🟢 Verified |
| A-004 | Entry point = load address (no offset) | ARM64 boot protocol: bootloader jumps to load address (offset 0 of Image). ABL jumps to `0x40000000` → `_start` → `b _real_start` branches to real entry. Stub uses PC-relative code. | EXP-002: ARM64 Image header analysis + boot protocol spec | 🟢 Verified |
| A-005 | ABL loads the kernel via ARM64 Linux boot protocol | boot.img format: Android v1, gzip kernel, page_size=4096, uses DTBO | EXP-001: boot.img header + partition inspection | 🟡 Partially Verified |
| A-006 | DTB is appended to the kernel image or in a separate dtbo partition | DTBO partition exists (`mmcblk0p52`); `androidboot.dtb_idx=11` selects DTB from table | EXP-001: `ls /dev/block/by-name/dtbo`, inspect `/proc/device-tree` | 🟢 Verified |
| A-007 | MSM UART DM register layout (TX at +0x00, SR at +0x08, bit 2 = TX ready) | Compatible string `qcom,msm-uartdm-v1.4` matches MSM UART DM register layout | EXP-001: DTB compatible string | 🟡 Partially Verified |
| A-008 | Exception level at kernel entry = EL1 | dmesg: `CPU: All CPU(s) started at EL1` — **this is the stock pmOS/Linux kernel's EL, not Vivanta's** | EXP-001: dmesg | 🟢 Verified |
| A-009 | MMU state at kernel entry = disabled | Standard ARM64 Linux boot protocol; kernel enables MMU in `__primary_switch` | EXP-001: kernel boot flow analysis | 📄 Spec-derived *(was: "🟢 Verified (by ARM spec)" — re-tagged 2026-09-29)* |

## Section B — M3 memory model (not board-specific)

| ID | Assumption | Evidence | Validation Method | Status |
|----|-----------|----------|-------------------|--------|
| A-010 | Page table walk on SDM660 uses standard ARMv8 MMU | ARMv8 architecture reference | Hardware bringup: observe MMU enable + page walk | 📄 Spec-derived *(was: "🟢 Verified (by ARM spec)" — re-tagged 2026-09-29)* |
| A-011 | MemoryObject lifecycle (Created→Allocated→Mapped→Shared→Revoked) is complete for M1-B use cases | M3-C QEMU validation | Hardware bringup: verify on lavender | 🟡 Partially Verified (QEMU only) |
| A-012 | The MRM-backed kernel heap is a consumer of MRM, not a bypass | `vivanta-boot/kernel/src/memory/kernel_heap.rs:5-13` states the design; `#[global_allocator]` at `vivanta-boot/kernel/src/lib.rs:9-10`. **This assumption was open in `docs/OS_MATURITY.md` until 2026-07-24 and is now implemented.** | Code review + QEMU gate churn | 🟡 Partially Verified (QEMU only; fixed 64 KiB heap, `MemoryCapability::check()` is a declared placeholder returning `true`) |

## Section C — Currently active boards

> Read this section first. Sections A and B describe a board Vivanta has never
> run on; this one describes the targets the project is actually working toward.
>
> **No board in this section has executed `kernel_main`.** The only silicon data
> point in the project is the RK3568 EL2-stage diagnostic (A-020). Gate
> definitions: `vivanta-boot/tools/qemu-gates.sh:28-47`.

### C.1 — Rockchip RK3568 (NVR board, Cortex-A55 ×4)

> Evidence tier 1: real UART output from a Vivanta binary on hardware. It stops
> at the EL2 diagnostic stage — `target-rk3568` does not link `vivanta-kernel`
> at all. Board is currently in Maskrom after a mainline U-Boot flash.

| ID | Assumption | Evidence | Validation Method | Status |
|----|-----------|----------|-------------------|--------|
| A-020 | RK3568 executes Vivanta code and can drive the on-board UART | Decoded UART signature `KC3FGZAB1CONSOLE OK231234WITH CONSOLE OK54X`, 2026-07-19. Source: `docs/hardware/rk3568/board-info.md:409-437` | Observe UART via USB-TTL at 1500000 baud | 🟢 Verified *(EL2 stage only — `kernel_main` never reached)* |
| A-021 | Kernel load address = `0x20500000` | U-Boot `loadaddr` = `0x20500000`; `boot_flashkernel` reads 12 MiB from SPI NAND offset `0xf80000`; works via `booti 0x20500000 - 0xebd753c0`. Source: `docs/hardware/rk3568/board-info.md:97-110`, `:193` | `booti` with a known payload | 🟢 Verified |
| A-022 | Boot entry EL = EL2 | `vivanta-boot/target-rk3568` runs its whole diagnostic at EL2 and never drops to EL1. The `CPACR_EL1`/`CPTR_EL2` fix at `vivanta-boot/target-rk3568/src/main.rs:190-191` is written for an EL2→EL1 path that this target does not take | Observe `CurrentEL` on the board | 🟡 Partially Verified *(inferred from the fact the EL2 path is the only one that works; never read back on hardware)* |
| A-023 | `CPACR_EL1.FPEN=0b11` + `CPTR_EL2.TFP=0` fixes the early hangs | Markers `C3` (FPEN=3 readback) and `Z` (Rust call about to be made) both appear in the captured signature. Source: `docs/hardware/rk3568/board-info.md:422-425`, `:519-520` | Register readback printed as a marker | 🟢 Verified *(on `target-rk3568`, the only target that writes the correct values — see A-050)* |
| A-024 | RK3568 boot reaches the MMU / scheduler stages | ❌ **REJECTED.** The bring-up log leaves `MMU enable` and `Scheduler` unchecked; the target never links the kernel. Source: `docs/architecture/master-roadmap.md:275-279` | Full `kernel_main` boot on the board | 🔴 Rejected |
| A-025 | The mainline U-Boot v2026.07 eMMC bring-up left the board bootable | Board last observed in **Maskrom** after the flash. Boot outcome unknown. Runbook: `docs/hardware/rk3568/uboot-emmc-bringup.md` | Power cycle, capture UART | 🔶 Unverified |

### C.2 — Raspberry Pi 3B+ (BCM2837B0, Cortex-A53 ×4)

> **No Vivanta code has run on this board.** The 2026-08-14 verification was
> booting *BalanSir's Buildroot Linux* — a different project's image. It
> established the board's UART routing, which is real and useful, but it says
> nothing about this kernel. `target-rpi3b-plus` links the kernel and calls
> `kernel_main` at `vivanta-boot/target-rpi3b-plus/src/main.rs:132`.

| ID | Assumption | Evidence | Validation Method | Status |
|----|-----------|----------|-------------------|--------|
| A-030 | The Pi 3B+ GPIO header UART is the **mini-UART** (UART1), not PL011 | Verified on hardware with Buildroot 2026-08-14: PL011 (`/dev/ttyAMA0`) routes to the Bluetooth module; mini-UART (`/dev/ttyS0`) routes to GPIO 14/15. Source: `docs/hardware/rpi3b/uart-bringup-verified.md:14-23` | Boot with `dtoverlay=miniuart-bt`, `enable_uart=1`; observe console | 🟢 Verified *(verified for Buildroot/Linux routing; the Vivanta path depends on the same firmware config)* |
| A-031 | `vivanta-platform-rpi3b` programs the correct GPIO function for the mini-UART | `init_uart_gpio` sets GPFSEL1 to function `4` on pins 14/15
(`vivanta-boot/platform-rpi3b/src/lib.rs:10-24`), which is the mini-UART alt function. This is consistent with A-030 | Code review; **no hardware run** | 🟡 Partially Verified *(code consistent with the verified board behaviour; never executed)* |
| A-032 | A bare-metal Vivanta image can reach the GPIO console without firmware configuration | ❌ **REJECTED as stated.** The verified bringup shows driving PL011 registers alone will not reach the header by default; the firmware routes it. `vivanta-boot/target-rpi3b-plus/src/main.rs:16` uses `PL011_BASE = 0x3F201000`. Source: `docs/hardware/rpi3b/uart-bringup-verified.md:47-58` | Flash a Vivanta image and observe the header pins | 🔴 Rejected *(as an unqualified claim; may still work with the GPU firmware + `config.txt` overlay path, which is untested)* |
| A-033 | `target-rpi3b-plus` boots to `kernel_main` and survives a timer interrupt | Never run. The target also carries the FP/SIMD defect of A-050 in its worst form — a no-op `msr CPTR_EL2, #(0b11<<20)` at `:42` and **no `msr CPACR_EL1` instruction anywhere**, so `CPACR_EL1.FPEN` stays at reset `0b00` and traps FP/SIMD to EL1 — and is listed as `compile-checked only` in the audit | Power on, capture UART | 🔶 Unverified |
| A-034 | RPi firmware images can be fetched reproducibly | CI downloads from floating `raspberrypi/firmware@master` with **no checksums**; `mkfs.vfat` runs without `--invariant` | Re-fetch and compare hashes across two days | 🔴 Rejected *(non-reproducible by construction)* |

### C.3 — X96Q TV box (Allwinner H313, Cortex-A53 ×4)

> Compile-checked only. Blocked on silicon by a 16 KiB boot stack and by the
> FP/SIMD defect (A-050).

| ID | Assumption | Evidence | Validation Method | Status |
|----|-----------|----------|-------------------|--------|
| A-040 | DRAM base = `0x40000000`, text_offset = `0x00280000`, load address = `0x40280000` | Stated in `vivanta-boot/target-x96q/src/main.rs:9` and as the U-Boot command `load mmc 0:1 0x40280000` (`:6-7`). **Source is the target's own header comment — no DTB, no vendor doc, no measurement** | Read the board's DTB; confirm against U-Boot's own `bdinfo` | 🔶 Unverified *(self-asserted; the origin of the number is not recorded anywhere in the repo)* |
| A-041 | UART0 at `0x05000000`, NS16550-compatible, reg_shift = 2 | `vivanta-boot/platform-allwinner-h616/src/lib.rs:8`, `:21`. Not read from this board's DTB | Dump the DTB `serial@` node and compare `reg` | 🔶 Unverified |
| A-042 | GIC-400 (GICv2) at `0x03000000` | `vivanta-boot/target-x96q/src/main.rs:10`. Not read from the DTB; the audit notes GIC regions are not fully mapped on this target (`docs/audit/2026-09-24-global-audit.md:66-70`) | Dump the DTB `interrupt-controller@` node | 🔶 Unverified |
| A-043 | The X96Q boot stack of 16 KiB is sufficient | ❌ **REJECTED.** `vivanta-boot/target-x96q/linker.ld:34` allocates 16384 bytes while the same ~82 KiB `-O0` frame forced qemu and rpi3b-plus up to 256 KiB. `target-x96q` does reach `kernel_main`, so the same overflow applies. Source: `vivanta-boot/target-x96q/linker.ld:34` vs `vivanta-boot/target-qemu-aarch64/linker.ld:41` | Raise the stack to 256 KiB and boot | 🔴 Rejected |
| A-044 | X96Q boots to `kernel_main` on silicon | Never run | `booti 0x40280000 - ${fdt_addr_r}`, capture UART | 🔶 Unverified |

### C.4 — Sumavision Q5 TV box (Amlogic S905L / GXL, Cortex-A53 ×4)

> **Cannot print a single character.** `platform-amlogic` never calls
> `set_console`, so `with_console()`'s `.expect("console not initialized")` at
> `vivanta-boot/boot_common/src/lib.rs:225` panics before any output. The target
> calls `kernel_main` at `vivanta-boot/target-suma-q5/src/main.rs:175` but
> panics first.

| ID | Assumption | Evidence | Validation Method | Status |
|----|-----------|----------|-------------------|--------|
| A-045 | Meson UART is at `UART_AO_BASE = 0xc81004e0` | `vivanta-boot/platform-amlogic/src/lib.rs:14-15`, `:72`. Read from documentation, not from the board | Read the DTB; drive the UART and observe TX | 🔶 Unverified *(cannot be tested: the target panics before it can print — see A-047)* |
| A-046 | The Meson UART is register-compatible with PL011 or NS16550 | ❌ **REJECTED as an implicit assumption.** `platform-amlogic/src/lib.rs:103-104` states plainly: "Meson UART driver not yet implemented. Currently stubbed". The remaining `TODO` in the whole tree is at `:106` | Implement the driver and compare its register map to the datasheet | 🔴 Rejected |
| A-047 | `target-suma-q5` can reach `kernel_main` on silicon | ❌ **REJECTED.** `init_console_from_fdt` returns without calling `set_console`
(`vivanta-boot/platform-amlogic/src/lib.rs:105-114`); the first `with_console()` call then hits `.expect(...)` and panics | Call `set_console()` in the platform init, then boot | 🔴 Rejected |
| A-048 | GIC-400 (GICv2) — distributor `0xc4301000`, CPU interface `0xc4302000` | `vivanta-boot/target-suma-q5/src/main.rs:10`. Not read from the DTB | Dump the DTB interrupt-controller node | 🔶 Unverified |
| A-049 | The suma-q5 boot stack of 16 KiB is sufficient | ❌ **REJECTED**, same evidence as A-043. `target-suma-q5/linker.ld:34` = 16384 bytes | Raise to 256 KiB and boot | 🔴 Rejected |

### C.5 — Cross-cutting

| ID | Assumption | Evidence | Validation Method | Status |
|----|-----------|----------|-------------------|--------|
| A-050 | The EL2→EL1 path enables FP/SIMD correctly | ❌ **REJECTED for 5 of 6 targets**, for **two independent reasons**. (1) *The wrong register is written*: `target-qemu-aarch64:31`, `target-suma-q5:60`, `target-lavender:40`, `target-rpi3b-plus:42` and `target-x96q:53` all execute `msr CPTR_EL2, #(0b11<<20)`. `0b11 << 20` is the encoding of `CPACR_EL1.FPEN[21:20]`, not of `CPTR_EL2.TFP` — **`CPTR_EL2.TFP` is bit 10** and bits 20:21 of `CPTR_EL2` are RES0, so the store is **an architecturally no-op**: it does not set TFP, and FP/SIMD control is left at its reset state. (2) *The `CPACR_EL1` store is dead code on the EL2 path*: the unconditional `b 7f` (or, on `x96q`, the `eret` returning to label `7f`) jumps past the `msr CPACR_EL1` at label 6, so `CPACR_EL1.FPEN` stays at its reset value `0b00`, **trapping FP/SIMD to EL1**. `target-rpi3b-plus` has no `msr CPACR_EL1` instruction anywhere in the target. Only `target-rk3568:190-191` does both correctly. Neither half is in effect on the affected targets, and the QEMU gate builds the **debug** profile and still passes, so the defect is invisible there. Full table: `docs/architecture/arm-exception-state.md` | Fix each target — `msr CPTR_EL2, xzr` **and** an unconditional `msr CPACR_EL1, #(0b11<<20)`; boot in the **debug** profile, which emits NEON in `write_volatile` | 🔴 Rejected |
| A-051 | `enable()`/`disable_interrupts()` mask interrupts | ✅ **VERIFIED (2026-09-29).** `vivanta-boot/arch-aarch64/src/interrupts.rs:13-28` uses `DAIFSet/DAIFClr #2`. The `imm4` field order is bit3=D, bit2=A (SError), bit1=I (IRQ), bit0=F, so `#2` is the IRQ mask (I); `PSTATE.I` (bit 7) is set/cleared this way, and `#4` would set the SError mask (A). Verified empirically on QEMU by reading DAIF after each instruction. *The previous "REJECTED — `#2` is SError, `PSTATE.I` never touched, GIC delivery defect" entry was disproven.* | Read DAIF after `DAIFSet/Clr #2` on QEMU and on silicon | ✅ Verified |
| A-052 | The QEMU gate passing means the kernel is verified | ❌ **REJECTED.** The CI job is `continue-on-error: true` (`.github/workflows/ci.yml:99`), `main` has no branch protection, and the `qemu` job concluded `failure` in **9 of the 21 non-cancelled `main` runs** of the `ci` workflow to date (**≈1 in 2.3**) — the most recent 4 are green, which 4 samples at that rate does not establish as a fix | Make the job blocking after the scheduler race is fixed | 🔴 Rejected |
| A-053 | An L1/L2 table descriptor should be encoded `0b10` per the ARM spec | ❌ **REJECTED for QEMU.** The default is `0b11` (spec-reserved) because QEMU hangs on `0b10`. A `spec-table-desc` feature exists for silicon and **hangs QEMU** — it must never reach `cargo build --workspace`. Source: `vivanta-boot/arch-aarch64/src/paging/descriptor.rs:22-28`; `.github/workflows/ci.yml:68-73` | Run both encodings on silicon | 🟡 Partially Verified *(rejected for QEMU; unverified on silicon)* |
| A-054 | AArch32 EL0 userspace is supported | ❌ **REJECTED.** `lower_aarch32_sync` halts the CPU (`vivanta-boot/arch-aarch64/src/vectors.rs:262`). `arch-armv7a` is an 8-line stub and `target-qemu-armv7a` was deleted 2026-09-25 | — | 🔴 Rejected |
| A-055 | EL0 FP/SIMD state is preserved across entry/exit | ❌ **REJECTED.** `d8-d15`, FPSR and FPCR appear nowhere in `vivanta-boot/arch-aarch64/src/`. Latent while A-050 stands; becomes silent user-state corruption once FP is enabled for EL0 | Add the save/restore, then run a user program that uses FP | 🔴 Rejected |
| A-056 | `vivanta-boot/platform-rk3568` is used by the build | ❌ **REJECTED.** It is a workspace member (`vivanta-boot/Cargo.toml:20`) that **no target depends on** — it is a dead crate | — | 🔴 Rejected |
| A-057 | `vivanta-boot/user/hello/` is the userspace program | ❌ **REJECTED.** It is not a workspace member and cannot be built (`cargo`: "current package believes it's in a workspace when it's not"). The `Hello, Vivanta!` string the gate matches is an assembly blob inside the kernel at `vivanta-boot/arch-aarch64/src/user.rs:107`, not from `user/hello`. The only real program is `vivanta-boot/user-init/` | — | 🔴 Rejected |
| A-058 | A fresh clone can build `vivanta-kernel` | ❌ **REJECTED.** `vivanta-boot/user-init/user-init.elf` is gitignored (`.gitignore:13`) and embedded via `include_bytes!` at `vivanta-boot/kernel/src/syscall/process.rs:545`, so `vivanta-boot/tools/build-user-init.sh` must run first | Clone to a clean directory and run `cargo build` | 🔴 Rejected |
| A-059 | ADR-016 is a ratified decision in this repository | ❌ **Rejected — not located.** No `ADR-016*` file exists in `vivanta-boot/docs/adr/`. **ADR-024 and ADR-025 likewise do not exist.** The set is ADR-011…015, 017…023, 030…035 (18 files). Any document citing 016/024/025 is wrong | `ls vivanta-boot/docs/adr/` | 🔴 Rejected *(kept as an explicit record — the absence is itself the finding)* |
| A-060 | `docs/history/` describes the current system | ❌ **REJECTED.** It is a frozen pre-R2 archive of 95 files under the names "Theseus OS" / "Vorkuta OS". Its `README.md` calls it non-authoritative but only 1 of its 68 `.md` files carries that banner | — | 🔴 Rejected |
| A-061 | The docs in `specs/rfc/` are superseded by the code | ❌ **REJECTED.** `specs/rfc/README.md:18-21` claims `0003` and `008` "cover the same ground" and are both "superseded by the code". They are orthogonal (identity boot state machine vs `BootInfo` ABI), and the shipped `BootInfo` (5 fields) matches neither RFC-008 (9 fields) nor that README. The canonical contract is ADR-021 + `vivanta-boot/docs/architecture/current-architecture.md` | — | 🔴 Rejected |
| A-062 | `docs/audit/Independent_Architecture_Audit.md` is an audit result | ❌ **REJECTED.** It was a prompt template for external LLMs with no answers filled in, filed beside two real audits. §9 instructs the reader to "assume the 25 validated architectural claims are accurate" — and row 24 is a **retracted** claim (`target-test` / `arch-test-stub`, deleted 2026-09-25). It has since been moved to `docs/history/audit-prompts/Independent_Architecture_Audit.prompt-template.md`, which is the correct home for it | — | 🔴 Rejected *(`docs/audit/` now contains only the two real audits and a README index)* |
| A-063 | A PSCI / SMP path exists | ❌ **REJECTED.** Zero PSCI code in the repository. The only mention is a forward-looking comment at `vivanta-boot/platform-amlogic/src/lib.rs:36`. `docs/hardware/aibox-1684x/platform-bringup.md:55` claims "Existing PSCI support | YES" — **false** | — | 🔴 Rejected |
| A-064 | `docs/research/cluster_research.md` is an architectural study | ❌ **REJECTED as characterised.** It is a used-phone buying survey (July 2026 prices, GeekBench figures) with zero measured hardware. It was cited as the architectural exploration behind the heterogeneous-cluster vision; `ROADMAP.md:92-94` now states that explicitly and demotes it to a node-selection input | — | 🔴 Rejected *(characterisation corrected in `ROADMAP.md` on 2026-09-29; the file itself is unchanged)* |

---

## Process

1. When introducing a new assumption in code or architecture, add a row to this register.
2. Assign a validation method and tag the experiment that will validate it.
3. When an experiment produces evidence, update the status.
4. If an assumption is rejected, document the alternative and **keep the row** for audit trail.
5. 🟢 requires a measurement on hardware or a committed run artefact. A datasheet,
   a specification, a code comment or a sibling project's boot log is 📄 or 🟡, not 🟢.
6. When a board is superseded, move its rows rather than deleting them, and say so in a
   scope note (as with section A).

### Review points

This register is reviewed at milestone gates. The current gate names are in
`ROADMAP.md`: **M1, M2, M3, M4, M4.5, M5.0, M6.0, G-M7, M9, M10**, with M11+
held behind a scope fence. The current goal is **M10 validation on the declared
hardware target** — which is exactly the evidence section C lacks.

The old text named "M1-B0 exit, M1-B1 entry" as review points. Those gates were
superseded by RK3568 in July 2026 and no longer exist anywhere in the roadmap.
