[English](README.md) · **Русский**

---

# Vivanta

![Status: experimental](https://img.shields.io/badge/status-experimental-red)
![License: GPLv3](https://img.shields.io/badge/license-GPLv3-blue)
![Language: Rust](https://img.shields.io/badge/language-Rust-orange)
![Platform: ARM64 / ARMv7](https://img.shields.io/badge/platform-ARM64%20%2F%20ARMv7-lightgrey)

Экспериментальная операционная система, исследующая **непрерывность идентичности**
(identity continuity), **ресурсно-ориентированную память** и **портативную
архитектуру загрузки**.

Vivanta изначально проектируется для работы на разнородном железе — системах на
ARM64 и ARMv7, от эмулируемых машин QEMU до реальных плат и старых смартфонов
(RK3568, Raspberry Pi 3B, SoC Qualcomm).

## Что работает сейчас

| Область | Статус |
|---------|-------|
| Загрузка ядра (QEMU AArch64) | ✅ |
| Менеджер физической памяти (PMM, вся доступная RAM) | ✅ |
| Страничная память / VMM (адресные пространства) | ✅ |
| Менеджер ресурсов памяти (MRM) с регламентом | ✅ |
| Планировщик (приоритеты, вытеснение, sleep/wake) | ✅ |
| Модель процессов (задачи, потоки, таблица процессов) | ✅ |
| Системные вызовы (`read`, `write`, `exit`, `yield`, `mmap`) | ✅ |
| **Copy-on-Write для анонимной приватной памяти** | ✅ **NEW** |
| Первая user-space программа в EL0 | ✅ |
| Граница пользовательской памяти (`access_ok`, copy, `-EFAULT`) | ✅ |
| Содержание EL0 (сдерживание ошибок EL0) | ✅ |
| Таймерная вытеснение (100 Hz, два живых потока) | ✅ |

> **M9 COW COMPLETE** — Copy-on-Write для анонимной приватной памяти реализован и проверен.
> Первая настоящая ELF64 AArch64 userland-программа выполняется в EL0 через syscall ABI,
> *demand-fills при первом доступе*, меняет защиту через `mprotect`, освобождает через `munmap`,
> *упражняет COW fork семантику*, и чисто завершается. **Все 9 QEMU гейтов пройдены.**

> **M9 COW COMPLETE** — Copy-on-Write для анонимной приватной памяти реализован и проверен.
> Первая настоящая ELF64 AArch64 userland-программа выполняется в EL0 через syscall ABI,
> *demand-fill при первом доступе*, меняет защиту через `mprotect`, освобождает через `munmap`,
> *упражняет COW fork семантику*, и чисто завершается. **Все 9 QEMU гейтов пройдены.**

> **M9 COW COMPLETE** — Copy-on-Write для анонимной приватной памяти реализован и проверен.
> Первая настоящая ELF64 AArch64 userland-программа выполняется в EL0 через syscall ABI,
> *demand-fill при первом доступе*, меняет защиту через `mprotect`, освобождает через `munmap`,
> *упражняет COW fork семантику*, и чисто завершается. **Все 9 QEMU гейтов пройдены.**

> **G-M7 CLOSED** — syscall ABI + VM syscalls + Process model **COMPLETE**
> **M9 COW COMPLETE** — Copy-on-Write для анонимной приватной памяти **COMPLETE**

> **M7 GREEN BASELINE — PASS** (2026-08-11). QEMU-correct baseline: all four gates verified on a clean clone. Honest status is "QEMU-correct", not "hardware-correct" — one deferred ARM MMU descriptor-encoding issue requires validation on physical hardware.

---

## Быстрый старт (QEMU AArch64)

Требуется: тулчейн Rust, таргет `aarch64-unknown-none` и QEMU.

```bash
rustup target add aarch64-unknown-none   # один раз

cd vivanta-boot
cargo build -p vivanta-target-qemu-aarch64

qemu-system-aarch64 -M virt -cpu cortex-a53 -m 512M -nographic \
  -kernel target/aarch64-unknown-none/debug/vivanta-target-qemu-aarch64 \
  -serial mon:stdio
```

В конце лога загрузки первая user-space программа запускается в EL0, печатает
через системный вызов `write` и чисто завершается:

```
Hello, Vivanta!
syscall: exit(0)
```

## Структура репозитория

Исходники ядра — Cargo-workspace из небольших крейтов `vivanta-*` в
[`vivanta-boot/`](vivanta-boot/):

```
vivanta-boot/
  arch-aarch64/    Поддержка AArch64 (MMU, исключения, вход в EL0)
  arch-armv7a/     Поддержка ARMv7 (заморожено, WIP)
  arch-api/        Контракты архитектурного API
  kernel/          Планировщик, системные вызовы, загрузка
  boot-info/       Контракт BootInfo, передаваемый загрузчиком
  boot_common/     Общие хелперы платформ
  platform-*/      Крейты поддержки плат (qemu, rk3568, rpi3b, sdm660, …)
  target-*/        Загружаемые бинарники (qemu-aarch64, rk3568, rpi3b, sdm660, …)
  tools/           Reliability/soak test скрипты
```

Архитектурные документы (ADR, RFC, чек-листы милстоунов) — в
[`vivanta-boot/docs/`](vivanta-boot/docs/); история проекта и организационные
заметки — в [`docs/`](docs/).

## Документация

- [Мастер-роадмап](docs/architecture/master-roadmap.md) — главный инженерный план
- [M5.0 GREEN BASELINE](vivanta-boot/docs/milestones/M5.0-green-baseline.md) — ратифицированное восстановление (источник правды)
- [Архитектурные решения](vivanta-boot/docs/adr/) — ADR-011 … ADR-034
- [Видение: сетевые сервисы и распределённая ОС](vivanta-boot/docs/rfc/network-services-vision.md)
- [Исследование: бюджетные смартфоны как узлы кластера](docs/research/cluster_research.md)

## Роадмап

Краткая версия — в [ROADMAP.md](ROADMAP.md). M5.0 (recovery baseline) —
**PASS/CLOSED**. Следующий milestone (M6) определяется из актуального состояния
репозитория, а не из пред-M5 дорожной карты. См.
[`vivanta-boot/docs/milestones/`](vivanta-boot/docs/milestones/).

## Лицензия

[GPLv3](LICENSE). Copyright (C) 2026 Egor Korostelev.