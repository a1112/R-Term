//! GUI-owned bridge from the standalone SSH backend to the terminal runtime.
pub use rterm_types::SshTerminalSize;
pub use ssh_core::*;
mod runtime_adapter;
pub use runtime_adapter::{SshControl, SshInterrupt, SshReader, SshTransport, SshWriter};
