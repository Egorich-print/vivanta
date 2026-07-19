pub mod qemu;

use vivanta_boot_common::Console;

pub trait Platform {
    fn console(&self) -> &dyn Console;
}
