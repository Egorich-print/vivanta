// Memory barrier wrappers — AArch64
// Context: any (boot, thread, interrupt)

pub fn dsb_sy() {
    unsafe { core::arch::asm!("dsb sy", options(nostack)); }
}

pub fn dsb_ish() {
    unsafe { core::arch::asm!("dsb ish", options(nostack)); }
}

pub fn dsb_ishst() {
    unsafe { core::arch::asm!("dsb ishst", options(nostack)); }
}

pub fn dmb_sy() {
    unsafe { core::arch::asm!("dmb sy", options(nostack)); }
}

pub fn dmb_ish() {
    unsafe { core::arch::asm!("dmb ish", options(nostack)); }
}

pub fn dmb_ishst() {
    unsafe { core::arch::asm!("dmb ishst", options(nostack)); }
}

pub fn isb() {
    unsafe { core::arch::asm!("isb", options(nostack)); }
}
