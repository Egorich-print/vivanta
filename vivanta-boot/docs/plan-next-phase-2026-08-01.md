# Vivanta Next Phase Plan — 2026-08-01

## Overview

Full sequential implementation: Scheduler v2 → Process Model → EL0/Userspace.
Total: 10 PRs across 3 phases.

## Phase 1: Scheduler v2 (PR1-PR4)

### PR1: ThreadState Expansion

**Goal:** Add proper thread lifecycle states.

**Changes:**
- `scheduler/thread.rs`: Expand `ThreadState` enum
  ```rust
  pub enum ThreadState {
      Created,    // Just allocated, not yet scheduled
      Ready,      // In runqueue, eligible to run
      Running,    // Currently executing on CPU
      Sleeping,   // Temporarily suspended (timer-based)
      Blocked,    // Waiting on resource/IPC
      Terminated, // Finished execution
  }
  ```
- `scheduler/mod.rs`: Update `find_next_ready()` — only consider `Ready` state
- `scheduler/mod.rs`: Add `thread_set_state(id, state)` helper
- `scheduler/mod.rs`: Update `maybe_reschedule()` — skip non-Ready threads

**Files:**
- `kernel/src/scheduler/thread.rs`
- `kernel/src/scheduler/mod.rs`

---

### PR2: Priority Scheduling

**Goal:** Priority-based scheduling with round-robin within same priority.

**Changes:**
- `scheduler/thread.rs`: Add `Priority` field
  ```rust
  #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
  pub enum Priority {
      Realtime = 0,  // Highest
      High = 1,
      Normal = 2,
      Low = 3,
      Idle = 4,      // Lowest (idle thread)
  }
  ```
- `scheduler/mod.rs`: Modify `find_next_ready()` — scan by priority
  ```rust
  fn find_next_ready(from: usize) -> usize {
      for priority in [Realtime, High, Normal, Low] {
          for i in 1..n {
              let idx = (from + i) % n;
              if idx == IDLE_SLOT { continue; }
              if let Some(ref t) = RUNQUEUE[idx] {
                  if t.state == ThreadState::Ready && t.priority == priority {
                      return idx;
                  }
              }
          }
      }
      IDLE_SLOT  // Fallback to idle
  }
  ```
- `scheduler/mod.rs`: Update `create_kernel_thread()` — accept `Priority` parameter
- `scheduler/mod.rs`: Idle thread has `Priority::Idle`

**Files:**
- `kernel/src/scheduler/thread.rs`
- `kernel/src/scheduler/mod.rs`

---

### PR3: Dynamic RunQueue

**Goal:** Replace static array with dynamic queue, remove MAX_THREADS limit.

**Changes:**
- `scheduler/runqueue.rs`: Implement `RunQueue` struct
  ```rust
  pub struct RunQueue {
      threads: Vec<Option<Thread>>,
      count: usize,
  }

  impl RunQueue {
      pub fn new() -> Self;
      pub fn insert(&mut self, thread: Thread) -> Result<ThreadId, SchedError>;
      pub fn remove(&mut self, id: ThreadId) -> Option<Thread>;
      pub fn get(&self, id: ThreadId) -> Option<&Thread>;
      pub fn get_mut(&mut self, id: ThreadId) -> Option<&mut Thread>;
      pub fn find_next_ready(&self, from: ThreadId, priority: Priority) -> Option<ThreadId>;
      pub fn iter_ready(&self, priority: Priority) -> impl Iterator<Item = &Thread>;
  }
  ```
- `scheduler/mod.rs`: Replace `static mut RUNQUEUE` with `static mut RUNQUEUE: RunQueue`
- `scheduler/mod.rs`: Remove `MAX_THREADS` constant
- `scheduler/mod.rs`: Update all access patterns

**Files:**
- `kernel/src/scheduler/runqueue.rs`
- `kernel/src/scheduler/mod.rs`

---

### PR4: Sleep/Wake Primitives

**Goal:** Timer-based sleep and resource-based blocking.

**Changes:**
- `scheduler/thread.rs`: Add sleep fields
  ```rust
  pub struct Thread {
      // ... existing fields
      pub sleep_until: Option<u64>,  // Tick count when to wake
      pub blocked_on: Option<ResourceId>,  // Resource waiting for
  }
  ```
- `scheduler/mod.rs`: Add `sleep(duration_ticks)` function
  - Set `thread.sleep_until = current_tick + duration`
  - Set `thread.state = ThreadState::Sleeping`
  - Call `yield_now()`
- `scheduler/mod.rs`: Add `wake(thread_id)` function
  - Set `thread.state = ThreadState::Ready`
  - Clear `thread.sleep_until`
- `scheduler/mod.rs`: Modify `scheduler_tick()` — check sleeping threads
  ```rust
  fn check_sleeping_threads() {
      let now = get_tick_count();
      for thread in runqueue.iter_mut() {
          if thread.state == ThreadState::Sleeping {
              if let Some(wake_at) = thread.sleep_until {
                  if now >= wake_at {
                      thread.state = ThreadState::Ready;
                      thread.sleep_until = None;
                  }
              }
          }
      }
  }
  ```
- `scheduler/mod.rs`: Add `block_on(resource_id)` function
  - Set `thread.blocked_on = Some(resource_id)`
  - Set `thread.state = ThreadState::Blocked`
  - Call `yield_now()`

**Files:**
- `kernel/src/scheduler/thread.rs`
- `kernel/src/scheduler/mod.rs`

---

## Phase 2: Process Model (PR5-PR7)

### PR5: Task Lifecycle

**Goal:** Full task lifecycle with exit, wait, and zombie states.

**Changes:**
- `scheduler/task.rs`: Expand `TaskState`
  ```rust
  pub enum TaskState {
      Created,    // Just allocated
      Running,    // Has active threads
      Exited,     // Finished, waiting for parent to collect
      Zombie,     // Exited but not yet waited on
  }
  ```
- `scheduler/task.rs`: Add `Task::exit(code: i32)`
  - Set `self.state = TaskState::Zombie`
  - Set `self.exit_code = code`
  - Wake parent if blocked on `wait()`
- `scheduler/task.rs`: Add `Task::wait(pid: TaskId)`
  - Block until child exits
  - Collect exit code
  - Remove child from process table
- `scheduler/task.rs`: Add parent/child tracking
  ```rust
  pub struct Task {
      // ... existing fields
      pub parent: Option<TaskId>,
      pub children: Vec<TaskId>,
      pub exit_code: Option<i32>,
  }
  ```

**Files:**
- `kernel/src/scheduler/task.rs`
- `kernel/src/scheduler/task_manager.rs`

---

### PR6: Task→Thread→AddressSpace Binding

**Goal:** Proper binding between Task, Thread, and AddressSpace.

**Changes:**
- `scheduler/task.rs`: Task owns AddressSpace
  ```rust
  pub struct Task {
      pub id: TaskId,
      pub address_space: AddressSpaceId,
      pub threads: Vec<ThreadId>,
      pub owned_objects: Vec<MemoryObject>,
      pub state: TaskState,
      pub parent: Option<TaskId>,
      pub children: Vec<TaskId>,
      pub exit_code: Option<i32>,
  }
  ```
- `scheduler/task_manager.rs`: `spawn()` creates all three
  ```rust
  fn spawn(&mut self, ...) -> TaskId {
      let as_id = vmm::register(root, AddressSpaceFlags::User);
      let tid = scheduler::create_user_thread(..., as_id);
      let task = Task::new(as_id, vec![tid], ...);
      self.tasks.insert(task)
  }
  ```
- `scheduler/task_manager.rs`: `fork()` — duplicates AddressSpace
  ```rust
  fn fork(&mut self, parent_id: TaskId) -> TaskId {
      let parent = self.get(parent_id);
      let new_root = copy_page_table(parent.address_space);
      let as_id = vmm::register(new_root, AddressSpaceFlags::User);
      // Copy memory objects...
      // Create new thread with same context...
  }
  ```

**Files:**
- `kernel/src/scheduler/task.rs`
- `kernel/src/scheduler/task_manager.rs`
- `kernel/src/vmm/address_space.rs`

---

### PR7: Process Table

**Goal:** Global process registry with parent/child tracking.

**Changes:**
- New file: `scheduler/process_table.rs`
  ```rust
  pub struct ProcessTable {
      tasks: Vec<Option<Task>>,
      next_pid: TaskId,
  }

  impl ProcessTable {
      pub fn new() -> Self;
      pub fn create(&mut self, task: Task) -> TaskId;
      pub fn lookup(&self, pid: TaskId) -> Option<&Task>;
      pub fn lookup_mut(&mut self, pid: TaskId) -> Option<&mut Task>;
      pub fn remove(&mut self, pid: TaskId) -> Option<Task>;
      pub fn children_of(&self, parent: TaskId) -> Vec<TaskId>;
  }
  ```
- `scheduler/mod.rs`: Add `static mut PROCESS_TABLE: ProcessTable`
- `scheduler/task_manager.rs`: Delegate to ProcessTable
- `scheduler/mod.rs`: Add `waitpid(pid)` syscall handler

**Files:**
- New: `kernel/src/scheduler/process_table.rs`
- `kernel/src/scheduler/mod.rs`
- `kernel/src/scheduler/task_manager.rs`

---

## Phase 3: EL0 / Userspace (PR8-PR10)

### PR8: SVC Handler + Syscall Table

**Goal:** Full syscall dispatch from EL0.

**Changes:**
- `arch-aarch64/src/exceptions.rs`: Enhance SVC handler
  ```rust
  fn handle_svc(frame: &mut ExceptionFrame, esr: u64) {
      let imm16 = (esr & 0xFFFF) as u16;
      match imm16 {
          0 => syscall_read(frame),
          1 => syscall_write(frame),
          2 => syscall_exit(frame),
          3 => syscall_yield(frame),
          4 => syscall_mmap(frame),
          _ => panic!("Unknown syscall {}", imm16),
      }
  }
  ```
- New file: `kernel/src/syscall/mod.rs`
  ```rust
  pub fn dispatch(imm16: u16, frame: &mut ExceptionFrame) {
      match imm16 {
          SYS_READ => sys_read(frame),
          SYS_WRITE => sys_write(frame),
          SYS_EXIT => sys_exit(frame),
          SYS_YIELD => sys_yield(frame),
          SYS_MMAP => sys_mmap(frame),
          _ => frame.x[0] = -ENOSYS as u64,
      }
  }
  ```
- New file: `kernel/src/syscall/io.rs` — `sys_read`, `sys_write`
- New file: `kernel/src/syscall/process.rs` — `sys_exit`, `sys_yield`, `sys_mmap`

**Files:**
- `arch-aarch64/src/exceptions.rs`
- New: `kernel/src/syscall/mod.rs`
- New: `kernel/src/syscall/io.rs`
- New: `kernel/src/syscall/process.rs`

---

### PR9: Signal Model (Minimal)

**Goal:** Basic signal delivery to user threads.

**Changes:**
- New file: `kernel/src/signal/mod.rs`
  ```rust
  pub enum Signal {
      SIGHUP = 1,
      SIGINT = 2,
      SIGTERM = 15,
      SIGKILL = 9,
      SIGSEGV = 11,
  }

  pub struct SignalState {
      pending: Option<Signal>,
      blocked: u64,  // Bitmask
  }
  ```
- `scheduler/task.rs`: Add `signal_state: SignalState`
- `scheduler/task.rs`: `Task::send_signal(sig)`
  - Set `signal_state.pending = Some(sig)`
  - Wake thread if sleeping
- `scheduler/mod.rs`: In scheduler loop, check pending signals
  - If SIGKILL/SIGSEGV → force exit
  - If other → deliver to user handler (if registered)

**Files:**
- New: `kernel/src/signal/mod.rs`
- `kernel/src/scheduler/task.rs`
- `kernel/src/scheduler/mod.rs`

---

### PR10: Context Switch Verification

**Goal:** Verify EL0 → kernel → EL0 round-trip works correctly.

**Changes:**
- New test: `arch-aarch64/src/context/test.rs`
  ```rust
  #[test]
  fn el0_round_trip() {
      // Create thread at EL0
      // Execute SVC
      // Verify registers preserved
      // Verify SP_EL0 preserved
      // Verify SPSR preserved
  }
  ```
- New test: `kernel/src/scheduler/test.rs`
  ```rust
  #[test]
  fn context_switch_preserves_state() {
      // Create two threads
      // Switch between them
      // Verify each resumes at correct PC
      // Verify each has correct SP
  }
  ```
- Verify `activate_address_space()` on AArch64
  - Currently no-op
  - Implement: write TTBR0_EL1

**Files:**
- New: `arch-aarch64/src/context/test.rs`
- New: `kernel/src/scheduler/test.rs`
- `arch-aarch64/src/mmu.rs` — implement `activate_address_space`

---

## Commit Sequence

```
PR1:  scheduler: expand ThreadState (Sleeping, Blocked, Waiting)
PR2:  scheduler: add priority levels (Realtime, High, Normal, Low, Idle)
PR3:  scheduler: dynamic RunQueue (replace static array)
PR4:  scheduler: sleep/wake primitives
PR5:  task: lifecycle (exit, wait, zombie)
PR6:  task: Task→Thread→AddressSpace binding
PR7:  scheduler: process table
PR8:  syscall: SVC handler + dispatch table
PR9:  signal: minimal signal model
PR10: context: EL0 round-trip verification
```

## Dependencies

```
PR1 ─┬─→ PR2 ──→ PR3 ──→ PR4
     │
PR5 ─┴─→ PR6 ──→ PR7
              │
PR8 ──────────┼──→ PR9 ──→ PR10
```

- PR1-PR4 are sequential (each builds on previous)
- PR5-PR7 are sequential but independent of PR1-PR4
- PR8 depends on PR7 (needs process table for fork/exec)
- PR9 depends on PR8 (signals delivered via syscall)
- PR10 depends on all previous

## Risk Assessment

| PR | Risk | Mitigation |
|----|------|------------|
| PR1 | Low | Simple enum expansion |
| PR2 | Low | Logic change in find_next_ready |
| PR3 | Medium | Many callers to update |
| PR4 | Medium | Timer interaction complexity |
| PR5 | Low | Task struct changes |
| PR6 | Medium | Borrow checker challenges |
| PR7 | Low | New module, no existing code affected |
| PR8 | Medium | Assembly changes, ESR parsing |
| PR9 | Low | New module, minimal interaction |
| PR10 | Medium | Hardware-specific testing |

## Estimated Effort

- Phase 1 (PR1-PR4): 2-3 sessions
- Phase 2 (PR5-PR7): 2-3 sessions
- Phase 3 (PR8-PR10): 2-3 sessions
- Total: 6-9 sessions

## Future Target: Google Pixel (GrapheneOS)

Зафиксированное решение (2026-08-11): целевой смартфон для Vivanta в будущем —
Google Pixel 6+ (Tensor GS101+), с GrapheneOS в качестве базовой ОС. Это
стратегическая цель, НЕ ближайшая работа.

Контекст исследования (hardware survey, 2026-08-11):
- Boot: бутлоадер Pixel — Little Kernel (не UEFI); свой бинарь грузится только
  как Android boot image через `fastboot boot/flash` на разблокированном BL
  (GKI-формат; процедура pmOS: пустой vbmeta + `erase dtbo`).
- GrapheneOS не меняет boot-цепочку (тот же boot.img/AVB/GKI) — для загрузки
  Vivanta неважно, какая Android-система установлена.
- Необходимый минимум для bringup: Samsung UART 0x10A00000 (+USB-C debug-донгл),
  GICv3, generic timer, PSCI, память по DTB. Дисплей/хранилище/сеть/SMP —
  треки вне scope первого bringup (в mainline для GS101 нет даже DSI-драйвера).
- Отдельный dev-девайс (бутлоадер открыт) + основной телефон с GrapheneOS
  (бутлоадер закрыт) — НЕ совмещать на одном устройстве: GrapheneOS требует
  закрытый BL, разработка ядра требует открытый.

Дорожная карта до цели:
- A. MMU-фикс L1/L2 0b11→0b10 (блокер реального кремния) + валидация на железе.
- B. SDM660/lavender — первый смартфон-класс (задел уже есть: target-lavender).
- C. Pixel 6 (GS101): boot image протокол → UART-консоль на 1 ядре.

См. также `docs/architecture/current-architecture.md` — актуальный снапшот
архитектуры Vivanta (обновлять при изменениях).
