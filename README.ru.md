[English](README.md) · **Русский**

---

# Vivanta

![Status: experimental](https://img.shields.io/badge/status-experimental-red)
![Лицензия: GPLv3](https://img.shields.io/badge/license-GPLv3-blue)
![Язык: Rust](https://img.shields.io/badge/language-Rust-orange)
![Платформа: ARM64 / ARMv7](https://img.shields.io/badge/platform-ARM64%20%2F%20ARMv7-lightgrey)

Экспериментальная операционная система, исследующая **непрерывность идентичности**
(identity continuity), **ресурсно-ориентированную память** и **портативную
архитектуру загрузки**.

Vivanta изначально проектируется для работы на разнородном железе — системах на
ARM64 и ARMv7: от эмулируемых машин QEMU до реальных плат и старых смартфонов
(RK3568, Raspberry Pi 3B, SoC Qualcomm).

## Что работает сейчас

| Область | Статус |
|---------|--------|
| Загрузка ядра (QEMU AArch64) | ✅ |
| Менеджер физической памяти (PMM) | ✅ |
| Страничная память / VMM (адресные пространства) | ✅ |
| Менеджер ресурсов памяти (MRM) | ✅ |
| Планировщик (приоритеты, вытеснение, sleep/wake) | ✅ |
| Модель процессов (задачи, потоки, таблица процессов) | ✅ |
| Системные вызовы (`read`, `write`, `exit`, `yield`, `mmap`) | ✅ |
| **Первая user-space программа в EL0** | ✅ milestone M4.5 |

Подробнее: [STATUS.md](STATUS.md) · [Зрелость ОС](docs/OS_MATURITY.md) ·
[Мастер-роадмап](docs/architecture/master-roadmap.md)

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
текст через системный вызов `write` и чисто завершается:

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
  arch-armv7a/     Поддержка ARMv7 (WIP)
  arch-api/        Контракты архитектурного API
  kernel/          Планировщик, системные вызовы, загрузка
  boot-info/       Контракт BootInfo, передаваемый загрузчику
  boot_common/     Общие хелперы платформ
  platform-*/      Крейты поддержки плат (qemu, rk3568, rpi3b, sdm660, …)
  target-*/        Загружаемые бинарники (qemu-aarch64, rk3568, …)
  user/            Минимальная user-space libc и hello-world программа
```

Архитектурные документы (ADR, RFC, чек-листы милстоунов) — в
[`vivanta-boot/docs/`](vivanta-boot/docs/); история проекта и организационные
заметки — в [`docs/`](docs/).

## Документация

- [Мастер-роадмап](docs/architecture/master-roadmap.md) — главный инженерный
  план (милстоуны M1–M5+)
- [Архитектурные решения](vivanta-boot/docs/adr/) — ADR-011 … ADR-030
- [Милстоуны](vivanta-boot/docs/architecture/milestones/) — чек-листы M4, M4.5
- [Видение: сетевые сервисы и распределённая ОС](vivanta-boot/docs/rfc/network-services-vision.md)
- [Исследование: б/у смартфоны как узлы кластера](docs/research/cluster_research.md)

## Роадмап

Краткая версия — в [ROADMAP.md](ROADMAP.md). Текущий фокус: милстоун M5 —
интеграция Memory Resource Manager (ADR-025), затем user-space сервисы, IPC
и драйверы.

## Лицензия

[GPLv3](LICENSE). Copyright (C) 2026 Egor Korostelev.
