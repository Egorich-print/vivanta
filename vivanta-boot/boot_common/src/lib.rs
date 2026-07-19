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
// EarlyPlatformInfo — platform-provided constants for early boot debug output.
// Must be set by adapter_main BEFORE any println! or write_direct call.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
pub struct EarlyPlatformInfo {
    pub uart_base: usize,
}

static mut EARLY_PLATFORM: Option<EarlyPlatformInfo> = None;

pub fn set_early_platform(info: EarlyPlatformInfo) {
    unsafe { EARLY_PLATFORM = Some(info); }
}

pub fn early_platform() -> Option<EarlyPlatformInfo> {
    unsafe { EARLY_PLATFORM }
}

unsafe fn early_uart_write(byte: u8) {
    if let Some(info) = EARLY_PLATFORM.as_ref() {
        (info.uart_base as *mut u32).write_volatile(byte as u32);
    }
}

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
    unsafe { early_uart_write(b'W'); }
    unsafe {
        if let Some(c) = (*GLOBAL_CONSOLE.inner.get()).as_ref() {
            early_uart_write(b'X');
            c.write_str(s);
            early_uart_write(b'Y');
        } else {
            early_uart_write(b'z');
        }
    }
}

pub fn with_console<F, R>(f: F) -> R
where
    F: FnOnce(&dyn Console) -> R,
{
    unsafe { early_uart_write(b'1'); }
    let p = GLOBAL_CONSOLE.inner.get();
    unsafe { early_uart_write(b'2'); }
    let opt = unsafe { (*p).as_ref() };
    unsafe { early_uart_write(b'3'); }
    let c = opt.expect("console not initialized");
    unsafe { early_uart_write(b'4'); }
    let result = f(*c);
    unsafe { early_uart_write(b'5'); }
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