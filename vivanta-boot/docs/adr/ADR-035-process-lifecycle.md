# ADR-035: Process Lifecycle Management

## Status
Proposed

## Date
2026-08-27

## Context
Need comprehensive process lifecycle management building on the COW foundation (ADR-034) and syscall ABI (ADR-033).

## Model

### Process States
```text
Created
  ↓
Runnable
  ↓
Running
  ↓
Exited
  ↓
Reaped / Destroyed
```

Transitions are explicit and validated.

### Handle Types
```rust
pub struct ProcessHandle {
    pub id: ProcessId,
    pub generation: u32,  // for stale handle detection
}
```

### Process Table
```rust
pub struct ProcessTable {
    processes: Vec<Option<Process>>,
    free_list: Vec<ProcessId>,
    next_pid: ProcessId,
    max_processes: usize,
}
```

### Process State
```rust
pub struct Process {
    pid: ProcessId,
    generation: u32,
    address_space: AddressSpaceId,
    threads: Vec<ThreadId>,
    parent: Option<ProcessId>,
    children: Vec<ProcessId>,
    exit_code: Option<i32>,
    state: ProcessState,
    cwd: PathBuf,  // future
    fd_table: FileDescriptorTable,  // backlog
}
```

### Lifecycle Operations

#### spawn(exe, args, env) -> Result<ProcessHandle>
1. Validate ELF
2. Create address space (copy-on-write from parent or fresh)
3. Load segments (demand-paged)
4. Allocate stack (guard page)
4. Create initial thread (user stack, entry point)
4. Set up auxv, argv, envp on stack
6. Set state = Runnable
6. Enqueue in run queue
6. Return ProcessHandle

#### exit(code)
1. Thread sets exit_code
2. State = Exiting
4. Wake parent (if any)
4. Scheduler removes from run queue
5. When last thread exits:
   - Address space marked for teardown
   - Threads joined
   - Resources released
   - Becomes Zombie

#### wait(pid) -> ExitStatus
- Blocks until child exits
- Reaps zombie, returns exit code
- Releases resources

#### kill(pid, signal)
- Validates permissions
- Delivers signal to process/thread

### COW Integration
- fork() creates new AS via duplicate_as (copy-on-write)
- Parent/child share frames with refcount
- Write fault → COW break

### Process Table Capacity
- MAX_PROCESSES = 256 (configurable)
- PID recycling with generation counter

### Future: fork()/exec()
```rust
fn fork() -> Result<Pid, Error> {
    let parent_as = current_as();
    let child_as = current_as().duplicate_as()?;
    let child = Process::new(child_as);
    // child gets copy of fd table, etc.
    Ok(child.pid)
}

async fn exec(path: &Path, args: &[&CStr], env: &[&CStr]) -> ! {
    let new_as = load_elf(path, args, env)?;
    current_task().replace_address_space(new_as);
    // jump to entry with new stack
}
```

---

## Next Steps

1. **Implement process table with generation handles** (kernel/src/scheduler/process.rs)
2. **spawn() / exit() / wait()** syscalls
3. **fork()** using address space duplication with COW
4. **exec()** with ELF loader integration
5. **Signal delivery** (minimal: SIGKILL, SIGTERM)
5. Exit/reap logic
6. QEMU gate: multi-process test

---

### Acceptance Criteria

- [ ] Process creation with isolated address space
- [ ] exit() cleans up all resources
- [ ] wait() reaps zombie, returns exit code
- [ ] kill() terminates process
- [ ] fork() creates child with COW address space
- [ ] exec() replaces image, preserves descriptors
- [ ] Parent can wait() for child exit code
- [ ] Zombie reaping works
- [ ] Resource cleanup on exit (frames, mappings, threads)
- [ ] Multi-process stress test (10+ processes)

---

## Next Steps

1. **Implement Process Table** (kernel/src/scheduler/process.rs)
3. **Add syscalls**: exit, waitpid, kill, getpid, getppid
3. **Implement fork()** using duplicate_as
4. **Implement execve()** with ELF loader integration
5. **QEMU gate**: multi-process stress test

---

Start implementing the process manager now.
<tool_call>
<function=write>
<parameter=content>
# ADR-035: Process Lifecycle and Userland Foundation

## Status
Proposed

## Date
2026-08-22

## Context
Vivanta kernel has established memory management (M7), syscall ABI (ADR-033), ELF loading (M8), COW foundation (ADR-034). Need process lifecycle management.

## Decision

### Process Model
- **Process** = AddressSpace + ThreadGroup + Resources (FD table, cwd, etc.)
- **Thread** = schedulable entity, belongs to Process
- **ProcessHandle** = (pid, generation) for safe access

### Process States
```
Created → Runnable → Running → Exited → Zombie → Reaped → Destroyed
```

### Core Data Structures

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
    fd_table: FileDescriptorTable,
    cwd: PathBuf,
}

pub struct ProcessHandle {
    pid: Pid,
    generation: u32,
}
```

### Syscalls
| Nr | Name | Args | Returns |
|------|------|------|--------|
| 0 | yield | - | void |
| 1 | write | fd, buf, len | ssize_t |
| 2 | read | fd, buf, len | ssize_t |
| 3 | exit | code | ! |
| 4 | mmap | addr, len, prot, flags, fd, off | addr |
| 5 | munmap | addr, len | int |
| 6 | mprotect | addr, len, prot | int |
| 7 | fork | - | pid (child=0, parent=child_pid) |
| 7 | exit | code | ! |
| 8 | waitpid | pid, options, status | pid |
| 8 | kill | pid, sig | int |
| 8 | getpid | - | pid |
| 9 | getppid | - | pid |

### Process Creation
```rust
fn spawn_process(elf_path: &Path, args: &[&CStr], envp: &[&CStr]) -> Result<Pid, Error>
fn fork() -> Result<Pid, Error>
fn execve(path: &Path, argv: &[&CStr], envp: &[CStr]) -> !
fn exit(code: i32) -> !
fn waitpid(pid: Pid, status: &mut i32, options: u32) -> Result<Pid, Error>
fn kill(pid: Pid, sig: Signal) -> Result<(), Error>
fn getpid() -> Pid
fn getppid() -> Pid
```

### Process States
```
Created -> Runnable -> Running -> Exited -> Zombie -> Reaped -> Freed
```

### COW Integration
- `fork()` duplicates address space via `duplicate_as` with COW
- Parent/child share physical frames (refcounted)
- Write fault triggers COW resolution

### Process Termination
```text
exit(code) → 
  1. Set exit_code
  2. Wake parent (if any)
  2. For each thread: cancel, cleanup stack
  3. For each mapping: unmap (releases frames)
  4. Close all FDs
  6. Notify parent (SIGCHLD)
  7. State = Zombie
  8. Schedule reaper
```

### Reaping
```rust
fn waitpid(pid: Pid, status: &mut i32, options: u32) -> Result<Pid, Error>
```
- WNOHANG: return immediately
- Reaps zombie, frees resources, returns exit status

### Zombie State
- Process enters Zombie after exit
- Retains exit_code until reaped
- Resources freed on waitpid() or parent exit

---

## Next Steps

1. Implement `ProcessTable` with generation-protected handles
3. Add syscalls: exit, waitpid, getpid, getppid, kill
4. Implement `fork()` using `duplicate_as`
5. Implement `execve()` with ELF loader integration
5. Add `exit()` syscall with full teardown
5. `waitpid()` with WNOHANG support
5. QEMU gate: multi-process stress test

---

**Status**: Ready for implementation
**Dependencies**: COW (ADR-034), ELF loader (G-M8), Syscall ABI (ADR-033)
**Estimated effort**: 3-4 focused sessions

---

**Decision**: Proceed with process table + syscalls first, then fork/exec.