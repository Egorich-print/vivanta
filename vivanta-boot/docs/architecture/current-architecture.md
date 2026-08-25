# Current Vivanta Architecture — Snapshot

> Живой документ: отражает актуальное состояние Vivanta на момент обновления.
> Обновлять при архитектурных изменениях, чтобы не приходилось проводить
> повторные исследования. Дата последней сверки: 2026-08-11.

## Итоговая картина

Single-core, QEMU-валидированное экспериментальное микро-ядро на AArch64
(~10 200 строк Rust суммарно). Полный boot до `kernel_main` на реальном
железе — только `target-x96q` (Allwinner H313). Честный статус: **«QEMU-correct,
NOT hardware-correct»**.

## Размер кодовой базы (строки, 2026-08-11)

| Крейт | Строк |
|-------|-------|
| `kernel` | 4 258 |
| `arch-aarch64` | 3 034 |
| `boot_common` | 1 644 |
| `arch-api` | 468 |
| `arch-test-stub` | 138 |
| `platform-*` (6) | 395 |
| `target-*` (9) | 1 772 |
| `arch-armv7a` | 8 (пустой стаб) |

## Слои и контракты

```
target-*        — composition (platform + kernel + arch) → final ELF
    ↓
platform-*      — board/SoC: console init, FDT, memory map. НЕ знает о ядре
    ↓
kernel          — арх-независимая логика (ноль inline-asm)
    ↕
arch-api        — extern "Rust" контракты (8+ модулей)
    ↓
arch-aarch64    — ISA: VMSAv8-64, GICv2/v3, generic timer, EL1→EL0
    ↓
boot-info / boot_common — ABI-типы, console (spinlock+IRQ guard), FDT-сканер
```

Инварианты: `kernel` никогда не импортирует `arch-*` напрямую; `platform-*` не
зависит от `kernel`/`arch-*`; `boot-info` zero-dep; только `target-*` выбирает
и платформу, и арх; никаких циклов зависимостей. Арх-независимость `kernel`
доказана крейтом `arch-test-stub` + `target-test` (build-time proof).

## Платформы (статус 2026-08-11)

| Platform | SoC | UART | GIC | Boot | Статус |
|----------|-----|------|-----|------|--------|
| `platform-qemu` | QEMU virt, Cortex-A53 | PL011 0x09000000 | GICv3 0x08000000 | `-kernel` | **Активна**, полный boot |
| `platform-allwinner-h616` | H313/H616 (X96Q) | NS16550 0x05000000 | GICv2 0x03000000 | U-Boot `booti` | **In dev**, полный boot на железе |
| `platform-rk3568` | RK3568 | NS16550 0xFE660000 | GIC (FDT) | U-Boot | Diagnostic only |
| `platform-sdm660` | SDM660 (Redmi Note 7) | MSM UARTDM 0x0C170000 | GICv3 | ABL→boot.img | Stalled/planned |
| `platform-rpi3b` | RPi 3B+ | PL011 + GPIO | — | U-Boot | Diagnostic only |
| `platform-amlogic` | S905L | Meson UART (нет драйвера) | GICv2 0xc4301000 | BootROM→ATF→U-Boot | Stalled/planned |

## Boot-контракт (RFC-008 / ADR-021)

- Каждая загрузка собирает один `BootInfo` и вызывает `kernel_main(&BootInfo)`;
  после этого `BootInfo` не влияет на runtime (SystemState, ADR-021).
- Вход AArch64: **EL1, MMU off (или identity map), SP=stack top, DTB в x0**.
- Три пути: QEMU `-kernel` (ELF по 0x40200000), U-Boot `booti` (ARM64 Image
  header, `DRAM_BASE + TEXT_OFFSET`), SDM660/ABL (Android boot image v1).
- FDT — да; ACPI/UEFI — нет (сознательно исключены, ADR-015).
- DDR-тренинг на реальных платформах делает бутлоадер, ядро не трогает.

## Подсистемы ядра и зрелость

| Подсистема | Состояние |
|-----------|-----------|
| Scheduler | Priority-очередь, преемптивность 100 Hz (таймер), ThreadId-based current, sleep/wake, Task-модель (exit/zombie/reap, M6). INV-002 закрыт (консоль-дедлок, 2026-08-11). Single-core. |
| PMM | Bitmap-аллокатор, мульти-регион, self-test + stress |
| VMM/MMU | 4-level page tables, runtime map/unmap, 8-слотный реестр AS. `protect()` — todo!, нет VA-аллокатора. **Блокер кремния: L1/L2 дескрипторы `0b11` вместо `0b10`**. TTBR1/ASID/LPA/stage2 — deferred |
| Interrupts | GICv2 и GICv3, таблица 256 IRQ-хендлеров |
| Timer | CNTP (non-secure physical), 100 Hz, IRQ 30, частота из CNTFRQ_EL0 |
| Syscalls | x8=num: READ (стаб→0), WRITE (рабочий), EXIT, YIELD, MMAP (→-ENOMEM) |
| User-space | EL1→EL0 есть: `eret_to_user_stub` (ADR-018), синтетический frame 272B, SVC-хендлер, EL0 fault containment. User-код — статичный asm-блоб, ELF-загрузчика нет |
| Fault handling | Kernel fault → dump+halt; EL0 fault → terminate task. Нет demand paging |

## Драйверы

**Есть:** PL011, NS16550 (reg-shift), MSM UARTDM (TX-only), GICv2/v3, ARM
Generic Timer, FDT-сканер (~980 строк), GPIO-init RPi3. Консоль — spinlock +
RAII `InterruptGuard` (фикс INV-002).

**НЕТ:** дисплей/framebuffer, eMMC/UFS/SD, USB, сеть/WiFi, DMA, IRQ периферии
(кроме timer IRQ 30; UART — polled TX), PMU/power, clocks, PSCI/SMP/secondary
CPU, ELF-загрузчик.

## Известные дефекты и долги

- **MMU L1/L2 `0b11` vs `0b10`** — единственная кодировка, с которой заводится
  QEMU cortex-a53; на физическом ARM64 зарезервирована (translation fault).
  См. `docs/investigations/MMU-descriptor-encoding-hardware-validation.md`.
- NS16550 32-bit доступ, ARM64 Image header 64B, x0 DTB — осознанные долги
  (evolution-plan).
- `target-lavender` (SDM660) печатает «Vivanta Boot v0.1» и спин-лупится —
  до boot не доходит.

## Scope fence (намеренно вне скоупа)

IPC, storage, драйверы, distributed AI, Ed25519, BIP-39, persistent identity,
TTBR1/ASID, signal delivery, новые hardware-таргеты, новые архитектуры.
Самооценка: `docs/OS_MATURITY.md` — **~35-40%**; линия — Minix 3 / seL4 / QNX,
не Linux.

## Долгосрочные цели

Distributed OS, миграция состояния между разнородным железом, resource-oriented
memory, гетерогенный compute-кластер из бюджетных смартфонов. Целевой телефон
будущего — Google Pixel 6+ с GrapheneOS (см.
`docs/plan-next-phase-2026-08-01.md` → «Future Target: Google Pixel»).

## Ключевые документы

- `docs/architecture/repository-layout.md` — слои и инварианты (частично устарел,
  сверять с этим снапшотом)
- `docs/adr/ADR-017/018/021/022/030` — execution context, EL0 transition,
  SystemState, driver lifecycle, paging split
- `docs/architecture/execution-context.md` — стек потока (ThreadContext 104B у
  низа, синтетический frame 272B у верха)
- `docs/rfcs/RFC-012…016` (память, frozen)
- `docs/plan-next-phase-2026-08-01.md` — план фаз + Future Target
- `PLATFORM_BRINGUP.md` — как добавить новую платформу
