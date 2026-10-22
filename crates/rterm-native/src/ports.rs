use std::{error::Error, fmt};

use rterm_domain::WindowId;
use rterm_runtime::PaneToken;

use crate::{
    ClipboardEffect, NotificationEffect, PersistenceEffect, RendererEffect, RuntimePortEffect,
    SpawnEffect, UriEffect, WindowPortEffect,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortKind {
    Runtime,
    Window,
    Renderer,
    Clipboard,
    Uri,
    Notification,
    Persistence,
    Spawn,
    Platform,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortErrorKind {
    Backpressure,
    Unavailable,
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortError {
    kind: PortErrorKind,
    message: String,
}

impl PortError {
    #[must_use]
    pub fn new(kind: PortErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    #[must_use]
    pub const fn kind(&self) -> PortErrorKind {
        self.kind
    }

    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for PortError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:?}: {}", self.kind, self.message)
    }
}

impl Error for PortError {}

/// Platform-owned implementations of every typed native side-effect port.
pub trait HostPorts {
    /// Executes a runtime command.
    ///
    /// # Errors
    /// Returns a typed runtime-port failure.
    fn runtime(&mut self, effect: &RuntimePortEffect) -> Result<(), PortError>;
    /// Executes a native-window command.
    ///
    /// # Errors
    /// Returns a typed window-port failure.
    fn window(&mut self, effect: &WindowPortEffect) -> Result<(), PortError>;
    /// Executes a renderer command.
    ///
    /// # Errors
    /// Returns a typed renderer-port failure.
    fn renderer(&mut self, effect: &RendererEffect) -> Result<(), PortError>;
    /// Executes a clipboard command.
    ///
    /// # Errors
    /// Returns a typed clipboard-port failure.
    fn clipboard(&mut self, effect: &ClipboardEffect) -> Result<(), PortError>;
    /// Executes a URI command.
    ///
    /// # Errors
    /// Returns a typed URI-port failure.
    fn uri(&mut self, effect: &UriEffect) -> Result<(), PortError>;
    /// Executes a notification command.
    ///
    /// # Errors
    /// Returns a typed notification-port failure.
    fn notification(&mut self, effect: &NotificationEffect) -> Result<(), PortError>;
    /// Executes a persistence command.
    ///
    /// # Errors
    /// Returns a typed persistence-port failure.
    fn persistence(&mut self, effect: &PersistenceEffect) -> Result<(), PortError>;
    /// Executes a process/window spawning command.
    ///
    /// # Errors
    /// Returns a typed spawning-port failure.
    fn spawn(&mut self, effect: &SpawnEffect) -> Result<(), PortError>;

    /// Drains controller intents for one pane within the requested batch cap.
    ///
    /// # Errors
    ///
    /// Returns a typed port failure when the runtime owner is unavailable.
    fn drain_runtime(
        &mut self,
        pane: PaneToken,
        max_batches: usize,
    ) -> Result<crate::RuntimeDrain, PortError>;

    /// Schedules the sole continuation wake for unfinished pane work.
    ///
    /// # Errors
    ///
    /// Returns a typed platform failure when the event loop rejects the wake.
    fn schedule_runtime_continuation(
        &mut self,
        window: WindowId,
        pane: PaneToken,
    ) -> Result<(), PortError>;
}
