use alloc::vec::Vec;
use vivanta_arch_api::pmm::FrameAllocator;
use crate::memory::{AllocationRequirements, MemoryResourceManager};
use crate::scheduler;
use crate::scheduler::task::{Task, TaskId, TaskState};
use crate::scheduler::thread::Priority;
use crate::vmm::AddressSpaceId;

/// Manages the lifecycle of all Tasks in the system.
///
/// Each Task owns its Thread and MemoryObjects.
/// The TaskManager ensures resources are freed when a Task exits.
pub struct TaskManager {
    tasks: Vec<Task>,
    next_id: TaskId,
}

impl TaskManager {
    pub fn new() -> Self {
        TaskManager {
            tasks: Vec::new(),
            next_id: 1,
        }
    }

    /// Spawn a new user task.
    ///
    /// Allocates:
    /// - Kernel stack (4 pages from PMM via `alloc`)
    /// - User stack (from MRM via `mrm`)
    /// - A Thread registered in the runqueue
    ///
    /// Returns the new TaskId on success.
    pub fn spawn_user(
        &mut self,
        code_entry: usize,
        user_stack_va: usize,
        address_space: AddressSpaceId,
        alloc: &mut impl FrameAllocator,
        mrm: &mut MemoryResourceManager,
        priority: Priority,
    ) -> Result<TaskId, &'static str> {
        let stack_base = alloc.alloc_frame().ok_or("kernel stack frame 0")?.addr;
        for _ in 1..4 {
            alloc.alloc_frame().ok_or("kernel stack frame")?;
        }
        let kernel_stack_top = (stack_base as usize) + 16384;

        let user_stack = mrm
            .allocate(&AllocationRequirements::new(4096), 0)
            .ok_or("user stack allocation failed")?;

        let thread_id = scheduler::create_user_thread(
            kernel_stack_top,
            user_stack_va,
            code_entry,
            address_space,
            priority,
        );

        let task_id = self.next_id;
        self.next_id += 1;

        let mut task = Task::new(task_id, thread_id, address_space);
        task.add_object(user_stack);
        self.tasks.push(task);

        Ok(task_id)
    }

    /// Spawn a new kernel task.
    ///
    /// Allocates kernel stack from PMM and registers a kernel thread.
    pub fn spawn_kernel(
        &mut self,
        entry: extern "C" fn(usize),
        arg: usize,
        address_space: AddressSpaceId,
        alloc: &mut impl FrameAllocator,
        priority: Priority,
    ) -> Result<TaskId, &'static str> {
        let thread_id = scheduler::create_kernel_thread(entry, arg, alloc, address_space, priority);

        let task_id = self.next_id;
        self.next_id += 1;

        let task = Task::new(task_id, thread_id, address_space);
        self.tasks.push(task);

        Ok(task_id)
    }

    /// Mark a Task as zombie.
    ///
    /// The owned MemoryObjects will be freed on the next
    /// `cleanup_zombies` call.
    pub fn kill(&mut self, task_id: TaskId) -> Result<(), &'static str> {
        let task = self
            .tasks
            .iter_mut()
            .find(|t| t.task_id == task_id)
            .ok_or("task not found")?;
        task.state = TaskState::Zombie;
        Ok(())
    }

    pub fn task_count(&self) -> usize {
        self.tasks.len()
    }

    pub fn running_count(&self) -> usize {
        self.tasks.iter().filter(|t| t.state == TaskState::Running).count()
    }

    pub fn iter(&self) -> core::slice::Iter<'_, Task> {
        self.tasks.iter()
    }

    pub fn get(&self, task_id: TaskId) -> Option<&Task> {
        self.tasks.iter().find(|t| t.task_id == task_id)
    }

    pub fn get_mut(&mut self, task_id: TaskId) -> Option<&mut Task> {
        self.tasks.iter_mut().find(|t| t.task_id == task_id)
    }
}
