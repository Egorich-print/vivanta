use alloc::vec::Vec;
use crate::memory::MemoryObject;
use crate::scheduler::thread::ThreadId;
use crate::vmm::AddressSpaceId;

pub type TaskId = u64;

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
    pub thread_id: ThreadId,
    pub address_space: AddressSpaceId,
    pub owned_objects: Vec<MemoryObject>,
    pub state: TaskState,
    pub parent: Option<TaskId>,
    pub children: Vec<TaskId>,
    pub exit_code: Option<i32>,
}

impl Task {
    pub fn new(task_id: TaskId, thread_id: ThreadId, address_space: AddressSpaceId) -> Self {
        Task {
            task_id,
            thread_id,
            address_space,
            owned_objects: Vec::new(),
            state: TaskState::Created,
            parent: None,
            children: Vec::new(),
            exit_code: None,
        }
    }

    pub fn add_object(&mut self, obj: MemoryObject) {
        self.owned_objects.push(obj);
    }

    pub fn exit(&mut self, code: i32) {
        self.state = TaskState::Exited;
        self.exit_code = Some(code);
    }

    pub fn set_parent(&mut self, parent_id: TaskId) {
        self.parent = Some(parent_id);
    }

    pub fn add_child(&mut self, child_id: TaskId) {
        self.children.push(child_id);
    }
}
