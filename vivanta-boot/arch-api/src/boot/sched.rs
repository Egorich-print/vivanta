unsafe extern "Rust" {
    /// Initialise the scheduler threads (boot thread + idle thread).
    /// Must be called after timer init (timer fires reschedule events).
    pub fn sched_init_boot();
}
