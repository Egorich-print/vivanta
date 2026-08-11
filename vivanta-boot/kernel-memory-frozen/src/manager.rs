// ---------------------------------------------------------------------------
// MemoryResourceManager — orchestrates MemoryBackend instances
// ---------------------------------------------------------------------------

use crate::capability::{MemRights, MemoryCapability, OwnerId};
use crate::object::{MemoryObject, MemoryObjectId};
use crate::policy::{evaluate, AllocationRequirements};
use crate::resource::{MemoryBackend, ResourceId};

/// Maximum number of memory backends that can be registered.
const MAX_BACKENDS: usize = 8;

/// The central registry of memory backends.
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
    pub fn register(&mut self, backend: *mut dyn MemoryBackend) -> Option<ResourceId> {
        if self.count >= MAX_BACKENDS {
            return None;
        }
        let id = self.next_resource_id;
        self.next_resource_id += 1;
        self.backends[self.count] = Some((id, backend));
        self.count += 1;
        Some(id)
    }

    /// Number of registered backends.
    pub fn backend_count(&self) -> usize {
        self.count
    }

    /// Allocate a MemoryObject from the backend that best matches `req`.
    pub fn allocate(
        &mut self,
        req: &AllocationRequirements,
        owner: OwnerId,
    ) -> Option<MemoryObject> {
        let idx = self.best_backend_idx(req)?;
        let entry = &mut self.backends[idx];
        let (rid, raw) = entry.as_mut()?;
        let backend = unsafe { &mut **raw };
        let _phys = backend.allocate(req.size, req.align).ok()?;
        let obj_id = self.next_object_id;
        self.next_object_id += 1;
        let cap_id = self.next_cap_id;
        self.next_cap_id += 1;
        let cap = MemoryCapability::new(cap_id, obj_id, MemRights::FULL, owner);
        let mut obj = MemoryObject::new(obj_id, req.size, *rid, cap);
        // Backend allocation succeeded → mark as Allocated.
        let _ = obj.mark_allocated();
        Some(obj)
    }

    /// Pick the backend index with the highest policy-engine score.
    fn best_backend_idx(&self, req: &AllocationRequirements) -> Option<usize> {
        let mut best: Option<(usize, u32)> = None;
        for i in 0..self.count {
            let entry = self.backends[i].as_ref()?;
            let (_rid, raw) = entry;
            let backend = unsafe { &**raw };
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

    /// Print registered backends for diagnostics.
    pub fn print_backends(&self) {
        use vivanta_boot_common::println;
        println!("  MemoryResourceManager:");
        println!("    {} backend(s) registered", self.count);
        for i in 0..self.count {
            let entry = self.backends[i].as_ref().unwrap();
            let (rid, raw) = entry;
            let backend = unsafe { &**raw };
            let p = backend.properties();
            println!(
                "    [{}] {}  cap={} bytes  lat={:?}  bw={:?}  persist={:?}",
                rid,
                backend.name(),
                p.capacity,
                p.latency_class,
                p.bandwidth_class,
                p.persistence,
            );
        }
    }
}
