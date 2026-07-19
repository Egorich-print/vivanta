#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MmioKind {
    Device,
    UserDevice,
}

impl MmioKind {
    pub fn is_user_accessible(self) -> bool {
        matches!(self, MmioKind::UserDevice)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct MmioRegion {
    pub base: u64,
    pub size: u64,
    pub kind: MmioKind,
}