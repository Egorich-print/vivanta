// ---------------------------------------------------------------------------
// Syscall ABI (ADR-033) — frozen contract constants and the dispatcher.
//
// Transport: SVC from EL0. Numbers in x8, args x0..x5, result x0
// (>=0 success, <0 -errno). The caller's address space is identified by
// the root page table passed by the arch entry path (TTBR0 at entry).
//
// This file is safe code: user pointers are never dereferenced directly;
// all memory effects go through VMM primitives that validate ranges.
// ---------------------------------------------------------------------------

use crate::scheduler;
use crate::vmm;
use vivanta_arch_api::mmu::MappingFlags;
use vivanta_boot_common::println;

/// Max bytes accepted by the `write` syscall per call (kernel stack buffer).
const WRITE_BUF_SIZE: usize = 256;

// --- frozen syscall numbers (ADR-033 §3) -----------------------------------
pub const SYS_READ: u64 = 0; // reserved -> -ENOSYS
pub const SYS_WRITE: u64 = 1;
pub const SYS_EXIT: u64 = 2;
pub const SYS_YIELD: u64 = 3;
pub const SYS_MMAP: u64 = 4;
pub const SYS_MUNMAP: u64 = 5;
pub const SYS_MPROTECT: u64 = 6;

// --- frozen errno encoding -------------------------------------------------
pub const EPERM_I: i64 = -1;
pub const ENOMEM_I: i64 = -12;
pub const EFAULT_I: i64 = -14;
pub const EINVAL_I: i64 = -22;
pub const ENOSYS_I: i64 = -38;

const ENOMEM: u64 = ENOMEM_I as u64;
const EFAULT: u64 = EFAULT_I as u64;
const EINVAL: u64 = EINVAL_I as u64;
const ENOSYS: u64 = ENOSYS_I as u64;

/// mmap length cap (ADR-033 §6): deterministic bound, not an overcommit
/// promise.
pub const MAX_MMAP_BYTES: u64 = 64 * 1024 * 1024;

/// prot bits (frozen): bit0=read, bit1=write, bit2=execute.
pub const PROT_READ: u64 = 1;
pub const PROT_WRITE: u64 = 2;
pub const PROT_EXEC: u64 = 4;

/// Synthetic object id for anonymous syscall mappings.
pub const OBJ_ANONYMOUS: u64 = u64::MAX;

/// Validate a `prot` bitmask into MappingFlags. W^X is enforced here:
/// write+execute is rejected before anything touches the VM state.
pub fn decode_prot(prot: u64) -> Option<MappingFlags> {
    let unknown = prot & !(PROT_READ | PROT_WRITE | PROT_EXEC);
    if unknown != 0 {
        return None;
    }
    if prot & PROT_READ == 0 {
        return None; // read must always be requested
    }
    // Syscall-created mappings are BY DEFINITION user-accessible
    // (ADR-033 §6): the user bit drives ap_bits toward EL0 encodings.
    let mut f = MappingFlags::user();
    if prot & PROT_WRITE != 0 {
        f = f | MappingFlags::read_write();
    }
    if prot & PROT_EXEC != 0 {
        f = f | MappingFlags::executable();
    }
    // Hard rule #10 / ADR-019: no writable+executable user mapping.
    if f.is_read_write() && f.is_executable() {
        return None;
    }
    Some(f)
}

#[inline]
fn page_round(len: u64) -> Option<u64> {
    len.checked_add(0xFFF).map(|v| v & !0xFFF)
}

#[unsafe(no_mangle)]
pub extern "Rust" fn syscall_dispatch(
    as_root: u64,
    num: u64,
    arg0: u64,
    arg1: u64,
    arg2: u64,
    _arg3: u64,
    _arg4: u64,
    _arg5: u64,
) -> u64 {
    match num {
        SYS_READ => ENOSYS, // reserved (ADR-033 §3)
        SYS_WRITE => sys_write(arg0, arg1, arg2),
        SYS_EXIT => {
            println!("  syscall: exit({})", arg0);
            scheduler::thread_exit(arg0 as i32);
        }
        SYS_YIELD => {
            scheduler::yield_now();
            0
        }
        SYS_MMAP => sys_mmap(as_root, arg0, arg1, arg2),
        SYS_MUNMAP => sys_munmap(as_root, arg0, arg1),
        SYS_MPROTECT => sys_mprotect(as_root, arg0, arg1, arg2),
        _ => {
            println!("  syscall: unknown num={}", num);
            ENOSYS
        }
    }
}

fn sys_write(fd: u64, buf: u64, count: u64) -> u64 {
    if fd != 1 && fd != 2 {
        return EINVAL;
    }
    if count > WRITE_BUF_SIZE as u64 {
        return EINVAL;
    }
    let count = count as usize;
    let mut kbuf = [0u8; WRITE_BUF_SIZE];
    // SAFETY: kbuf is a kernel stack buffer; copy_from_user validates the
    // source range against the active address space first.
    if unsafe { vivanta_arch_api::user_memory::copy_from_user(kbuf.as_mut_ptr(), buf, count) }
        .is_err()
    {
        return EFAULT;
    }
    for &byte in kbuf[..count].iter() {
        let uart = 0x0900_0000 as *mut u32;
        unsafe {
            while core::ptr::read_volatile(uart.add(0x18 / 4)) & (1 << 5) != 0 {}
            core::ptr::write_volatile(uart, byte as u32);
        }
    }
    count as u64
}

/// MMAP(addr=0, len, prot) → base VA or -errno. Anonymous lazy only.
fn sys_mmap(as_root: u64, addr: u64, len: u64, prot: u64) -> u64 {
    let Some(aspace) = vmm::find_by_root(as_root) else {
        return EFAULT;
    };
    if addr != 0 {
        return EINVAL; // fixed mappings unsupported (ADR-033 §6)
    }
    if len == 0 || len > MAX_MMAP_BYTES {
        return EINVAL;
    }
    let Some(flags) = decode_prot(prot) else {
        return if prot & PROT_WRITE != 0 && prot & PROT_EXEC != 0 {
            EPERM_I as u64
        } else {
            EINVAL
        };
    };
    let Some(len_r) = page_round(len) else {
        return EINVAL;
    };
    match aspace.reserve_lazy(len_r, flags, crate::syscall::OBJ_ANONYMOUS, 4096) {
        Ok(va) => va,
        Err(_) => ENOMEM,
    }
}

/// MUNMAP(addr, len) → 0 or -errno.
fn sys_munmap(as_root: u64, addr: u64, len: u64) -> u64 {
    let Some(aspace) = vmm::find_by_root(as_root) else {
        return EFAULT;
    };
    if addr % 4096 != 0 || addr < vmm::USER_VA_BASE {
        return EINVAL;
    }
    if len == 0 {
        return EINVAL;
    }
    let Some(len_r) = page_round(len) else {
        return EINVAL;
    };
    match aspace.unmap_range(addr, len_r, &mut as_alloc_for(aspace.id)) {
        Ok(()) => 0,
        Err(vmm::VmmError::NotMapped) => ENOMEM,
        Err(_) => EINVAL,
    }
}

/// MPROTECT(addr, len, prot) → 0 or -errno.
fn sys_mprotect(as_root: u64, addr: u64, len: u64, prot: u64) -> u64 {
    let Some(aspace) = vmm::find_by_root(as_root) else {
        return EFAULT;
    };
    if addr % 4096 != 0 || addr < vmm::USER_VA_BASE {
        return EINVAL;
    }
    if len == 0 {
        return EINVAL;
    }
    let Some(len_r) = page_round(len) else {
        return EINVAL;
    };
    let Some(flags) = decode_prot(prot) else {
        return if prot & PROT_WRITE != 0 && prot & PROT_EXEC != 0 {
            EPERM_I as u64
        } else {
            EINVAL
        };
    };
    match aspace.protect(addr, len_r, flags, &mut as_alloc_for(aspace.id)) {
        Ok(()) => 0,
        Err(vmm::VmmError::NotMapped) => ENOMEM,
        Err(_) => EINVAL,
    }
}

// -- plumbing ---------------------------------------------------------------

fn as_alloc_for(as_id: u64) -> crate::memory::AsPageTableAllocator {
    let (mrm, backend) = crate::vmm::faults::backing_context()
        .expect("syscall VM op before backing context established");
    // SAFETY: context pointers were established during boot and outlive it.
    unsafe { crate::memory::AsPageTableAllocator::new(mrm, backend, as_id) }
}
