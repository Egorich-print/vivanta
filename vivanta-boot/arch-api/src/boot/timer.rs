extern "Rust" {
    /// Initialise the architecture timer.
    /// Must be called after interrupt controller init (timer registers
    /// its IRQ handler via the controller).
    pub fn timer_init();

    /// Return the current tick count from the architecture timer.
    pub fn ticks() -> u64;
}
