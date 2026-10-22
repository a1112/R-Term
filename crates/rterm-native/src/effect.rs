use std::num::NonZeroU64;

use rterm_runtime::{EffectSequence, PaneToken, RuntimeRevision, TerminalStateSummary};
use rterm_types::{DamageRegion, TerminalSize};

use crate::PaneSplitDirection;

/// Ordering identity attached to a lossless host-facing runtime effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HostEffectContext {
    pub pane: PaneToken,
    pub revision: RuntimeRevision,
    pub sequence: EffectSequence,
}

/// Commands sent to the pane/session runtime port.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimePortEffect {
    SubmitInput {
        pane: PaneToken,
        bytes: Vec<u8>,
    },
    ResizePane {
        pane: PaneToken,
        size: TerminalSize,
    },
    WriteTransport {
        context: HostEffectContext,
        bytes: Vec<u8>,
    },
    ObserveHostStream {
        context: HostEffectContext,
        bytes: Vec<u8>,
    },
    WriteSessionLog {
        context: HostEffectContext,
        bytes: Vec<u8>,
    },
    ApplyModeChange {
        context: HostEffectContext,
        change: rterm_runtime::TerminalModeChange,
    },
    Restart {
        pane: PaneToken,
    },
    BeginClose {
        pane: PaneToken,
    },
}

/// Commands sent to the native window host.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WindowPortEffect {
    SetFocused(bool),
    SetTitle(String),
    RequestRedraw,
    CloseAfterRuntimes,
    CloseNow,
    ReportDiagnostic(String),
    RuntimeDiagnostic {
        context: HostEffectContext,
        message: String,
    },
}

/// Commands sent to renderer ownership.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RendererEffect {
    ResizeSurface(TerminalSize),
    ApplyConfig {
        revision: u64,
        theme: Option<String>,
    },
    ApplyPane {
        pane: PaneToken,
        revision: RuntimeRevision,
        snapshot: TerminalStateSummary,
        damage: Vec<DamageRegion>,
    },
    Bell {
        context: HostEffectContext,
        count: NonZeroU64,
    },
    RecoverDevice,
    Present,
}

/// Commands sent to clipboard ownership.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClipboardEffect {
    Write {
        context: Option<HostEffectContext>,
        selection: Option<String>,
        contents: String,
    },
    Read {
        context: HostEffectContext,
        selection: String,
    },
}

/// Commands sent to URI handling.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UriEffect {
    Open(String),
}

/// Commands sent to desktop notification ownership.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotificationEffect {
    Show {
        context: HostEffectContext,
        title: Option<String>,
        body: String,
    },
}

/// Commands sent to process/session spawning ownership.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpawnEffect {
    Pane,
    SplitPane {
        source: PaneToken,
        direction: PaneSplitDirection,
    },
    Window,
}

/// Typed output ports produced by [`crate::reduce`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WindowEffect {
    Runtime(RuntimePortEffect),
    Window(WindowPortEffect),
    Renderer(RendererEffect),
    Clipboard(ClipboardEffect),
    Uri(UriEffect),
    Notification(NotificationEffect),
    Persistence(crate::PersistenceEffect),
    Spawn(SpawnEffect),
}
