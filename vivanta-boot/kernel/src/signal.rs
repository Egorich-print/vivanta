// ---------------------------------------------------------------------------
// Signal model (minimal)
// ---------------------------------------------------------------------------

pub const SIGHUP: u8 = 1;
pub const SIGINT: u8 = 2;
pub const SIGKILL: u8 = 9;
pub const SIGTERM: u8 = 15;
pub const SIGSEGV: u8 = 11;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Signal {
    Hup = 1,
    Int = 2,
    Kill = 9,
    Term = 15,
    Segv = 11,
}

impl Signal {
    pub fn from_num(num: u8) -> Option<Signal> {
        match num {
            1 => Some(Signal::Hup),
            2 => Some(Signal::Int),
            9 => Some(Signal::Kill),
            11 => Some(Signal::Segv),
            15 => Some(Signal::Term),
            _ => None,
        }
    }
}

pub struct SignalState {
    pub pending: Option<Signal>,
    pub blocked: u64, // Bitmask
}

impl SignalState {
    pub fn new() -> Self {
        SignalState {
            pending: None,
            blocked: 0,
        }
    }

    pub fn send(&mut self, sig: Signal) {
        if !self.is_blocked(sig) {
            self.pending = Some(sig);
        }
    }

    pub fn is_blocked(&self, sig: Signal) -> bool {
        (self.blocked & (1 << (sig as u8))) != 0
    }

    pub fn block(&mut self, sig: Signal) {
        self.blocked |= 1 << (sig as u8);
    }

    pub fn unblock(&mut self, sig: Signal) {
        self.blocked &= !(1 << (sig as u8));
    }

    pub fn take(&mut self) -> Option<Signal> {
        self.pending.take()
    }
}
