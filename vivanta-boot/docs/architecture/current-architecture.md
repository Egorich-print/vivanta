# Current Vivanta Architecture — Snapshot

> Живой документ: отражает актуальное состояние Vivanta на момент обновления.
> Обновлять при архитектурных изменениях, чтобы не приходилось проводить
> повторные исследования. **Дата последней сверки: 2026-09-29** (ранее стояло
> 2026-08-11 — все разделы ниже перепроверены по коду на HEAD `273f9bc`).

## Итоговая картина

Single-core AArch64-микроядро, **QEMU-валидированное, но не железо-валидированное**.
Полный boot до `kernel_main` с работающим user space в EL0 есть только у
`target-qemu-aarch64` и только в QEMU. **Ни одно ядро Vivanta ни разу не
доходило до `kernel_main` на физической плате.** Честный статус:
**«QEMU-correct, NOT hardware-correct»**.

Единственная точка данных на реальном кремнии — вывод UART от диагностического
EL2-бинарника на RK3568 (июль 2026). Этот бинарник **не линкует ядро**
(`target-rk3568` не имеет зависимости от `vivanta-kernel`), MMU на нём не
включён, планировщик не запущен.

## Размер кодовой базы (`.rs`, 2026-09-29)

Пересчитано `wc -l` по каждому крейту. Исключены `target/`, `archive/`,
`docs/history/`, `kernel-memory-frozen/`, `user/hello`, `user/libc` и
`vivanta-boot/tests/minimal_test.rs` (сам себя объявляет «not a real test file»).

| Крейт | Строк |
|-------|-------:|
| `kernel` | 8 658 |
| `arch-aarch64` | 4 172 |
| `boot_common` | 1 680 |
| `vm` | 785 |
| `arch-api` | 713 |
| `exec` | 549 |
| `boot-info` | 145 |
| `arch-armv7a` | 8 (пустой стаб, ADR-011) |
| `platform-*` (6) | 409 |
| `target-*` (6) | 1 135 |
| **Итого: 20 членов воркспейса** | **18 254** |
| `user-init` (отдельный workspace) | 111 |
| **Итого: члены + `user-init`** | **18 365** |

Два итога — не разные подсчёты, а разные границы: **18 254** — только 20 членов
воркспейса; **18 365** — те же 20 членов плюс отдельный воркспейс `user-init`.

`platform-*` = qemu 76 + allwinner-h616 71 + amlogic 123 + rk3568 51 + rpi3b 32 +
sdm660 56. `target-*` = qemu-aarch64 146 + x96q 193 + suma-q5 187 + lavender 91 +
rk3568 375 + rpi3b-plus 143.

> Не в сумме, потому что не собираются: `kernel-memory-frozen/` (788),
> `user/hello` + `user/libc` (119), `archive/boot_legacy/` (886).
> В воркспейсе 20 членов (`vivanta-boot/Cargo.toml:5-34`); `user-init/` —
> отдельный воркспейс, встроенный в ядро через `include_bytes!`
> (`kernel/src/syscall/process.rs:545`). `*.elf` в `.gitignore` → свежий клон
> не соберёт `vivanta-kernel`, пока не отработает `tools/build-user-init.sh`.

## Слои и контракты

```
target-*        — composition (platform + kernel + arch) → final ELF
    ↓
platform-*      — board/SoC: console init, FDT, memory map. НЕ знает о ядре
    ↓
kernel          — арх-независимая логика (ноль inline-asm)
    ↕
arch-api        — extern "Rust" контракты
    ↓
arch-aarch64    — ISA: VMSAv8-64, GICv2/v3, generic timer, EL1→EL0
    ↓
boot-info / boot_common — ABI-типы, console (spinlock+IRQ guard), FDT-сканер
```

Инварианты: `kernel` никогда не импортирует `arch-*` напрямую; `platform-*` не
зависит от `kernel`/`arch-*`; `boot-info` zero-dep; только `target-*` выбирает
и платформу, и арх; никаких циклов зависимостей. Арх-независимость `kernel`
обеспечивается направлением зависимостей в `vivanta-boot/Cargo.toml`.

> **Обновлено 2026-09-25.** Крейты `arch-test-stub` и `target-test` удалены:
> `target-test` никогда не ссылался на `kernel_main`, поэтому ядро не линковалось,
> а в стабе не хватало 13 символов `arch-api`. Заявление «доказано target-test»
> было неверным. См. [docs/audit/2026-09-25-cleanup.md](../../../docs/audit/2026-09-25-cleanup.md).

## Платформы (статус 2026-09-29)

| Target | SoC/плата | UART | GIC | Линкует ядро? | Доходит до `kernel_main`? | Проверено на кремнии? |
|--------|-----------|------|-----|:--------------:|:------------------------:|:-------------------:|
| `target-qemu-aarch64` | QEMU virt, Cortex-A53 | PL011 0x09000000 | GICv3 0x08000000 | ✅ | ✅ | ❌ **только QEMU, гейт advisory** |
| `target-rk3568` | RK3568 NVR | NS16550 0xFE660000 | GIC (FDT) | ❌ **зависимости от kernel нет** | ❌ только EL2-диагностика: UART + отчёт по FDT + `test_brk` | ✅ **да — вывод UART, июль 2026** |
| `target-rpi3b-plus` | BCM2837B0 Pi 3B+ | PL011 + GPIO (`miniuart-bt`) | GICv2 | ✅ | ✅ (`src/main.rs:132`) | ❌ **код написан, ни разу не запускался** |
| `target-x96q` | Allwinner H313 (X96Q) | NS16550 0x05000000 | GICv2 0x03000000 | ✅ | ✅ (`src/main.rs:181`) | ❌ **«compile-checked only, needs silicon»** (коммит `5005542`) |
| `target-lavender` | SDM660 (Redmi Note 7) | MSM UARTDM 0x0C170000 | GICv3 | зависимости объявлены, **не используются** | ❌ — 5 `println!` и бесконечный спин | ❌ |
| `target-suma-q5` | Amlogic S905L (Sumavision Q5) | Meson UART (нет драйвера) | GICv2 0xc4301000 | ✅ | вызов есть (`:175`), но **паникует раньше** | ❌ |

### Платформенные крейты

| Platform | Роль |
|----------|------|
| `platform-qemu` | рабочая: PL011, FDT, memory map |
| `platform-allwinner-h616` | X96Q. ~~In dev, полный boot на железе~~ — **это неверно**: boot на железе не подтверждён, коммит `5005542` прямо пишет «Compile-checked only, needs silicon» |
| `platform-rk3568` | диагностика; **не линкуется ни одним target'ом** — мёртвый крейт |
| `platform-rpi3b` | ~~Diagnostic only~~ — **неверно**: `target-rpi3b-plus` линкует `vivanta-kernel` и вызывает `kernel_main`. Это не «standalone diagnostic (early_mmu identity map)»: у `arch-aarch64/src/early_mmu.rs` **ноль вызывающего кода**, а комментарий `arch-aarch64/src/lib.rs:13`, который так его описывает, устарел |
| `platform-sdm660` | stalled/planned; Vivanta на lavender/whyred не запускался (только внешнее обследование) |
| `platform-amlogic` | **дефект**: никогда не вызывает `set_console` → `with_console()` паникует на `.expect("console not initialized")` (`boot_common/src/lib.rs:225`), т.е. `target-suma-q5` не может напечатать ни одного символа |

## Boot-контракт (ADR-021 + `vivanta-boot/boot-info/src/lib.rs`)

Канонический контракт — **ADR-021** и сам тип `BootInfo`, а не RFC-008.
`BootInfo` — `#[repr(C)]`, ровно **5 полей**
(`vivanta-boot/boot-info/src/lib.rs:11-18`):

```rust
pub struct BootInfo {
    pub memory_map: &'static MemoryMap,
    pub mmio_regions: &'static [MmioRegion],
    pub interrupt_controller: Option<InterruptControllerInfo>,
    pub cpu_count: usize,
    pub dtb: Option<usize>,
}
```

> **RFC-008 в этом не участвует.** `specs/rfc/README.md:18-21` утверждает, что
> RFC `0003` и `008` «cover the same ground» и оба «superseded by the code» —
> оба утверждения ложны. Они ортогональны (машина состояний бутовой identity
> против ABI `BootInfo`), а RFC-008 требует **9 полей** (`acpi`, `framebuffer`,
> `cmdline`, `initrd` среди прочих), тогда как в коде их 5. Ни RFC-008, ни
> `0003` не описывают то, что зашипировано.

- Каждая загрузка собирает один `BootInfo` и вызывает `kernel_main(&BootInfo)`;
  после этого `BootInfo` не влияет на runtime (SystemState, ADR-021).
- Вход AArch64: **EL1, MMU off (или identity map), SP=stack top, DTB в x0**.
- **Два живых входных формата, не три.** QEMU `-kernel` (ELF, грузится по
  0x40200000) и U-Boot `booti` (ARM64 Image header, `DRAM_BASE + TEXT_OFFSET`) —
  последний у x96q, rpi3b-plus и lavender. Путь «SDM660/ABL, Android boot image v1»
  из предыдущей редакции этого документа к живым крейтам не относится: это
  до-ACS эпоха, её код лежит в `vivanta-boot/archive/boot_legacy/`.
  `target-rk3568` грузится как плоский бинарник через U-Boot
  (`booti 0x20500000`), но это диагностика, а не путь в ядро.
- FDT — да; ACPI/UEFI — нет (сознательно исключены, ADR-015).
- `BootInfo.interrupt_controller` пишут x96q/suma-q5, он хранится в
  `HardwareState` и **никогда не читается**.
- DDR-тренинг на реальных платформах делает бутлоадер, ядро не трогает.

## Подсистемы ядра и зрелость

| Подсистема | Состояние |
|-----------|-----------|
| Paging | 4 KiB, L1→L2→L3, 39-bit VA (`T0SZ=25`, TG0=4K, SH=inner), `mair_el1=0x44_FF`, `tcr_el1=25\|IRGN01\|ORGN01\|SH11\|TG0 00\|IPS3` (`arch-aarch64/src/mmu.rs:239-243`). 2 MiB блоки только для boot identity map, дальше дробятся до 4 KiB по требованию. **Дескриптор L1/L2-таблицы — `0b11` (по спецификации зарезервировано), потому что QEMU зависает на `0b10`**; фича `spec-table-desc` даёт `0b10` для кремния и **вешает QEMU** — поэтому `cargo build --workspace` запускать нельзя (CI делает per-target, `ci.yml:68-73`) |
| VMM | `kernel/src/vmm/` — 1 896 строк. `MAX_ADDRESS_SPACES=16` (`address_space.rs:19`), `MappingSet` = фиксированный `[Option<Mapping>; 64]`, `MAX_TABLES=256`, `MAX_COW_FRAMES=256`. Demand paging (`LazyAnonymous`), COW (`CoWShared{refcount}`), forward+reverse верификаторы INV-VM-001. `protect()` **реализован** (не `todo!()`), whole-mapping гранулярности. TTBR1/ASID/LPA/stage2 — вне скоупа. **Дефект (2026-09-24):** конверсия `Backing::Present` → `CoWShared` в `address_space.rs:608-619` игнорирует прежнего владельца `External`, поэтому последний владелец может освободить фрейм, которым VMM не владел |
| `vivanta-boot/vm` | **это не VMM.** 785 строк, только аллокатор VA-диапазонов (`VaAllocator`, first-fit список интервалов, `MAX_FREE_RANGES=256`, `PAGE_SIZE=4096`). Настоящий VMM — `kernel/src/vmm/` |
| PMM | Bitmap, `FRAME_SIZE=4096`, `alloc_frame` — **линейный скан с нуля** (`kernel/src/pmm.rs:20`). Ни buddy, ни slab, ни зон. `free_frame` не различает reserved и allocated. `PmmBackend::new_dram` жёстко кодит `dram_4gb()` — реальный DRAM из DTB игнорируется (завышение в 8× на прогонах QEMU с 512 MiB) |
| Kernel heap | free-list со склейкой, фиксированные **64 KiB**, `#[global_allocator]` в `kernel/src/lib.rs:9-10`. MRM-аллокация страниц под ним, bypass PMM |
| Scheduler | Round-robin по 5-уровневой лестнице приоритетов, `RunQueue` — **линейный скан `Vec<Option<Thread>>`** (`scheduler/runqueue.rs:79-122`). Таймер `TICK_HZ=100` (`arch-aarch64/src/timer.rs:8`) → `NEED_RESCHEDULE` на **каждом** тике, без учёта кванта. Кооперативный `yield_now()` — основной путь. `MAX_TASKS=64`, `KERNEL_STACK_SIZE=65536`. **Открытый P0:** тик может попасть в окно переключения контекста и потерять код выхода user-потока либо оставить форкнутый поток нерасписанным. Добавление трассировки скрывает гонку (heisenbug) |
| Interrupts | GICv2 и GICv3 (`arch-aarch64/src/interrupts/gic.rs`), выбор по FDT `compatible`, содержащему `gic-v3`. **Без RWP-поллинга.** `irq_init` использует жёсткий `.expect()` на узле FDT interrupt-controller (`arch-aarch64/src/boot.rs:169-203`). **Маскирование IRQ корректно:** `enable()` пишет `msr DAIFClr, #2`, а `disable_interrupts()` — `msr DAIFSet, #2`; `imm4` кодируется bit3=D, bit2=A (SError), bit1=I (IRQ), bit0=F, поэтому `#2` — это IRQ-маска (I), а `PSTATE.I` (бит 7) управляется ею. `#4` ставил бы SError-маску (A). (Прежняя запись «бит 2 — SError, IRQ сломаны, дефект доставки в GIC» опровергнута 2026-09-29; см. поправку в аудите.) |
| Exceptions | Таблица 16 векторов × 128 B. **6 возвращаются, 10 гасят CPU** (`arch-aarch64/src/vectors.rs:252-269`): `el1h_sync` — единственный возобновляемый класс EL1 (demand-fill + COW), `lower_aarch64_sync` диспетчеризует SVC и EL0-фолты, `lower_aarch32_sync` гасит → **поддержки AArch32 EL0 нет**. Не возвращается ни один FIQ/SError вектор |
| Timer | CNTP (non-secure physical), 100 Hz, IRQ 30, частота из `CNTFRQ_EL0` |
| Syscalls | ABI заморожен в ADR-033: номер в `x8`, аргументы `x0..x5`, результат в `x0` (`>=0` — ок, `<0` = `-errno`). **16 номеров, 13 живых, 3 отдают `-ENOSYS`** (`kernel/src/syscall.rs:24-39`, `:111`, `:133-134`): live — `write(1) exit(2) yield(3) mmap(4) munmap(5) mprotect(6) fork(7) waitpid(8) kill(9) getpid(10) getppid(11) execve(12) rt_sigaction(13)`; reserved — `read(0)`, `rt_sigprocmask(14)`, `rt_sigreturn(15)`. `MAX_MMAP_BYTES = 64 MiB`, `WRITE_BUF_SIZE = 256`. **Не 14/16** |
| Process model | `fork`/`execve`/`waitpid`/`kill`/`getpid`/`getppid` реализованы (`kernel/src/syscall/process.rs`). `execve` резолвит **только** compile-time таблицу `BUILTIN_PROGRAMS = [("/init", include_bytes!(…))]` (`:544-545`) и **игнорирует `argv`/`envp`** (`:582-583`). `waitpid` **не освобождает** адресное пространство, маппинги и поток ребёнка — и маскирует сам себя: гейт M10.4 вручную делает тот teardown, который должен делать `waitpid`. `kill` работает только с `SIGKILL`, остальные сигналы ставятся в очередь и не доставляются |
| Signals | Только dispositions (`kernel/src/signal.rs`), `MAX_SIG=32`. `pending` — write-only, явный scope fence (`signal.rs:133-140`). Асинхронной доставки в EL0 нет |
| User-space | EL1→EL0 есть: `eret_to_user_stub` (ADR-018), синтетический frame, SVC-хендлер, EL0 fault containment. **Ровно одна настоящая программа** — `user-init/` (111 строк, целиком naked `global_asm!` без стека и без Rust), встроенная в ядро. `user/hello` + `user/libc` — мёртвые, вне воркспейса. Строка `Hello, Vivanta!`, которую матчит CI, — это asm-блоб **внутри ядра** (`arch-aarch64/src/user.rs:107`), а не вывод из `user/hello` |
| ELF loader | `vivanta-boot/exec` — валидирует magic/class/machine/type, отклоняет W/X-сегменты, перекрытия, entry вне exec-сегмента, `memsz<filesz`; 10 хост-тестов. **Открытый дефект:** icache обслуживается только для страницы входа, а не всего исполняемого сегмента — многoстраничный `PF_X` может выполниться из старого I-cache. FP/SIMD-контекст EL0 (`d8-d15`, FPSR, FPCR) не сохраняется |

## Драйверы

**Есть:** PL011, NS16550 (reg-shift), MSM UARTDM (TX-only), GICv2/v3, ARM
Generic Timer, FDT-сканер (1 008 строк), GPIO-init RPi3. Консоль — spinlock +
RAII `InterruptGuard` (фикс INV-002).

**НЕТ:** дисплей/framebuffer, eMMC/UFS/SD, USB, сеть/WiFi, DMA, IRQ периферии
(кроме timer IRQ 30; UART — polled TX), PMU/power, clocks, **PSCI/SMP/secondary
CPU**.

## Тесты

- **22 хост-теста**, и CI их запускает: 12 в `vivanta-vm`
  (`vm/src/va_tests.rs`), 10 в `vivanta-exec` (`exec/src/elf_tests.rs`) —
  нативно на x86_64 (`ci.yml:53`).
- **Ещё 6 хост-тестов, которые CI никогда не запускает:** 3 в
  `boot-info/src/mmap.rs`, 3 в `boot_common/src/memory_discovery.rs`.
- `kernel` и `arch-aarch64` выставляют `[lib] test = false` — их проверка это
  QEMU-гейт.
- Ни одного `#[ignore]`, ни одного `#[should_panic]`, ни одного интеграционного
  теста. `clippy` закреплён в `rust-toolchain.toml`, но **CI его не запускает**.
- В дереве **нет ни одного `todo!()` и `unimplemented!()`**. Единственный
  комментарий `TODO` — `platform-amlogic/src/lib.rs:106`.

## Известные дефекты и долги

Полный приоритизированный список — [docs/audit/2026-09-24-global-audit.md](../../../docs/audit/2026-09-24-global-audit.md).

- **P0:** ~~маскирование прерываний (`DAIFSet #2` = SError)~~ — **снято
  2026-09-29:** `#2` — корректная IRQ-маска (I), `PSTATE.I` управляется ею.
  Остаются: гонка планировщика; COW-ownership в
  `vmm/address_space.rs:608-619`; утечка в `waitpid`; нет сохранения
  EL0 FP/SIMD; запись в GICD без RWP-поллинга; icache на exec-пути.
- **P0 (кремний):** boot stack 16 KiB на x96q/suma-q5/rk3568/lavender против
  256 KiB на qemu и rpi3b-plus — при том что x96q/suma-q5 **доходят** до
  `kernel_main` с тем же ~82 KiB фреймом, который и вынудил увеличение;
  `CPTR_EL2 = 0b11 << 20` и не выставленный `CPACR_EL1` на EL2→EL1h путях
  rpi3b+ и x96q (см. `docs/architecture/arm-exception-state.md`, где
  предписано ровно обратное); x96q и suma-q5 не загружаются на кремнии.
- **MMU L1/L2 `0b11` против `0b10`** — единственная кодировка, с которой
  заводится QEMU cortex-a53; на физическом ARM64 она зарезервирована.
  См. `docs/investigations/MMU-descriptor-encoding-hardware-validation.md`.
- NS16550 32-bit доступ, ARM64 Image header 64B, x0 DTB — осознанные долги
  (evolution-plan).
- `target-lavender` (SDM660) печатает «Vivanta Boot v0.1» и спин-лупится —
  до boot не доходит.
- rustflags во всех `target-*/.cargo/config.toml` — мёртвая конфигурация
  (cargo читает конфиг от каталога запуска, а все скрипты и CI запускаются из
  `vivanta-boot/`); реальный механизм — `build.rs` каждого крейта.
- `arch-api/src/syscall.rs:18-27` объявляет `syscall_dispatch` с **8**
  параметрами, реальная сигнатура — с **9** (добавлен `as_root: u64` первым).
  `arch-aarch64` держит собственную приватную копию, объявленная в arch-api
  никогда не вызывается. Выживает только потому, что `extern "Rust"` манглит
  по арности. **Любой, кто её вызовет, получит UB.**

## Scope fence (намеренно вне скоупа)

IPC, storage, драйверы, distributed AI, Ed25519, BIP-39, persistent identity,
TTBR1/ASID, асинхронная доставка сигналов, новые hardware-таргеты, новые
архитектуры.

> Оценка «**~35-40%**» из `docs/OS_MATURITY.md` **не должна цитироваться**:
> документ датирован 2026-07-24, советует не строить heap-аллокатор, потому что
> «`StubAllocator` уродлив» — **такого типа в коде нет**, его заменил
> MRM-аллокационный free-list; а его таблица «Remaining» перечисляет три
> подсистемы, которые уже реализованы. Линия сравнения — Minix 3 / seL4 / QNX,
> не Linux.

## Долгосрочные цели

Distributed OS, миграция состояния между разнородным железом, resource-oriented
memory, гетерогенный compute-кластер из бюджетных смартфонов. Целевой телефон
будущего — Google Pixel 6+ с GrapheneOS (см.
`docs/plan-next-phase-2026-08-01.md` → «Future Target: Google Pixel»).
Это исследовательские направления, а не обязательства.

## Ключевые документы

- [repository-layout.md](repository-layout.md) — слои и инварианты
- [arm-exception-state.md](../../../docs/architecture/arm-exception-state.md) — FP/SIMD init, требуемая
  последовательность `CPACR_EL1` / `CPTR_EL2`
- [execution-context.md](execution-context.md) — стек потока (ThreadContext у
  низа, синтетический frame у верха)
- ADRs: [ADR-017](../../../vivanta-boot/docs/adr/ADR-017-unified-execution-context.md),
  [ADR-018](../../../vivanta-boot/docs/adr/ADR-018-user-entry-transition.md),
  [ADR-021](../../../vivanta-boot/docs/adr/ADR-021-bootinfo-escape-prevention.md),
  [ADR-022](../../../vivanta-boot/docs/adr/ADR-022-driver-lifecycle-contract.md),
  [ADR-030](../../../vivanta-boot/docs/adr/ADR-030-paging-architecture.md) —
  execution context, EL0 transition, SystemState, driver lifecycle, paging split
- [docs/rfcs/RFC-012…016](../../../vivanta-boot/docs/rfcs/) — память, frozen
- `plan-next-phase-2026-08-01.md` — план фаз + Future Target
- [PLATFORM_BRINGUP.md](../../PLATFORM_BRINGUP.md) — как добавить новую платформу
