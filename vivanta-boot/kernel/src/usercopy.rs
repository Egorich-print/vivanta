// ---------------------------------------------------------------------------
// usercopy.rs — Safe user memory access abstractions (Security Gate P0)
// ---------------------------------------------------------------------------

use crate::error::{KernelError, KernelResult};
use crate::scheduler::current_thread_address_space;
pub use vivanta_arch_api::user_memory::AccessType as Access;
use vivanta_arch_api::user_memory::{access_ok, AccessType};

/// A validated user-space pointer wrapper representing a buffer of type `T`.
pub struct UserPtr<T> {
    ptr: *const T,
    _marker: core::marker::PhantomData<T>,
}

impl<T> UserPtr<T> {
    /// Create a new `UserPtr` after verifying address validity and permissions.
    pub fn new(ptr: *const T, len: usize, access: AccessType) -> KernelResult<Self> {
        let aspace = current_thread_address_space();
        let addr = ptr as u64;
        let size = (len * core::mem::size_of::<T>()) as u64;

        // SAFETY: access_ok only reads the active page table and returns bool.
        let ok = unsafe { access_ok(aspace as usize, addr, size, access) };
        if !ok {
            return Err(KernelError::InvalidAddress);
        }

        Ok(Self {
            ptr,
            _marker: core::marker::PhantomData,
        })
    }

    /// Read value safely from user space.
    pub fn read(&self) -> KernelResult<T> {
        unsafe {
            // SAFETY: pointer has been validated via access_ok during UserPtr creation
            let val = core::ptr::read_volatile(self.ptr);
            Ok(val)
        }
    }
}

/// A validated user-space slice representing a contiguous buffer.
pub struct UserSlice {
    ptr: *const u8,
    len: usize,
}

impl UserSlice {
    /// Create a validated `UserSlice` for reading from user space.
    pub fn read(ptr: *const u8, len: usize) -> KernelResult<Self> {
        let aspace = current_thread_address_space();
        // SAFETY: access_ok only reads the active page table and returns bool.
        let ok = unsafe { access_ok(aspace as usize, ptr as u64, len as u64, AccessType::Read) };
        if !ok {
            return Err(KernelError::InvalidAddress);
        }

        Ok(Self { ptr, len })
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Read a single byte at offset `i` from the user slice.
    pub fn read_byte(&self, idx: usize) -> KernelResult<u8> {
        if idx >= self.len {
            return Err(KernelError::InvalidAddress);
        }
        unsafe { Ok(core::ptr::read_volatile(self.ptr.add(idx))) }
    }
    pub fn copy_to_kernel(&self, dest: &mut [u8]) -> KernelResult<()> {
        if dest.len() < self.len {
            return Err(KernelError::InvalidAddress);
        }
        unsafe {
            // SAFETY: both source (validated) and destination buffers are within bounds
            core::ptr::copy_nonoverlapping(self.ptr, dest.as_mut_ptr(), self.len);
        }
        Ok(())
    }
}
