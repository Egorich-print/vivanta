unsafe extern "Rust" {
    /// Set up exception vectors, enable SIMD/FP, etc.
    /// Called once at the very start of kernel_main, before PMM.
    pub fn early_init();

    /// Halt the CPU until the next interrupt. Used when idle.
    pub fn wait_for_interrupt();
}
