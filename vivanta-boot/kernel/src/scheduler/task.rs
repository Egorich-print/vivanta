use crate::memory::MemoryObject;
use crate::scheduler::thread::ThreadId;
use crate::signal::SignalState;
use crate::vmm::AddressSpaceId;
use alloc::vec;
use alloc::vec::Vec;

pub type TaskId = u64;

/// Generation-protected process handle (M7.4): `(id, generation)` pairs.
/// A generation increments every time a TaskId is reaped, so a stale
/// handle can never silently resolve to a *newer* process with the same
/// raw id.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProcessHandle {
    pub id: TaskId,
    pub generation: u32,
}

/// Hard cap on simultaneously tracked tasks (deterministic exhaustion).
pub const MAX_TASKS: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    Created,
    Running,
    Exited,
    Zombie,
}

/// A Task is the unit of resource ownership.
///
/// "Every resource must have an owner, a lifetime, and an authority boundary."
/// The Task is the owner; dropping it releases its resources.
pub struct Task {
    pub task_id: TaskId,
    /// Bumped on reap; used by ProcessHandle validation.
    pub generation: u32,
    pub address_space: AddressSpaceId,
    pub threads: Vec<ThreadId>,
    pub owned_objects: Vec<MemoryObject>,
    pub state: TaskState,
    pub parent: Option<TaskId>,
    pub children: Vec<TaskId>,
    pub exit_code: Option<i32>,
    pub signals: SignalState,
}

impl Task {
    pub fn new(task_id: TaskId, thread_id: ThreadId, address_space: AddressSpaceId) -> Self {
        Task {
            task_id,
            generation: 0,
            address_space,
            threads: vec![thread_id],
            owned_objects: Vec::new(),
            state: TaskState::Created,
            parent: None,
            children: Vec::new(),
            exit_code: None,
            signals: SignalState::new(),
        }
    }

    pub fn add_thread(&mut self, thread_id: ThreadId) {
        self.threads.push(thread_id);
    }

    pub fn add_object(&mut self, obj: MemoryObject) {
        self.owned_objects.push(obj);
    }

    /// Mark the task as terminated (zombie) with the given exit code.
    ///
    /// M6: an exited task that has not been reaped is a `Zombie`; the parent
    /// or monitor collects it via `reap_zombie`, which releases its resources.
    pub fn exit(&mut self, code: i32) {
        self.state = TaskState::Zombie;
        self.exit_code = Some(code);
    }

    pub fn set_parent(&mut self, parent_id: TaskId) {
        self.parent = Some(parent_id);
    }

    pub fn add_child(&mut self, child_id: TaskId) {
        self.children.push(child_id);
    }
}
