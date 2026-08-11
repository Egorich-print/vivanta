#[derive(Debug, Clone, Copy)]
pub struct InterruptControllerInfo {
    pub compatible: &'static str,
    pub distributor_base: u64,
    pub distributor_size: u64,
    pub redistributor_base: Option<u64>,
    pub redistributor_size: Option<u64>,
}
