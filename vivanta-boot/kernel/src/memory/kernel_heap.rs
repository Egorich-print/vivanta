use core::alloc::{GlobalAlloc, Layout};
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

// ---------------------------------------------------------------------------
// KernelHeap — free-list allocator backed by MRM-allocated physical pages.
//
// G2 (M5.0): `dealloc` must actually reclaim memory because the heap backs
// lifetime-managed structures (scheduler Vecs). A bump-only allocator would
// leak every transient allocation and exhaust the 64 KiB heap under churn.
//
// Concurrency: single-core kernel. The timer IRQ can preempt a thread while it
// is inside `alloc`/`dealloc`, so the critical section runs with interrupts
// disabled (via vivanta_arch_api) and a short spin guard.
// ---------------------------------------------------------------------------

/// Header stored before every live block: `[size: usize][next: *mut u8]`.
/// `size` is the usable payload size in bytes (aligned). A live block has its
/// low bit of `size` clear; a free block has it set and `next` pointing to the
/// next free block header.
const FREE_BIT: usize = 1;
const MIN_ALIGN: usize = 16;

pub struct KernelHeap {
    base: AtomicUsize,
    end: AtomicUsize,
    free_head: AtomicUsize,
    // Short spin guard (single-core: only protects against the same context
    // re-entering through a bug; interrupts are disabled around the critical
    // section, which is the real protection).
    lock: AtomicBool,
    allocs: AtomicUsize,
    deallocs: AtomicUsize,
}

impl KernelHeap {
    pub const fn uninitialized() -> Self {
        KernelHeap {
            base: AtomicUsize::new(0),
            end: AtomicUsize::new(0),
            free_head: AtomicUsize::new(0),
            lock: AtomicBool::new(false),
            allocs: AtomicUsize::new(0),
            deallocs: AtomicUsize::new(0),
        }
    }

    pub fn init(&self, base: usize, size: usize) {
        self.base.store(base, Ordering::SeqCst);
        self.end.store(base + size, Ordering::SeqCst);
        self.free_head.store(base, Ordering::SeqCst);
        // One free block covering the whole region.
        unsafe {
            core::ptr::write_volatile(base as *mut usize, size - core::mem::size_of::<usize>() * 2);
            core::ptr::write_volatile((base + core::mem::size_of::<usize>()) as *mut usize, 0);
        }
    }

    pub fn is_initialized(&self) -> bool {
        self.base.load(Ordering::Acquire) != 0
    }

    fn acquire(&self) {
        while self
            .lock
            .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            core::hint::spin_loop();
        }
    }

    fn release(&self) {
        self.lock.store(false, Ordering::Release);
    }

    fn block_size(header: *mut usize) -> usize {
        unsafe { core::ptr::read_volatile(header) & !FREE_BIT }
    }

    fn set_block_size(header: *mut usize, size: usize) {
        unsafe {
            let cur = core::ptr::read_volatile(header);
            core::ptr::write_volatile(header, (cur & FREE_BIT) | size);
        }
    }

    fn set_free(header: *mut usize, free: bool) {
        unsafe {
            let sz = core::ptr::read_volatile(header) & !FREE_BIT;
            core::ptr::write_volatile(header, sz | if free { FREE_BIT } else { 0 });
        }
    }

    fn next_free(header: *mut usize) -> *mut usize {
        let p = unsafe { (header as *mut u8).add(core::mem::size_of::<usize>()) };
        unsafe { core::ptr::read_volatile(p as *mut usize) as *mut usize }
    }

    fn set_next_free(header: *mut usize, next: *mut usize) {
        let p = unsafe { (header as *mut u8).add(core::mem::size_of::<usize>()) };
        unsafe {
            core::ptr::write_volatile(p as *mut usize, next as usize);
        }
    }

    fn payload(header: *mut usize) -> *mut u8 {
        unsafe { (header as *mut u8).add(core::mem::size_of::<usize>() * 2) }
    }

    fn header_of(payload: *mut u8) -> *mut usize {
        unsafe { (payload as *mut usize).sub(2) }
    }

    fn alloc_locked(&self, layout: Layout) -> *mut u8 {
        let align = layout.align().max(MIN_ALIGN);
        // Round size up to a 16-byte multiple so every free block stays
        // 16-aligned (keeps all `*usize` accesses aligned).
        let size = (layout.size().max(MIN_ALIGN) + 15) & !15;
        let end = self.end.load(Ordering::Relaxed);

        let mut prev: *mut usize = core::ptr::null_mut();
        let mut cur = self.free_head.load(Ordering::Relaxed) as *mut usize;
        while !cur.is_null() && (cur as usize) < end {
            let sz = Self::block_size(cur);
            // Align the payload within this free block.
            let payload = Self::payload(cur) as usize;
            let aligned = (payload + align - 1) & !(align - 1);
            let header_shift = aligned - (cur as usize + core::mem::size_of::<usize>() * 2);
            let usable = sz.checked_sub(header_shift).unwrap_or(0);

            if usable >= size {
                let alloc_end = aligned + size;
                let block_end = (cur as usize) + core::mem::size_of::<usize>() * 2 + sz;
                if alloc_end > block_end {
                    // Allocation would overrun this free block: skip it.
                    prev = cur;
                    cur = Self::next_free(cur);
                    continue;
                }
                // Remove `cur` from the free list.
                let next = Self::next_free(cur);
                if prev.is_null() {
                    self.free_head.store(next as usize, Ordering::Relaxed);
                } else {
                    Self::set_next_free(prev, next);
                }

                // Allocated header sits 2 words before the ALIGNED payload.
                let alloc_header = unsafe { (aligned as *mut usize).sub(2) };
                Self::set_block_size(alloc_header, size);
                Self::set_free(alloc_header, false);

                // Leftover fragment starts right after the allocation, at the
                // aligned payload end (size is 16-rounded, so this stays
                // 16-aligned).
                let leftover = aligned + size;
                let leftover_end = (cur as usize) + core::mem::size_of::<usize>() * 2 + sz;
                if leftover + core::mem::size_of::<usize>() * 2 <= leftover_end {
                    let frag = leftover as *mut usize;
                    let frag_size = leftover_end - leftover - core::mem::size_of::<usize>() * 2;
                    Self::set_block_size(frag, frag_size);
                    Self::set_free(frag, true);
                    Self::set_next_free(frag, self.free_head.load(Ordering::Relaxed) as *mut usize);
                    self.free_head.store(frag as usize, Ordering::Relaxed);
                }

                return Self::payload(alloc_header);
            }
            prev = cur;
            cur = Self::next_free(cur);
        }
        core::ptr::null_mut()
    }

    fn dealloc_locked(&self, ptr: *mut u8) {
        if ptr.is_null() {
            return;
        }
        let header = Self::header_of(ptr);
        // Insert `header` at the head of the free list (coalescing deferred:
        // M5.0 scope keeps the free list simple; adjacent-coalescing is a
        // follow-up). Mark as free.
        Self::set_free(header, true);
        let head = self.free_head.load(Ordering::Relaxed) as *mut usize;
        Self::set_next_free(header, head);
        self.free_head.store(header as usize, Ordering::Relaxed);
    }
}

unsafe impl GlobalAlloc for KernelHeap {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if !self.is_initialized() {
            return core::ptr::null_mut();
        }
        let _guard = crate::interrupts_guard();
        self.acquire();
        let r = self.alloc_locked(layout);
        self.release();
        if !r.is_null() {
            self.allocs.fetch_add(1, Ordering::Relaxed);
        }
        r
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        if !self.is_initialized() {
            return;
        }
        let _guard = crate::interrupts_guard();
        self.acquire();
        self.dealloc_locked(ptr);
        self.release();
        self.deallocs.fetch_add(1, Ordering::Relaxed);
    }
}

unsafe impl Send for KernelHeap {}
unsafe impl Sync for KernelHeap {}
