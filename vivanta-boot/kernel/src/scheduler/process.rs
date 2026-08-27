//! Process management module.

pub mod process;
pub mod process_table;

pub use process::{Process, ProcessState, ProcessHandle, Pid, ProcessId, ProcessHandle, Signal, SignalState};
pub use process_table::{ProcessTable, ProcessHandle, Pid, ProcessId};