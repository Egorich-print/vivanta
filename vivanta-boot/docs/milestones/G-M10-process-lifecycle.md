# G-M10: Process Lifecycle & True COW Foundation

## Status: PLANNING
## Date: 2026-08-27

---

## Mission Objective

Transform Vivanta from a kernel with COW primitives into a fully functional process-oriented OS with:
- Complete process lifecycle (spawn, exec, exit, wait, signals)
- True copy-on-write fork() with shared physical pages
- Deterministic process teardown and resource reclamation
- Multi-process isolation with verified COW semantics

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                        PROCESS LIFECYCLE                         │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  spawn() → [Created] → Runnable → Running → Exited → Zombie    │
│                    ↑                    │          │              │
│                    └────────────────────┘          │              │
│                                                   ▼              │
│                                              [Reaper]            │
│                                              (waitpid/reap)      │
└─────────────────────────────────────────────────────────────────┘
```

---

## Phase 1: Core Process Infrastructure (Week 1-2)

### 1.1 Process Control Block (PCB) Enhancement
**File:** `kernel/src/scheduler/process.rs`

```rust
pub struct Process {
    pid: Pid,
    generation: u32,
    asid: AddressSpaceId,
    threads: Vec<ThreadId>,
    parent: Option<Pid>,
    children: Vec<Pid>,
    exit_code: Option<i32>,
    state: ProcessState,
    cwd: PathBuf,
    fd_table: FileDescriptorTable,
    signal_state: SignalState,
    // COW support
    cow_domains: Vec<CowDomain>,  // shared memory regions
}
```

### 1.2 Process States
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessState {
    Created,      // Allocated, not yet runnable
    Runnable,     // Ready to run
    Running,      // Currently executing
    Blocked,      // Waiting for event (IPC, I/O, signal)
    Exiting,      // exit() called, draining
    Zombie,       // Exited, awaiting waitpid()
    Dead,         // Reaped, resources freed
}
```

### 1.3 Process Table with Generational Handles
```rust
pub struct ProcessTable {
    processes: Vec<Option<Process>>,
    free_list: Vec<Pid>,
    next_pid: Pid,
    max_processes: usize,
}

// Handles are (pid, generation) pairs - prevents ABA problems
pub struct ProcessHandle {
    pid: Pid,
    generation: u32,
}

impl ProcessTable {
    pub fn allocate(&mut self, proc: Process) -> ProcessHandle { ... }
    pub fn get(&self, handle: ProcessHandle) -> Option<&Process> { ... }
    pub fn get_mut(&mut self, handle: ProcessHandle) -> Option<&mut Process> { ... }
    pub fn remove(&mut self, pid: Pid) -> Option<Process> { ... }
}
```

---

## Phase 2: Syscall Interface (Week 2)

### 2.1 Core Process Syscalls
| Nr | Name | Args | Returns |
|----|------|------|---------|
| 0 | `yield()` | - | `()` |
| 1 | `write(fd, buf, len)` | `fd, buf, len` | `ssize_t` |
| 2 | `read(fd, buf, len)` | `fd, buf, len` | `ssize_t` |
| 3 | `exit(code)` | `code` | `!` |
| 4 | `fork()` | - | `pid` (0 in child, child_pid in parent) |
| 5 | `exit(code)` | `code` | `!` (noreturn) |
| 6 | `waitpid(pid, opts, status)` | `pid, opts, *mut i32` | `pid` |
| 7 | `kill(pid, sig)` | `pid, sig` | `int` |
| 8 | `getpid()` | - | `pid` |
| 9 | `getppid()` | - | `pid` |

### 2.2 Syscall Infrastructure
- Extend `syscall.rs` with new syscall numbers
- Add argument validation
- Implement proper error codes (`EAGAIN`, `ECHILD`, `ESRCH`, `EINVAL`)

---

## Phase 3: Fork Implementation (Week 3)

### 3.1 fork() Semantics
```rust
// Kernel side
fn sys_fork() -> Result<Pid, Error> {
    let current = current_task();
    let child_as = current.address_space().duplicate_as()?;  // COW!
    let child_pid = scheduler.spawn_thread(child_entry, child_as);
    
    // Parent gets child PID
    // Child gets PID 0 (via register)
    Ok(child_pid)
}
```

### 3.2 COW Fork Implementation
```
fork()
  │
  ├─ duplicate_as()           // Copy address space metadata (COW)
  │    ├─ Copy page table hierarchy (lazy, copy-on-write)
  │    ├─ Increment refcounts on shared frames
  │    └─ Clear write bits in both parent/child PTEs
  │
  ├─ Allocate child task struct
  │    ├─ Copy registers (x0=0 for child, =pid for parent)
  │    ├─ Inherit file descriptors (refcount++)
  │    └─ Inherit signal handlers
  │
  ├─ Schedule child as runnable
  └─ Return child PID to parent, 0 to child
```

### 3.3 COW Page Fault Handler
```rust
// In page fault handler:
if fault.is_write() && mapping.is_cow() {
    // Allocate new frame
    let new_frame = alloc_frame()?;
    
    // Copy data from old frame
    copy_page(old_frame, new_frame);
    
    // Update page table: new frame, writable, same permissions
    map_page(fault_addr, new_frame, PERM_RW);
    
    // Decrement old frame refcount
    decrement_refcount(old_frame);
    
    // Update page table entry to point to new frame, writable
    // TLBI flush
    
    // Resume execution (retry instruction)
} else {
    // Not COW -> SIGSEGV / SIGBUS
}
```

---

## Phase 4: Execve (Week 4)

### 4.1 execve() Implementation
```rust
fn sys_execve(path: &CStr, argv: &[&CStr], envp: &[CStr]) -> ! {
    // 1. Validate current process can exec
    // 2. Load ELF (reuse existing loader)
    // 3. Create new address space
    // 4. Map segments (text, data, bss)
    // 5. Setup stack with argv/envp/auxv
    // 6. Destroy old address space (free pages, close fds)
    // 6. Switch to new address space
    // 7. Setup initial registers (SP, PC, argc/argv/envp)
    // 9. Return to user mode (ERET)
}
```

### 4.2 ELF Loader Integration
- Reuse existing `exec::load_elf()`
- Map PT_LOAD segments with proper permissions
- Handle BSS (zero-fill)
- Set up stack with argv/envp/auxv
- Set entry point in TCR

---

## ADR-036: Process Model
- File: `docs/adr/ADR-036-process-model.md`
- Covers: Process states, fork/exec semantics, signal handling, zombie reaping

---

## Testing Strategy

### Unit Tests (Host)
```rust
#[test]
fn fork_child_gets_cow_pages() { ... }
#[test]
fn parent_write_triggers_cow() { ... }
#[test]
fn child_modification_isolated() { ... }
#[test]
fn double_fork_cow_independence() { ... }
```

### QEMU Integration Tests
```bash
# Run fork test
cargo test -p vivanta-target-qemu-aarch64 fork_test -- --nocapture

# Stress test
for i in 1..100; do
    cargo test fork_stress -- --nocapture
done
```

---

## Milestone Acceptance Criteria

| Milestone | Criteria | Status |
|-----------|----------|--------|
| M1: Process Table | Create/exit/waitpid work | ⬜ |
| M2 | fork() with COW | Verified by stress test |
| M3 | execve() loads ELF | M6 compatibility |
| M4 | Full init → fork → exec → wait | Integration test |
| M5 | Signal handling (SIGKILL, SIGTERM) | ⏳ Later |

---

## Dependencies
- ✅ COW (ADR-034) - DONE
- ✅ ELF loader (M8) - DONE
- ⏳ Process table - IN PROGRESS
- ⏳ Syscall framework - IN PROGRESS

---

## Next Immediate Actions

1. **This Week**: Implement `ProcessTable` with generational handles
2. **Next**: `fork()` syscall with COW page table duplication
3. **Then**: `exit()` + `waitpid()` + zombie reaping
4. **Then**: `execve()` with ELF loader integration
5. **Finally**: Multi-process QEMU stress test

---

## References
- ADR-034: COW Semantics
- ADR-033: Syscall ABI
- ADR-032: User VM Fault Policy
- Linux fork(2)/execve(2) man pages
- xv6 book Chapter 3 (Process Management)
- FreeBSD/illumos procfs implementation notes

---

## Notes
- Keep kernel preemptible during fork (use fine-grained locks)
- Track COW pages with reference counting per physical frame
- Handle OOM during fork gracefully (EAGAIN/ENOMEM)
- Preserve file descriptor table across fork (dup fd table)
- Signal handling: inherit dispositions, reset handlers on exec

---

*Document version: 1.0 | Last updated: 2026-08-27*
*Next review: After M10.1 completion*