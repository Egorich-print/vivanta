# Hardware Bring-Up Checklist

Use this checklist when bringing up Vivanta on a new board.

## Boot

- [ ] UART output visible (correct baud rate, pinout)
- [ ] Kernel image loaded (U-Boot `booti` or direct boot)
- [ ] Stack and BSS initialized
- [ ] `kernel_main` reached

## Platform Discovery

- [ ] FDT detected (x0 preserved from bootloader)
- [ ] FDT magic validated (0xD00DFEED)
- [ ] Memory map parsed
- [ ] CPU count detected

## Memory

- [ ] PMM bitmap initialized
- [ ] Kernel memory reserved
- [ ] DTB memory reserved
- [ ] Frame allocation works
- [ ] Frame freeing works

## MMU

- [ ] Page tables allocated
- [ ] RAM identity-mapped
- [ ] UART MMIO mapped
- [ ] GIC MMIO mapped
- [ ] MMU enabled
- [ ] UART still works after MMU enable

## Exceptions

- [ ] VBAR_EL1 set (2048-byte aligned)
- [ ] Exception vectors installed
- [ ] Crash dump works (trigger_fault)
- [ ] ESR decoded correctly
- [ ] Register dump complete

## Interrupt Controller

- [ ] GIC discovered via FDT
- [ ] GIC version detected correctly
- [ ] Distributor initialized
- [ ] CPU interface initialized
- [ ] SGI self-test passes
- [ ] IRQ entry/return works (no crash on IRQ)

## Interrupt Safety

- [ ] `barrier::dsb_sy()` works
- [ ] `barrier::isb()` works
- [ ] `mmio_read32` / `mmio_write32` work
- [ ] `IrqGuard` saves/restores DAIF correctly
- [ ] `SpinLock` acquire/release works
- [ ] `SpinLock` + `IrqGuard` composition works

## Timer

- [ ] Generic Timer frequency read
- [ ] `CNTP_TVAL` set correctly
- [ ] Timer IRQ (ID 30) fires
- [ ] Periodic tick established
- [ ] Tick counter increments

## Scheduler

- [ ] Thread structure defined
- [ ] Context switch works
- [ ] Run queue implemented
- [ ] Multiple threads run
- [ ] Timer preemption works

## Userspace

- [ ] EL0 execution enabled
- [ ] Page table isolation
- [ ] Syscall handler in place
- [ ] ELF loader works
- [ ] Userspace process runs
