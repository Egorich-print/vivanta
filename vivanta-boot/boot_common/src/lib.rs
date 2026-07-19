// ---------------------------------------------------------------------------
// boot-common — runtime utilities and diagnostic types for Vivanta boot
//
// Contract types (BootInfo, MemoryMap, MmioRegion, etc.) live in boot-info.
// This crate provides: Console, println!, FDT scanner, NS16550 driver,
// and diagnostic enums (Architecture, BootSource) for early output.
// ---------------------------------------------------------------------------

#![no_std]

pub mod ns16550;
pub mod hardware;
pub mod fdt;

// Re-export boot-info contract types so existing code keeps working
pub use vivanta_boot_info::{
    BootInfo, MemoryMap, MemoryRegion, MemoryRegionKind, MmioRegion, MmioKind,
    InterruptControllerInfo,
};

use core::cell::UnsafeCell;
use core::fmt;

// ---------------------------------------------------------------------------
// BootContext — entry information passed from bootloader to vivanta_kernel
// ---------------------------------------------------------------------------

#[repr(C)]
pub struct BootContext {
    pub dtb: usize,
    pub flags: usize,
}

#[no_mangle]
pub static mut BOOT_CONTEXT: BootContext = BootContext { dtb: 0, flags: 0 };

// ---------------------------------------------------------------------------
// Diagnostic types (NOT part of BootInfo — used for println! only)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Architecture {
    AArch64,
    AArch32(LpaeMode),
    X86_64,
    Riscv64,
    Riscv32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LpaeMode {
    Short,
    Lpae,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum BootSource {
    Uefi,
    Bios,
    Uboot,
    OpenSbi,
    QemuKernel,
    Raw,
    ArmTrustedFirmware,
}

// ---------------------------------------------------------------------------
// MemoryGeometry
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
pub struct MemoryGeometry {
    pub page_size: usize,
    pub page_shift: u8,
    pub page_mask: usize,
    pub table_levels: u8,
    pub supported_block_sizes: &'static [usize],
}

// ---------------------------------------------------------------------------
// Console
// ---------------------------------------------------------------------------

pub trait Console {
    fn write_str(&self, s: &str);
}

pub struct FmtAdapter<'a>(pub &'a dyn Console);

impl fmt::Write for FmtAdapter<'_> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0.write_str(s);
        Ok(())
    }
}

struct GlobalConsole {
    inner: UnsafeCell<Option<&'static dyn Console>>,
}

unsafe impl Sync for GlobalConsole {}

static GLOBAL_CONSOLE: GlobalConsole = GlobalConsole {
    inner: UnsafeCell::new(None),
};

pub fn set_console(c: &'static dyn Console) {
    unsafe {
        *GLOBAL_CONSOLE.inner.get() = Some(c);
    }
}

/// Bypass lock and write directly to the console (for early boot debug).
pub fn write_direct(s: &str) {
    unsafe {
        let base = 0xFE66_0000 as *mut u32;
        base.write_volatile(b'W' as u32);
    }
    unsafe {
        if let Some(c) = (*GLOBAL_CONSOLE.inner.get()).as_ref() {
            (0xFE66_0000 as *mut u32).write_volatile(b'X' as u32);
            c.write_str(s);
            (0xFE66_0000 as *mut u32).write_volatile(b'Y' as u32);
        } else {
            (0xFE66_0000 as *mut u32).write_volatile(b'z' as u32);
        }
    }
}

pub fn with_console<F, R>(f: F) -> R
where
    F: FnOnce(&dyn Console) -> R,
{
    unsafe { (0xFE66_0000 as *mut u32).write_volatile(b'1' as u32); }
    let p = GLOBAL_CONSOLE.inner.get();
    unsafe { (0xFE66_0000 as *mut u32).write_volatile(b'2' as u32); }
    let opt = unsafe { (*p).as_ref() };
    unsafe { (0xFE66_0000 as *mut u32).write_volatile(b'3' as u32); }
    let c = opt.expect("console not initialized");
    unsafe { (0xFE66_0000 as *mut u32).write_volatile(b'4' as u32); }
    let result = f(*c);
    unsafe { (0xFE66_0000 as *mut u32).write_volatile(b'5' as u32); }
    result
}

// ---------------------------------------------------------------------------
// Print macros
// ---------------------------------------------------------------------------

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {{
        $crate::with_console(|__c| {
            let mut __w = $crate::FmtAdapter(__c);
            let _ = core::fmt::write(&mut __w, format_args!($($arg)*));
        });
    }};
}

#[macro_export]
macro_rules! println {
    () => { $crate::with_console(|__c| __c.write_str("\n")); };
    ($($arg:tt)*) => {{
        $crate::with_console(|__c| {
            let mut __w = $crate::FmtAdapter(__c);
            let _ = core::fmt::write(&mut __w, format_args!($($arg)*));
            __c.write_str("\n");
        });
    }};
}