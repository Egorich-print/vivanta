use crate::scheduler;
use crate::scheduler::task::{ProcessHandle, Task, TaskId, TaskState};
use crate::scheduler::thread::Priority;
use crate::vmm::AddressSpaceId;
use alloc::vec::Vec;
use vivanta_arch_api::pmm::{FrameAllocator, PhysFrame};

/// Manages the lifecycle of all Tasks in the system.
///
/// Each Task owns its Thread and MemoryObjects.
/// The TaskManager ensures resources are freed when a Task exits.
pub struct TaskManager {
    // TaskManager now delegates to global ProcessTable
}

impl TaskManager {
    pub fn new() -> Self {
        TaskManager {}
    }

    /// Spawn a new user task.
    ///
    /// Allocates:
    /// - Kernel stack (4 pages from PMM via `alloc`)
    /// - A Thread registered in the runqueue
    ///
    /// The user stack VA is caller-mapped (boot ghosts or an explicit
    /// reservation); no MRM frame is allocated here.
    ///
    /// Returns the new TaskId on success.
    pub fn spawn_user(
        &mut self,
        code_entry: usize,
        user_stack_va: usize,
        address_space: AddressSpaceId,
        alloc: &mut impl FrameAllocator,
        priority: Priority,
        parent: Option<TaskId>,
    ) -> Result<ProcessHandle, &'static str> {
        let stack_base = alloc
            .alloc_contiguous(crate::scheduler::KERNEL_STACK_SIZE / 4096)
            .ok_or("kernel stack contiguous alloc failed")?
            .addr;
        let kernel_stack_top = (stack_base as usize) + crate::scheduler::KERNEL_STACK_SIZE;

        let thread_id = scheduler::create_user_thread(
            kernel_stack_top,
            stack_base as usize,
            user_stack_va,
            code_entry,
            address_space,
            priority,
        );

        let mut task = Task::new(0, thread_id, address_space); // ID will be set by ProcessTable
        if let Some(parent_id) = parent {
            task.set_parent(parent_id);
        }
        let Some(handle) = scheduler::pt().create(task) else {
            // Table full: the thread stays unpublished, and its stack goes
            // back to the allocator instead of leaking with a dead entry.
            scheduler::remove_thread(thread_id);
            for i in 0..(crate::scheduler::KERNEL_STACK_SIZE / 4096) {
                alloc.free_frame(PhysFrame {
                    addr: stack_base + (i * 4096) as u64,
                });
            }
            return Err("process table full");
        };

        // Update parent's children list
        if let Some(parent_id) = parent {
            if let Some(parent_task) = scheduler::pt().lookup_mut(parent_id) {
                parent_task.add_child(handle.id);
            }
        }

        scheduler::publish_thread(thread_id);
        Ok(handle)
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
        parent: Option<TaskId>,
    ) -> Result<ProcessHandle, &'static str> {
        let thread_id = scheduler::create_kernel_thread(entry, arg, alloc, address_space, priority);

        let mut task = Task::new(0, thread_id, address_space); // ID will be set by ProcessTable
        if let Some(parent_id) = parent {
            task.set_parent(parent_id);
        }
        let Some(handle) = scheduler::pt().create(task) else {
            scheduler::remove_thread(thread_id);
            return Err("process table full");
        };

        // Update parent's children list
        if let Some(parent_id) = parent {
            if let Some(parent_task) = scheduler::pt().lookup_mut(parent_id) {
                parent_task.add_child(handle.id);
            }
        }

        scheduler::publish_thread(thread_id);
        Ok(handle)
    }

    /// Count of tasks.
    pub fn task_count(&self) -> usize {
        scheduler::pt().count()
    }

    /// Count of running tasks.
    pub fn running_count(&self) -> usize {
        scheduler::pt()
            .iter()
            .filter(|t| t.state == TaskState::Running)
            .count()
    }

    /// Get task by ID.
    /// Generation-validated lookup.
    pub fn get_handle(&self, h: ProcessHandle) -> Option<&Task> {
        scheduler::pt().lookup_handle(h)
    }

    pub fn get(&self, task_id: TaskId) -> Option<&Task> {
        scheduler::pt().lookup(task_id)
    }

    /// Get all zombie tasks (for cleanup).
    pub fn zombies(&self) -> Vec<TaskId> {
        scheduler::pt()
            .iter()
            .filter(|t| t.state == TaskState::Zombie)
            .map(|t| t.task_id)
            .collect()
    }

    /// Remove zombie task and return it.
    pub fn reap_zombie(&mut self, task_id: TaskId) -> Option<Task> {
        scheduler::pt().remove(task_id)
    }
}
