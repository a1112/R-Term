//! Compatibility facade for the Stage 1 package split.
//!
//! New foundational code should import terminal primitives from
//! `rterm-types` and application-domain state from `rterm-domain` directly.

pub use rterm_domain::{PaneId, TabId, WindowId, WorkspaceId, app_shell, session};
pub use rterm_types::{DamageRegion, SessionId, TerminalSize};

#[cfg(test)]
mod tests {
    use super::{DamageRegion, PaneId, SessionId, TabId, TerminalSize, WindowId, WorkspaceId};

    #[test]
    fn preserves_stage_zero_public_paths() {
        assert_eq!(SessionId::new(42).get(), 42);
        assert_eq!(WindowId::new(1).get(), 1);
        assert_eq!(WorkspaceId::new(2).get(), 2);
        assert_eq!(TabId::new(3).get(), 3);
        assert_eq!(PaneId::new(4).get(), 4);
        assert_eq!(TerminalSize::new(120, 30).cells(), 3600);
        assert!(DamageRegion::new(0, 0, 0, 1).is_empty());
        assert_eq!(DamageRegion::new(2, 0, 3, 1).right(), 5);
    }
}
