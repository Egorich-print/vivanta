// ── Validity & table-type bits ───────────────────────────────────────────────

pub const DESC_VALID: u64 = 1 << 0;
pub const DESC_TABLE: u64 = 1 << 1;

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
