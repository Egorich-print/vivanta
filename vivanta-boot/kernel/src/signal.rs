// ---------------------------------------------------------------------------
// Signal model (minimal) — sigaction extension
// ---------------------------------------------------------------------------

pub const SIGHUP: u8 = 1;
pub const SIGINT: u8 = 2;
pub const SIGKILL: u8 = 9;
pub const SIGTERM: u8 = 15;
pub const SIGSEGV: u8 = 11;
pub const SIGCHLD: u8 = 17;

/// sigaction handler sentinels (POSIX): 0 = SIG_DFL, 1 = SIG_IGN
pub const SIG_DFL: u64 = 0;
pub const SIG_IGN: u64 = 1;
/// Max signal number supported (array size). Index 0 unused.
pub const MAX_SIG: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Signal {
    Hup = 1,
    Int = 2,
    Kill = 9,
    Term = 15,
    Segv = 11,
    Chld = 17,
}

impl Signal {
    pub fn from_num(num: u8) -> Option<Signal> {
        match num {
            1 => Some(Signal::Hup),
            2 => Some(Signal::Int),
            9 => Some(Signal::Kill),
            11 => Some(Signal::Segv),
            15 => Some(Signal::Term),
            17 => Some(Signal::Chld),
            _ => None,
        }
    }
}

/// Minimal sigaction-like descriptor.
///
/// `handler` encodes user handler VA: 0 = SIG_DFL, 1 = SIG_IGN, otherwise
/// user VA to invoke on delivery. Stored as integer to avoid `extern fn`
/// ABI coupling; the dispatch path will interpret it as a user PC.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct SigAction {
    /// User handler VA (0 = default, 1 = ignore).
    pub handler: u64,
    /// Additional signals to block during handler execution.
    pub mask: u64,
    /// Flags (e.g. SA_RESTART); stored verbatim, not yet interpreted.
    pub flags: u32,
}

impl Default for SigAction {
    fn default() -> Self {
        SigAction {
            handler: SIG_DFL,
            mask: 0,
            flags: 0,
        }
    }
}

pub struct SignalState {
    pub pending: Option<Signal>,
    pub blocked: u64, // Bitmask
    /// Per-signal disposition table (index = signal number). Entry 0 unused.
    pub handlers: [SigAction; MAX_SIG],
}

impl SignalState {
    pub fn new() -> Self {
        SignalState {
            pending: None,
            blocked: 0,
            handlers: [SigAction::default(); MAX_SIG],
        }
    }

    pub fn send(&mut self, sig: Signal) {
        if !self.is_blocked(sig) {
            // Respect SIG_IGN disposition: ignored signals are not queued.
            let idx = sig as usize;
            if idx < MAX_SIG && self.handlers[idx].handler == SIG_IGN {
                return;
            }
            self.pending = Some(sig);
        }
    }

    pub fn is_blocked(&self, sig: Signal) -> bool {
        // SIGKILL (and SIGSTOP if ever added) is unblockable per POSIX.
        if sig == Signal::Kill {
            return false;
        }
        (self.blocked & (1 << (sig as u8))) != 0
    }

    /// Raw signal-number check for sigaction path (accepts 1..31).
    pub fn is_blocked_num(&self, sig: u8) -> bool {
        if sig == SIGKILL {
            return false;
        }
        (self.blocked & (1u64 << sig)) != 0
    }

    pub fn block(&mut self, sig: Signal) {
        // SIGKILL cannot be blocked.
        if sig == Signal::Kill {
            return;
        }
        self.blocked |= 1 << (sig as u8);
    }

    pub fn unblock(&mut self, sig: Signal) {
        self.blocked &= !(1 << (sig as u8));
    }

    pub fn take(&mut self) -> Option<Signal> {
        self.pending.take()
    }

    /// Fetch SigAction for raw signal number (1..31). Returns None if OOB.
    pub fn get_action(&self, sig: u8) -> Option<SigAction> {
        if (sig as usize) < MAX_SIG {
            Some(self.handlers[sig as usize])
        } else {
            None
        }
    }

    /// Install SigAction for raw signal number. Returns previous action.
    /// Caller must validate sig != SIGKILL and range.
    pub fn set_action(&mut self, sig: u8, act: SigAction) -> Option<SigAction> {
        if (sig as usize) < MAX_SIG {
            let prev = self.handlers[sig as usize];
            self.handlers[sig as usize] = act;
            Some(prev)
        } else {
            None
        }
    }
}
