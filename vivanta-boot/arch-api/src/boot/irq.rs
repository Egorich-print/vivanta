unsafe extern "Rust" {
    /// Initialise the interrupt controller from the Device Tree.
    /// `dtb`: physical address of the flattened device tree blob.
    pub fn irq_init(dtb: usize);

    /// Enable interrupts at the CPU level.
    pub fn irq_cpu_enable();
}
