[English](README.md) · **Русский**

---

# Vivanta

[![CI](https://github.com/Egorich-print/vivanta/actions/workflows/ci.yml/badge.svg)](https://github.com/Egorich-print/vivanta/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/Egorich-print/vivanta?include_prereleases&label=release&color=blue)](https://github.com/Egorich-print/vivanta/releases)
![License: GPLv3](https://img.shields.io/badge/license-GPLv3-blue)
![Rust 1.98](https://img.shields.io/badge/rust-1.98.0-orange)
![Platform: AArch64](https://img.shields.io/badge/platform-AArch64-lightgrey)

Экспериментальная операционная система, исследующая **непрерывность идентичности**
(identity continuity), **ресурсно-ориентированную память** и **портативную
архитектуру загрузки**.

Vivanta пишется bare-metal под разнородное железо — системы на AArch64 и ARMv7,
от эмулируемых машин QEMU до реальных плат и старых смартфонов (RK3568,
Raspberry Pi 3B+, SoC Qualcomm). Ядро — `no_std` Cargo-workspace на Rust.

## Статус

**QEMU-correct, но ещё не hardware-correct.** На QEMU `virt` AArch64-ядро
доходит до `kernel_main`, проходит всю матрицу гейтов (**22/22 PASS, 0 паник**)
и запускает настоящий EL0-userland. Валидации на кремнии пока нет.

| Область | Статус |
|---------|--------|
| Загрузка (QEMU AArch64) | ✅ |
| Менеджер физической памяти (PMM, вся доступная RAM) | ✅ |
| Виртуальная память (адресные пространства, map/unmap, владение и рекламация таблиц) | ✅ |
| Demand paging (ленивая анонимная память) | ✅ |
| Copy-on-Write для анонимной приватной памяти | ✅ |
| Планировщик (приоритеты, вытеснение 100 Hz, sleep/wake) | ✅ |
| Модель процессов (задачи, потоки, таблица процессов, жизненный цикл) | ✅ |
| ELF64 AArch64 загрузчик (с валидацией, W^X) | ✅ |
| Граница пользовательской памяти (`access_ok`, `-EFAULT`) и сдерживание EL0-ошибок | ✅ |
| Системные вызовы (`read`, `write`, `exit`, `yield`, `mmap`, `munmap`, `mprotect`, `fork`, `waitpid`, `execve`) | ✅ |
| EL0-userland: hello через `write`, `execve`, round-trip `fork`/`waitpid` | ✅ |
| Сигналы: `SIGKILL`/`SIGCHLD` на синхронных путях | ✅ |

Вне области (сознательно не реализовано): IPC · хранилища · драйверы · сеть ·
Ed25519/BIP-39 идентичность · TTBR1/ASID · асинхронная доставка сигналов в EL0.
См. [STATUS.md](STATUS.md).

## Быстрый старт (QEMU AArch64)

Требуется [Rust](https://rustup.rs) (1.98.0, зафиксирован в
`vivanta-boot/rust-toolchain.toml`) и QEMU.

```bash
git clone https://github.com/Egorich-print/vivanta.git
cd vivanta/vivanta-boot

rustup target add aarch64-unknown-none   # один раз
./tools/build-user-init.sh               # EL0-образ; ядро встраивает его
cargo build -p vivanta-target-qemu-aarch64

qemu-system-aarch64 -M virt -cpu cortex-a53 -m 512M -nographic \
  -kernel target/aarch64-unknown-none/debug/vivanta-target-qemu-aarch64 \
  -serial mon:stdio
```

В конце лога первая user-space программа выполняется в EL0 через syscall ABI
и чисто завершается:

```
Hello, Vivanta!
syscall: exit(0)
```

Проверить загрузку автоматически (тот же гейт, что и в CI):

```bash
./tools/qemu-gates.sh          # 18 маркеров, PASS/FAIL, ненулевой код при провале
./tools/ci-local.sh            # весь пайплайн: fmt, lint, host-тесты, QEMU
```

## Непрерывная интеграция

Каждый push и pull request запускает [`.github/workflows/ci.yml`](.github/workflows/ci.yml):

- **lint** — `rustfmt`, синтаксис shell/python, exec-биты;
- **test** — host unit-тесты (`vivanta-vm`, `vivanta-exec`) нативно на x86_64;
- **build** — сборка встроенного EL0-образа и QEMU-ядра;
- **qemu** — загрузка ядра под `cortex-a53` (TCG) с проверкой матрицы гейтов;
- **image** — сборка прошиваемого SD-образа для Raspberry Pi 3B+.

Релизы по тегу публикуются через
[`.github/workflows/release.yml`](.github/workflows/release.yml) в
[Releases](https://github.com/Egorich-print/vivanta/releases): SD-образ,
`kernel8.img`, QEMU-ELF, плоский бинарник для RK3568 и EL0-образ.
Детали и локальный ритуал: [`docs/tooling/ci-cd.md`](docs/tooling/ci-cd.md).

## Запуск на Raspberry Pi 3B+

`tools/make-rpi3b-image.sh` собирает прошиваемый SD-образ (MBR + FAT32
boot-раздел с прошивкой, `config.txt` и `kernel8.img`) для основной
аппаратной цели. Ядро само опускается EL2→EL1 и работает с PL011 на
GPIO 14/15. На кремнии пока не проверялось — см.
[`vivanta-boot/PLATFORM_BRINGUP.md`](vivanta-boot/PLATFORM_BRINGUP.md).

## Структура репозитория

Ядро — Cargo-workspace из небольших крейтов `vivanta-*` в
[`vivanta-boot/`](vivanta-boot/):

```
vivanta-boot/
  arch-aarch64/    Поддержка AArch64 (MMU, исключения, вход в EL0)
  arch-armv7a/     Поддержка ARMv7 (замороженный stub)
  arch-api/        Контракты архитектурного API
  kernel/          Планировщик, системные вызовы, VMM, загрузка
  boot-info/       Контракт BootInfo, передаваемый загрузчиком
  boot_common/     Общие хелперы платформ
  platform-*/      Крейты поддержки плат (qemu, rk3568, rpi3b, sdm660, …)
  target-*/        Загружаемые бинарники (qemu-aarch64, rpi3b-plus, rk3568, …)
  user-init/       EL0-программа, встраиваемая в ядро
  tools/           Скрипты CI, QEMU-гейта и сборки образов
```

Архитектурные документы (ADR, RFC, чек-листы милстоунов) — в
[`vivanta-boot/docs/`](vivanta-boot/docs/); история проекта и мастер-роадмап —
в [`docs/`](docs/).

## Документация

- [Мастер-роадмап](docs/architecture/master-roadmap.md) — главный инженерный план
- [ROADMAP.md](ROADMAP.md) — краткая публичная сводка милстоунов
- [STATUS.md](STATUS.md) — честный статус проверенного/непроверенного
- [Архитектурные решения](vivanta-boot/docs/adr/) — ADR-011 … ADR-035
- [CI/CD](docs/tooling/ci-cd.md) — пайплайны, релизный флоу, локальный ритуал
- [Видение: сетевые сервисы и распределённая ОС](vivanta-boot/docs/rfc/network-services-vision.md)
- [Исследование: бюджетные смартфоны как узлы кластера](docs/research/cluster_research.md)

## Лицензия

[GPLv3](LICENSE). Copyright (C) 2026 Egor Korostelev.
