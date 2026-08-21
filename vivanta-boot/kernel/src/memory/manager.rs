use crate::memory::capability::{MemRights, MemoryCapability, OwnerId};
use crate::memory::object::{MemoryObject, MemoryObjectId};
use crate::memory::policy::{AllocationRequirements, evaluate};
use crate::memory::resource::{MemoryBackend, ResourceId};

const MAX_BACKENDS: usize = 8;

pub struct MemoryResourceManager {
    backends: [Option<(ResourceId, *mut dyn MemoryBackend)>; MAX_BACKENDS],
    count: usize,
    next_resource_id: ResourceId,
    next_object_id: MemoryObjectId,
    next_cap_id: u64,
}

impl MemoryResourceManager {
    pub fn new() -> Self {
        const INIT: Option<(ResourceId, *mut dyn MemoryBackend)> = None;
        MemoryResourceManager {
            backends: [INIT; MAX_BACKENDS],
            count: 0,
            next_resource_id: 0,
            next_object_id: 1,
            next_cap_id: 1,
        }
    }

    /// Register a memory backend. Returns the assigned ResourceId.
    ///
    /// The backend must be `'static` (no borrowed references)
    /// so its raw pointer can be stored in the backend array.
    pub unsafe fn register(
        &mut self,
        backend: &mut (dyn MemoryBackend + 'static),
    ) -> Option<ResourceId> {
        if self.count >= MAX_BACKENDS {
            return None;
        }
        let id = self.next_resource_id;
        self.next_resource_id += 1;
        let ptr: *mut dyn MemoryBackend = backend;
        self.backends[self.count] = Some((id, ptr));
        self.count += 1;
        Some(id)
    }

    pub fn backend_count(&self) -> usize {
        self.count
    }

    pub fn allocate(
        &mut self,
        req: &AllocationRequirements,
        owner: OwnerId,
    ) -> Option<MemoryObject> {
        let idx = self.best_backend_idx(req)?;
        let entry = self.backends[idx].as_mut()?;
        // entry: &mut (ResourceId, *mut dyn MemoryBackend)
        let (rid, raw_ptr) = *entry;
        let backend: &mut dyn MemoryBackend = unsafe { &mut *raw_ptr };
        let phys_addr = backend.allocate(req.size, req.align).ok()?;
        let obj_id = self.next_object_id;
        self.next_object_id += 1;
        let cap_id = self.next_cap_id;
        self.next_cap_id += 1;
        let cap = MemoryCapability::new(cap_id, obj_id, MemRights::FULL, owner);
        let mut obj = MemoryObject::new(obj_id, req.size, rid, cap);
        obj.set_backend(raw_ptr);
        obj.set_phys_addr(phys_addr);
        let _ = obj.mark_allocated();
        Some(obj)
    }

    fn best_backend_idx(&self, req: &AllocationRequirements) -> Option<usize> {
        let mut best: Option<(usize, u32)> = None;
        for i in 0..self.count {
            let entry = self.backends[i].as_ref()?;
            // entry: &(ResourceId, *mut dyn MemoryBackend)
            let raw_ptr = entry.1;
            let backend: &dyn MemoryBackend = unsafe { &*raw_ptr };
            let props = backend.properties();
            let score = evaluate(&props, req);
            if score == 0 {
                continue;
            }
            if best.map_or(true, |(_, s)| score > s) {
                best = Some((i, score));
            }
        }
        best.map(|(i, _)| i)
    }
}
