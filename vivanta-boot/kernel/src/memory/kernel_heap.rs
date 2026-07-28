use core::alloc::{GlobalAlloc, Layout};
use core::sync::atomic::{AtomicUsize, Ordering};

/// Bump allocator backed by MRM-allocated physical pages.
///
/// Replaces the boot-time `StubAllocator`. Memory is obtained from the
/// Memory Resource Manager during `kernel_main` init. No deallocation
/// support (leak-all) — acceptable for early kernel heap.
pub struct KernelHeap {
    base: AtomicUsize,
    end: AtomicUsize,
    pos: AtomicUsize,
}

impl KernelHeap {
    pub const fn uninitialized() -> Self {
        KernelHeap {
            base: AtomicUsize::new(0),
            end: AtomicUsize::new(0),
            pos: AtomicUsize::new(0),
        }
    }

    pub fn init(&self, base: usize, size: usize) {
        self.base.store(base, Ordering::SeqCst);
        self.end.store(base + size, Ordering::SeqCst);
        self.pos.store(base, Ordering::SeqCst);
    }

    pub fn is_initialized(&self) -> bool {
        self.base.load(Ordering::Acquire) != 0
    }
}

unsafe impl GlobalAlloc for KernelHeap {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        loop {
            let pos = self.pos.load(Ordering::Relaxed);
            let align = layout.align();
            let aligned = (pos + align - 1) & !(align - 1);
            let new_pos = aligned + layout.size();
            let end = self.end.load(Ordering::Relaxed);
            if new_pos > end {
                return core::ptr::null_mut();
            }
            if self
                .pos
                .compare_exchange_weak(pos, new_pos, Ordering::AcqRel, Ordering::Relaxed)
                .is_ok()
            {
                return aligned as *mut u8;
            }
        }
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {}
}

unsafe impl Send for KernelHeap {}
unsafe impl Sync for KernelHeap {}
