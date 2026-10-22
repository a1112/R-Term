//! Deterministic native-window state and effect contracts.

pub mod accessibility;
pub mod commands;
mod controller;
mod effect;
mod host;
pub mod input;
mod intent;
mod layout;
pub mod overlays;
pub mod panes;
pub mod persistence;
pub mod platform;
mod ports;
mod presentation;
mod state;
pub mod tabs;

pub use commands::CommandIntent;
pub use controller::reduce;
pub use effect::{
    ClipboardEffect, HostEffectContext, NotificationEffect, RendererEffect, RuntimePortEffect,
    SpawnEffect, UriEffect, WindowEffect, WindowPortEffect,
};
pub use host::{HostError, HostTurn, PlatformEvent, RuntimeDrain, TurnBudget, WinitHost};
pub use intent::{ConfigDiff, PlatformIntent, TimerId, TimerIntent, WindowIntent};
pub use layout::{
    PaneLayout, PaneLayoutPane, PaneLayoutSpec, PanePlacement, PaneRenderRect, PaneSeparator,
    PaneSplitDirection, PaneSplitSpec, build_pane_layout,
};
pub use overlays::OverlayPresentation;
pub use panes::{PaneCommand, PaneLifecycleIntent, PaneState};
pub use persistence::{PersistenceCommand, PersistenceEffect};
pub use ports::{HostPorts, PortError, PortErrorKind, PortKind};
pub use presentation::{
    CellMetrics, CursorPresentation, FrameRevision, PaneFrameCandidate, PresentationError,
    PresentationFrame, PresentationInput, PresentedPane, RenderMode, ScaleFactor,
    ScrollbarPresentation, SelectionPresentation, SurfacePresentation, build_presentation,
};
pub use state::{
    ConfigState, LifecycleState, PlatformState, PresentationState, TimerState, WindowState,
};
pub use tabs::TabPresentation;
