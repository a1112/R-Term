//! GUI-owned bridge from the standalone SSH backend to the terminal runtime.
pub use ssh_core::*;
mod runtime_adapter;
pub use runtime_adapter::{SshControl, SshInterrupt, SshReader, SshTransport, SshWriter};
