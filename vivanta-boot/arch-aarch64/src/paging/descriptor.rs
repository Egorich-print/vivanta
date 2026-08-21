// ── Validity & table-type bits ───────────────────────────────────────────────

pub const DESC_VALID: u64 = 1 << 0;
pub const DESC_TABLE: u64 = 1 << 1;

/// Type selector bits [1:0].
///
/// ARM VMSAv8-64: 0b00 invalid, 0b01 block (L1/L2), 0b10 table (L0-L2),
/// 0b11 page (L3). The kernel's table descriptors currently use 0b11
/// (DESC_VALID|DESC_TABLE) for L1/L2 because that is the encoding QEMU's
/// cortex-a53 model boots with (see `table_desc` note in mmu.rs). The
/// predicates below therefore treat any non-zero type as valid and distinguish
/// blocks from the rest; they are encoding-agnostic where possible.
pub const DESC_TYPE_MASK: u64 = 0b11;

#[inline]
pub const fn desc_is_valid(desc: u64) -> bool {
    desc & DESC_TYPE_MASK != 0
}

/// True when the descriptor is NOT a block (i.e. it is a table or page).
/// Used by walkers to decide "walk deeper" vs "leaf". Because the kernel's
/// L1/L2 tables use 0b11 (reserved per spec but QEMU-working), we treat
/// "non-block" as table/leaf-continue.
#[inline]
pub const fn desc_is_table(desc: u64) -> bool {
    desc & DESC_TYPE_MASK != 0 && desc & DESC_TYPE_MASK != 0b01
}

#[inline]
pub const fn desc_is_block(desc: u64) -> bool {
    desc & DESC_TYPE_MASK == 0b01
}

#[inline]
pub const fn desc_is_page(desc: u64) -> bool {
    desc & DESC_TYPE_MASK == 0b11
}

// ── Shareability attributes (bits [9:8]) ─────────────────────────────────────

pub const DESC_SH_NON: u64 = 0 << 8;
pub const DESC_SH_OUTER: u64 = 2 << 8;
pub const DESC_SH_INNER: u64 = 3 << 8;

// ── Memory attributes ────────────────────────────────────────────────────────

pub const DESC_AF: u64 = 1 << 10;

// ── Access permissions (bits [7:6]) ──────────────────────────────────────────

pub const DESC_AP_RW_EL1: u64 = 0 << 6;
pub const DESC_AP_RO_EL1: u64 = 2 << 6;
pub const DESC_AP_RW_EL0: u64 = 1 << 6;
pub const DESC_AP_RO_EL0: u64 = 3 << 6;
pub const DESC_AP_MASK: u64 = 0b11 << 6;

/// AP[2:1] access-permission bits for a (privilege, writability) pair.
///
/// Single source of truth for permission-bit encoding across all descriptor
/// builders in this crate (G3 W^X invariant):
///
/// ```text
///   EL1 RW → 0b00   EL1 RO → 0b10
///   EL0 RW → 0b01   EL0 RO → 0b11
/// ```
///
/// History: before 2026-08-21 the encoders mapped every `user` page to
/// AP=01 regardless of writability, silently making user code pages
/// EL0-*writable*+executable (RWX) and violating the M5.0 G3 acceptance
/// criterion "user code = RX". See
/// `docs/investigations/WX-user-code-ap-encoding.md`.
#[inline]
pub const fn ap_bits(user: bool, writable: bool) -> u64 {
    match (user, writable) {
        (false, false) => DESC_AP_RO_EL1,
        (false, true) => DESC_AP_RW_EL1,
        (true, false) => DESC_AP_RO_EL0,
        (true, true) => DESC_AP_RW_EL0,
    }
}

// ── Execute permissions ──────────────────────────────────────────────────────

pub const DESC_PXN: u64 = 1 << 53;
pub const DESC_XN: u64 = 1 << 54;

// ── Attribute index (bits [4:2]) ─────────────────────────────────────────────

pub const DESC_ATTRIDX_SHIFT: u64 = 2;
pub const DESC_ATTRIDX_MASK: u64 = 7 << DESC_ATTRIDX_SHIFT;

pub const DESC_ATTRIDX_NORMAL: u64 = 0 << DESC_ATTRIDX_SHIFT;
pub const DESC_ATTRIDX_DEVICE: u64 = 1 << DESC_ATTRIDX_SHIFT;
pub const DESC_ATTRIDX_NORMAL_NC: u64 = 2 << DESC_ATTRIDX_SHIFT;

// ── Address masks ────────────────────────────────────────────────────────────

pub const ADDR_MASK: u64 = 0x0000_FFFF_FFFF_F000;
pub const ADDR_MASK_BLOCK: u64 = 0x0000_FFFF_FFE0_0000;
