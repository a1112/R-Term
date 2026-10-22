use rterm_runtime::{PaneToken, RuntimeProgress};

use crate::WindowState;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccessibilityPane {
    pub pane: PaneToken,
    pub label: String,
    pub active: bool,
    pub progress: RuntimeProgress,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AccessibilitySnapshot {
    pub panes: Vec<AccessibilityPane>,
}

#[must_use]
pub fn build_accessibility_snapshot(state: &WindowState) -> AccessibilitySnapshot {
    AccessibilitySnapshot {
        panes: state
            .pane_order
            .iter()
            .filter_map(|pane_id| state.panes.get(pane_id))
            .map(|pane| AccessibilityPane {
                pane: pane.token,
                label: pane
                    .title
                    .clone()
                    .unwrap_or_else(|| format!("Pane {}", pane.token.pane().get())),
                active: state.active_pane == Some(pane.token.pane()),
                progress: pane.progress,
            })
            .collect(),
    }
}
