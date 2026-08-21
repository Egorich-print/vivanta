// ---------------------------------------------------------------------------
// boot-common — runtime utilities and diagnostic types for Vivanta boot
//
// Contract types (BootInfo, MemoryMap, MmioRegion, etc.) live in boot-info.
// This crate provides: Console, println!, FDT scanner, NS16550 driver,
// and diagnostic enums (Architecture, BootSource) for early output.
// ---------------------------------------------------------------------------

#![no_std]

pub mod fdt;
pub mod hardware;
pub mod memory_discovery;
pub mod ns16550;
pub mod pl011;

// Re-export boot-info contract types so existing code keeps working
pub use vivanta_boot_info::{
    BootInfo, InterruptControllerInfo, MemoryMap, MemoryRegion, MemoryRegionKind, MmioKind,
    MmioRegion,
};

use core::cell::UnsafeCell;
use core::fmt;
use core::sync::atomic::{AtomicBool, Ordering};

/// Console spin lock: serialises console output across threads and IRQ
/// contexts. Interrupts are disabled while the lock is held so a timer IRQ
/// handler (which may itself print) cannot self-deadlock on the lock (G4).
struct ConsoleLock {
    held: AtomicBool,
}

impl ConsoleLock {
    const fn new() -> Self {
        ConsoleLock {
            held: AtomicBool::new(false),
        }
    }

    fn acquire(&self) {
        while self
            .held
            .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            core::hint::spin_loop();
        }
    }

    fn release(&self) {
        self.held.store(false, Ordering::Release);
    }
}

static CONSOLE_LOCK: ConsoleLock = ConsoleLock::new();

/// IRQ-disable hook for the console lock (G4).
///
/// The console lock serialises output across threads and IRQ contexts. To
/// prevent a timer IRQ handler (which may itself print) from self-deadlocking
/// on the lock, the lock must be held with interrupts disabled. The arch layer
/// registers its `disable_interrupts` here; platforms that never run the
/// scheduler leave it unset (single-threaded bring-up still works).
static CONSOLE_IRQ_GUARD: core::sync::atomic::AtomicPtr<()> =
    core::sync::atomic::AtomicPtr::new(core::ptr::null_mut());

/// Register the arch's `disable_interrupts` as the console IRQ guard factory.
pub fn set_console_irq_guard(f: fn() -> vivanta_arch_api::interrupts::InterruptGuard) {
    CONSOLE_IRQ_GUARD.store(f as *mut (), core::sync::atomic::Ordering::Relaxed);
}

fn console_guard() -> Option<vivanta_arch_api::interrupts::InterruptGuard> {
    let p = CONSOLE_IRQ_GUARD.load(core::sync::atomic::Ordering::Relaxed);
    if p.is_null() {
        None
    } else {
        Some(unsafe {
            core::mem::transmute::<*mut (), fn() -> vivanta_arch_api::interrupts::InterruptGuard>(p)
        }())
    }
}

// ---------------------------------------------------------------------------
// EarlyPlatformInfo — platform-provided constants for early boot debug output.
// Must be set by adapter_main before console initialization.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
pub struct EarlyPlatformInfo {
    pub uart_base: usize,
}

static mut EARLY_PLATFORM: Option<EarlyPlatformInfo> = None;

pub fn set_early_platform(info: EarlyPlatformInfo) {
    unsafe {
        EARLY_PLATFORM = Some(info);
    }
}

pub fn early_platform() -> Option<EarlyPlatformInfo> {
    unsafe { EARLY_PLATFORM }
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
    match unsafe { (*GLOBAL_CONSOLE.inner.get()).replace(c) } {
        None => {}
        Some(_) => panic!("set_console: already initialized"),
    }
}

pub fn write_direct(s: &str) {
    let _guard = console_guard();
    CONSOLE_LOCK.acquire();
    unsafe {
        if let Some(c) = (*GLOBAL_CONSOLE.inner.get()).as_ref() {
            c.write_str(s);
        }
    }
    CONSOLE_LOCK.release();
}

pub fn with_console<F, R>(f: F) -> R
where
    F: FnOnce(&dyn Console) -> R,
{
    let _guard = console_guard();
    CONSOLE_LOCK.acquire();
    let p = GLOBAL_CONSOLE.inner.get();
    let opt = unsafe { (*p).as_ref() };
    let c = opt.expect("console not initialized");
    let r = f(*c);
    CONSOLE_LOCK.release();
    r
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
