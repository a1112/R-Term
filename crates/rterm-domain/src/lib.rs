//! R-Term application domain state, independent of UI and transport runtimes.

pub mod app_shell;
pub mod session;

pub use rterm_types::{PaneId, SessionId};

macro_rules! domain_id {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub struct $name(u64);

        impl $name {
            #[must_use]
            pub const fn new(value: u64) -> Self {
                Self(value)
            }

            #[must_use]
            pub const fn get(self) -> u64 {
                self.0
            }
        }
    };
}

domain_id!(WindowId);
domain_id!(WorkspaceId);
domain_id!(TabId);
