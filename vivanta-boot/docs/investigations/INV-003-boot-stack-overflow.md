# INV-003: Boot-stack overflow via oversized `-O0` kernel_main frame

## Status

Closed (fixed 2026-09-09 during the P0 corrective mission).

## Date

2026-09-09

## Symptom

QEMU boot output stopped at exactly 97 lines (end of `dump_critical_tables`,
`OutputAddr=...`), then silence until the timeout kill. No panic, no fault,
no exception (`-d int` log empty). Content-sensitive: adding ~160 lines of
never-yet-executed boot-gate code to `kernel_main` flipped a booting tree to
a hanging one and back.

## Root cause (proven, not inferred)

1. `kernel_main` runs every boot gate inline. Its `-O0` (dev profile) frame
   had grown to **82 KiB** (measured: prologue `sub sp, #0x14000` in disassembly),
   against a **128 KiB** boot stack (`__stack_bottom..__stack_top`).
2. With gate temporaries (`AddressSpace` by value is ~10 KiB, `VaAllocator`
   ~4 KiB) plus nesting (`register → new → try_new`, ~20 KiB of frames),
   peak usage passed 128 KiB. The stack spilled below `__stack_bottom`
   straight into `.bss` — no guard page exists pre-MMU on flat RAM.
3. The overflowed writes landed on the console area: `GLOBAL_CONSOLE`
   (fat pointer) zeroed to `None`, `CONSOLE_LOCK` left held. The next
   `println!` acquired the lock, then `opt.expect("console not initialized")`
   panicked **while holding it**; the panic handler's own `println!` spun on
   the lock forever. Silent by construction.

## Evidence trail

- lldb + QEMU gdbstub: spinner always in `ConsoleLock::acquire`
  (`boot_common/src/lib.rs:44`), lock word `1`, console pointer `None`.
- Hardware watchpoint on the console area: writer is a 6 KiB `memcpy`
  (`copy_forward_aligned_words`) with dst below `__stack_bottom`,
  backtrace `memcpy ← VaAllocator::try_new ← AddressSpace::new ←
  register ← kernel_main` — i.e. a stack-slot copy executing with SP
  already past the stack bottom.
- Disassembly: `try_new`'s copy targets `sp+0x30b0`; SP at that moment put
  the slot below `__stack_bottom`. Threshold behavior (a few KiB decide
  boot vs hang) matches frame-size sensitivity exactly.
- Counter-evidence ruled out: `-d int` empty (no IRQ/fault), SP healthy at
  spin time (overflow is transient — depth recedes, damage stays), binary
  fresh (md5-pinned), `-cpu max` identical (guest logic, not TCG).

## Fix

1. Boot stack 128 → 256 KiB (`target-qemu-aarch64/linker.ld`), comment keeps
   the history (M6: 16→32, mission-4: 32→64, G-M9: 64→128, P0: 128→256).
2. All six P0 boot gates extracted from `kernel_main` into `#[inline(never)]`
   fns; `kernel_main` frame measured 0x14000 → 0xa000 after.
3. Regressions impossible to miss: the gates themselves now run at greater
   depth than the old inline layout, at 19/19 PASS.

## Addendum (2026-09-09): second instance — 16 KiB thread stacks

The same bug class fired again, one layer down. First EL0 `fork()` died
silently: UART MMIO vanished from the parent's tables mid-`sys_fork`
(measured live: SP 18 KiB past the bottom of the 16 KiB thread stack at
fork entry, before `duplicate_as` even ran). The `-O0` fork path needs
~35 KiB live (`AddressSpace` by value + `VaAllocator` inline array +
`duplicate_as` frames); the overflow smashed adjacent memory, which
presented as wiped page-table entries and a console-lock deadlock —
then, after the 64 KiB fix, as an `ELR=0` abort exposing a second,
independent defect: `context_fork` copied the parent's kernel return
address into the child's x30, so the first-ever scheduled fork child
jumped nowhere. Now fixed (child x30 = `eret_to_user_stub`, like every
fresh user thread — which is what consumes the copied ExceptionFrame).

Fix: `KERNEL_STACK_SIZE` 16→64 KiB (`kernel/src/scheduler/mod.rs`; all
uses go through the constant). Lesson: 16 KiB thread stacks plus
multi-KiB by-value VM structs plus `-O0` is not a combination — audit
any path that moves `AddressSpace`/`VaAllocator` by value on a thread
stack, and distrust "it works synthetically": the M10.2 synthetic fork
never scheduled its child, so neither defect could fire.

## Prevention

- Keep `kernel_main`'s `-O0` frame small: new boot gates go to
  `#[inline(never)]` fns (established pattern: `gate_m10_3_execve`,
  `gate_double_execve`, `gate_ptable_lifetime`, `gate_kill_sigchld`,
  `gate_fault_oom`, `gate_e2e_delta`).
- The linker comment is the tripwire: bump the stack again if a measured
  prologue approaches half of it. Measure with:
  `llvm-objdump --arch-name=aarch64 -d <elf> --disassemble-symbols=...kernel_main`
  and read the `sub sp` probe prologue total.
- Never print while holding the console lock across a fallible path: the
  `expect("console not initialized")` inside `with_console` turns any
  console-pointer corruption into an undebuggable silent deadlock. A
  future hardening is to release-then-panic there (or a lock-free early
  print for the panic path).
