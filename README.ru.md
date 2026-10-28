[English](README.md) · **Русский**

---

# Vivanta

[![CI](https://github.com/Egorich-print/vivanta/actions/workflows/ci.yml/badge.svg)](https://github.com/Egorich-print/vivanta/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/Egorich-print/vivanta?include_prereleases&label=release&color=blue)](https://github.com/Egorich-print/vivanta/releases)
![License: GPLv3](https://img.shields.io/badge/license-GPLv3-blue)
![Rust 1.98.0](https://img.shields.io/badge/rust-1.98.0-orange)
![Platform: AArch64](https://img.shields.io/badge/platform-AArch64-lightgrey)

Экспериментальная операционная система, исследующая **непрерывность идентичности**
(identity continuity), **ресурсно-ориентированную память** и **портативную
архитектуру загрузки**.

Vivanta — это `no_std`-ядро на Rust, bare-metal, только под **AArch64**. Цели:
QEMU `virt` и несколько реальных плат (RK3568, Raspberry Pi 3B+, Allwinner H313,
SDM660, Amlogic S905L); аппаратные цели на ранней стадии — см. таблицу с
уровнями ниже. `vivanta-boot/arch-armv7a` — это 8-строчная замороженная заглушка,
сохранённая для справки (ADR-011); кода ARMv7 в ней нет.

## Статус

**QEMU-correct, но не hardware-correct.** AArch64-ядро доходит до `kernel_main`
на QEMU `virt`, проходит матрицу загрузочных гейтов — **18 обязательных
маркеров** в последовательном логе (`tools/qemu-gates.sh`), без паник — и
прогоняет настоящий EL0-userland через syscall ABI. Этот гейт проверен на
локальных прогонах; в CI джоб **совещательный** (advisory) из-за открытой гонки
планировщика (см. [CI](#непрерывная-интеграция)). Валидации самого ядра на
железе нет.

| Область | Статус |
|---------|--------|
| Загрузка до `kernel_main` (QEMU AArch64) | ✅ |
| Менеджер физической памяти (PMM по всей доступной RAM) | ✅ |
| Виртуальная память: адресные пространства, map/unmap/protect, владение таблицами и их рекламация | ✅ |
| Demand paging (ленивая анонимная память), один возобновляемый класс EL1-ошибок (ADR-032) | ✅ |
| Copy-on-Write для анонимной приватной памяти | ✅ |
| Куча ядра (free-list со слиянием свободных блоков) | ✅ |
| Планировщик: лестница приоритетов, таймер 100 Hz, основной путь — кооперативный `yield` | ✅ |
| Модель процессов: задачи, потоки, таблица процессов, жизненный цикл | ✅ |
| Планировщик загрузки ELF64 AArch64 — magic/class/machine, W^X, проверки пересечений и границ | ✅ |
| Граница пользовательской памяти (`access_ok`, `-EFAULT`) и сдерживание EL0-ошибок | ✅ |
| Живые системные вызовы (13): `write`, `exit`, `yield`, `mmap`, `munmap`, `mprotect`, `fork`, `waitpid`, `kill`, `getpid`, `getppid`, `execve`, `rt_sigaction` | ✅ |
| Зарезервированные вызовы (3): `read`, `rt_sigprocmask`, `rt_sigreturn` → `-ENOSYS` | — |
| EL0-userland: write, `execve`, round-trip `fork`/`waitpid`, kill при ошибке | ✅ |
| Сигналы | ⚠️ только диспозиции — см. ниже |
| Идентичность | ⚠️ номинальная — счётчик, а не ключевая пара |

`execve` разрешает **только** встроенный образ `/init` (таблица
`include_bytes!` на этапе компиляции). Файловой системы нет, `argv`/`envp`
игнорируются.

### Что НЕ реализовано

Файловая система · `execve` отдаёт только встроенный ELF `/init` и игнорирует
`argv`/`envp` · сигналы — **только диспозиции**: `pending` доступен для записи,
асинхронной доставки в EL0 нет, реально действует лишь `SIGKILL`
(`rt_sigreturn` возвращает `-ENOSYS`) · `waitpid` не освобождает адресное
пространство, маппинги и поток дочернего процесса · ни SMP, ни PSCI, ни
вторичных CPU · нет драйверов, хранилищ, сети, IPC · нет криптографической
идентичности (UUID заполняется из монотонного счётчика) · нет сохранения
FP/SIMD-контекста EL0 · TTBR1/ASID не реализованы.

Ни одного `todo!()` и ни одного `unimplemented!()` во всём дереве. Пробелы
живут в прозе, в вызовах `expect()` и в
[`docs/audit/2026-09-24-global-audit.md`](docs/audit/2026-09-24-global-audit.md).

## Статус по железу, по уровням

| Уровень | Цель | Что верно на самом деле |
|---|---|---|
| Работает | `target-qemu-aarch64` | Доходит до `kernel_main`, матрица гейтов зелёная локально. В CI — advisory. |
| Никогда не запускался на кремнии | `target-rpi3b-plus` | Линкует настоящее ядро и вызывает `kernel_main`. Рабочий захват UART на RPi получен от загрузки **BalanSir'ов** Buildroot-образа, а не от Vivanta. |
| Никогда не запускался на кремнии | `target-x96q` (Allwinner H313) | Компилируется и доходит до вызова `kernel_main`. Только проверка компиляцией. |
| Только диагностика | `target-rk3568` | **Ядро не линкует.** Единственная плата, где бинарник Vivanta дал реальный вывод в UART на кремнии (диагностика Stage-1/2, июль 2026). Позже плата была замечена в **Maskrom**; загрузка не подтверждена. |
| Не загружается | `target-lavender` (SDM660) | Печатает 5 строк и крутится вечно. Зависимости платформы объявлены, но не используются. |
| Не загружается | `target-suma-q5` (Amlogic S905L) | `platform-amlogic` не регистрирует консоль, поэтому первый же `println!` паникует. |
| Только исследования | AIBOX-1684X (BM1684X) | Разведка с реестром доказательств `[OBSERVED]/[INFERRED]/[UNAVAILABLE]`. Кода нет, в роадмапе не значится. |

Данные по платам лежат в [`docs/hardware/`](docs/hardware/), заметки по
bring-up — в [`vivanta-boot/PLATFORM_BRINGUP.md`](vivanta-boot/PLATFORM_BRINGUP.md).

## Быстрый старт (QEMU AArch64)

Требуется [Rust](https://rustup.rs) 1.98.0 (зафиксирован в
`vivanta-boot/rust-toolchain.toml`) и QEMU.

```bash
git clone https://github.com/Egorich-print/vivanta.git
cd vivanta/vivanta-boot

rustup target add aarch64-unknown-none   # один раз
./tools/build-user-init.sh               # ОБЯЗАТЕЛЬНО: ядро встраивает этот ELF
cargo build -p vivanta-target-qemu-aarch64

qemu-system-aarch64 -M virt -cpu cortex-a53 -m 512M -nographic \
  -kernel target/aarch64-unknown-none/debug/vivanta-target-qemu-aarch64 \
  -serial mon:stdio
```

`user-init/user-init.elf` в `.gitignore` и встраивается в ядро на этапе
компиляции, поэтому свежий клон **не может** собрать `vivanta-kernel`, пока не
отработал `./tools/build-user-init.sh`.

> **Не запускайте `cargo build --workspace`.** Унификация фич затягивает
> `spec-table-desc` из `target-rpi3b-plus` (корректный для кремния дескриптор
> таблиц L1/L2 `0b10`) в QEMU-сборку, и полученное ядро **зависает на включении
> MMU**. QEMU нужен дефолтный `0b11`. Собирайте цель явно, как показано выше.
> (`.github/workflows/ci.yml:68-73`)

В конце лога отрабатывает встроенный EL0-демо-поток: `write` со строкой
`Hello, Vivanta!`, затем заведомый выход за границу буфера, который обязан
вернуть `-EFAULT`, и `exit(0)`:

```
Hello, Vivanta!
  syscall: exit(0)
```

Автоматическая проверка (тот же гейт, что и в CI):

```bash
./tools/qemu-gates.sh          # 18 маркеров, PASS/FAIL, ненулевой код при провале
./tools/ci-local.sh            # весь пайплайн: fmt, синтаксис shell/py, host-тесты, QEMU
```

## Непрерывная интеграция

Пуш в `main` и pull request в `main` запускают
[`.github/workflows/ci.yml`](.github/workflows/ci.yml):

- **lint** — `rustfmt`, синтаксис shell/python, exec-биты;
- **test** — 22 host unit-теста (`vivanta-vm`, `vivanta-exec`) нативно на x86_64;
- **build** — сборка встроенного EL0-образа, `cargo check --workspace`, затем
  QEMU-ядро отдельной командой (по причине выше);
- **qemu** — загрузка артефакта на раннере `ubuntu-24.04-arm` и проверка 18
  маркеров. **`continue-on-error: true` — advisory, не блокирующий.** Открытая
  гонка планировщика (тик может попасть внутрь переключения контекста; само
  маскирование IRQ корректно) стоит потерянного exit code или
  не запланированного форкнутого потомка примерно в 1 прогоне из 2.5 на этом
  раннере; длинные локальные прогоны остаются зелёными. У `main` нет branch
  protection, так что зелёный прогон всё равно никогда не блокировал мержа.
  Диагноз: [`docs/audit/2026-09-24-global-audit.md`](docs/audit/2026-09-24-global-audit.md).
- **image** — сборка прошиваемого SD-образа для Raspberry Pi 3B+.

Релизы публикуются ручным запуском
[`.github/workflows/release.yml`](.github/workflows/release.yml) с указанием тега
в [Releases](https://github.com/Egorich-print/vivanta/releases): SD-образ,
`kernel8.img`, QEMU-ELF ядра, плоский бинарник **диагностического адаптера**
RK3568 (никогда не ядро) и EL0-образ. Детали и локальный ритуал:
[`docs/tooling/ci-cd.md`](docs/tooling/ci-cd.md).

## Raspberry Pi 3B+

`tools/make-rpi3b-image.sh` собирает прошиваемый SD-образ (MBR + один 64 МиБ
FAT32 boot-раздел с прошивкой, `config.txt`, `kernel8.img`). Ядро само
опускается EL2→EL1 и работает с PL011; `dtoverlay=miniuart-bt` обязателен, иначе
записи уходят в Bluetooth-модуль. **Никогда не запускалось на кремнии.** Образ
собирается в CI на каждом пуше — дальше этих данных нет.

## Структура репозитория

Ядро — это Cargo-workspace из 20 крейтов `vivanta-*` в
[`vivanta-boot/`](vivanta-boot/):

```
vivanta-boot/
  arch-aarch64/    Поддержка AArch64 (MMU/пейджинг, исключения, GIC, вход в EL0)
  arch-api/        Контракты архитектурного API
  arch-armv7a/     8-строчная замороженная заглушка (ADR-011) — кода ARMv7 нет
  kernel/          Планировщик, системные вызовы, VMM (`kernel/src/vmm/`), модель процессов, загрузка
  exec/            Чистый парсер ELF64 + построитель плана загрузки (без таблиц страниц)
  vm/              Аллокатор диапазонов виртуальных адресов — НЕ виртуализация (VMM — в kernel/src/vmm/)
  boot-info/       Контракт BootInfo, передаваемый загрузчиком
  boot_common/     Общие хелперы загрузки для платформ
  platform-*/      Крейты поддержки плат (qemu, rk3568, rpi3b, sdm660, amlogic, allwinner-h616)
  target-*/        Загружаемые бинарники (qemu-aarch64, rpi3b-plus, rk3568, x96q, lavender, suma-q5)
  user-init/       Единственная настоящая EL0-программа, встраивается в ядро
  tools/           Скрипты CI, QEMU-гейта, сборки образов и soak
```

`user-init/` — отдельный workspace, собираемый `tools/build-user-init.sh`.
Архитектурные документы (ADR, RFC, спеки милстоунов) лежат в
[`vivanta-boot/docs/`](vivanta-boot/docs/); аудиты, заметки по железу и мастер-
роадмап — в [`docs/`](docs/).

## Документация

- [STATUS.md](STATUS.md) — проверенный/непроверенный статус по подсистемам
  (содержит заведомо устаревшие строки; таблица выше перекрывает их там, где они
  расходятся)
- [Открытый бэклог корректности](docs/audit/2026-09-24-global-audit.md) — список P0/P1/P2, который этот README суммирует
- [Архитектурные решения](vivanta-boot/docs/adr/) — 18 ADR, ADR-011 … ADR-035 (016, 024, 025 не написаны)
- [CI/CD](docs/tooling/ci-cd.md) — пайплайны, релизный флоу, локальный ритуал
- [Заметки по bring-up платформ](vivanta-boot/PLATFORM_BRINGUP.md) — состояние загрузки по каждой плате
- [Досье по железу](docs/hardware/) — правда по RK3568, обследования SDM660/whyred, разведка AIBOX-1684X
- [ROADMAP.md](ROADMAP.md) и [мастер-роадмап](docs/architecture/master-roadmap.md) — инженерный план
- [Видение: сетевые сервисы и распределённая ОС](vivanta-boot/docs/rfc/network-services-vision.md)

## Лицензия

[GPLv3](LICENSE). Copyright (C) 2026 Egor Korostelev.
