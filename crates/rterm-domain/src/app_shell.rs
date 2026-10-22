use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{PaneId, TabId, WindowId, WorkspaceId};

const DEFAULT_WORKSPACE_NAME: &str = "default";
const PANE_DIRECTION_LAYOUT_COLUMNS: i32 = 10_000;
const PANE_DIRECTION_LAYOUT_ROWS: i32 = 10_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaneLaunch {
    domain: PaneLaunchDomain,
    program: String,
    args: Vec<String>,
    cwd: Option<String>,
    environment: HashMap<String, String>,
}

/// Identifies the transport used to create a pane.
///
/// The local variant preserves the historical `PaneLaunch` behavior. The SSH
/// variant contains only connection metadata; interactive secrets are always
/// requested by the runtime and are deliberately not representable here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaneLaunchDomain {
    Local,
    Ssh(SshPaneLaunch),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SshPaneLaunch {
    target: String,
    target_kind: SshTargetKind,
    username: Option<String>,
    port: Option<u16>,
    auth: SshAuthDescription,
    known_hosts_policy: SshKnownHostsPolicy,
    remote_command: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SshTargetKind {
    Direct,
    OpenSsh,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SshAuthDescription {
    Agent,
    PasswordPrompt,
    PrivateKey { path: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SshKnownHostsPolicy {
    RejectUnknown,
    Prompt,
    TrustOnFirstUse,
    AcceptUnknown,
}

impl SshPaneLaunch {
    #[must_use]
    pub fn new(
        target: impl Into<String>,
        auth: SshAuthDescription,
        known_hosts_policy: SshKnownHostsPolicy,
    ) -> Self {
        Self {
            target: target.into(),
            target_kind: SshTargetKind::Direct,
            username: None,
            port: None,
            auth,
            known_hosts_policy,
            remote_command: Vec::new(),
        }
    }

    #[must_use]
    pub fn openssh(
        target: impl Into<String>,
        auth: SshAuthDescription,
        known_hosts_policy: SshKnownHostsPolicy,
    ) -> Self {
        Self {
            target: target.into(),
            target_kind: SshTargetKind::OpenSsh,
            username: None,
            port: None,
            auth,
            known_hosts_policy,
            remote_command: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_remote_command<I, S>(mut self, command: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.remote_command = command.into_iter().map(Into::into).collect();
        self
    }

    #[must_use]
    pub fn target(&self) -> &str {
        &self.target
    }

    #[must_use]
    pub const fn target_kind(&self) -> SshTargetKind {
        self.target_kind
    }

    /// Applies explicit OpenSSH user/port overrides without retaining any
    /// authentication secret. The values are resolved by the native worker
    /// after the bootstrap frame has been presented.
    #[must_use]
    pub fn with_target_overrides(mut self, username: Option<String>, port: Option<u16>) -> Self {
        self.username = username;
        self.port = port;
        self
    }

    #[must_use]
    pub fn username(&self) -> Option<&str> {
        self.username.as_deref()
    }

    #[must_use]
    pub const fn port(&self) -> Option<u16> {
        self.port
    }

    #[must_use]
    pub fn auth(&self) -> &SshAuthDescription {
        &self.auth
    }

    #[must_use]
    pub const fn known_hosts_policy(&self) -> SshKnownHostsPolicy {
        self.known_hosts_policy
    }

    #[must_use]
    pub fn remote_command(&self) -> &[String] {
        &self.remote_command
    }
}

impl PaneLaunch {
    #[must_use]
    pub fn local(program: impl Into<String>) -> Self {
        Self {
            domain: PaneLaunchDomain::Local,
            program: program.into(),
            args: Vec::new(),
            cwd: None,
            environment: HashMap::new(),
        }
    }

    #[must_use]
    pub fn ssh(launch: SshPaneLaunch) -> Self {
        Self {
            domain: PaneLaunchDomain::Ssh(launch),
            program: String::new(),
            args: Vec::new(),
            cwd: None,
            environment: HashMap::new(),
        }
    }

    #[must_use]
    pub fn domain(&self) -> &PaneLaunchDomain {
        &self.domain
    }

    /// Returns the launch metadata inherited by a newly created tab, split,
    /// or window. SSH targets are carried over, but an explicit remote
    /// command is intentionally not repeated: child panes start a shell
    /// unless the caller supplied a new command.
    #[must_use]
    pub fn for_child_pane(&self) -> Self {
        let PaneLaunchDomain::Ssh(ssh) = &self.domain else {
            return self.clone();
        };

        Self::ssh(SshPaneLaunch {
            target: ssh.target.clone(),
            target_kind: ssh.target_kind,
            username: ssh.username.clone(),
            port: ssh.port,
            auth: ssh.auth.clone(),
            known_hosts_policy: ssh.known_hosts_policy,
            remote_command: Vec::new(),
        })
    }

    #[must_use]
    pub fn with_args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.args.extend(args.into_iter().map(Into::into));
        self
    }

    #[must_use]
    pub fn with_cwd(mut self, cwd: impl Into<String>) -> Self {
        self.cwd = Some(cwd.into());
        self
    }

    #[must_use]
    pub fn with_environment<I, K, V>(mut self, environment: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        self.environment.extend(
            environment
                .into_iter()
                .map(|(key, value)| (key.into(), value.into())),
        );
        self
    }

    #[must_use]
    pub fn program(&self) -> &str {
        &self.program
    }

    #[must_use]
    pub fn args(&self) -> &[String] {
        &self.args
    }

    #[must_use]
    pub fn cwd(&self) -> Option<&str> {
        self.cwd.as_deref()
    }

    #[must_use]
    pub fn environment(&self) -> &HashMap<String, String> {
        &self.environment
    }

    fn set_cwd(&mut self, cwd: Option<String>) {
        self.cwd = cwd;
    }
}

/// Durable information needed to create a fresh tab from an existing tab.
///
/// This type deliberately contains launch configuration and layout only.  It
/// never owns a live PTY, SSH transport, terminal screen, or runtime-only pane
/// metadata, so callers can safely use it for duplicate and recently-closed
/// tab flows without accidentally claiming that a process can be resumed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TabReconnectSnapshot {
    title: Option<String>,
    panes: Vec<PaneReconnectSnapshot>,
    active_pane_index: usize,
    pane_activation_order: Vec<usize>,
    zoomed_pane_index: Option<usize>,
}

impl TabReconnectSnapshot {
    #[must_use]
    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    #[must_use]
    pub fn panes(&self) -> &[PaneReconnectSnapshot] {
        &self.panes
    }

    #[must_use]
    pub const fn active_pane_index(&self) -> usize {
        self.active_pane_index
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaneReconnectSnapshot {
    launch: PaneLaunch,
    split: Option<PaneReconnectSplit>,
}

impl PaneReconnectSnapshot {
    #[must_use]
    pub fn launch(&self) -> &PaneLaunch {
        &self.launch
    }

    #[must_use]
    pub const fn split(&self) -> Option<PaneReconnectSplit> {
        self.split
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PaneReconnectSplit {
    pub source_pane_index: usize,
    pub direction: SplitDirection,
    pub source_size_delta: i16,
}

/// A live tab model extracted from one shell and ready for insertion in
/// another.  The application layer keeps the corresponding runtime resources
/// alongside this value while transferring a tab between native windows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetachedTab {
    tab: Tab,
    source_index: usize,
}

/// Result of importing a detached tab into a shell.  IDs are always fresh in
/// the destination, and the mapping lets window hosts retain live runtime
/// ownership while redirecting events from the source identities.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportedTab {
    tab_id: TabId,
    pane_id_map: HashMap<PaneId, PaneId>,
}

impl ImportedTab {
    #[must_use]
    pub const fn tab_id(&self) -> TabId {
        self.tab_id
    }

    #[must_use]
    pub fn pane_id_map(&self) -> &HashMap<PaneId, PaneId> {
        &self.pane_id_map
    }
}

impl DetachedTab {
    #[must_use]
    pub fn tab(&self) -> &Tab {
        &self.tab
    }

    #[must_use]
    pub const fn source_index(&self) -> usize {
        self.source_index
    }
}

/// An in-memory entry for the browser-style "reopen closed tab" command.
/// The entry is intentionally process-local and contains no live runtime.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClosedTabEntry {
    snapshot: TabReconnectSnapshot,
    origin_window_id: WindowId,
    origin_workspace_id: WorkspaceId,
    origin_index: usize,
}

impl ClosedTabEntry {
    #[must_use]
    pub const fn new(
        snapshot: TabReconnectSnapshot,
        origin_window_id: WindowId,
        origin_workspace_id: WorkspaceId,
        origin_index: usize,
    ) -> Self {
        Self {
            snapshot,
            origin_window_id,
            origin_workspace_id,
            origin_index,
        }
    }

    #[must_use]
    pub const fn origin_window_id(&self) -> WindowId {
        self.origin_window_id
    }

    #[must_use]
    pub const fn origin_workspace_id(&self) -> WorkspaceId {
        self.origin_workspace_id
    }

    #[must_use]
    pub const fn origin_index(&self) -> usize {
        self.origin_index
    }

    #[must_use]
    pub fn snapshot(&self) -> &TabReconnectSnapshot {
        &self.snapshot
    }

    #[must_use]
    pub fn into_snapshot(self) -> TabReconnectSnapshot {
        self.snapshot
    }
}

/// Bounded, last-in-first-out storage for recently closed tabs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClosedTabHistory {
    capacity: usize,
    entries: Vec<ClosedTabEntry>,
}

impl ClosedTabHistory {
    #[must_use]
    pub const fn new(capacity: usize) -> Self {
        Self {
            capacity,
            entries: Vec::new(),
        }
    }

    #[must_use]
    pub const fn capacity(&self) -> usize {
        self.capacity
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn set_capacity(&mut self, capacity: usize) {
        self.capacity = capacity;
        let excess = self.entries.len().saturating_sub(capacity);
        if excess > 0 {
            self.entries.drain(..excess);
        }
    }

    pub fn push(&mut self, entry: ClosedTabEntry) {
        if self.capacity == 0 {
            return;
        }
        let excess = self
            .entries
            .len()
            .saturating_add(1)
            .saturating_sub(self.capacity);
        if excess > 0 {
            self.entries.drain(..excess);
        }
        self.entries.push(entry);
    }

    pub fn pop(&mut self) -> Option<ClosedTabEntry> {
        self.entries.pop()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloseTabSelection {
    /// Select the right neighbor, or the left neighbor at the end of the bar.
    Adjacent,
    /// Select the most recently active surviving tab when one exists.
    LastActive,
    /// Always select the left neighbor when possible.
    Left,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppShell {
    workspaces: Vec<Workspace>,
    active_workspace_id: WorkspaceId,
    pending_windows: Vec<PendingWindow>,
    next_window_id: u64,
    next_workspace_id: u64,
    next_tab_id: u64,
    next_pane_id: u64,
    default_launch: PaneLaunch,
}

impl AppShell {
    #[must_use]
    pub fn new(default_launch: PaneLaunch) -> Self {
        Self::new_with_workspace_name(default_launch, DEFAULT_WORKSPACE_NAME)
    }

    #[must_use]
    pub fn new_with_workspace_name(
        default_launch: PaneLaunch,
        workspace_name: impl Into<String>,
    ) -> Self {
        let workspace_id = WorkspaceId::new(1);
        let tab_id = TabId::new(1);
        let pane_id = PaneId::new(1);
        let first_workspace = Workspace::new(
            workspace_id,
            workspace_name.into(),
            vec![Tab::new(
                tab_id,
                vec![Pane::new(pane_id, default_launch.clone())],
            )],
            tab_id,
        );

        Self {
            workspaces: vec![first_workspace],
            active_workspace_id: workspace_id,
            pending_windows: Vec::new(),
            next_window_id: 2,
            next_workspace_id: 2,
            next_tab_id: 2,
            next_pane_id: 2,
            default_launch,
        }
    }

    #[must_use]
    pub fn from_pending_window(pending_window: PendingWindow) -> Self {
        let window_id = pending_window.id();
        let workspace_id = pending_window.workspace_id;
        let default_launch = pending_window
            .tab()
            .panes()
            .first()
            .map_or_else(|| PaneLaunch::local(""), |pane| pane.launch().clone());
        let workspace_name = pending_window.workspace_name;
        let tab = pending_window.tab;
        let active_tab_id = tab.id();
        let next_tab_id = tab.id().get().saturating_add(1);
        let next_pane_id = tab
            .panes()
            .iter()
            .map(|pane| pane.id().get())
            .max()
            .unwrap_or(0)
            .saturating_add(1);
        let workspace = Workspace::new(workspace_id, workspace_name, vec![tab], active_tab_id);

        Self {
            workspaces: vec![workspace],
            active_workspace_id: workspace_id,
            pending_windows: Vec::new(),
            next_window_id: window_id.get().saturating_add(1),
            next_workspace_id: workspace_id.get().saturating_add(1),
            next_tab_id,
            next_pane_id,
            default_launch,
        }
    }

    #[must_use]
    pub fn workspaces(&self) -> &[Workspace] {
        &self.workspaces
    }

    #[must_use]
    pub fn pending_windows(&self) -> &[PendingWindow] {
        &self.pending_windows
    }

    pub fn take_next_pending_window(&mut self) -> Option<PendingWindow> {
        if self.pending_windows.is_empty() {
            return None;
        }
        Some(self.pending_windows.remove(0))
    }

    #[must_use]
    pub fn active_workspace_id(&self) -> WorkspaceId {
        self.active_workspace_id
    }

    #[must_use]
    pub fn active_tab_id(&self) -> TabId {
        self.active_workspace().active_tab_id()
    }

    #[must_use]
    pub fn last_active_tab_id(&self) -> Option<TabId> {
        self.active_workspace().last_active_tab_id()
    }

    #[must_use]
    pub fn active_pane_id(&self) -> PaneId {
        self.active_workspace().active_pane_id()
    }

    /// Returns the active workspace.
    ///
    /// # Panics
    /// Panics if the recorded active workspace id no longer exists in the
    /// workspace list. This should be impossible through public APIs because
    /// active IDs are created from existing entries, and all mutating APIs
    /// validate IDs before storing them.
    #[must_use]
    pub fn active_workspace(&self) -> &Workspace {
        self.workspace(self.active_workspace_id)
            .expect("active workspace must exist")
    }

    #[must_use]
    pub fn active_tab(&self) -> &Tab {
        self.active_workspace().active_tab()
    }

    /// Captures enough of a tab to create an equivalent, newly connected tab.
    /// Runtime projection fields such as user variables, progress and unseen
    /// output are intentionally excluded.
    ///
    /// # Errors
    ///
    /// Returns [`AppShellError::InvalidTab`] if `tab` is not in the active
    /// workspace.
    pub fn tab_reconnect_snapshot(
        &self,
        tab: TabId,
    ) -> Result<TabReconnectSnapshot, AppShellError> {
        let tab = self
            .active_workspace()
            .tab(tab)
            .ok_or(AppShellError::InvalidTab(tab))?;
        Ok(TabReconnectSnapshot::from_tab(tab))
    }

    /// Restores a reconnect snapshot at `index` in the active workspace and
    /// activates the new tab.  Values beyond the end of the tab list append.
    ///
    /// # Errors
    ///
    /// Returns [`AppShellError::InvalidTab`] when the snapshot contains an
    /// invalid pane layout.  Snapshots produced by [`Self::tab_reconnect_snapshot`]
    /// are always valid.
    ///
    /// # Panics
    ///
    /// Panics only if the shell invariant that an active workspace exists is
    /// already broken.
    pub fn restore_tab_snapshot(
        &mut self,
        snapshot: TabReconnectSnapshot,
        index: usize,
    ) -> Result<TabId, AppShellError> {
        let tab_id = self.next_tab_id();
        let pane_ids = (0..snapshot.panes.len())
            .map(|_| self.next_pane_id())
            .collect::<Vec<_>>();
        let tab = Tab::from_reconnect_snapshot(tab_id, snapshot, &pane_ids)?;
        let workspace = self
            .active_workspace_mut()
            .expect("active workspace must exist");
        workspace.insert_tab(tab, index);
        Ok(tab_id)
    }

    /// Creates a fresh reconnect clone directly to the right of `tab`.
    ///
    /// # Errors
    ///
    /// Returns [`AppShellError::InvalidTab`] if `tab` is not in the active
    /// workspace.
    pub fn duplicate_tab(&mut self, tab: TabId) -> Result<TabId, AppShellError> {
        let index = self
            .active_workspace()
            .tab_position(tab)
            .ok_or(AppShellError::InvalidTab(tab))?;
        let snapshot = self.tab_reconnect_snapshot(tab)?;
        self.restore_tab_snapshot(snapshot, index.saturating_add(1))
    }

    /// Removes a non-final tab from the active workspace without destroying
    /// its model.  The caller owns the returned value until it is inserted
    /// into another shell or restored to this one.
    ///
    /// # Errors
    ///
    /// Returns [`AppShellError::CannotCloseLastTab`] for the final tab and
    /// [`AppShellError::InvalidTab`] when the tab is not in the active
    /// workspace.
    ///
    /// # Panics
    ///
    /// Panics only if the shell invariant that an active workspace exists is
    /// already broken.
    pub fn detach_tab(&mut self, tab: TabId) -> Result<DetachedTab, AppShellError> {
        self.active_workspace_mut()
            .expect("active workspace must exist")
            .detach_tab(tab)
    }

    /// Produces a detached representation for a tab that a window host is
    /// about to transfer while retiring the source window.  Unlike
    /// [`Self::detach_tab`], this also supports the final tab because the
    /// source shell remains valid only until the host has installed the
    /// destination owner and closed that window.
    ///
    /// # Errors
    ///
    /// Returns [`AppShellError::InvalidTab`] when the tab is not in the
    /// active workspace.
    pub fn clone_tab_for_transfer(&self, tab: TabId) -> Result<DetachedTab, AppShellError> {
        let workspace = self.active_workspace();
        let source_index = workspace
            .tab_position(tab)
            .ok_or(AppShellError::InvalidTab(tab))?;
        Ok(DetachedTab {
            tab: workspace.tabs[source_index].clone(),
            source_index,
        })
    }

    /// Inserts a detached tab into the active workspace, assigning fresh tab
    /// and pane IDs so it cannot collide with state already owned by this
    /// shell.  The inserted tab becomes active.
    ///
    /// # Errors
    ///
    /// Returns [`AppShellError::InvalidTab`] if the detached tab has invalid
    /// internal pane references.
    pub fn insert_detached_tab(
        &mut self,
        detached: DetachedTab,
        index: usize,
    ) -> Result<TabId, AppShellError> {
        self.import_detached_tab(detached, index)
            .map(|imported| imported.tab_id)
    }

    /// Imports a detached tab with fresh destination identities and returns
    /// the complete source-to-destination pane identity mapping.
    ///
    /// # Errors
    ///
    /// Returns [`AppShellError::InvalidTab`] if the detached tab contains an
    /// invalid pane layout or split reference.
    ///
    /// # Panics
    ///
    /// Panics only if the shell invariant that an active workspace exists is
    /// already broken.
    pub fn import_detached_tab(
        &mut self,
        mut detached: DetachedTab,
        index: usize,
    ) -> Result<ImportedTab, AppShellError> {
        let tab_id = self.next_tab_id();
        let pane_id_map = detached
            .tab
            .panes()
            .iter()
            .map(|pane| (pane.id(), self.next_pane_id()))
            .collect::<HashMap<_, _>>();
        detached.tab.remap_ids(tab_id, &pane_id_map)?;
        let workspace = self
            .active_workspace_mut()
            .expect("active workspace must exist");
        workspace.insert_tab(detached.tab, index);
        Ok(ImportedTab {
            tab_id,
            pane_id_map,
        })
    }

    /// Rebalances every split tree from its current cell layout when the
    /// available pane grid changes.
    pub fn preserve_split_layout_for_resize(
        &mut self,
        old_columns: u16,
        old_rows: u16,
        new_columns: u16,
        new_rows: u16,
    ) {
        if old_columns == new_columns && old_rows == new_rows {
            return;
        }

        for workspace in &mut self.workspaces {
            for tab in &mut workspace.tabs {
                tab.preserve_split_layout_for_resize(old_columns, old_rows, new_columns, new_rows);
            }
        }
    }

    #[must_use]
    pub fn active_pane(&self) -> &Pane {
        self.active_workspace().active_pane()
    }

    #[must_use]
    pub fn active_tab_position(&self) -> usize {
        self.active_workspace()
            .tabs()
            .iter()
            .position(|tab| tab.id == self.active_tab_id())
            .map_or(0, |index| index + 1)
    }

    #[must_use]
    pub fn active_pane_position(&self) -> usize {
        self.active_tab()
            .panes()
            .iter()
            .position(|pane| pane.id == self.active_pane_id())
            .map_or(0, |index| index + 1)
    }

    #[must_use]
    pub fn pane_ids(&self) -> Vec<PaneId> {
        self.workspaces
            .iter()
            .flat_map(Workspace::tabs)
            .flat_map(Tab::panes)
            .chain(
                self.pending_windows
                    .iter()
                    .flat_map(|window| window.tab().panes()),
            )
            .map(Pane::id)
            .collect()
    }

    /// Clear metadata projected from the current runtime for one pane while
    /// preserving its durable identity, launch configuration, and layout.
    ///
    /// # Errors
    ///
    /// Returns [`AppShellError::InvalidPane`] when `pane` is not owned by this
    /// shell.
    pub fn reset_pane_runtime_projection(&mut self, pane: PaneId) -> Result<(), AppShellError> {
        for workspace in &mut self.workspaces {
            for tab in &mut workspace.tabs {
                if let Some(candidate) = tab.panes.iter_mut().find(|candidate| candidate.id == pane)
                {
                    candidate.reset_runtime_projection();
                    return Ok(());
                }
            }
        }
        Err(AppShellError::InvalidPane(pane))
    }

    /// Applies a shell action and returns whether it was accepted.
    ///
    /// # Panics
    /// Panics only if internal active IDs are missing from their backing
    /// collections. This should not happen for valid state transitions; active IDs
    /// are created from existing items and all mutating operations validate IDs
    /// before selecting them.
    ///
    /// # Errors
    /// Returns an [`AppShellError`] when the action references an invalid ID or
    /// requests a forbidden operation (for example, closing the last tab/pane).
    #[expect(
        clippy::too_many_lines,
        reason = "the complete action dispatch stays together so every variant remains visible"
    )]
    pub fn apply_action(&mut self, action: AppAction) -> Result<(), AppShellError> {
        let clears_active_pane_unseen_output = !matches!(
            &action,
            AppAction::Nop | AppAction::SetPaneHasUnseenOutput { .. }
        );
        let result = match action {
            AppAction::Nop => Ok(()),
            AppAction::Multiple { actions } => self.apply_multiple(actions),
            AppAction::NewTab { launch } => {
                self.apply_new_tab(launch);
                Ok(())
            }
            AppAction::SpawnWindow { launch } => {
                self.apply_spawn_window(launch);
                Ok(())
            }
            AppAction::CloseTab {
                tab,
                switch_to_last_active,
            } => self.apply_close_tab(tab, switch_to_last_active),
            AppAction::CloseTabWithSelection { tab, selection } => {
                self.apply_close_tab_with_selection(tab, selection)
            }
            AppAction::ActivateTab { tab } => self.apply_activate_tab(tab),
            AppAction::ActivateTabIndex { index } => {
                self.apply_activate_tab_index(index);
                Ok(())
            }
            AppAction::ActivateTabRelative { offset } => self.apply_activate_tab_relative(offset),
            AppAction::ActivateTabRelativeNoWrap { offset } => {
                self.apply_activate_tab_relative_no_wrap(offset)
            }
            AppAction::ActivateLastTab => {
                self.apply_activate_last_tab();
                Ok(())
            }
            AppAction::SetTabTitle { tab, title } => self.apply_set_tab_title(tab, &title),
            AppAction::MoveTab { index } => self.apply_move_tab(index),
            AppAction::MoveTabRelative { offset } => self.apply_move_tab_relative(offset),
            AppAction::MoveTabToNewWindow { tab } => self.apply_move_tab_to_new_window(tab),
            AppAction::RotatePanes { direction } => self.apply_rotate_panes(direction),
            action @ (AppAction::SplitPane { .. }
            | AppAction::SplitPaneWithSize { .. }
            | AppAction::SplitTopLevelPane { .. }
            | AppAction::SplitTopLevelPaneWithSize { .. }) => self.apply_split_action(action),
            AppAction::ClosePane { pane } => self.apply_close_pane(pane),
            AppAction::ActivatePane { pane } => self.apply_activate_pane(pane),
            AppAction::ActivatePaneByIndex { index } => self.apply_activate_pane_by_index(index),
            AppAction::ActivatePaneDirection { direction } => {
                self.apply_activate_pane_direction(direction)
            }
            AppAction::SwapPanes {
                active,
                selected,
                keep_focus,
            } => self.apply_swap_panes(active, selected, keep_focus),
            AppAction::MovePaneToNewTab { pane } => self.apply_move_pane_to_new_tab(pane),
            AppAction::MovePaneToNewWindow { pane } => self.apply_move_pane_to_new_window(pane),
            AppAction::ResizePane {
                pane,
                direction,
                amount,
            } => self.apply_resize_pane(pane, direction, amount),
            AppAction::SetPaneZoomState { pane, zoomed } => {
                self.apply_set_pane_zoom_state(pane, zoomed)
            }
            AppAction::TogglePaneZoom { pane } => self.apply_toggle_pane_zoom(pane),
            AppAction::SetPaneCurrentWorkingDir { pane, cwd } => {
                self.apply_set_pane_current_working_dir(pane, cwd)
            }
            AppAction::SetPaneUserVar { pane, name, value } => {
                self.apply_set_pane_user_var(pane, name, value)
            }
            AppAction::SetPaneBadgeFormat { pane, badge_format } => {
                self.apply_set_pane_badge_format(pane, badge_format)
            }
            AppAction::SetPaneHasUnseenOutput {
                pane,
                has_unseen_output,
            } => self.apply_set_pane_has_unseen_output(pane, has_unseen_output),
            AppAction::SetPaneProgress { pane, progress } => {
                self.apply_set_pane_progress(pane, progress)
            }
            AppAction::FocusNextPane => self.apply_focus_next_pane(),
            AppAction::FocusPreviousPane => self.apply_focus_previous_pane(),
            AppAction::SwitchWorkspace { workspace } => self.apply_switch_workspace(workspace),
            AppAction::SwitchWorkspaceRelative { offset } => {
                self.apply_switch_workspace_relative(offset)
            }
            AppAction::SwitchToWorkspace { name, launch } => {
                self.apply_switch_to_workspace(name, launch);
                Ok(())
            }
            AppAction::CloseWorkspace { workspace } => self.apply_close_workspace(workspace),
            AppAction::RenameWorkspace { workspace, name } => {
                self.apply_rename_workspace(workspace, name)
            }
            AppAction::NewWorkspace { name, launch } => {
                self.apply_new_workspace(name, launch);
                Ok(())
            }
        };

        if result.is_ok() && clears_active_pane_unseen_output {
            self.clear_active_pane_unseen_output();
        }

        result
    }

    fn clear_active_pane_unseen_output(&mut self) {
        let pane = self.active_pane_id();
        for workspace in &mut self.workspaces {
            for tab in &mut workspace.tabs {
                if let Some(index) = tab.pane_position(pane) {
                    tab.panes[index].set_has_unseen_output(false);
                    return;
                }
            }
        }
    }

    fn apply_multiple(&mut self, actions: Vec<AppAction>) -> Result<(), AppShellError> {
        for action in actions {
            self.apply_action(action)?;
        }
        Ok(())
    }

    fn apply_new_tab(&mut self, launch: Option<PaneLaunch>) {
        let launch = launch.unwrap_or_else(|| self.active_pane().launch.for_child_pane());
        let tab_id = self.next_tab_id();
        let pane_id = self.next_pane_id();
        let active_workspace = self
            .active_workspace_mut()
            .expect("active workspace must exist");
        active_workspace.add_tab(Tab::new(tab_id, vec![Pane::new(pane_id, launch)]));
    }

    fn apply_spawn_window(&mut self, launch: Option<PaneLaunch>) {
        let launch = launch.unwrap_or_else(|| self.active_pane().launch.for_child_pane());
        let window_id = self.next_window_id();
        let tab_id = self.next_tab_id();
        let pane_id = self.next_pane_id();
        let active_workspace = self.active_workspace();
        let workspace_id = active_workspace.id();
        let workspace_name = active_workspace.name().to_owned();
        self.pending_windows.push(PendingWindow::new(
            window_id,
            workspace_id,
            workspace_name,
            Tab::new(tab_id, vec![Pane::new(pane_id, launch)]),
        ));
    }

    fn apply_close_tab(
        &mut self,
        tab: TabId,
        switch_to_last_active: bool,
    ) -> Result<(), AppShellError> {
        let active_workspace = self
            .active_workspace_mut()
            .expect("active workspace must exist");
        let selection = if switch_to_last_active {
            CloseTabSelection::LastActive
        } else {
            CloseTabSelection::Left
        };
        active_workspace.close_tab(tab, selection)
    }

    fn apply_close_tab_with_selection(
        &mut self,
        tab: TabId,
        selection: CloseTabSelection,
    ) -> Result<(), AppShellError> {
        let active_workspace = self
            .active_workspace_mut()
            .expect("active workspace must exist");
        active_workspace.close_tab(tab, selection)
    }

    fn apply_activate_tab(&mut self, tab: TabId) -> Result<(), AppShellError> {
        let active_workspace = self
            .active_workspace_mut()
            .expect("active workspace must exist");
        active_workspace.activate_tab(tab)
    }

    fn apply_activate_tab_index(&mut self, index: isize) {
        let active_workspace = self
            .active_workspace_mut()
            .expect("active workspace must exist");
        active_workspace.activate_tab_index(index);
    }

    fn apply_activate_tab_relative(&mut self, offset: isize) -> Result<(), AppShellError> {
        let active_workspace = self
            .active_workspace_mut()
            .expect("active workspace must exist");
        active_workspace.activate_tab_relative(offset)
    }

    fn apply_activate_tab_relative_no_wrap(&mut self, offset: isize) -> Result<(), AppShellError> {
        let active_workspace = self
            .active_workspace_mut()
            .expect("active workspace must exist");
        active_workspace.activate_tab_relative_no_wrap(offset)
    }

    fn apply_activate_last_tab(&mut self) {
        let active_workspace = self
            .active_workspace_mut()
            .expect("active workspace must exist");
        active_workspace.activate_last_tab();
    }

    fn apply_set_tab_title(&mut self, tab: TabId, title: &str) -> Result<(), AppShellError> {
        let active_workspace = self
            .active_workspace_mut()
            .expect("active workspace must exist");
        active_workspace.set_tab_title(tab, title)
    }

    fn apply_move_tab(&mut self, index: usize) -> Result<(), AppShellError> {
        let active_workspace = self
            .active_workspace_mut()
            .expect("active workspace must exist");
        active_workspace.move_tab(index)
    }

    fn apply_move_tab_relative(&mut self, offset: isize) -> Result<(), AppShellError> {
        let active_workspace = self
            .active_workspace_mut()
            .expect("active workspace must exist");
        active_workspace.move_tab_relative(offset)
    }

    fn apply_move_tab_to_new_window(&mut self, tab: TabId) -> Result<(), AppShellError> {
        let window_id = self.next_window_id();
        let active_workspace = self
            .active_workspace_mut()
            .expect("active workspace must exist");
        let workspace_id = active_workspace.id();
        let workspace_name = active_workspace.name().to_owned();
        // A window host must materialize the pending owner before it can
        // retire its source window.  Keep the final tab model in the source
        // until then; the host transfers its live runtime and immediately
        // closes that source window.  Non-final tabs retain normal ownership
        // transfer semantics.
        let tab = if active_workspace.tabs.len() == 1 {
            active_workspace
                .tab(tab)
                .cloned()
                .ok_or(AppShellError::InvalidTab(tab))?
        } else {
            active_workspace.detach_tab(tab)?.tab
        };
        self.pending_windows.push(PendingWindow::new(
            window_id,
            workspace_id,
            workspace_name,
            tab,
        ));
        Ok(())
    }

    fn apply_rotate_panes(
        &mut self,
        direction: PaneRotationDirection,
    ) -> Result<(), AppShellError> {
        let active_tab = self
            .active_workspace_mut()
            .expect("active workspace must exist")
            .active_tab_mut()?;
        active_tab.rotate_panes(direction);
        Ok(())
    }

    fn apply_split_pane(
        &mut self,
        pane: PaneId,
        direction: SplitDirection,
        launch: Option<PaneLaunch>,
    ) -> Result<(), AppShellError> {
        self.apply_split_pane_with_size(pane, direction, launch, 0)
    }

    fn apply_split_action(&mut self, action: AppAction) -> Result<(), AppShellError> {
        match action {
            AppAction::SplitPane {
                pane,
                direction,
                launch,
            } => self.apply_split_pane(pane, direction, launch),
            AppAction::SplitPaneWithSize {
                pane,
                direction,
                launch,
                source_size_delta,
            } => self.apply_split_pane_with_size(pane, direction, launch, source_size_delta),
            AppAction::SplitTopLevelPane { direction, launch } => {
                self.apply_split_top_level_pane_with_size(direction, launch, 0)
            }
            AppAction::SplitTopLevelPaneWithSize {
                direction,
                launch,
                source_size_delta,
            } => self.apply_split_top_level_pane_with_size(direction, launch, source_size_delta),
            _ => unreachable!("split helper received non-split action"),
        }
    }

    fn apply_split_pane_with_size(
        &mut self,
        pane: PaneId,
        direction: SplitDirection,
        launch: Option<PaneLaunch>,
        source_size_delta: i16,
    ) -> Result<(), AppShellError> {
        let new_pane_id = self.next_pane_id();
        let active_tab = self
            .active_workspace_mut()
            .expect("active workspace must exist")
            .active_tab_mut()?;
        let launch = launch.unwrap_or_else(|| active_tab.active_pane().launch.for_child_pane());
        active_tab.split_pane(pane, new_pane_id, direction, launch, source_size_delta)
    }

    fn apply_split_top_level_pane_with_size(
        &mut self,
        direction: SplitDirection,
        launch: Option<PaneLaunch>,
        source_size_delta: i16,
    ) -> Result<(), AppShellError> {
        let new_pane_id = self.next_pane_id();
        let active_tab = self
            .active_workspace_mut()
            .expect("active workspace must exist")
            .active_tab_mut()?;
        let launch = launch.unwrap_or_else(|| active_tab.active_pane().launch.for_child_pane());
        active_tab.split_top_level_pane(new_pane_id, direction, launch, source_size_delta)
    }

    fn apply_close_pane(&mut self, pane: PaneId) -> Result<(), AppShellError> {
        let active_workspace = self
            .active_workspace_mut()
            .expect("active workspace must exist");
        let Some(tab_index) = active_workspace
            .tabs
            .iter()
            .position(|tab| tab.pane_position(pane).is_some())
        else {
            return Err(AppShellError::InvalidPane(pane));
        };
        if active_workspace.tabs[tab_index].panes().len() <= 1 {
            let tab_id = active_workspace.tabs[tab_index].id();
            return active_workspace.close_tab(tab_id, CloseTabSelection::Left);
        }

        active_workspace.tabs[tab_index].close_pane(pane)
    }

    fn apply_activate_pane(&mut self, pane: PaneId) -> Result<(), AppShellError> {
        let active_tab = self
            .active_workspace_mut()
            .expect("active workspace must exist")
            .active_tab_mut()?;
        active_tab.focus_pane(pane)
    }

    fn apply_activate_pane_by_index(&mut self, index: usize) -> Result<(), AppShellError> {
        let active_tab = self
            .active_workspace_mut()
            .expect("active workspace must exist")
            .active_tab_mut()?;
        active_tab.focus_pane_by_index(index);
        Ok(())
    }

    fn apply_activate_pane_direction(
        &mut self,
        direction: PaneDirection,
    ) -> Result<(), AppShellError> {
        let active_tab = self
            .active_workspace_mut()
            .expect("active workspace must exist")
            .active_tab_mut()?;
        active_tab.activate_pane_direction(direction);
        Ok(())
    }

    fn apply_swap_panes(
        &mut self,
        active: PaneId,
        selected: PaneId,
        keep_focus: bool,
    ) -> Result<(), AppShellError> {
        let active_tab = self
            .active_workspace_mut()
            .expect("active workspace must exist")
            .active_tab_mut()?;
        active_tab.swap_panes(active, selected, keep_focus)
    }

    fn apply_move_pane_to_new_tab(&mut self, pane: PaneId) -> Result<(), AppShellError> {
        let tab_id = self.next_tab_id();
        let active_workspace = self
            .active_workspace_mut()
            .expect("active workspace must exist");
        active_workspace.move_pane_to_new_tab(pane, tab_id)
    }

    fn apply_move_pane_to_new_window(&mut self, pane: PaneId) -> Result<(), AppShellError> {
        let window_id = self.next_window_id();
        let tab_id = self.next_tab_id();
        let active_workspace = self
            .active_workspace_mut()
            .expect("active workspace must exist");
        let pending_window = active_workspace.move_pane_to_new_window(pane, window_id, tab_id)?;
        self.pending_windows.push(pending_window);
        Ok(())
    }

    fn apply_resize_pane(
        &mut self,
        pane: PaneId,
        direction: ResizeDirection,
        amount: u16,
    ) -> Result<(), AppShellError> {
        let active_tab = self
            .active_workspace_mut()
            .expect("active workspace must exist")
            .active_tab_mut()?;
        active_tab.resize_pane(pane, direction, amount)
    }

    fn apply_toggle_pane_zoom(&mut self, pane: PaneId) -> Result<(), AppShellError> {
        let active_tab = self
            .active_workspace_mut()
            .expect("active workspace must exist")
            .active_tab_mut()?;
        active_tab.toggle_pane_zoom(pane)
    }

    fn apply_set_pane_zoom_state(
        &mut self,
        pane: PaneId,
        zoomed: bool,
    ) -> Result<(), AppShellError> {
        let active_tab = self
            .active_workspace_mut()
            .expect("active workspace must exist")
            .active_tab_mut()?;
        active_tab.set_pane_zoom_state(pane, zoomed)
    }

    fn apply_set_pane_current_working_dir(
        &mut self,
        pane: PaneId,
        cwd: Option<String>,
    ) -> Result<(), AppShellError> {
        for workspace in &mut self.workspaces {
            for tab in &mut workspace.tabs {
                if tab.pane_position(pane).is_some() {
                    return tab.set_pane_current_working_dir(pane, cwd);
                }
            }
        }

        Err(AppShellError::InvalidPane(pane))
    }

    fn apply_set_pane_user_var(
        &mut self,
        pane: PaneId,
        name: String,
        value: String,
    ) -> Result<(), AppShellError> {
        for workspace in &mut self.workspaces {
            for tab in &mut workspace.tabs {
                if tab.pane_position(pane).is_some() {
                    return tab.set_pane_user_var(pane, name, value);
                }
            }
        }

        Err(AppShellError::InvalidPane(pane))
    }

    fn apply_set_pane_badge_format(
        &mut self,
        pane: PaneId,
        badge_format: Option<String>,
    ) -> Result<(), AppShellError> {
        for workspace in &mut self.workspaces {
            for tab in &mut workspace.tabs {
                if tab.pane_position(pane).is_some() {
                    return tab.set_pane_badge_format(pane, badge_format);
                }
            }
        }

        Err(AppShellError::InvalidPane(pane))
    }

    fn apply_set_pane_progress(
        &mut self,
        pane: PaneId,
        progress: PaneProgress,
    ) -> Result<(), AppShellError> {
        for workspace in &mut self.workspaces {
            for tab in &mut workspace.tabs {
                if tab.pane_position(pane).is_some() {
                    return tab.set_pane_progress(pane, progress);
                }
            }
        }

        Err(AppShellError::InvalidPane(pane))
    }

    fn apply_set_pane_has_unseen_output(
        &mut self,
        pane: PaneId,
        has_unseen_output: bool,
    ) -> Result<(), AppShellError> {
        for workspace in &mut self.workspaces {
            for tab in &mut workspace.tabs {
                if tab.pane_position(pane).is_some() {
                    return tab.set_pane_has_unseen_output(pane, has_unseen_output);
                }
            }
        }

        Err(AppShellError::InvalidPane(pane))
    }

    fn apply_focus_next_pane(&mut self) -> Result<(), AppShellError> {
        let active_tab = self
            .active_workspace_mut()
            .expect("active workspace must exist")
            .active_tab_mut()?;
        active_tab.focus_next_pane();
        Ok(())
    }

    fn apply_focus_previous_pane(&mut self) -> Result<(), AppShellError> {
        let active_tab = self
            .active_workspace_mut()
            .expect("active workspace must exist")
            .active_tab_mut()?;
        active_tab.focus_previous_pane();
        Ok(())
    }

    fn apply_switch_workspace(&mut self, workspace: WorkspaceId) -> Result<(), AppShellError> {
        if self.workspace_position(workspace).is_some() {
            self.active_workspace_id = workspace;
            Ok(())
        } else {
            Err(AppShellError::InvalidWorkspace(workspace))
        }
    }

    fn apply_switch_workspace_relative(&mut self, offset: isize) -> Result<(), AppShellError> {
        if offset == 0 || self.workspaces.len() <= 1 {
            return Ok(());
        }

        let workspace_order = self.ordered_workspace_ids();
        let Some(current_pos) = workspace_order
            .iter()
            .position(|workspace| *workspace == self.active_workspace_id)
        else {
            return Err(AppShellError::InvalidWorkspace(self.active_workspace_id));
        };

        let len = workspace_order.len();
        let next_pos = (i128::try_from(current_pos).unwrap_or_default() + (offset as i128))
            .rem_euclid(i128::try_from(len).unwrap_or(1));
        let next_pos = usize::try_from(next_pos).unwrap_or(0);
        self.active_workspace_id = workspace_order[next_pos];
        Ok(())
    }

    fn apply_switch_to_workspace(&mut self, name: Option<String>, launch: Option<PaneLaunch>) {
        let name = name.unwrap_or_else(|| self.random_workspace_name());
        if let Some(workspace) = self
            .workspaces
            .iter()
            .find(|workspace| workspace.name() == name)
        {
            self.active_workspace_id = workspace.id();
            return;
        }

        self.apply_new_workspace(name, launch);
    }

    fn random_workspace_name(&self) -> String {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |duration| duration.as_nanos());
        let process = u128::from(std::process::id());
        let mut seed = timestamp ^ (process << 64) ^ u128::from(self.next_workspace_id);
        loop {
            let candidate = format!("random-{seed:032x}");
            if self
                .workspaces
                .iter()
                .all(|workspace| workspace.name() != candidate)
            {
                return candidate;
            }
            seed = seed.wrapping_add(0x9e37_79b9_7f4a_7c15);
        }
    }

    fn apply_close_workspace(&mut self, workspace: WorkspaceId) -> Result<(), AppShellError> {
        let Some(index) = self.workspace_position(workspace) else {
            return Err(AppShellError::InvalidWorkspace(workspace));
        };
        if self.workspaces.len() <= 1 {
            return Err(AppShellError::CannotCloseLastWorkspace);
        }

        self.workspaces.remove(index);

        if self.active_workspace_id == workspace {
            let next_index = if index == 0 {
                0
            } else {
                index.saturating_sub(1)
            };
            self.active_workspace_id = self.workspaces[next_index].id();
        }

        Ok(())
    }

    fn apply_rename_workspace(
        &mut self,
        workspace: WorkspaceId,
        name: String,
    ) -> Result<(), AppShellError> {
        let workspace = self
            .workspace_mut(workspace)
            .ok_or(AppShellError::InvalidWorkspace(workspace))?;
        workspace.name = name;
        Ok(())
    }

    fn apply_new_workspace(&mut self, name: String, launch: Option<PaneLaunch>) {
        let launch = launch.unwrap_or_else(|| self.default_launch.clone());
        let workspace_id = self.next_workspace_id();
        let tab_id = self.next_tab_id();
        let pane_id = self.next_pane_id();
        let workspace = Workspace::new(
            workspace_id,
            name,
            vec![Tab::new(tab_id, vec![Pane::new(pane_id, launch)])],
            tab_id,
        );
        self.workspaces.push(workspace);
        self.active_workspace_id = workspace_id;
    }

    fn workspace_position(&self, workspace_id: WorkspaceId) -> Option<usize> {
        self.workspaces
            .iter()
            .position(|workspace| workspace.id == workspace_id)
    }

    fn ordered_workspace_ids(&self) -> Vec<WorkspaceId> {
        let mut ordered = self.workspaces.iter().collect::<Vec<_>>();
        ordered.sort_unstable_by_key(|workspace| workspace.name());
        ordered.into_iter().map(Workspace::id).collect()
    }

    fn workspace(&self, workspace_id: WorkspaceId) -> Option<&Workspace> {
        let index = self.workspace_position(workspace_id)?;
        self.workspaces.get(index)
    }

    fn workspace_mut(&mut self, workspace_id: WorkspaceId) -> Option<&mut Workspace> {
        let index = self.workspace_position(workspace_id)?;
        self.workspaces.get_mut(index)
    }

    fn active_workspace_mut(&mut self) -> Option<&mut Workspace> {
        let active_workspace_id = self.active_workspace_id;
        self.workspace_mut(active_workspace_id)
    }

    fn next_workspace_id(&mut self) -> WorkspaceId {
        let id = WorkspaceId::new(self.next_workspace_id);
        self.next_workspace_id = self.next_workspace_id.saturating_add(1);
        id
    }

    fn next_tab_id(&mut self) -> TabId {
        let id = TabId::new(self.next_tab_id);
        self.next_tab_id = self.next_tab_id.saturating_add(1);
        id
    }

    fn next_pane_id(&mut self) -> PaneId {
        let id = PaneId::new(self.next_pane_id);
        self.next_pane_id = self.next_pane_id.saturating_add(1);
        id
    }

    fn next_window_id(&mut self) -> WindowId {
        let id = WindowId::new(self.next_window_id);
        self.next_window_id = self.next_window_id.saturating_add(1);
        id
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingWindow {
    id: WindowId,
    workspace_id: WorkspaceId,
    workspace_name: String,
    tab: Tab,
}

impl PendingWindow {
    fn new(id: WindowId, workspace_id: WorkspaceId, workspace_name: String, tab: Tab) -> Self {
        Self {
            id,
            workspace_id,
            workspace_name,
            tab,
        }
    }

    #[must_use]
    pub const fn id(&self) -> WindowId {
        self.id
    }

    #[must_use]
    pub const fn workspace_id(&self) -> WorkspaceId {
        self.workspace_id
    }

    #[must_use]
    pub fn workspace_name(&self) -> &str {
        &self.workspace_name
    }

    #[must_use]
    pub const fn tab(&self) -> &Tab {
        &self.tab
    }

    #[must_use]
    pub const fn active_tab_id(&self) -> TabId {
        self.tab.id()
    }

    #[must_use]
    pub fn active_pane_id(&self) -> PaneId {
        self.tab.active_pane_id()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Workspace {
    id: WorkspaceId,
    pub(crate) name: String,
    tabs: Vec<Tab>,
    active_tab_id: TabId,
    last_active_tab_id: Option<TabId>,
}

impl Workspace {
    fn new(id: WorkspaceId, name: String, tabs: Vec<Tab>, active_tab_id: TabId) -> Self {
        Self {
            id,
            name,
            tabs,
            active_tab_id,
            last_active_tab_id: None,
        }
    }

    #[must_use]
    pub fn id(&self) -> WorkspaceId {
        self.id
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn tabs(&self) -> &[Tab] {
        &self.tabs
    }

    fn tab(&self, tab_id: TabId) -> Option<&Tab> {
        self.tab_position(tab_id)
            .and_then(|index| self.tabs.get(index))
    }

    fn tab_position(&self, tab_id: TabId) -> Option<usize> {
        self.tabs.iter().position(|tab| tab.id == tab_id)
    }

    #[must_use]
    pub const fn active_tab_id(&self) -> TabId {
        self.active_tab_id
    }

    fn last_active_tab_id(&self) -> Option<TabId> {
        self.last_active_tab_id
    }

    fn active_tab(&self) -> &Tab {
        self.tab_position(self.active_tab_id)
            .and_then(|index| self.tabs.get(index))
            .unwrap_or_else(|| panic!("active tab missing: {:#?}", self.active_tab_id))
    }

    fn active_tab_mut(&mut self) -> Result<&mut Tab, AppShellError> {
        let index = self
            .tab_position(self.active_tab_id)
            .ok_or(AppShellError::InvalidTab(self.active_tab_id))?;
        self.tabs
            .get_mut(index)
            .ok_or(AppShellError::InvalidTab(self.active_tab_id))
    }

    fn add_tab(&mut self, tab: Tab) {
        self.tabs.push(tab);
        if let Some(tab_id) = self.tabs.last().map(Tab::id) {
            self.activate_existing_tab(tab_id);
        }
    }

    fn insert_tab(&mut self, tab: Tab, index: usize) {
        let tab_id = tab.id();
        self.tabs.insert(index.min(self.tabs.len()), tab);
        self.activate_existing_tab(tab_id);
    }

    fn detach_tab(&mut self, tab_id: TabId) -> Result<DetachedTab, AppShellError> {
        let Some(index) = self.tab_position(tab_id) else {
            return Err(AppShellError::InvalidTab(tab_id));
        };
        if self.tabs.len() <= 1 {
            return Err(AppShellError::CannotCloseLastTab);
        }

        let tab = self.tabs.remove(index);
        if self.last_active_tab_id == Some(tab_id) {
            self.last_active_tab_id = None;
        }
        if self.active_tab_id == tab_id {
            let next_index = index.min(self.tabs.len() - 1);
            self.active_tab_id = self.tabs[next_index].id();
        }
        Ok(DetachedTab {
            tab,
            source_index: index,
        })
    }

    fn move_pane_to_new_tab(
        &mut self,
        pane_id: PaneId,
        new_tab_id: TabId,
    ) -> Result<(), AppShellError> {
        let active_tab_index = self
            .tab_position(self.active_tab_id)
            .ok_or(AppShellError::InvalidTab(self.active_tab_id))?;
        let pane = self.tabs[active_tab_index].take_pane_for_new_tab(pane_id)?;
        self.add_tab(Tab::new(new_tab_id, vec![pane.with_split(None)]));
        Ok(())
    }

    fn move_pane_to_new_window(
        &mut self,
        pane_id: PaneId,
        new_window_id: WindowId,
        new_tab_id: TabId,
    ) -> Result<PendingWindow, AppShellError> {
        let active_tab_index = self
            .tab_position(self.active_tab_id)
            .ok_or(AppShellError::InvalidTab(self.active_tab_id))?;
        let pane = self.tabs[active_tab_index].take_pane_for_new_tab(pane_id)?;
        Ok(PendingWindow::new(
            new_window_id,
            self.id,
            self.name.clone(),
            Tab::new(new_tab_id, vec![pane.with_split(None)]),
        ))
    }

    fn activate_tab(&mut self, tab_id: TabId) -> Result<(), AppShellError> {
        self.tab_position(tab_id)
            .ok_or(AppShellError::InvalidTab(tab_id))?;
        self.activate_existing_tab(tab_id);
        Ok(())
    }

    fn activate_tab_index(&mut self, index: isize) {
        if self.tabs.is_empty() {
            return;
        }

        let len = i128::try_from(self.tabs.len()).unwrap_or_default();
        let index = i128::try_from(index).unwrap_or_default();
        let target = if index < 0 {
            (len + index).max(0)
        } else {
            index
        };
        if target >= len {
            return;
        }

        let target = usize::try_from(target).unwrap_or_default();
        self.activate_existing_tab(self.tabs[target].id());
    }

    fn activate_tab_relative(&mut self, offset: isize) -> Result<(), AppShellError> {
        if offset == 0 || self.tabs.is_empty() {
            return Ok(());
        }

        let active_position = self
            .tab_position(self.active_tab_id)
            .ok_or(AppShellError::InvalidTab(self.active_tab_id))?;
        let target_position = (i128::try_from(active_position).unwrap_or_default()
            + i128::try_from(offset).unwrap_or_default())
        .rem_euclid(i128::try_from(self.tabs.len()).unwrap_or(1));
        let target_position = usize::try_from(target_position).unwrap_or_default();
        self.activate_existing_tab(self.tabs[target_position].id());
        Ok(())
    }

    fn activate_tab_relative_no_wrap(&mut self, offset: isize) -> Result<(), AppShellError> {
        if offset == 0 || self.tabs.is_empty() {
            return Ok(());
        }

        let active_position = self
            .tab_position(self.active_tab_id)
            .ok_or(AppShellError::InvalidTab(self.active_tab_id))?;
        let last_position = self.tabs.len() - 1;
        let target_position = (i128::try_from(active_position).unwrap_or_default()
            + i128::try_from(offset).unwrap_or_default())
        .clamp(0, i128::try_from(last_position).unwrap_or_default());
        let target_position = usize::try_from(target_position).unwrap_or_default();
        self.activate_existing_tab(self.tabs[target_position].id());
        Ok(())
    }

    fn activate_last_tab(&mut self) {
        let Some(tab_id) = self.last_active_tab_id else {
            return;
        };
        if self.tab_position(tab_id).is_none() {
            self.last_active_tab_id = None;
            return;
        }

        self.activate_existing_tab(tab_id);
    }

    fn set_tab_title(&mut self, tab_id: TabId, title: &str) -> Result<(), AppShellError> {
        let index = self
            .tab_position(tab_id)
            .ok_or(AppShellError::InvalidTab(tab_id))?;
        self.tabs[index].set_title(title);
        Ok(())
    }

    fn move_tab(&mut self, index: usize) -> Result<(), AppShellError> {
        if index >= self.tabs.len() {
            return Err(AppShellError::InvalidTabIndex(index));
        }
        if self.tabs.len() <= 1 {
            return Ok(());
        }

        let active_position = self
            .tab_position(self.active_tab_id)
            .ok_or(AppShellError::InvalidTab(self.active_tab_id))?;
        if active_position == index {
            return Ok(());
        }

        let tab = self.tabs.remove(active_position);
        self.tabs.insert(index, tab);
        Ok(())
    }

    fn move_tab_relative(&mut self, offset: isize) -> Result<(), AppShellError> {
        if offset == 0 || self.tabs.len() <= 1 {
            return Ok(());
        }

        let active_position = self
            .tab_position(self.active_tab_id)
            .ok_or(AppShellError::InvalidTab(self.active_tab_id))?;
        let last_position = self.tabs.len() - 1;
        let target_position = (i128::try_from(active_position).unwrap_or_default()
            + i128::try_from(offset).unwrap_or_default())
        .clamp(0, i128::try_from(last_position).unwrap_or_default());
        let target_position = usize::try_from(target_position).unwrap_or_default();
        if target_position == active_position {
            return Ok(());
        }

        let tab = self.tabs.remove(active_position);
        self.tabs.insert(target_position, tab);
        Ok(())
    }

    fn close_tab(
        &mut self,
        tab_id: TabId,
        selection: CloseTabSelection,
    ) -> Result<(), AppShellError> {
        let Some(index) = self.tab_position(tab_id) else {
            return Err(AppShellError::InvalidTab(tab_id));
        };
        if self.tabs.len() <= 1 {
            return Err(AppShellError::CannotCloseLastTab);
        }

        let next_active_tab = if self.active_tab_id == tab_id {
            match selection {
                CloseTabSelection::Adjacent => self.tabs.get(index.saturating_add(1)).map(Tab::id),
                CloseTabSelection::LastActive => self
                    .last_active_tab_id
                    .filter(|last_active_tab_id| *last_active_tab_id != tab_id)
                    .filter(|last_active_tab_id| self.tab_position(*last_active_tab_id).is_some()),
                CloseTabSelection::Left => None,
            }
        } else {
            None
        };
        self.tabs.remove(index);
        if self.last_active_tab_id == Some(tab_id) {
            self.last_active_tab_id = None;
        }
        if self.active_tab_id == tab_id {
            self.active_tab_id = if let Some(next_active_tab) = next_active_tab {
                next_active_tab
            } else {
                let next_index = index.saturating_sub(1).min(self.tabs.len() - 1);
                self.tabs[next_index].id()
            };
        }
        Ok(())
    }

    fn activate_existing_tab(&mut self, tab_id: TabId) {
        if self.active_tab_id == tab_id {
            return;
        }

        self.last_active_tab_id = Some(self.active_tab_id);
        self.active_tab_id = tab_id;
    }

    fn active_pane_id(&self) -> PaneId {
        self.active_tab().active_pane_id()
    }

    fn active_pane(&self) -> &Pane {
        self.active_tab().active_pane()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tab {
    id: TabId,
    panes: Vec<Pane>,
    active_pane_id: PaneId,
    pane_activation_order: Vec<PaneId>,
    zoomed_pane_id: Option<PaneId>,
    title: Option<String>,
}

impl Tab {
    fn new(id: TabId, panes: Vec<Pane>) -> Self {
        let active_pane_id = panes
            .first()
            .map(Pane::id)
            .expect("tab requires at least one pane");
        Self {
            id,
            panes,
            active_pane_id,
            pane_activation_order: vec![active_pane_id],
            zoomed_pane_id: None,
            title: None,
        }
    }

    #[must_use]
    pub const fn id(&self) -> TabId {
        self.id
    }

    #[must_use]
    pub fn panes(&self) -> &[Pane] {
        &self.panes
    }

    #[must_use]
    pub fn panes_mut(&mut self) -> &mut Vec<Pane> {
        &mut self.panes
    }

    #[must_use]
    pub const fn zoomed_pane_id(&self) -> Option<PaneId> {
        self.zoomed_pane_id
    }

    #[must_use]
    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    fn set_title(&mut self, title: &str) {
        let title = title.trim();
        self.title = (!title.is_empty()).then(|| title.to_owned());
    }

    fn from_reconnect_snapshot(
        id: TabId,
        snapshot: TabReconnectSnapshot,
        pane_ids: &[PaneId],
    ) -> Result<Self, AppShellError> {
        if snapshot.panes.is_empty()
            || snapshot.panes.len() != pane_ids.len()
            || snapshot.active_pane_index >= pane_ids.len()
            || snapshot
                .pane_activation_order
                .iter()
                .any(|index| *index >= pane_ids.len())
            || snapshot
                .zoomed_pane_index
                .is_some_and(|index| index >= pane_ids.len())
        {
            return Err(AppShellError::InvalidTab(id));
        }

        let panes = snapshot
            .panes
            .iter()
            .enumerate()
            .map(|(index, pane)| {
                let split = pane.split.map(|split| {
                    let source_pane = *pane_ids
                        .get(split.source_pane_index)
                        .ok_or(AppShellError::InvalidTab(id))?;
                    Ok(PaneSplit {
                        source_pane,
                        direction: split.direction,
                        source_size_delta: split.source_size_delta,
                    })
                });
                let split = split.transpose()?;
                Ok(Pane::new(pane_ids[index], pane.launch.clone()).with_split(split))
            })
            .collect::<Result<Vec<_>, AppShellError>>()?;
        let active_pane_id = pane_ids[snapshot.active_pane_index];
        let pane_activation_order = snapshot
            .pane_activation_order
            .into_iter()
            .map(|index| pane_ids[index])
            .collect::<Vec<_>>();
        let zoomed_pane_id = snapshot.zoomed_pane_index.map(|index| pane_ids[index]);

        Ok(Self {
            id,
            panes,
            active_pane_id,
            pane_activation_order,
            zoomed_pane_id,
            title: snapshot.title,
        })
    }

    fn remap_ids(
        &mut self,
        tab_id: TabId,
        pane_id_map: &HashMap<PaneId, PaneId>,
    ) -> Result<(), AppShellError> {
        let remap = |pane_id| {
            pane_id_map
                .get(&pane_id)
                .copied()
                .ok_or(AppShellError::InvalidPane(pane_id))
        };
        let active_pane_id = remap(self.active_pane_id)?;
        let pane_activation_order = self
            .pane_activation_order
            .iter()
            .copied()
            .map(remap)
            .collect::<Result<Vec<_>, _>>()?;
        let zoomed_pane_id = self.zoomed_pane_id.map(remap).transpose()?;

        for pane in &mut self.panes {
            let old_id = pane.id;
            pane.id = remap(old_id)?;
            if let Some(mut split) = pane.split {
                split.source_pane = remap(split.source_pane)?;
                pane.split = Some(split);
            }
        }

        self.id = tab_id;
        self.active_pane_id = active_pane_id;
        self.pane_activation_order = pane_activation_order;
        self.zoomed_pane_id = zoomed_pane_id;
        Ok(())
    }

    fn pane_position(&self, pane_id: PaneId) -> Option<usize> {
        self.panes.iter().position(|pane| pane.id == pane_id)
    }

    #[must_use]
    pub const fn active_pane_id(&self) -> PaneId {
        self.active_pane_id
    }

    fn active_pane(&self) -> &Pane {
        self.pane_position(self.active_pane_id)
            .and_then(|index| self.panes.get(index))
            .unwrap_or_else(|| panic!("active pane missing: {:#?}", self.active_pane_id))
    }

    fn focus_pane(&mut self, pane_id: PaneId) -> Result<(), AppShellError> {
        if self.pane_position(pane_id).is_none() {
            return Err(AppShellError::InvalidPane(pane_id));
        }

        self.set_active_pane(pane_id);
        Ok(())
    }

    fn focus_pane_by_index(&mut self, index: usize) {
        if let Some(pane_id) = self.panes.get(index).map(Pane::id) {
            self.set_active_pane(pane_id);
        }
    }

    fn set_active_pane(&mut self, pane_id: PaneId) {
        let switching_panes = self.active_pane_id != pane_id;
        self.active_pane_id = pane_id;
        if let Some(index) = self.pane_position(pane_id) {
            self.panes[index].set_has_unseen_output(false);
        }
        self.record_pane_activation(pane_id);
        if switching_panes {
            self.zoomed_pane_id = None;
        }
    }

    fn record_pane_activation(&mut self, pane_id: PaneId) {
        self.pane_activation_order
            .retain(|candidate| *candidate != pane_id);
        self.pane_activation_order.push(pane_id);
    }

    fn swap_panes(
        &mut self,
        active: PaneId,
        selected: PaneId,
        keep_focus: bool,
    ) -> Result<(), AppShellError> {
        let Some(active_index) = self.pane_position(active) else {
            return Err(AppShellError::InvalidPane(active));
        };
        let Some(selected_index) = self.pane_position(selected) else {
            return Err(AppShellError::InvalidPane(selected));
        };

        if active != selected {
            let active_launch = self.panes[active_index].launch.clone();
            self.panes[active_index].id = selected;
            self.panes[active_index].launch = self.panes[selected_index].launch.clone();
            self.panes[selected_index].id = active;
            self.panes[selected_index].launch = active_launch;

            for pane in &mut self.panes {
                let Some(split) = pane.split.as_mut() else {
                    continue;
                };
                if split.source_pane == active {
                    split.source_pane = selected;
                } else if split.source_pane == selected {
                    split.source_pane = active;
                }
            }
        }

        self.set_active_pane(if keep_focus { active } else { selected });
        Ok(())
    }

    fn rotate_panes(&mut self, direction: PaneRotationDirection) {
        if self.panes.len() <= 1 {
            return;
        }

        let old_ids = self.panes.iter().map(Pane::id).collect::<Vec<_>>();
        let mut rotated_identity = self
            .panes
            .iter()
            .map(|pane| (pane.id, pane.launch.clone()))
            .collect::<Vec<_>>();
        match direction {
            PaneRotationDirection::Clockwise => rotated_identity.rotate_right(1),
            PaneRotationDirection::CounterClockwise => rotated_identity.rotate_left(1),
        }
        let rotated_ids = rotated_identity
            .iter()
            .map(|(pane_id, _)| *pane_id)
            .collect::<Vec<_>>();

        for (pane, (pane_id, launch)) in self.panes.iter_mut().zip(rotated_identity) {
            pane.id = pane_id;
            pane.launch = launch;
        }

        for pane in &mut self.panes {
            let Some(split) = pane.split.as_mut() else {
                continue;
            };
            if let Some(source_index) = old_ids
                .iter()
                .position(|pane_id| *pane_id == split.source_pane)
            {
                split.source_pane = rotated_ids[source_index];
            }
        }
    }

    fn resize_pane(
        &mut self,
        pane_id: PaneId,
        direction: ResizeDirection,
        amount: u16,
    ) -> Result<(), AppShellError> {
        if self.pane_position(pane_id).is_none() {
            return Err(AppShellError::InvalidPane(pane_id));
        }
        if amount == 0 {
            return Ok(());
        }

        let amount = i16::try_from(amount).unwrap_or(i16::MAX);
        for index in (0..self.panes.len()).rev() {
            let new_pane_id = self.panes[index].id;
            let Some(split) = self.panes[index].split else {
                continue;
            };
            let Some(delta) = split_resize_source_delta(pane_id, new_pane_id, split, direction)
            else {
                continue;
            };
            if let Some(split) = self.panes[index].split.as_mut() {
                split.source_size_delta = split.source_size_delta.saturating_add(delta * amount);
            }
            return Ok(());
        }

        Ok(())
    }

    fn preserve_split_layout_for_resize(
        &mut self,
        old_columns: u16,
        old_rows: u16,
        new_columns: u16,
        new_rows: u16,
    ) {
        let Some(first_pane) = self.panes.first() else {
            return;
        };
        let old_root = SplitLayoutSize {
            columns: i32::from(old_columns),
            rows: i32::from(old_rows),
        };
        let new_root = SplitLayoutSize {
            columns: i32::from(new_columns),
            rows: i32::from(new_rows),
        };
        let mut old_sizes = HashMap::from([(first_pane.id(), old_root)]);
        let mut new_sizes = HashMap::from([(first_pane.id(), new_root)]);

        for pane in self.panes.iter_mut().skip(1) {
            let pane_id = pane.id();
            let Some(old_split) = pane.split else {
                continue;
            };
            let Some(old_source) = old_sizes.remove(&old_split.source_pane) else {
                continue;
            };
            let Some(new_source) = new_sizes.remove(&old_split.source_pane) else {
                continue;
            };
            let (old_source_next, old_new_pane) =
                split_layout_size(old_source, old_split.direction, old_split.source_size_delta);
            let old_total = split_layout_axis_size(old_source, old_split.direction);
            let new_total = split_layout_axis_size(new_source, old_split.direction);
            let source_size_delta = if old_total == new_total {
                old_split.source_size_delta
            } else {
                preserve_split_source_size_delta(old_total, old_split.source_size_delta, new_total)
            };
            let new_split = PaneSplit {
                source_size_delta,
                ..old_split
            };
            let (new_source_next, new_new_pane) =
                split_layout_size(new_source, new_split.direction, source_size_delta);

            pane.split = Some(new_split);
            old_sizes.insert(old_split.source_pane, old_source_next);
            old_sizes.insert(pane_id, old_new_pane);
            new_sizes.insert(new_split.source_pane, new_source_next);
            new_sizes.insert(pane_id, new_new_pane);
        }
    }

    fn toggle_pane_zoom(&mut self, pane_id: PaneId) -> Result<(), AppShellError> {
        if self.pane_position(pane_id).is_none() {
            return Err(AppShellError::InvalidPane(pane_id));
        }

        let was_zoomed = self.zoomed_pane_id == Some(pane_id);
        self.set_active_pane(pane_id);
        self.zoomed_pane_id = (!was_zoomed).then_some(pane_id);
        Ok(())
    }

    fn set_pane_zoom_state(&mut self, pane_id: PaneId, zoomed: bool) -> Result<(), AppShellError> {
        if self.pane_position(pane_id).is_none() {
            return Err(AppShellError::InvalidPane(pane_id));
        }

        self.set_active_pane(pane_id);
        self.zoomed_pane_id = zoomed.then_some(pane_id);
        Ok(())
    }

    fn set_pane_current_working_dir(
        &mut self,
        pane_id: PaneId,
        cwd: Option<String>,
    ) -> Result<(), AppShellError> {
        let Some(index) = self.pane_position(pane_id) else {
            return Err(AppShellError::InvalidPane(pane_id));
        };
        self.panes[index].launch.set_cwd(cwd);
        Ok(())
    }

    fn set_pane_user_var(
        &mut self,
        pane_id: PaneId,
        name: String,
        value: String,
    ) -> Result<(), AppShellError> {
        let Some(index) = self.pane_position(pane_id) else {
            return Err(AppShellError::InvalidPane(pane_id));
        };
        self.panes[index].set_user_var(name, value);
        Ok(())
    }

    fn set_pane_badge_format(
        &mut self,
        pane_id: PaneId,
        badge_format: Option<String>,
    ) -> Result<(), AppShellError> {
        let Some(index) = self.pane_position(pane_id) else {
            return Err(AppShellError::InvalidPane(pane_id));
        };
        self.panes[index].set_badge_format(badge_format);
        Ok(())
    }

    fn set_pane_progress(
        &mut self,
        pane_id: PaneId,
        progress: PaneProgress,
    ) -> Result<(), AppShellError> {
        let Some(index) = self.pane_position(pane_id) else {
            return Err(AppShellError::InvalidPane(pane_id));
        };
        self.panes[index].set_progress(progress);
        Ok(())
    }

    fn set_pane_has_unseen_output(
        &mut self,
        pane_id: PaneId,
        has_unseen_output: bool,
    ) -> Result<(), AppShellError> {
        let Some(index) = self.pane_position(pane_id) else {
            return Err(AppShellError::InvalidPane(pane_id));
        };
        self.panes[index].set_has_unseen_output(has_unseen_output);
        Ok(())
    }

    fn focus_next_pane(&mut self) {
        if self.panes.len() <= 1 {
            return;
        }
        let index = self.pane_position(self.active_pane_id).unwrap_or(0);
        let next = (index + 1) % self.panes.len();
        self.set_active_pane(self.panes[next].id());
    }

    fn focus_previous_pane(&mut self) {
        if self.panes.len() <= 1 {
            return;
        }
        let index = self.pane_position(self.active_pane_id).unwrap_or(0);
        let previous = (index + self.panes.len() - 1) % self.panes.len();
        self.set_active_pane(self.panes[previous].id());
    }

    fn activate_pane_direction(&mut self, direction: PaneDirection) {
        match direction {
            PaneDirection::Next => self.focus_next_pane(),
            PaneDirection::Previous => self.focus_previous_pane(),
            PaneDirection::Left
            | PaneDirection::Right
            | PaneDirection::Up
            | PaneDirection::Down => {
                if let Some(pane_id) = self.pane_id_in_direction(direction) {
                    self.set_active_pane(pane_id);
                }
            }
        }
    }

    fn split_pane(
        &mut self,
        source: PaneId,
        new_pane_id: PaneId,
        direction: SplitDirection,
        launch: PaneLaunch,
        source_size_delta: i16,
    ) -> Result<(), AppShellError> {
        if self.pane_position(source).is_none() {
            return Err(AppShellError::InvalidPane(source));
        }

        self.panes
            .push(Pane::new(new_pane_id, launch).with_split(Some(PaneSplit {
                source_pane: source,
                direction,
                source_size_delta,
            })));
        self.set_active_pane(new_pane_id);
        Ok(())
    }

    fn split_top_level_pane(
        &mut self,
        new_pane_id: PaneId,
        direction: SplitDirection,
        launch: PaneLaunch,
        source_size_delta: i16,
    ) -> Result<(), AppShellError> {
        let Some(source) = self.panes.first().map(Pane::id) else {
            return Err(AppShellError::CannotCloseLastPane);
        };

        self.panes.insert(
            1,
            Pane::new(new_pane_id, launch).with_split(Some(PaneSplit {
                source_pane: source,
                direction,
                source_size_delta,
            })),
        );
        self.set_active_pane(new_pane_id);
        Ok(())
    }

    fn close_pane(&mut self, pane_id: PaneId) -> Result<(), AppShellError> {
        let Some(index) = self.pane_position(pane_id) else {
            return Err(AppShellError::InvalidPane(pane_id));
        };
        if self.panes.len() <= 1 {
            return Err(AppShellError::CannotCloseLastPane);
        }

        self.panes.remove(index);
        self.pane_activation_order
            .retain(|candidate| *candidate != pane_id);
        if self.zoomed_pane_id == Some(pane_id) {
            self.zoomed_pane_id = None;
        }
        if self.active_pane_id == pane_id {
            let next_index = index.saturating_sub(1).min(self.panes.len() - 1);
            self.set_active_pane(self.panes[next_index].id());
        }
        Ok(())
    }

    fn take_pane_for_new_tab(&mut self, pane_id: PaneId) -> Result<Pane, AppShellError> {
        let Some(index) = self.pane_position(pane_id) else {
            return Err(AppShellError::InvalidPane(pane_id));
        };
        if self.panes.len() <= 1 {
            return Err(AppShellError::CannotCloseLastPane);
        }

        let pane = self.panes.remove(index);
        self.pane_activation_order
            .retain(|candidate| *candidate != pane_id);
        if self.zoomed_pane_id == Some(pane_id) {
            self.zoomed_pane_id = None;
        }
        if self.active_pane_id == pane_id {
            let next_index = index.saturating_sub(1).min(self.panes.len() - 1);
            self.set_active_pane(self.panes[next_index].id());
        }
        self.normalize_splits_after_pane_removal();
        Ok(pane)
    }

    fn normalize_splits_after_pane_removal(&mut self) {
        let Some(first) = self.panes.first_mut() else {
            return;
        };
        first.split = None;
        let mut valid_sources = vec![first.id()];

        for pane in self.panes.iter_mut().skip(1) {
            if let Some(split) = pane.split.as_mut()
                && !valid_sources.contains(&split.source_pane)
            {
                split.source_pane = valid_sources[0];
            }
            valid_sources.push(pane.id());
        }
    }

    fn pane_id_in_direction(&self, direction: PaneDirection) -> Option<PaneId> {
        let rects = self.pane_direction_layout_rects();
        let active = rects
            .iter()
            .find(|rect| rect.pane_id == self.active_pane_id)
            .copied()?;
        let mut candidates = rects
            .iter()
            .enumerate()
            .filter_map(|(index, rect)| {
                if rect.pane_id == active.pane_id {
                    return None;
                }
                pane_direction_candidate(active, *rect, direction).map(|(distance, overlap)| {
                    PaneDirectionCandidate {
                        pane_id: rect.pane_id,
                        distance,
                        overlap,
                        recency: self.pane_activation_recency(rect.pane_id),
                        index,
                    }
                })
            })
            .collect::<Vec<_>>();
        candidates.sort_unstable_by(|left, right| {
            left.distance
                .cmp(&right.distance)
                .then_with(|| right.recency.cmp(&left.recency))
                .then_with(|| right.overlap.cmp(&left.overlap))
                .then_with(|| left.index.cmp(&right.index))
        });
        candidates.first().map(|candidate| candidate.pane_id)
    }

    fn pane_activation_recency(&self, pane_id: PaneId) -> usize {
        self.pane_activation_order
            .iter()
            .position(|candidate| *candidate == pane_id)
            .map_or(0, |index| index + 1)
    }

    fn pane_direction_layout_rects(&self) -> Vec<PaneDirectionRect> {
        let Some(first_pane) = self.panes.first() else {
            return Vec::new();
        };

        let first_rect = PaneDirectionRect {
            pane_id: first_pane.id(),
            row: 0,
            column: 0,
            rows: PANE_DIRECTION_LAYOUT_ROWS,
            columns: PANE_DIRECTION_LAYOUT_COLUMNS,
        };
        let mut rects = vec![(first_pane.id(), first_rect)];

        for pane in self.panes.iter().skip(1) {
            let Some(split) = pane.split() else {
                continue;
            };
            let Some(source_index) = rects
                .iter()
                .position(|(pane_id, _)| *pane_id == split.source_pane)
            else {
                continue;
            };
            let source_rect = rects[source_index].1;
            let Some((next_source, new_rect)) =
                split_pane_direction_rect(source_rect, pane.id(), split)
            else {
                continue;
            };
            rects[source_index].1 = next_source;
            rects.push((pane.id(), new_rect));
        }

        self.panes
            .iter()
            .filter_map(|pane| {
                rects
                    .iter()
                    .find(|(pane_id, _)| *pane_id == pane.id())
                    .map(|(_, rect)| *rect)
            })
            .collect()
    }
}

impl TabReconnectSnapshot {
    fn from_tab(tab: &Tab) -> Self {
        let pane_indices = tab
            .panes
            .iter()
            .enumerate()
            .map(|(index, pane)| (pane.id, index))
            .collect::<HashMap<_, _>>();
        let pane_index = |pane_id| {
            *pane_indices
                .get(&pane_id)
                .expect("tab pane references remain internally valid")
        };
        let panes = tab
            .panes
            .iter()
            .map(|pane| PaneReconnectSnapshot {
                launch: pane.launch.clone(),
                split: pane.split.map(|split| PaneReconnectSplit {
                    source_pane_index: pane_index(split.source_pane),
                    direction: split.direction,
                    source_size_delta: split.source_size_delta,
                }),
            })
            .collect();

        Self {
            title: tab.title.clone(),
            panes,
            active_pane_index: pane_index(tab.active_pane_id),
            pane_activation_order: tab
                .pane_activation_order
                .iter()
                .copied()
                .map(pane_index)
                .collect(),
            zoomed_pane_index: tab.zoomed_pane_id.map(pane_index),
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct PaneDirectionCandidate {
    pane_id: PaneId,
    distance: i32,
    overlap: i32,
    recency: usize,
    index: usize,
}

#[derive(Debug, Clone, Copy)]
struct PaneDirectionRect {
    pane_id: PaneId,
    row: i32,
    column: i32,
    rows: i32,
    columns: i32,
}

impl PaneDirectionRect {
    const fn right(self) -> i32 {
        self.column + self.columns
    }

    const fn bottom(self) -> i32 {
        self.row + self.rows
    }
}

fn pane_direction_candidate(
    active: PaneDirectionRect,
    candidate: PaneDirectionRect,
    direction: PaneDirection,
) -> Option<(i32, i32)> {
    match direction {
        PaneDirection::Left => {
            let distance = active.column - candidate.right();
            let overlap = range_overlap(
                active.row,
                active.bottom(),
                candidate.row,
                candidate.bottom(),
            );
            (distance >= 0 && overlap > 0).then_some((distance, overlap))
        }
        PaneDirection::Right => {
            let distance = candidate.column - active.right();
            let overlap = range_overlap(
                active.row,
                active.bottom(),
                candidate.row,
                candidate.bottom(),
            );
            (distance >= 0 && overlap > 0).then_some((distance, overlap))
        }
        PaneDirection::Up => {
            let distance = active.row - candidate.bottom();
            let overlap = range_overlap(
                active.column,
                active.right(),
                candidate.column,
                candidate.right(),
            );
            (distance >= 0 && overlap > 0).then_some((distance, overlap))
        }
        PaneDirection::Down => {
            let distance = candidate.row - active.bottom();
            let overlap = range_overlap(
                active.column,
                active.right(),
                candidate.column,
                candidate.right(),
            );
            (distance >= 0 && overlap > 0).then_some((distance, overlap))
        }
        PaneDirection::Next | PaneDirection::Previous => None,
    }
}

fn range_overlap(first_start: i32, first_end: i32, second_start: i32, second_end: i32) -> i32 {
    first_end
        .min(second_end)
        .saturating_sub(first_start.max(second_start))
}

fn split_pane_direction_rect(
    source: PaneDirectionRect,
    new_pane_id: PaneId,
    split: PaneSplit,
) -> Option<(PaneDirectionRect, PaneDirectionRect)> {
    match split.direction {
        SplitDirection::Right => split_pane_direction_rect_right(source, new_pane_id, split),
        SplitDirection::Left => split_pane_direction_rect_left(source, new_pane_id, split),
        SplitDirection::Down => split_pane_direction_rect_down(source, new_pane_id, split),
        SplitDirection::Up => split_pane_direction_rect_up(source, new_pane_id, split),
    }
}

fn split_pane_direction_rect_right(
    source: PaneDirectionRect,
    new_pane_id: PaneId,
    split: PaneSplit,
) -> Option<(PaneDirectionRect, PaneDirectionRect)> {
    let (source_columns, new_columns) =
        split_pane_direction_columns(source.columns, split.source_size_delta, source.rows)?;
    Some((
        PaneDirectionRect {
            columns: source_columns,
            ..source
        },
        PaneDirectionRect {
            pane_id: new_pane_id,
            row: source.row,
            column: source.column + source_columns + 1,
            rows: source.rows,
            columns: new_columns,
        },
    ))
}

fn split_pane_direction_rect_left(
    source: PaneDirectionRect,
    new_pane_id: PaneId,
    split: PaneSplit,
) -> Option<(PaneDirectionRect, PaneDirectionRect)> {
    let (source_columns, new_columns) =
        split_pane_direction_columns(source.columns, split.source_size_delta, source.rows)?;
    Some((
        PaneDirectionRect {
            column: source.column + new_columns + 1,
            columns: source_columns,
            ..source
        },
        PaneDirectionRect {
            pane_id: new_pane_id,
            row: source.row,
            column: source.column,
            rows: source.rows,
            columns: new_columns,
        },
    ))
}

fn split_pane_direction_rect_down(
    source: PaneDirectionRect,
    new_pane_id: PaneId,
    split: PaneSplit,
) -> Option<(PaneDirectionRect, PaneDirectionRect)> {
    let (source_rows, new_rows) =
        split_pane_direction_rows(source.rows, split.source_size_delta, source.columns)?;
    Some((
        PaneDirectionRect {
            rows: source_rows,
            ..source
        },
        PaneDirectionRect {
            pane_id: new_pane_id,
            row: source.row + source_rows + 1,
            column: source.column,
            rows: new_rows,
            columns: source.columns,
        },
    ))
}

fn split_pane_direction_rect_up(
    source: PaneDirectionRect,
    new_pane_id: PaneId,
    split: PaneSplit,
) -> Option<(PaneDirectionRect, PaneDirectionRect)> {
    let (source_rows, new_rows) =
        split_pane_direction_rows(source.rows, split.source_size_delta, source.columns)?;
    Some((
        PaneDirectionRect {
            row: source.row + new_rows + 1,
            rows: source_rows,
            ..source
        },
        PaneDirectionRect {
            pane_id: new_pane_id,
            row: source.row,
            column: source.column,
            rows: new_rows,
            columns: source.columns,
        },
    ))
}

fn split_pane_direction_columns(
    total_columns: i32,
    source_size_delta: i16,
    rows: i32,
) -> Option<(i32, i32)> {
    if total_columns < 3 || rows <= 0 {
        return None;
    }
    split_pane_direction_sizes(total_columns, source_size_delta)
}

fn split_pane_direction_rows(
    total_rows: i32,
    source_size_delta: i16,
    columns: i32,
) -> Option<(i32, i32)> {
    if total_rows < 3 || columns <= 0 {
        return None;
    }
    split_pane_direction_sizes(total_rows, source_size_delta)
}

fn split_pane_direction_sizes(total_cells: i32, source_size_delta: i16) -> Option<(i32, i32)> {
    let source_cells = adjusted_direction_source_size(
        total_cells,
        total_cells.saturating_sub(1) / 2,
        source_size_delta,
    );
    let new_cells = total_cells.saturating_sub(source_cells).saturating_sub(1);
    (source_cells > 0 && new_cells > 0).then_some((source_cells, new_cells))
}

fn adjusted_direction_source_size(total_cells: i32, default_source_cells: i32, delta: i16) -> i32 {
    let max_source_cells = total_cells.saturating_sub(2).max(1);
    let adjusted = default_source_cells + i32::from(delta);
    adjusted.clamp(1, max_source_cells)
}

#[derive(Debug, Clone, Copy)]
struct SplitLayoutSize {
    columns: i32,
    rows: i32,
}

fn split_layout_axis_size(size: SplitLayoutSize, direction: SplitDirection) -> i32 {
    match direction {
        SplitDirection::Left | SplitDirection::Right => size.columns,
        SplitDirection::Up | SplitDirection::Down => size.rows,
    }
}

fn split_layout_size(
    source: SplitLayoutSize,
    direction: SplitDirection,
    source_size_delta: i16,
) -> (SplitLayoutSize, SplitLayoutSize) {
    let total = split_layout_axis_size(source, direction).max(3);
    let (source_cells, new_cells) = split_pane_direction_sizes(total, source_size_delta)
        .expect("a normalized split span always fits two panes and a separator");
    match direction {
        SplitDirection::Right | SplitDirection::Left => (
            SplitLayoutSize {
                columns: source_cells,
                ..source
            },
            SplitLayoutSize {
                columns: new_cells,
                ..source
            },
        ),
        SplitDirection::Down | SplitDirection::Up => (
            SplitLayoutSize {
                rows: source_cells,
                ..source
            },
            SplitLayoutSize {
                rows: new_cells,
                ..source
            },
        ),
    }
}

fn preserve_split_source_size_delta(
    old_total_cells: i32,
    old_source_size_delta: i16,
    new_total_cells: i32,
) -> i16 {
    // A valid split needs two pane cells and one separator. Treat smaller
    // spans as the fully clamped 1:1 layout so growing again has a stable
    // current-layout baseline.
    let old_total_cells = old_total_cells.max(3);
    let new_total_cells = new_total_cells.max(3);
    let (old_source_cells, _) = split_pane_direction_sizes(old_total_cells, old_source_size_delta)
        .expect("a normalized split span always fits");
    let old_usable_cells = old_total_cells - 1;
    let new_usable_cells = new_total_cells - 1;
    // Round to the nearest cell; rendering may therefore differ from the
    // exact rational ratio by at most one cell.
    let scaled_source_cells = (i64::from(old_source_cells) * i64::from(new_usable_cells)
        + i64::from(old_usable_cells) / 2)
        / i64::from(old_usable_cells);
    let source_cells = i32::try_from(scaled_source_cells)
        .unwrap_or(i32::MAX)
        .clamp(1, new_usable_cells - 1);
    let default_source_cells = new_usable_cells / 2;
    i16::try_from(source_cells - default_source_cells).unwrap_or({
        if source_cells < default_source_cells {
            i16::MIN
        } else {
            i16::MAX
        }
    })
}

fn split_resize_source_delta(
    pane_id: PaneId,
    new_pane_id: PaneId,
    split: PaneSplit,
    direction: ResizeDirection,
) -> Option<i16> {
    match (
        split.direction,
        pane_id == split.source_pane,
        pane_id == new_pane_id,
    ) {
        (SplitDirection::Right, true, false) => match direction {
            ResizeDirection::Right => Some(1),
            ResizeDirection::Left => Some(-1),
            ResizeDirection::Up | ResizeDirection::Down => None,
        },
        (SplitDirection::Right, false, true) => match direction {
            ResizeDirection::Left => Some(-1),
            ResizeDirection::Right => Some(1),
            ResizeDirection::Up | ResizeDirection::Down => None,
        },
        (SplitDirection::Left, true, false) => match direction {
            ResizeDirection::Left => Some(1),
            ResizeDirection::Right => Some(-1),
            ResizeDirection::Up | ResizeDirection::Down => None,
        },
        (SplitDirection::Left, false, true) => match direction {
            ResizeDirection::Right => Some(-1),
            ResizeDirection::Left => Some(1),
            ResizeDirection::Up | ResizeDirection::Down => None,
        },
        (SplitDirection::Down, true, false) => match direction {
            ResizeDirection::Down => Some(1),
            ResizeDirection::Up => Some(-1),
            ResizeDirection::Left | ResizeDirection::Right => None,
        },
        (SplitDirection::Down, false, true) => match direction {
            ResizeDirection::Up => Some(-1),
            ResizeDirection::Down => Some(1),
            ResizeDirection::Left | ResizeDirection::Right => None,
        },
        (SplitDirection::Up, true, false) => match direction {
            ResizeDirection::Up => Some(1),
            ResizeDirection::Down => Some(-1),
            ResizeDirection::Left | ResizeDirection::Right => None,
        },
        (SplitDirection::Up, false, true) => match direction {
            ResizeDirection::Down => Some(-1),
            ResizeDirection::Up => Some(1),
            ResizeDirection::Left | ResizeDirection::Right => None,
        },
        _ => None,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pane {
    id: PaneId,
    launch: PaneLaunch,
    user_vars: HashMap<String, String>,
    badge_format: Option<String>,
    progress: PaneProgress,
    has_unseen_output: bool,
    split: Option<PaneSplit>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum PaneProgress {
    #[default]
    None,
    Percentage(u8),
    Error(u8),
    Indeterminate,
}

impl Pane {
    fn new(id: PaneId, launch: PaneLaunch) -> Self {
        Self {
            id,
            launch,
            user_vars: HashMap::new(),
            badge_format: None,
            progress: PaneProgress::default(),
            has_unseen_output: false,
            split: None,
        }
    }

    fn with_split(mut self, split: Option<PaneSplit>) -> Self {
        self.split = split;
        self
    }

    #[must_use]
    pub const fn id(&self) -> PaneId {
        self.id
    }

    #[must_use]
    pub fn launch(&self) -> &PaneLaunch {
        &self.launch
    }

    #[must_use]
    pub fn user_vars(&self) -> &HashMap<String, String> {
        &self.user_vars
    }

    fn set_user_var(&mut self, name: String, value: String) {
        self.user_vars.insert(name, value);
    }

    #[must_use]
    pub fn badge_format(&self) -> Option<&str> {
        self.badge_format.as_deref()
    }

    fn set_badge_format(&mut self, badge_format: Option<String>) {
        self.badge_format = badge_format;
    }

    #[must_use]
    pub const fn progress(&self) -> PaneProgress {
        self.progress
    }

    fn set_progress(&mut self, progress: PaneProgress) {
        self.progress = progress;
    }

    #[must_use]
    pub const fn has_unseen_output(&self) -> bool {
        self.has_unseen_output
    }

    fn set_has_unseen_output(&mut self, has_unseen_output: bool) {
        self.has_unseen_output = has_unseen_output;
    }

    fn reset_runtime_projection(&mut self) {
        self.user_vars.clear();
        self.badge_format = None;
        self.progress = PaneProgress::None;
        self.has_unseen_output = false;
    }

    #[must_use]
    pub const fn split(&self) -> Option<PaneSplit> {
        self.split
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PaneSplit {
    pub source_pane: PaneId,
    pub direction: SplitDirection,
    pub source_size_delta: i16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitDirection {
    Left,
    Right,
    Up,
    Down,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResizeDirection {
    Left,
    Right,
    Up,
    Down,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaneDirection {
    Left,
    Right,
    Up,
    Down,
    Next,
    Previous,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaneRotationDirection {
    Clockwise,
    CounterClockwise,
}

#[derive(Debug, Clone)]
pub enum AppAction {
    Nop,
    Multiple {
        actions: Vec<AppAction>,
    },
    NewTab {
        launch: Option<PaneLaunch>,
    },
    SpawnWindow {
        launch: Option<PaneLaunch>,
    },
    CloseTab {
        tab: TabId,
        switch_to_last_active: bool,
    },
    CloseTabWithSelection {
        tab: TabId,
        selection: CloseTabSelection,
    },
    ActivateTab {
        tab: TabId,
    },
    ActivateTabIndex {
        index: isize,
    },
    ActivateTabRelative {
        offset: isize,
    },
    ActivateTabRelativeNoWrap {
        offset: isize,
    },
    ActivateLastTab,
    SetTabTitle {
        tab: TabId,
        title: String,
    },
    MoveTab {
        index: usize,
    },
    MoveTabRelative {
        offset: isize,
    },
    MoveTabToNewWindow {
        tab: TabId,
    },
    RotatePanes {
        direction: PaneRotationDirection,
    },
    SplitPane {
        pane: PaneId,
        direction: SplitDirection,
        launch: Option<PaneLaunch>,
    },
    SplitPaneWithSize {
        pane: PaneId,
        direction: SplitDirection,
        launch: Option<PaneLaunch>,
        source_size_delta: i16,
    },
    SplitTopLevelPane {
        direction: SplitDirection,
        launch: Option<PaneLaunch>,
    },
    SplitTopLevelPaneWithSize {
        direction: SplitDirection,
        launch: Option<PaneLaunch>,
        source_size_delta: i16,
    },
    ClosePane {
        pane: PaneId,
    },
    ActivatePane {
        pane: PaneId,
    },
    ActivatePaneByIndex {
        index: usize,
    },
    ActivatePaneDirection {
        direction: PaneDirection,
    },
    SwapPanes {
        active: PaneId,
        selected: PaneId,
        keep_focus: bool,
    },
    MovePaneToNewTab {
        pane: PaneId,
    },
    MovePaneToNewWindow {
        pane: PaneId,
    },
    ResizePane {
        pane: PaneId,
        direction: ResizeDirection,
        amount: u16,
    },
    SetPaneZoomState {
        pane: PaneId,
        zoomed: bool,
    },
    TogglePaneZoom {
        pane: PaneId,
    },
    SetPaneCurrentWorkingDir {
        pane: PaneId,
        cwd: Option<String>,
    },
    SetPaneUserVar {
        pane: PaneId,
        name: String,
        value: String,
    },
    SetPaneBadgeFormat {
        pane: PaneId,
        badge_format: Option<String>,
    },
    SetPaneHasUnseenOutput {
        pane: PaneId,
        has_unseen_output: bool,
    },
    SetPaneProgress {
        pane: PaneId,
        progress: PaneProgress,
    },
    FocusNextPane,
    FocusPreviousPane,
    SwitchWorkspace {
        workspace: WorkspaceId,
    },
    SwitchWorkspaceRelative {
        offset: isize,
    },
    SwitchToWorkspace {
        name: Option<String>,
        launch: Option<PaneLaunch>,
    },
    CloseWorkspace {
        workspace: WorkspaceId,
    },
    RenameWorkspace {
        workspace: WorkspaceId,
        name: String,
    },
    NewWorkspace {
        name: String,
        launch: Option<PaneLaunch>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppShellError {
    InvalidWorkspace(WorkspaceId),
    InvalidTab(TabId),
    InvalidTabIndex(usize),
    InvalidPane(PaneId),
    CannotCloseLastTab,
    CannotCloseLastPane,
    CannotCloseLastWorkspace,
    UnsupportedAction,
}

#[cfg(test)]
mod tests {
    use crate::{PaneId, TabId, WindowId, WorkspaceId};

    use super::*;

    fn pane_layout_rects(tab: &Tab, columns: u16, rows: u16) -> HashMap<PaneId, PaneDirectionRect> {
        let first_pane = tab.panes().first().expect("tab has a root pane");
        let mut rects = HashMap::from([(
            first_pane.id(),
            PaneDirectionRect {
                pane_id: first_pane.id(),
                row: 0,
                column: 0,
                rows: i32::from(rows),
                columns: i32::from(columns),
            },
        )]);
        for pane in tab.panes().iter().skip(1) {
            let split = pane.split().expect("non-root pane has split metadata");
            let source = rects
                .get(&split.source_pane)
                .copied()
                .expect("split source has a layout rect");
            let (next_source, new_pane) =
                split_pane_direction_rect(source, pane.id(), split).expect("split fits");
            rects.insert(split.source_pane, next_source);
            rects.insert(pane.id(), new_pane);
        }
        rects
    }

    fn assert_ratio_within_one_cell(
        old_source: i32,
        old_other: i32,
        new_source: i32,
        new_other: i32,
    ) {
        let old_usable = old_source + old_other;
        let new_usable = new_source + new_other;
        let expected = (old_source * new_usable + old_usable / 2).div_euclid(old_usable);
        assert!(
            new_source.abs_diff(expected) <= 1,
            "ratio changed from {old_source}/{old_usable} to {new_source}/{new_usable}, expected {expected}±1 cells"
        );
    }

    #[test]
    fn app_shell_starts_with_default_workspace_tab_and_pane() {
        let shell = AppShell::new(PaneLaunch::local("pwsh"));

        assert_eq!(shell.active_workspace_id(), WorkspaceId::new(1));
        assert_eq!(shell.active_tab_id(), TabId::new(1));
        assert_eq!(shell.active_pane_id(), PaneId::new(1));
        assert_eq!(shell.workspaces().len(), 1);
        assert_eq!(shell.active_workspace().tabs().len(), 1);
    }

    #[test]
    fn active_pane_exposes_local_launch_command() {
        let shell = AppShell::new(PaneLaunch::local("pwsh").with_args(["-NoLogo"]));

        assert_eq!(shell.active_pane().launch().program(), "pwsh");
        assert_eq!(shell.active_pane().launch().args(), ["-NoLogo"]);
    }

    #[test]
    fn active_pane_exposes_launch_current_working_dir() {
        let shell = AppShell::new(PaneLaunch::local("pwsh").with_cwd("file://host/home/ops"));

        assert_eq!(
            shell.active_pane().launch().cwd(),
            Some("file://host/home/ops")
        );
    }

    #[test]
    fn action_new_tab_creates_and_selects_tab() {
        let mut shell = AppShell::new(PaneLaunch::local("pwsh"));

        shell
            .apply_action(AppAction::NewTab { launch: None })
            .unwrap();

        assert_eq!(shell.active_tab_id(), TabId::new(2));
        assert_eq!(shell.active_workspace().tabs().len(), 2);
        assert_eq!(shell.active_pane().launch().program(), "pwsh");
    }

    #[test]
    fn action_multiple_applies_actions_in_sequence() {
        let mut shell = AppShell::new(PaneLaunch::local("pwsh"));

        shell
            .apply_action(AppAction::Multiple {
                actions: vec![
                    AppAction::NewTab { launch: None },
                    AppAction::SetTabTitle {
                        tab: TabId::new(2),
                        title: "build".to_owned(),
                    },
                    AppAction::NewWorkspace {
                        name: "ops".to_owned(),
                        launch: None,
                    },
                ],
            })
            .unwrap();

        assert_eq!(shell.workspaces().len(), 2);
        assert_eq!(shell.active_workspace().name(), "ops");
        assert_eq!(
            shell.workspace(WorkspaceId::new(1)).unwrap().tabs()[1].title(),
            Some("build")
        );
    }

    #[test]
    fn action_nop_leaves_shell_state_unchanged() {
        let mut shell = AppShell::new(PaneLaunch::local("pwsh"));
        shell
            .active_workspace_mut()
            .unwrap()
            .active_tab_mut()
            .unwrap()
            .set_pane_has_unseen_output(PaneId::new(1), true)
            .unwrap();

        shell.apply_action(AppAction::Nop).unwrap();

        assert_eq!(shell.active_workspace_id(), WorkspaceId::new(1));
        assert_eq!(shell.active_tab_id(), TabId::new(1));
        assert_eq!(shell.active_pane_id(), PaneId::new(1));
        assert_eq!(shell.workspaces().len(), 1);
        assert_eq!(shell.active_workspace().tabs().len(), 1);
        assert!(shell.active_pane().has_unseen_output());
    }

    #[test]
    fn action_set_pane_has_unseen_output_preserves_requested_active_value() {
        let mut shell = AppShell::new(PaneLaunch::local("pwsh"));

        shell
            .apply_action(AppAction::SetPaneHasUnseenOutput {
                pane: PaneId::new(1),
                has_unseen_output: true,
            })
            .unwrap();

        assert!(shell.active_pane().has_unseen_output());
    }

    #[test]
    fn action_new_tab_inherits_active_pane_launch_cwd() {
        let mut shell = AppShell::new(PaneLaunch::local("pwsh").with_cwd("file://host/home/ops"));

        shell
            .apply_action(AppAction::NewTab { launch: None })
            .unwrap();

        assert_eq!(
            shell.active_pane().launch().cwd(),
            Some("file://host/home/ops")
        );
    }

    #[test]
    fn action_set_pane_current_working_dir_updates_launch_metadata() {
        let mut shell = AppShell::new(PaneLaunch::local("pwsh"));

        shell
            .apply_action(AppAction::SetPaneCurrentWorkingDir {
                pane: PaneId::new(1),
                cwd: Some("file://host/home/ops".to_owned()),
            })
            .unwrap();

        assert_eq!(
            shell.active_pane().launch().cwd(),
            Some("file://host/home/ops")
        );
    }

    #[test]
    fn action_set_pane_current_working_dir_updates_inactive_tab_pane() {
        let mut shell = AppShell::new(PaneLaunch::local("pwsh"));
        shell
            .apply_action(AppAction::NewTab { launch: None })
            .unwrap();
        shell
            .apply_action(AppAction::ActivateTab { tab: TabId::new(1) })
            .unwrap();

        shell
            .apply_action(AppAction::SetPaneCurrentWorkingDir {
                pane: PaneId::new(2),
                cwd: Some("file://host/home/ops".to_owned()),
            })
            .unwrap();

        assert_eq!(
            shell.active_workspace().tabs()[1].panes()[0].launch().cwd(),
            Some("file://host/home/ops")
        );
    }

    #[test]
    fn action_set_pane_user_var_updates_inactive_tab_pane_metadata() {
        let mut shell = AppShell::new(PaneLaunch::local("pwsh"));
        shell
            .apply_action(AppAction::NewTab { launch: None })
            .unwrap();
        shell
            .apply_action(AppAction::ActivateTab { tab: TabId::new(1) })
            .unwrap();

        shell
            .apply_action(AppAction::SetPaneUserVar {
                pane: PaneId::new(2),
                name: "WEZTERM_PROG".to_owned(),
                value: "bar".to_owned(),
            })
            .unwrap();

        assert_eq!(
            shell.active_workspace().tabs()[1].panes()[0]
                .user_vars()
                .get("WEZTERM_PROG")
                .map(String::as_str),
            Some("bar")
        );
    }

    #[test]
    fn action_set_pane_badge_format_updates_inactive_tab_pane_metadata() {
        let mut shell = AppShell::new(PaneLaunch::local("pwsh"));
        shell
            .apply_action(AppAction::NewTab { launch: None })
            .unwrap();
        shell
            .apply_action(AppAction::ActivateTab { tab: TabId::new(1) })
            .unwrap();

        shell
            .apply_action(AppAction::SetPaneBadgeFormat {
                pane: PaneId::new(2),
                badge_format: Some("hello".to_owned()),
            })
            .unwrap();

        assert_eq!(
            shell.active_workspace().tabs()[1].panes()[0].badge_format(),
            Some("hello")
        );
    }

    #[test]
    fn action_set_pane_progress_updates_inactive_tab_pane_metadata() {
        let mut shell = AppShell::new(PaneLaunch::local("pwsh"));
        shell
            .apply_action(AppAction::NewTab { launch: None })
            .unwrap();
        shell
            .apply_action(AppAction::ActivateTab { tab: TabId::new(1) })
            .unwrap();

        shell
            .apply_action(AppAction::SetPaneProgress {
                pane: PaneId::new(2),
                progress: PaneProgress::Percentage(42),
            })
            .unwrap();

        assert_eq!(
            shell.active_workspace().tabs()[1].panes()[0].progress(),
            PaneProgress::Percentage(42)
        );
    }

    #[test]
    fn reset_pane_runtime_projection_clears_only_target_runtime_metadata() {
        let mut shell = AppShell::new(
            PaneLaunch::local("pwsh")
                .with_args(["-NoLogo"])
                .with_cwd("file://host/original")
                .with_environment([("KEEP", "yes")]),
        );
        shell
            .apply_action(AppAction::SplitPane {
                pane: PaneId::new(1),
                direction: SplitDirection::Right,
                launch: Some(PaneLaunch::local("sibling")),
            })
            .unwrap();
        let target = shell.active_pane_id();
        let sibling = PaneId::new(1);
        for pane in [target, sibling] {
            shell
                .apply_action(AppAction::SetPaneUserVar {
                    pane,
                    name: "RUNTIME".to_owned(),
                    value: pane.get().to_string(),
                })
                .unwrap();
            shell
                .apply_action(AppAction::SetPaneBadgeFormat {
                    pane,
                    badge_format: Some(format!("badge-{}", pane.get())),
                })
                .unwrap();
            shell
                .apply_action(AppAction::SetPaneProgress {
                    pane,
                    progress: PaneProgress::Percentage(42),
                })
                .unwrap();
            shell
                .apply_action(AppAction::SetPaneHasUnseenOutput {
                    pane,
                    has_unseen_output: true,
                })
                .unwrap();
        }
        let target_launch = shell.active_pane().launch().clone();
        let target_split = shell.active_pane().split();
        let active_workspace = shell.active_workspace_id();
        let active_tab = shell.active_tab_id();

        shell.reset_pane_runtime_projection(target).unwrap();

        let target_pane = shell.active_pane();
        assert_eq!(target_pane.id(), target);
        assert_eq!(target_pane.launch(), &target_launch);
        assert_eq!(target_pane.split(), target_split);
        assert!(target_pane.user_vars().is_empty());
        assert_eq!(target_pane.badge_format(), None);
        assert_eq!(target_pane.progress(), PaneProgress::None);
        assert!(!target_pane.has_unseen_output());
        assert_eq!(shell.active_workspace_id(), active_workspace);
        assert_eq!(shell.active_tab_id(), active_tab);

        let sibling_pane = shell
            .active_tab()
            .panes()
            .iter()
            .find(|pane| pane.id() == sibling)
            .unwrap();
        assert_eq!(
            sibling_pane.user_vars().get("RUNTIME").map(String::as_str),
            Some("1")
        );
        assert_eq!(sibling_pane.badge_format(), Some("badge-1"));
        assert_eq!(sibling_pane.progress(), PaneProgress::Percentage(42));
        assert!(sibling_pane.has_unseen_output());
    }

    #[test]
    fn action_set_tab_title_updates_explicit_tab_title() {
        let mut shell = AppShell::new(PaneLaunch::local("pwsh"));

        shell
            .apply_action(AppAction::SetTabTitle {
                tab: TabId::new(1),
                title: "build".to_owned(),
            })
            .unwrap();

        assert_eq!(shell.active_tab().title(), Some("build"));
    }

    #[test]
    fn action_set_tab_title_clears_empty_explicit_title() {
        let mut shell = AppShell::new(PaneLaunch::local("pwsh"));
        shell
            .apply_action(AppAction::SetTabTitle {
                tab: TabId::new(1),
                title: "build".to_owned(),
            })
            .unwrap();

        shell
            .apply_action(AppAction::SetTabTitle {
                tab: TabId::new(1),
                title: "  ".to_owned(),
            })
            .unwrap();

        assert_eq!(shell.active_tab().title(), None);
    }

    #[test]
    fn action_set_tab_title_rejects_invalid_tab() {
        let mut shell = AppShell::new(PaneLaunch::local("pwsh"));

        let error = shell
            .apply_action(AppAction::SetTabTitle {
                tab: TabId::new(99),
                title: "build".to_owned(),
            })
            .unwrap_err();

        assert_eq!(error, AppShellError::InvalidTab(TabId::new(99)));
    }

    #[test]
    fn action_move_tab_relative_reorders_active_tab_and_keeps_it_active() {
        let mut shell = AppShell::new(PaneLaunch::local("pwsh"));
        shell
            .apply_action(AppAction::NewTab { launch: None })
            .unwrap();
        shell
            .apply_action(AppAction::NewTab { launch: None })
            .unwrap();
        shell
            .apply_action(AppAction::ActivateTab { tab: TabId::new(1) })
            .unwrap();
        let tab_order = |shell: &AppShell| -> Vec<TabId> {
            shell
                .active_workspace()
                .tabs()
                .iter()
                .map(Tab::id)
                .collect()
        };

        shell
            .apply_action(AppAction::MoveTabRelative { offset: 1 })
            .unwrap();
        assert_eq!(shell.active_tab_id(), TabId::new(1));
        assert_eq!(
            tab_order(&shell),
            vec![TabId::new(2), TabId::new(1), TabId::new(3)]
        );

        shell
            .apply_action(AppAction::MoveTabRelative { offset: -1 })
            .unwrap();
        assert_eq!(shell.active_tab_id(), TabId::new(1));
        assert_eq!(
            tab_order(&shell),
            vec![TabId::new(1), TabId::new(2), TabId::new(3)]
        );

        shell
            .apply_action(AppAction::MoveTabRelative { offset: 0 })
            .unwrap();
        assert_eq!(shell.active_tab_id(), TabId::new(1));
        assert_eq!(
            tab_order(&shell),
            vec![TabId::new(1), TabId::new(2), TabId::new(3)]
        );
    }

    #[test]
    fn action_move_tab_reorders_active_tab_to_absolute_index() {
        let mut shell = AppShell::new(PaneLaunch::local("pwsh"));
        shell
            .apply_action(AppAction::NewTab { launch: None })
            .unwrap();
        shell
            .apply_action(AppAction::NewTab { launch: None })
            .unwrap();
        shell
            .apply_action(AppAction::ActivateTab { tab: TabId::new(1) })
            .unwrap();
        let tab_order = |shell: &AppShell| -> Vec<TabId> {
            shell
                .active_workspace()
                .tabs()
                .iter()
                .map(Tab::id)
                .collect()
        };

        shell.apply_action(AppAction::MoveTab { index: 2 }).unwrap();
        assert_eq!(shell.active_tab_id(), TabId::new(1));
        assert_eq!(
            tab_order(&shell),
            vec![TabId::new(2), TabId::new(3), TabId::new(1)]
        );

        shell.apply_action(AppAction::MoveTab { index: 0 }).unwrap();
        assert_eq!(shell.active_tab_id(), TabId::new(1));
        assert_eq!(
            tab_order(&shell),
            vec![TabId::new(1), TabId::new(2), TabId::new(3)]
        );

        let error = shell
            .apply_action(AppAction::MoveTab { index: 3 })
            .unwrap_err();
        assert_eq!(error, AppShellError::InvalidTabIndex(3));
        assert_eq!(
            tab_order(&shell),
            vec![TabId::new(1), TabId::new(2), TabId::new(3)]
        );
    }

    #[test]
    fn action_activate_last_tab_toggles_previous_active_tab() {
        let mut shell = AppShell::new(PaneLaunch::local("pwsh"));
        shell
            .apply_action(AppAction::NewTab { launch: None })
            .unwrap();
        shell
            .apply_action(AppAction::NewTab { launch: None })
            .unwrap();

        shell.apply_action(AppAction::ActivateLastTab).unwrap();
        assert_eq!(shell.active_tab_id(), TabId::new(2));

        shell.apply_action(AppAction::ActivateLastTab).unwrap();
        assert_eq!(shell.active_tab_id(), TabId::new(3));
    }

    #[test]
    fn action_activate_tab_exposes_last_active_tab_id() {
        let mut shell = AppShell::new(PaneLaunch::local("pwsh"));
        shell
            .apply_action(AppAction::NewTab { launch: None })
            .unwrap();

        assert_eq!(shell.active_tab_id(), TabId::new(2));
        assert_eq!(shell.last_active_tab_id(), Some(TabId::new(1)));

        shell
            .apply_action(AppAction::ActivateTab { tab: TabId::new(1) })
            .unwrap();

        assert_eq!(shell.active_tab_id(), TabId::new(1));
        assert_eq!(shell.last_active_tab_id(), Some(TabId::new(2)));
    }

    #[test]
    fn action_activate_last_tab_without_previous_tab_is_noop() {
        let mut shell = AppShell::new(PaneLaunch::local("pwsh"));

        shell.apply_action(AppAction::ActivateLastTab).unwrap();

        assert_eq!(shell.active_tab_id(), TabId::new(1));
    }

    #[test]
    fn action_activate_last_tab_after_previous_tab_closes_is_noop() {
        let mut shell = AppShell::new(PaneLaunch::local("pwsh"));
        shell
            .apply_action(AppAction::NewTab { launch: None })
            .unwrap();
        shell
            .apply_action(AppAction::NewTab { launch: None })
            .unwrap();
        assert_eq!(shell.active_tab_id(), TabId::new(3));
        shell
            .apply_action(AppAction::CloseTab {
                tab: TabId::new(2),
                switch_to_last_active: false,
            })
            .unwrap();

        shell.apply_action(AppAction::ActivateLastTab).unwrap();

        assert_eq!(shell.active_tab_id(), TabId::new(3));
    }

    #[test]
    fn action_activate_tab_relative_wraps_at_edges() {
        let mut shell = AppShell::new(PaneLaunch::local("pwsh"));
        shell
            .apply_action(AppAction::NewTab { launch: None })
            .unwrap();
        shell
            .apply_action(AppAction::NewTab { launch: None })
            .unwrap();

        shell
            .apply_action(AppAction::ActivateTabRelative { offset: 1 })
            .unwrap();
        assert_eq!(shell.active_tab_id(), TabId::new(1));

        shell
            .apply_action(AppAction::ActivateTabRelative { offset: -1 })
            .unwrap();
        assert_eq!(shell.active_tab_id(), TabId::new(3));

        shell
            .apply_action(AppAction::ActivateTabRelative { offset: -4 })
            .unwrap();
        assert_eq!(shell.active_tab_id(), TabId::new(2));

        shell
            .apply_action(AppAction::ActivateTabRelative { offset: 0 })
            .unwrap();
        assert_eq!(shell.active_tab_id(), TabId::new(2));
    }

    #[test]
    fn action_activate_tab_relative_no_wrap_stops_at_edges() {
        let mut shell = AppShell::new(PaneLaunch::local("pwsh"));
        shell
            .apply_action(AppAction::NewTab { launch: None })
            .unwrap();
        shell
            .apply_action(AppAction::NewTab { launch: None })
            .unwrap();
        shell
            .apply_action(AppAction::ActivateTab { tab: TabId::new(2) })
            .unwrap();

        shell
            .apply_action(AppAction::ActivateTabRelativeNoWrap { offset: -1 })
            .unwrap();
        assert_eq!(shell.active_tab_id(), TabId::new(1));

        shell
            .apply_action(AppAction::ActivateTabRelativeNoWrap { offset: -1 })
            .unwrap();
        assert_eq!(shell.active_tab_id(), TabId::new(1));

        shell
            .apply_action(AppAction::ActivateTabRelativeNoWrap { offset: 2 })
            .unwrap();
        assert_eq!(shell.active_tab_id(), TabId::new(3));

        shell
            .apply_action(AppAction::ActivateTabRelativeNoWrap { offset: 1 })
            .unwrap();
        assert_eq!(shell.active_tab_id(), TabId::new(3));

        shell
            .apply_action(AppAction::ActivateTabRelativeNoWrap { offset: 0 })
            .unwrap();
        assert_eq!(shell.active_tab_id(), TabId::new(3));
    }

    #[test]
    fn action_activate_tab_index_uses_zero_based_and_negative_indices() {
        let mut shell = AppShell::new(PaneLaunch::local("pwsh"));
        shell
            .apply_action(AppAction::NewTab { launch: None })
            .unwrap();
        shell
            .apply_action(AppAction::NewTab { launch: None })
            .unwrap();

        shell
            .apply_action(AppAction::ActivateTabIndex { index: 0 })
            .unwrap();
        assert_eq!(shell.active_tab_id(), TabId::new(1));

        shell
            .apply_action(AppAction::ActivateTabIndex { index: 1 })
            .unwrap();
        assert_eq!(shell.active_tab_id(), TabId::new(2));

        shell
            .apply_action(AppAction::ActivateTabIndex { index: -1 })
            .unwrap();
        assert_eq!(shell.active_tab_id(), TabId::new(3));

        shell
            .apply_action(AppAction::ActivateTabIndex { index: -2 })
            .unwrap();
        assert_eq!(shell.active_tab_id(), TabId::new(2));

        shell
            .apply_action(AppAction::ActivateTabIndex { index: 99 })
            .unwrap();
        assert_eq!(shell.active_tab_id(), TabId::new(2));

        shell
            .apply_action(AppAction::ActivateTabIndex { index: -99 })
            .unwrap();
        assert_eq!(shell.active_tab_id(), TabId::new(1));
    }

    #[test]
    fn action_close_tab_selects_neighbor() {
        let mut shell = AppShell::new(PaneLaunch::local("pwsh"));
        shell
            .apply_action(AppAction::NewTab { launch: None })
            .unwrap();

        shell
            .apply_action(AppAction::CloseTab {
                tab: TabId::new(2),
                switch_to_last_active: false,
            })
            .unwrap();

        assert_eq!(shell.active_tab_id(), TabId::new(1));
        assert_eq!(shell.active_workspace().tabs().len(), 1);
    }

    #[test]
    fn action_close_tab_can_select_last_active_tab() {
        let mut shell = AppShell::new(PaneLaunch::local("pwsh"));
        shell
            .apply_action(AppAction::NewTab { launch: None })
            .unwrap();
        shell
            .apply_action(AppAction::NewTab { launch: None })
            .unwrap();
        shell
            .apply_action(AppAction::NewTab { launch: None })
            .unwrap();
        shell
            .apply_action(AppAction::ActivateTab { tab: TabId::new(2) })
            .unwrap();
        shell
            .apply_action(AppAction::ActivateTab { tab: TabId::new(4) })
            .unwrap();

        shell
            .apply_action(AppAction::CloseTab {
                tab: TabId::new(4),
                switch_to_last_active: true,
            })
            .unwrap();

        assert_eq!(shell.active_tab_id(), TabId::new(2));
        assert_eq!(shell.active_workspace().tabs().len(), 3);
    }

    #[test]
    fn close_tab_with_adjacent_selection_prefers_the_right_neighbor() {
        let mut shell = AppShell::new(PaneLaunch::local("pwsh"));
        shell
            .apply_action(AppAction::NewTab { launch: None })
            .unwrap();
        shell
            .apply_action(AppAction::NewTab { launch: None })
            .unwrap();
        shell
            .apply_action(AppAction::ActivateTab { tab: TabId::new(2) })
            .unwrap();

        shell
            .apply_action(AppAction::CloseTabWithSelection {
                tab: TabId::new(2),
                selection: CloseTabSelection::Adjacent,
            })
            .unwrap();

        assert_eq!(shell.active_tab_id(), TabId::new(3));
        assert_eq!(
            shell
                .active_workspace()
                .tabs()
                .iter()
                .map(Tab::id)
                .collect::<Vec<_>>(),
            vec![TabId::new(1), TabId::new(3)]
        );
    }

    #[test]
    fn reconnect_snapshot_restores_a_full_split_tab_with_fresh_ids() {
        let mut shell = AppShell::new(PaneLaunch::local("pwsh").with_cwd("C:/work"));
        shell
            .apply_action(AppAction::SplitPane {
                pane: PaneId::new(1),
                direction: SplitDirection::Right,
                launch: Some(PaneLaunch::local("ssh").with_args(["ops"])),
            })
            .unwrap();
        shell
            .apply_action(AppAction::SetTabTitle {
                tab: TabId::new(1),
                title: "production".to_owned(),
            })
            .unwrap();
        shell
            .apply_action(AppAction::SetPaneZoomState {
                pane: PaneId::new(2),
                zoomed: true,
            })
            .unwrap();

        let snapshot = shell.tab_reconnect_snapshot(TabId::new(1)).unwrap();
        let restored = shell.restore_tab_snapshot(snapshot, 1).unwrap();

        assert_eq!(restored, TabId::new(2));
        assert_eq!(shell.active_tab_id(), restored);
        let tab = shell.active_tab();
        assert_eq!(tab.title(), Some("production"));
        assert_eq!(tab.panes().len(), 2);
        assert_eq!(tab.active_pane_id(), PaneId::new(4));
        assert_eq!(tab.zoomed_pane_id(), Some(PaneId::new(4)));
        assert_eq!(tab.panes()[0].id(), PaneId::new(3));
        assert_eq!(tab.panes()[0].launch().cwd(), Some("C:/work"));
        assert_eq!(tab.panes()[1].id(), PaneId::new(4));
        assert_eq!(tab.panes()[1].launch().program(), "ssh");
        assert_eq!(
            tab.panes()[1]
                .split()
                .expect("split is retained")
                .source_pane,
            PaneId::new(3)
        );
    }

    #[test]
    fn duplicate_tab_inserts_a_full_reconnect_clone_to_the_right() {
        let mut shell = AppShell::new(PaneLaunch::local("pwsh"));
        shell
            .apply_action(AppAction::SplitPane {
                pane: PaneId::new(1),
                direction: SplitDirection::Down,
                launch: Some(PaneLaunch::local("ssh").with_args(["ops"])),
            })
            .unwrap();
        shell
            .apply_action(AppAction::SetTabTitle {
                tab: TabId::new(1),
                title: "ops".to_owned(),
            })
            .unwrap();

        let duplicate = shell.duplicate_tab(TabId::new(1)).unwrap();

        assert_eq!(duplicate, TabId::new(2));
        assert_eq!(shell.active_tab_id(), duplicate);
        assert_eq!(shell.active_tab().title(), Some("ops"));
        assert_eq!(shell.active_tab().panes().len(), 2);
        assert_eq!(
            shell.active_tab().panes()[1]
                .split()
                .expect("split is retained")
                .direction,
            SplitDirection::Down
        );
        assert_eq!(
            shell
                .active_workspace()
                .tabs()
                .iter()
                .map(Tab::id)
                .collect::<Vec<_>>(),
            vec![TabId::new(1), TabId::new(2)]
        );
    }

    #[test]
    fn detached_tab_import_remaps_conflicting_tab_and_pane_ids() {
        let mut source = AppShell::new(PaneLaunch::local("pwsh"));
        source
            .apply_action(AppAction::NewTab { launch: None })
            .unwrap();
        source
            .apply_action(AppAction::SplitPane {
                pane: PaneId::new(2),
                direction: SplitDirection::Right,
                launch: Some(PaneLaunch::local("ssh")),
            })
            .unwrap();
        let detached = source.detach_tab(TabId::new(2)).unwrap();

        let mut target = AppShell::new(PaneLaunch::local("pwsh"));
        target
            .apply_action(AppAction::SplitPane {
                pane: PaneId::new(1),
                direction: SplitDirection::Left,
                launch: None,
            })
            .unwrap();

        let imported = target.insert_detached_tab(detached, 0).unwrap();

        assert_eq!(imported, TabId::new(2));
        assert_eq!(target.active_tab_id(), imported);
        assert_eq!(target.active_tab().panes()[0].id(), PaneId::new(3));
        assert_eq!(target.active_tab().panes()[1].id(), PaneId::new(4));
        assert_eq!(
            target.active_tab().panes()[1]
                .split()
                .expect("split source is remapped")
                .source_pane,
            PaneId::new(3)
        );
    }

    #[test]
    fn closed_tab_history_is_bounded_and_reopens_the_most_recent_tab_first() {
        let shell = AppShell::new(PaneLaunch::local("pwsh"));
        let first = shell.tab_reconnect_snapshot(TabId::new(1)).unwrap();
        let second = first.clone();
        let third = first.clone();
        let mut history = ClosedTabHistory::new(2);

        history.push(ClosedTabEntry::new(
            first,
            WindowId::new(3),
            WorkspaceId::new(1),
            0,
        ));
        history.push(ClosedTabEntry::new(
            second,
            WindowId::new(4),
            WorkspaceId::new(2),
            1,
        ));
        history.push(ClosedTabEntry::new(
            third,
            WindowId::new(5),
            WorkspaceId::new(3),
            2,
        ));

        assert_eq!(history.len(), 2);
        assert_eq!(history.pop().unwrap().origin_window_id(), WindowId::new(5));
        assert_eq!(
            history.pop().unwrap().origin_workspace_id(),
            WorkspaceId::new(2)
        );
        assert!(history.pop().is_none());
    }

    #[test]
    fn move_tab_to_new_window_preserves_the_complete_tab_model() {
        let mut shell = AppShell::new(PaneLaunch::local("pwsh"));
        shell
            .apply_action(AppAction::NewTab { launch: None })
            .unwrap();
        shell
            .apply_action(AppAction::SplitPane {
                pane: PaneId::new(2),
                direction: SplitDirection::Right,
                launch: Some(PaneLaunch::local("ssh").with_args(["ops"])),
            })
            .unwrap();

        shell
            .apply_action(AppAction::MoveTabToNewWindow { tab: TabId::new(2) })
            .unwrap();

        assert_eq!(shell.active_workspace().tabs().len(), 1);
        let pending = shell.pending_windows().first().expect("pending window");
        assert_eq!(pending.tab().panes().len(), 2);
        assert_eq!(pending.tab().panes()[0].id(), PaneId::new(2));
        assert_eq!(pending.tab().panes()[1].id(), PaneId::new(3));
        assert_eq!(
            pending.tab().panes()[1]
                .split()
                .expect("split is retained")
                .source_pane,
            PaneId::new(2)
        );
    }

    #[test]
    fn move_final_tab_to_new_window_keeps_a_pending_copy_for_the_host_to_transfer() {
        let mut shell = AppShell::new(PaneLaunch::local("pwsh"));

        shell
            .apply_action(AppAction::MoveTabToNewWindow { tab: TabId::new(1) })
            .expect("the host must be able to move a final tab before closing its source window");

        assert_eq!(shell.active_workspace().tabs().len(), 1);
        let pending = shell.pending_windows().first().expect("pending window");
        assert_eq!(pending.tab().id(), TabId::new(1));
        assert_eq!(pending.active_pane_id(), PaneId::new(1));
    }

    #[test]
    fn action_close_last_pane_closes_active_tab_when_neighbor_exists() {
        let mut shell = AppShell::new(PaneLaunch::local("pwsh"));
        shell
            .apply_action(AppAction::NewTab { launch: None })
            .unwrap();

        shell
            .apply_action(AppAction::ClosePane {
                pane: PaneId::new(2),
            })
            .unwrap();

        assert_eq!(shell.active_tab_id(), TabId::new(1));
        assert_eq!(shell.active_workspace().tabs().len(), 1);
        assert_eq!(shell.active_pane_id(), PaneId::new(1));
    }

    #[test]
    fn action_close_pane_finds_an_inactive_tab_without_switching_tabs() {
        let mut shell = AppShell::new(PaneLaunch::local("pwsh"));
        shell
            .apply_action(AppAction::NewTab { launch: None })
            .unwrap();
        shell
            .apply_action(AppAction::ActivateTab { tab: TabId::new(1) })
            .unwrap();

        shell
            .apply_action(AppAction::ClosePane {
                pane: PaneId::new(2),
            })
            .unwrap();

        assert_eq!(shell.active_tab_id(), TabId::new(1));
        assert_eq!(shell.active_workspace().tabs().len(), 1);
        assert_eq!(shell.active_pane_id(), PaneId::new(1));
    }

    #[test]
    fn action_close_last_tab_is_rejected() {
        let mut shell = AppShell::new(PaneLaunch::local("pwsh"));

        let error = shell
            .apply_action(AppAction::CloseTab {
                tab: TabId::new(1),
                switch_to_last_active: false,
            })
            .unwrap_err();

        assert_eq!(error, AppShellError::CannotCloseLastTab);
        assert_eq!(shell.active_tab_id(), TabId::new(1));
    }

    #[test]
    fn action_split_pane_creates_and_focuses_new_pane() {
        let mut shell = AppShell::new(PaneLaunch::local("pwsh"));

        shell
            .apply_action(AppAction::SplitPane {
                pane: PaneId::new(1),
                direction: SplitDirection::Right,
                launch: None,
            })
            .unwrap();

        assert_eq!(shell.active_pane_id(), PaneId::new(2));
        assert_eq!(shell.active_tab().panes().len(), 2);
        assert_eq!(
            shell.active_tab().panes()[1]
                .split
                .as_ref()
                .expect("split should be present")
                .source_pane,
            PaneId::new(1)
        );
        assert_eq!(
            shell.active_tab().panes()[1]
                .split
                .as_ref()
                .expect("split should be present")
                .direction,
            SplitDirection::Right
        );
    }

    #[test]
    fn focus_next_pane_cycles_within_active_tab() {
        let mut shell = AppShell::new(PaneLaunch::local("pwsh"));
        shell
            .apply_action(AppAction::SplitPane {
                pane: PaneId::new(1),
                direction: SplitDirection::Right,
                launch: None,
            })
            .unwrap();

        shell.apply_action(AppAction::FocusNextPane).unwrap();

        assert_eq!(shell.active_pane_id(), PaneId::new(1));
    }

    #[test]
    fn action_activate_pane_by_index_uses_current_tab_order_and_ignores_invalid_index() {
        let mut shell = AppShell::new(PaneLaunch::local("pwsh"));
        shell
            .apply_action(AppAction::SplitPane {
                pane: PaneId::new(1),
                direction: SplitDirection::Right,
                launch: None,
            })
            .unwrap();
        shell
            .apply_action(AppAction::SplitPane {
                pane: PaneId::new(2),
                direction: SplitDirection::Down,
                launch: None,
            })
            .unwrap();
        assert_eq!(shell.active_pane_id(), PaneId::new(3));

        shell
            .apply_action(AppAction::ActivatePaneByIndex { index: 0 })
            .unwrap();
        assert_eq!(shell.active_pane_id(), PaneId::new(1));

        shell
            .apply_action(AppAction::ActivatePaneByIndex { index: 2 })
            .unwrap();
        assert_eq!(shell.active_pane_id(), PaneId::new(3));

        shell
            .apply_action(AppAction::ActivatePaneByIndex { index: 99 })
            .unwrap();
        assert_eq!(shell.active_pane_id(), PaneId::new(3));
    }

    #[test]
    fn action_rotate_panes_clockwise_moves_last_pane_to_first_position() {
        let mut shell = AppShell::new(PaneLaunch::local("pwsh"));
        shell
            .apply_action(AppAction::SplitPane {
                pane: PaneId::new(1),
                direction: SplitDirection::Right,
                launch: None,
            })
            .unwrap();
        shell
            .apply_action(AppAction::SplitPane {
                pane: PaneId::new(2),
                direction: SplitDirection::Right,
                launch: None,
            })
            .unwrap();
        shell
            .apply_action(AppAction::ResizePane {
                pane: PaneId::new(2),
                direction: ResizeDirection::Right,
                amount: 3,
            })
            .unwrap();

        shell
            .apply_action(AppAction::RotatePanes {
                direction: PaneRotationDirection::Clockwise,
            })
            .unwrap();

        let panes = shell.active_tab().panes();
        assert_eq!(
            panes.iter().map(Pane::id).collect::<Vec<_>>(),
            vec![PaneId::new(3), PaneId::new(1), PaneId::new(2)]
        );
        assert_eq!(shell.active_pane_id(), PaneId::new(3));
        assert_eq!(shell.active_pane_position(), 1);
        assert_eq!(panes[0].split(), None);
        assert_eq!(
            panes[1].split(),
            Some(PaneSplit {
                source_pane: PaneId::new(3),
                direction: SplitDirection::Right,
                source_size_delta: 0,
            })
        );
        assert_eq!(
            panes[2].split(),
            Some(PaneSplit {
                source_pane: PaneId::new(1),
                direction: SplitDirection::Right,
                source_size_delta: 3,
            })
        );
    }

    #[test]
    fn action_rotate_panes_counter_clockwise_moves_first_pane_to_last_position() {
        let mut shell = AppShell::new(PaneLaunch::local("pwsh"));
        shell
            .apply_action(AppAction::SplitPane {
                pane: PaneId::new(1),
                direction: SplitDirection::Right,
                launch: None,
            })
            .unwrap();
        shell
            .apply_action(AppAction::SplitPane {
                pane: PaneId::new(2),
                direction: SplitDirection::Right,
                launch: None,
            })
            .unwrap();

        shell
            .apply_action(AppAction::ActivatePane {
                pane: PaneId::new(1),
            })
            .unwrap();
        shell
            .apply_action(AppAction::RotatePanes {
                direction: PaneRotationDirection::CounterClockwise,
            })
            .unwrap();

        let panes = shell.active_tab().panes();
        assert_eq!(
            panes.iter().map(Pane::id).collect::<Vec<_>>(),
            vec![PaneId::new(2), PaneId::new(3), PaneId::new(1)]
        );
        assert_eq!(shell.active_pane_id(), PaneId::new(1));
        assert_eq!(shell.active_pane_position(), 3);
        assert_eq!(panes[0].split(), None);
        assert_eq!(
            panes[1].split().map(|split| split.source_pane),
            Some(PaneId::new(2))
        );
        assert_eq!(
            panes[2].split().map(|split| split.source_pane),
            Some(PaneId::new(3))
        );
    }

    #[test]
    fn action_activate_pane_direction_moves_to_adjacent_split_pane() {
        let mut shell = AppShell::new(PaneLaunch::local("pwsh"));
        shell
            .apply_action(AppAction::SplitPane {
                pane: PaneId::new(1),
                direction: SplitDirection::Right,
                launch: None,
            })
            .unwrap();
        shell
            .apply_action(AppAction::ActivatePane {
                pane: PaneId::new(1),
            })
            .unwrap();
        shell
            .apply_action(AppAction::SplitPane {
                pane: PaneId::new(1),
                direction: SplitDirection::Down,
                launch: None,
            })
            .unwrap();

        shell
            .apply_action(AppAction::ActivatePaneDirection {
                direction: PaneDirection::Up,
            })
            .unwrap();
        assert_eq!(shell.active_pane_id(), PaneId::new(1));

        shell
            .apply_action(AppAction::ActivatePaneDirection {
                direction: PaneDirection::Down,
            })
            .unwrap();
        assert_eq!(shell.active_pane_id(), PaneId::new(3));

        shell
            .apply_action(AppAction::ActivatePaneDirection {
                direction: PaneDirection::Right,
            })
            .unwrap();
        assert_eq!(shell.active_pane_id(), PaneId::new(2));
    }

    #[test]
    fn action_activate_pane_direction_moves_to_left_and_up_splits() {
        let mut shell = AppShell::new(PaneLaunch::local("pwsh"));
        shell
            .apply_action(AppAction::SplitPane {
                pane: PaneId::new(1),
                direction: SplitDirection::Left,
                launch: None,
            })
            .unwrap();
        assert_eq!(shell.active_pane_id(), PaneId::new(2));

        shell
            .apply_action(AppAction::ActivatePaneDirection {
                direction: PaneDirection::Right,
            })
            .unwrap();
        assert_eq!(shell.active_pane_id(), PaneId::new(1));

        shell
            .apply_action(AppAction::SplitPane {
                pane: PaneId::new(1),
                direction: SplitDirection::Up,
                launch: None,
            })
            .unwrap();
        assert_eq!(shell.active_pane_id(), PaneId::new(3));

        shell
            .apply_action(AppAction::ActivatePaneDirection {
                direction: PaneDirection::Down,
            })
            .unwrap();
        assert_eq!(shell.active_pane_id(), PaneId::new(1));
    }

    #[test]
    fn action_activate_pane_direction_without_neighbor_is_noop() {
        let mut shell = AppShell::new(PaneLaunch::local("pwsh"));
        shell
            .apply_action(AppAction::SplitPane {
                pane: PaneId::new(1),
                direction: SplitDirection::Right,
                launch: None,
            })
            .unwrap();
        assert_eq!(shell.active_pane_id(), PaneId::new(2));

        shell
            .apply_action(AppAction::ActivatePaneDirection {
                direction: PaneDirection::Right,
            })
            .unwrap();

        assert_eq!(shell.active_pane_id(), PaneId::new(2));
    }

    #[test]
    fn action_activate_pane_direction_unzooms_before_switching_pane() {
        let mut shell = AppShell::new(PaneLaunch::local("pwsh"));
        shell
            .apply_action(AppAction::SplitPane {
                pane: PaneId::new(1),
                direction: SplitDirection::Right,
                launch: None,
            })
            .unwrap();
        shell
            .apply_action(AppAction::ActivatePane {
                pane: PaneId::new(1),
            })
            .unwrap();
        shell
            .apply_action(AppAction::TogglePaneZoom {
                pane: PaneId::new(1),
            })
            .unwrap();
        assert_eq!(shell.active_tab().zoomed_pane_id(), Some(PaneId::new(1)));

        shell
            .apply_action(AppAction::ActivatePaneDirection {
                direction: PaneDirection::Right,
            })
            .unwrap();

        assert_eq!(shell.active_pane_id(), PaneId::new(2));
        assert_eq!(shell.active_tab().zoomed_pane_id(), None);
    }

    #[test]
    fn action_activate_pane_direction_uses_most_recent_candidate_for_ambiguity() {
        let mut shell = AppShell::new(PaneLaunch::local("pwsh"));
        shell
            .apply_action(AppAction::SplitPane {
                pane: PaneId::new(1),
                direction: SplitDirection::Right,
                launch: None,
            })
            .unwrap();
        shell
            .apply_action(AppAction::ActivatePane {
                pane: PaneId::new(1),
            })
            .unwrap();
        shell
            .apply_action(AppAction::SplitPane {
                pane: PaneId::new(1),
                direction: SplitDirection::Down,
                launch: None,
            })
            .unwrap();
        shell
            .apply_action(AppAction::ActivatePane {
                pane: PaneId::new(2),
            })
            .unwrap();

        shell
            .apply_action(AppAction::ActivatePaneDirection {
                direction: PaneDirection::Left,
            })
            .unwrap();

        assert_eq!(shell.active_pane_id(), PaneId::new(3));
    }

    #[test]
    fn action_activate_pane_focuses_requested_pane() {
        let mut shell = AppShell::new(PaneLaunch::local("pwsh"));
        shell
            .apply_action(AppAction::SplitPane {
                pane: PaneId::new(1),
                direction: SplitDirection::Right,
                launch: None,
            })
            .unwrap();

        shell
            .apply_action(AppAction::ActivatePane {
                pane: PaneId::new(1),
            })
            .unwrap();

        assert_eq!(shell.active_pane_id(), PaneId::new(1));
    }

    #[test]
    fn action_resize_pane_updates_split_size_delta() {
        let mut shell = AppShell::new(PaneLaunch::local("pwsh"));
        shell
            .apply_action(AppAction::SplitPane {
                pane: PaneId::new(1),
                direction: SplitDirection::Right,
                launch: None,
            })
            .unwrap();

        shell
            .apply_action(AppAction::ResizePane {
                pane: PaneId::new(2),
                direction: ResizeDirection::Left,
                amount: 3,
            })
            .unwrap();

        let split = shell.active_tab().panes()[1]
            .split()
            .expect("split should be present");
        assert_eq!(split.source_size_delta, -3);
    }

    #[test]
    fn preserve_split_layout_scales_down_split_with_height_only() {
        let mut shell = AppShell::new(PaneLaunch::local("pwsh"));
        shell
            .apply_action(AppAction::SplitPane {
                pane: PaneId::new(1),
                direction: SplitDirection::Down,
                launch: None,
            })
            .unwrap();
        shell
            .apply_action(AppAction::ResizePane {
                pane: PaneId::new(1),
                direction: ResizeDirection::Down,
                amount: 4,
            })
            .unwrap();
        let before = pane_layout_rects(shell.active_tab(), 80, 25);

        shell.preserve_split_layout_for_resize(80, 25, 80, 49);

        let after = pane_layout_rects(shell.active_tab(), 80, 49);
        assert_ratio_within_one_cell(
            before[&PaneId::new(1)].rows,
            before[&PaneId::new(2)].rows,
            after[&PaneId::new(1)].rows,
            after[&PaneId::new(2)].rows,
        );
        let height_delta = shell.active_tab().panes()[1]
            .split()
            .expect("split metadata")
            .source_size_delta;

        shell.preserve_split_layout_for_resize(80, 49, 160, 49);

        assert_eq!(
            shell.active_tab().panes()[1]
                .split()
                .expect("split metadata")
                .source_size_delta,
            height_delta,
            "changing columns must not alter a vertical split"
        );
    }

    #[test]
    fn preserve_split_layout_keeps_vertical_delta_on_width_only_resize() {
        let mut shell = AppShell::new(PaneLaunch::local("pwsh"));
        shell
            .apply_action(AppAction::SplitPaneWithSize {
                pane: PaneId::new(1),
                direction: SplitDirection::Down,
                launch: None,
                source_size_delta: i16::MAX,
            })
            .unwrap();

        shell.preserve_split_layout_for_resize(80, 24, 160, 24);

        assert_eq!(
            shell.active_tab().panes()[1]
                .split()
                .expect("split metadata")
                .source_size_delta,
            i16::MAX
        );
    }

    #[test]
    fn preserve_split_layout_keeps_horizontal_delta_on_height_only_resize() {
        let mut shell = AppShell::new(PaneLaunch::local("pwsh"));
        shell
            .apply_action(AppAction::SplitPaneWithSize {
                pane: PaneId::new(1),
                direction: SplitDirection::Right,
                launch: None,
                source_size_delta: i16::MAX,
            })
            .unwrap();

        shell.preserve_split_layout_for_resize(80, 24, 80, 48);

        assert_eq!(
            shell.active_tab().panes()[1]
                .split()
                .expect("split metadata")
                .source_size_delta,
            i16::MAX
        );
    }

    #[test]
    fn preserve_split_layout_rebalances_clamped_delta_when_split_axis_changes() {
        let mut shell = AppShell::new(PaneLaunch::local("pwsh"));
        shell
            .apply_action(AppAction::SplitPaneWithSize {
                pane: PaneId::new(1),
                direction: SplitDirection::Right,
                launch: None,
                source_size_delta: i16::MAX,
            })
            .unwrap();
        let before = pane_layout_rects(shell.active_tab(), 80, 24);

        shell.preserve_split_layout_for_resize(80, 24, 160, 24);

        let after = pane_layout_rects(shell.active_tab(), 160, 24);
        assert_ne!(
            shell.active_tab().panes()[1]
                .split()
                .expect("split metadata")
                .source_size_delta,
            i16::MAX
        );
        assert_ratio_within_one_cell(
            before[&PaneId::new(1)].columns,
            before[&PaneId::new(2)].columns,
            after[&PaneId::new(1)].columns,
            after[&PaneId::new(2)].columns,
        );
    }

    #[test]
    fn preserve_split_layout_recurses_through_mixed_local_splits() {
        let mut shell = AppShell::new(PaneLaunch::local("pwsh"));
        shell
            .apply_action(AppAction::SplitPaneWithSize {
                pane: PaneId::new(1),
                direction: SplitDirection::Right,
                launch: None,
                source_size_delta: 14,
            })
            .unwrap();
        shell
            .apply_action(AppAction::SplitPaneWithSize {
                pane: PaneId::new(1),
                direction: SplitDirection::Down,
                launch: None,
                source_size_delta: -5,
            })
            .unwrap();
        shell
            .apply_action(AppAction::TogglePaneZoom {
                pane: PaneId::new(3),
            })
            .unwrap();
        let before = pane_layout_rects(shell.active_tab(), 101, 41);
        let pane_ids = shell
            .active_tab()
            .panes()
            .iter()
            .map(Pane::id)
            .collect::<Vec<_>>();

        shell.preserve_split_layout_for_resize(101, 41, 181, 73);

        let after = pane_layout_rects(shell.active_tab(), 181, 73);
        assert_eq!(
            shell
                .active_tab()
                .panes()
                .iter()
                .map(Pane::id)
                .collect::<Vec<_>>(),
            pane_ids
        );
        assert_eq!(shell.active_pane_id(), PaneId::new(3));
        assert_eq!(shell.active_tab().zoomed_pane_id(), Some(PaneId::new(3)));
        assert_ratio_within_one_cell(
            before[&PaneId::new(1)].columns,
            before[&PaneId::new(2)].columns,
            after[&PaneId::new(1)].columns,
            after[&PaneId::new(2)].columns,
        );
        assert_ratio_within_one_cell(
            before[&PaneId::new(1)].rows,
            before[&PaneId::new(3)].rows,
            after[&PaneId::new(1)].rows,
            after[&PaneId::new(3)].rows,
        );
    }

    #[test]
    fn preserve_split_layout_uses_dragged_ratio_as_resize_baseline() {
        let mut shell = AppShell::new(PaneLaunch::local("pwsh"));
        shell
            .apply_action(AppAction::SplitPane {
                pane: PaneId::new(1),
                direction: SplitDirection::Right,
                launch: None,
            })
            .unwrap();
        shell
            .apply_action(AppAction::ResizePane {
                pane: PaneId::new(1),
                direction: ResizeDirection::Right,
                amount: 17,
            })
            .unwrap();
        let before = pane_layout_rects(shell.active_tab(), 80, 24);

        shell.preserve_split_layout_for_resize(80, 24, 140, 24);

        let after = pane_layout_rects(shell.active_tab(), 140, 24);
        assert_ratio_within_one_cell(
            before[&PaneId::new(1)].columns,
            before[&PaneId::new(2)].columns,
            after[&PaneId::new(1)].columns,
            after[&PaneId::new(2)].columns,
        );
        let width_delta = shell.active_tab().panes()[1]
            .split()
            .expect("split metadata")
            .source_size_delta;

        shell.preserve_split_layout_for_resize(140, 24, 140, 48);

        assert_eq!(
            shell.active_tab().panes()[1]
                .split()
                .expect("split metadata")
                .source_size_delta,
            width_delta,
            "changing rows must not alter a horizontal split"
        );
    }

    #[test]
    fn preserve_split_layout_clamps_tiny_sizes_and_uses_clamp_as_next_baseline() {
        let mut shell = AppShell::new(PaneLaunch::local("pwsh"));
        shell
            .apply_action(AppAction::SplitPaneWithSize {
                pane: PaneId::new(1),
                direction: SplitDirection::Right,
                launch: None,
                source_size_delta: 30,
            })
            .unwrap();

        shell.preserve_split_layout_for_resize(80, 24, 3, 24);

        let tiny = pane_layout_rects(shell.active_tab(), 3, 24);
        assert_eq!(tiny[&PaneId::new(1)].columns, 1);
        assert_eq!(tiny[&PaneId::new(2)].columns, 1);

        shell.preserve_split_layout_for_resize(3, 24, 0, 24);
        assert_eq!(
            shell.active_tab().panes()[1]
                .split()
                .expect("split metadata")
                .source_size_delta,
            0
        );

        shell.preserve_split_layout_for_resize(0, 24, 80, 24);

        let expanded = pane_layout_rects(shell.active_tab(), 80, 24);
        assert_eq!(expanded[&PaneId::new(1)].columns, 40);
        assert_eq!(expanded[&PaneId::new(2)].columns, 39);
    }

    #[test]
    fn preserve_split_layout_same_size_is_exact_noop() {
        let mut shell = AppShell::new(PaneLaunch::local("pwsh"));
        shell
            .apply_action(AppAction::SplitPaneWithSize {
                pane: PaneId::new(1),
                direction: SplitDirection::Right,
                launch: None,
                source_size_delta: 7,
            })
            .unwrap();
        shell
            .apply_action(AppAction::TogglePaneZoom {
                pane: PaneId::new(2),
            })
            .unwrap();
        let before = shell.clone();

        shell.preserve_split_layout_for_resize(80, 24, 80, 24);

        assert_eq!(shell, before);
    }

    #[test]
    fn action_split_pane_accepts_initial_source_size_delta() {
        let mut shell = AppShell::new(PaneLaunch::local("pwsh"));

        shell
            .apply_action(AppAction::SplitPaneWithSize {
                pane: PaneId::new(1),
                direction: SplitDirection::Right,
                launch: None,
                source_size_delta: -12,
            })
            .unwrap();

        let split = shell.active_tab().panes()[1]
            .split()
            .expect("split should be present");
        assert_eq!(split.source_size_delta, -12);
    }

    #[test]
    fn action_split_top_level_pane_splits_the_full_tab_region() {
        let mut shell = AppShell::new(PaneLaunch::local("pwsh"));
        shell
            .apply_action(AppAction::SplitPane {
                pane: PaneId::new(1),
                direction: SplitDirection::Right,
                launch: None,
            })
            .unwrap();

        shell
            .apply_action(AppAction::SplitTopLevelPane {
                direction: SplitDirection::Down,
                launch: Some(PaneLaunch::local("top")),
            })
            .unwrap();

        assert_eq!(shell.active_pane_id(), PaneId::new(3));
        assert_eq!(
            shell
                .active_tab()
                .panes()
                .iter()
                .map(Pane::id)
                .collect::<Vec<_>>(),
            vec![PaneId::new(1), PaneId::new(3), PaneId::new(2)]
        );

        shell
            .apply_action(AppAction::ActivatePane {
                pane: PaneId::new(1),
            })
            .unwrap();
        shell
            .apply_action(AppAction::ActivatePaneDirection {
                direction: PaneDirection::Down,
            })
            .unwrap();

        assert_eq!(shell.active_pane_id(), PaneId::new(3));
        assert_eq!(shell.active_pane().launch().program(), "top");
    }

    #[test]
    fn action_toggle_pane_zoom_sets_and_restores_zoomed_pane() {
        let mut shell = AppShell::new(PaneLaunch::local("pwsh"));
        shell
            .apply_action(AppAction::SplitPane {
                pane: PaneId::new(1),
                direction: SplitDirection::Right,
                launch: None,
            })
            .unwrap();

        shell
            .apply_action(AppAction::TogglePaneZoom {
                pane: PaneId::new(2),
            })
            .unwrap();

        assert_eq!(shell.active_tab().zoomed_pane_id(), Some(PaneId::new(2)));

        shell
            .apply_action(AppAction::TogglePaneZoom {
                pane: PaneId::new(2),
            })
            .unwrap();

        assert_eq!(shell.active_tab().zoomed_pane_id(), None);
    }

    #[test]
    fn action_set_pane_zoom_state_sets_and_clears_zoom_idempotently() {
        let mut shell = AppShell::new(PaneLaunch::local("pwsh"));
        shell
            .apply_action(AppAction::SplitPane {
                pane: PaneId::new(1),
                direction: SplitDirection::Right,
                launch: None,
            })
            .unwrap();

        shell
            .apply_action(AppAction::SetPaneZoomState {
                pane: PaneId::new(2),
                zoomed: true,
            })
            .unwrap();
        assert_eq!(shell.active_pane_id(), PaneId::new(2));
        assert_eq!(shell.active_tab().zoomed_pane_id(), Some(PaneId::new(2)));

        shell
            .apply_action(AppAction::SetPaneZoomState {
                pane: PaneId::new(2),
                zoomed: true,
            })
            .unwrap();
        assert_eq!(shell.active_pane_id(), PaneId::new(2));
        assert_eq!(shell.active_tab().zoomed_pane_id(), Some(PaneId::new(2)));

        shell
            .apply_action(AppAction::SetPaneZoomState {
                pane: PaneId::new(2),
                zoomed: false,
            })
            .unwrap();
        assert_eq!(shell.active_pane_id(), PaneId::new(2));
        assert_eq!(shell.active_tab().zoomed_pane_id(), None);

        shell
            .apply_action(AppAction::SetPaneZoomState {
                pane: PaneId::new(2),
                zoomed: false,
            })
            .unwrap();
        assert_eq!(shell.active_pane_id(), PaneId::new(2));
        assert_eq!(shell.active_tab().zoomed_pane_id(), None);
    }

    #[test]
    fn action_swap_panes_exchanges_layout_positions_and_focuses_selected() {
        let mut shell = AppShell::new(PaneLaunch::local("pwsh"));
        shell
            .apply_action(AppAction::SplitPane {
                pane: PaneId::new(1),
                direction: SplitDirection::Right,
                launch: None,
            })
            .unwrap();
        shell
            .apply_action(AppAction::ActivatePane {
                pane: PaneId::new(1),
            })
            .unwrap();

        shell
            .apply_action(AppAction::SwapPanes {
                active: PaneId::new(1),
                selected: PaneId::new(2),
                keep_focus: false,
            })
            .unwrap();

        assert_eq!(shell.active_pane_id(), PaneId::new(2));
        assert_eq!(shell.active_tab().panes()[0].id(), PaneId::new(2));
        assert_eq!(shell.active_tab().panes()[1].id(), PaneId::new(1));
        assert_eq!(
            shell.active_tab().panes()[1]
                .split()
                .expect("split should remain present")
                .source_pane,
            PaneId::new(2)
        );
    }

    #[test]
    fn action_swap_panes_can_keep_active_focus() {
        let mut shell = AppShell::new(PaneLaunch::local("pwsh"));
        shell
            .apply_action(AppAction::SplitPane {
                pane: PaneId::new(1),
                direction: SplitDirection::Right,
                launch: None,
            })
            .unwrap();
        shell
            .apply_action(AppAction::ActivatePane {
                pane: PaneId::new(1),
            })
            .unwrap();

        shell
            .apply_action(AppAction::SwapPanes {
                active: PaneId::new(1),
                selected: PaneId::new(2),
                keep_focus: true,
            })
            .unwrap();

        assert_eq!(shell.active_pane_id(), PaneId::new(1));
        assert_eq!(shell.active_tab().panes()[0].id(), PaneId::new(2));
        assert_eq!(shell.active_tab().panes()[1].id(), PaneId::new(1));
    }

    #[test]
    fn action_move_pane_to_new_tab_moves_selected_pane_and_activates_new_tab() {
        let mut shell = AppShell::new(PaneLaunch::local("pwsh"));
        shell
            .apply_action(AppAction::SplitPane {
                pane: PaneId::new(1),
                direction: SplitDirection::Right,
                launch: None,
            })
            .unwrap();
        shell
            .apply_action(AppAction::ActivatePane {
                pane: PaneId::new(1),
            })
            .unwrap();

        shell
            .apply_action(AppAction::MovePaneToNewTab {
                pane: PaneId::new(2),
            })
            .unwrap();

        assert_eq!(shell.active_tab_id(), TabId::new(2));
        assert_eq!(shell.active_pane_id(), PaneId::new(2));
        assert_eq!(shell.active_workspace().tabs().len(), 2);
        assert_eq!(shell.active_workspace().tabs()[0].panes().len(), 1);
        assert_eq!(
            shell.active_workspace().tabs()[0].panes()[0].id(),
            PaneId::new(1)
        );
        assert_eq!(shell.active_tab().panes().len(), 1);
        assert_eq!(shell.active_tab().panes()[0].id(), PaneId::new(2));
        assert!(shell.active_tab().panes()[0].split().is_none());
    }

    #[test]
    fn action_move_pane_to_new_window_detaches_selected_pane_into_pending_window() {
        let mut shell = AppShell::new(PaneLaunch::local("pwsh"));
        shell
            .apply_action(AppAction::SplitPane {
                pane: PaneId::new(1),
                direction: SplitDirection::Right,
                launch: None,
            })
            .unwrap();
        shell
            .apply_action(AppAction::ActivatePane {
                pane: PaneId::new(1),
            })
            .unwrap();

        shell
            .apply_action(AppAction::MovePaneToNewWindow {
                pane: PaneId::new(2),
            })
            .unwrap();

        assert_eq!(shell.active_workspace().tabs().len(), 1);
        assert_eq!(shell.active_tab_id(), TabId::new(1));
        assert_eq!(shell.active_pane_id(), PaneId::new(1));
        assert_eq!(shell.active_tab().panes().len(), 1);
        assert_eq!(shell.active_tab().panes()[0].id(), PaneId::new(1));

        let pending_window = shell
            .pending_windows()
            .first()
            .expect("move should request a new window");
        assert_eq!(pending_window.id(), WindowId::new(2));
        assert_eq!(pending_window.workspace_id(), WorkspaceId::new(1));
        assert_eq!(pending_window.workspace_name(), "default");
        assert_eq!(pending_window.tab().id(), TabId::new(2));
        assert_eq!(pending_window.tab().active_pane_id(), PaneId::new(2));
        assert_eq!(pending_window.tab().panes().len(), 1);
        assert_eq!(pending_window.tab().panes()[0].id(), PaneId::new(2));
        assert!(pending_window.tab().panes()[0].split().is_none());
        assert_eq!(shell.pane_ids(), vec![PaneId::new(1), PaneId::new(2)]);
    }

    #[test]
    fn pending_window_can_be_consumed_into_independent_shell_state() {
        let mut shell = AppShell::new(PaneLaunch::local("pwsh").with_args(["-NoLogo"]));
        shell
            .apply_action(AppAction::SplitPane {
                pane: PaneId::new(1),
                direction: SplitDirection::Right,
                launch: None,
            })
            .unwrap();

        shell
            .apply_action(AppAction::MovePaneToNewWindow {
                pane: PaneId::new(2),
            })
            .unwrap();

        let pending_window = shell
            .take_next_pending_window()
            .expect("pending window should be consumable");
        let new_shell = AppShell::from_pending_window(pending_window);

        assert!(shell.pending_windows().is_empty());
        assert_eq!(shell.pane_ids(), vec![PaneId::new(1)]);
        assert_eq!(new_shell.active_workspace_id(), WorkspaceId::new(1));
        assert_eq!(new_shell.active_workspace().name(), "default");
        assert_eq!(new_shell.active_tab_id(), TabId::new(2));
        assert_eq!(new_shell.active_pane_id(), PaneId::new(2));
        assert_eq!(new_shell.active_tab().panes().len(), 1);
        assert_eq!(new_shell.active_pane().launch().program(), "pwsh");
        assert_eq!(new_shell.active_pane().launch().args(), &["-NoLogo"]);
        assert!(new_shell.pending_windows().is_empty());
    }

    #[test]
    fn action_spawn_window_creates_pending_window_with_new_default_tab() {
        let mut shell = AppShell::new(PaneLaunch::local("pwsh").with_args(["-NoLogo"]));

        shell
            .apply_action(AppAction::SpawnWindow { launch: None })
            .unwrap();

        assert_eq!(shell.active_workspace_id(), WorkspaceId::new(1));
        assert_eq!(shell.active_tab_id(), TabId::new(1));
        assert_eq!(shell.active_pane_id(), PaneId::new(1));
        assert_eq!(shell.pane_ids(), vec![PaneId::new(1), PaneId::new(2)]);

        let pending_window = shell
            .pending_windows()
            .first()
            .expect("spawn window should request a new window");
        assert_eq!(pending_window.id(), WindowId::new(2));
        assert_eq!(pending_window.workspace_id(), WorkspaceId::new(1));
        assert_eq!(pending_window.workspace_name(), "default");
        assert_eq!(pending_window.tab().id(), TabId::new(2));
        assert_eq!(pending_window.active_pane_id(), PaneId::new(2));
        assert_eq!(pending_window.tab().panes().len(), 1);
        assert_eq!(pending_window.tab().panes()[0].launch().program(), "pwsh");
        assert_eq!(
            pending_window.tab().panes()[0].launch().args(),
            &["-NoLogo"]
        );
        assert!(pending_window.tab().panes()[0].split().is_none());
    }

    #[test]
    fn action_spawn_window_inherits_active_pane_launch_cwd() {
        let mut shell = AppShell::new(PaneLaunch::local("pwsh"));
        shell
            .apply_action(AppAction::SetPaneCurrentWorkingDir {
                pane: PaneId::new(1),
                cwd: Some("file://host/home/ops".to_owned()),
            })
            .unwrap();

        shell
            .apply_action(AppAction::SpawnWindow { launch: None })
            .unwrap();

        let pending_window = shell
            .pending_windows()
            .first()
            .expect("spawn window should request a new window");

        assert_eq!(
            pending_window.tab().panes()[0].launch().cwd(),
            Some("file://host/home/ops")
        );
    }

    #[test]
    fn action_new_workspace_creates_and_selects_workspace() {
        let mut shell = AppShell::new(PaneLaunch::local("pwsh"));

        shell
            .apply_action(AppAction::NewWorkspace {
                name: "ops".to_owned(),
                launch: None,
            })
            .unwrap();

        assert_eq!(shell.active_workspace_id(), WorkspaceId::new(2));
        assert_eq!(shell.active_workspace().name(), "ops");
        assert_eq!(shell.workspaces().len(), 2);
    }

    #[test]
    fn action_switch_workspace_selects_existing_workspace() {
        let mut shell = AppShell::new(PaneLaunch::local("pwsh"));
        shell
            .apply_action(AppAction::NewWorkspace {
                name: "ops".to_owned(),
                launch: None,
            })
            .unwrap();

        shell
            .apply_action(AppAction::SwitchWorkspace {
                workspace: WorkspaceId::new(1),
            })
            .unwrap();

        assert_eq!(shell.active_workspace_id(), WorkspaceId::new(1));
    }

    #[test]
    fn action_switch_to_workspace_selects_existing_named_workspace_without_creating() {
        let mut shell = AppShell::new(PaneLaunch::local("pwsh"));
        shell
            .apply_action(AppAction::NewWorkspace {
                name: "ops".to_owned(),
                launch: None,
            })
            .unwrap();
        shell
            .apply_action(AppAction::NewWorkspace {
                name: "monitoring".to_owned(),
                launch: None,
            })
            .unwrap();
        shell
            .apply_action(AppAction::SwitchWorkspace {
                workspace: WorkspaceId::new(1),
            })
            .unwrap();

        shell
            .apply_action(AppAction::SwitchToWorkspace {
                name: Some("ops".to_owned()),
                launch: Some(PaneLaunch::local("ignored")),
            })
            .unwrap();

        assert_eq!(shell.workspaces().len(), 3);
        assert_eq!(shell.active_workspace().name(), "ops");
        assert_eq!(shell.active_workspace_id(), WorkspaceId::new(2));
        assert_eq!(shell.active_pane().launch().program(), "pwsh");
    }

    #[test]
    fn action_switch_to_workspace_creates_missing_named_workspace_with_spawn_command() {
        let mut shell = AppShell::new(PaneLaunch::local("pwsh"));

        shell
            .apply_action(AppAction::SwitchToWorkspace {
                name: Some("monitoring".to_owned()),
                launch: Some(PaneLaunch::local("top").with_args(["-d", "1"])),
            })
            .unwrap();

        assert_eq!(shell.workspaces().len(), 2);
        assert_eq!(shell.active_workspace().name(), "monitoring");
        assert_eq!(shell.active_workspace_id(), WorkspaceId::new(2));
        assert_eq!(shell.active_pane().launch().program(), "top");
        assert_eq!(shell.active_pane().launch().args(), &["-d", "1"]);
    }

    #[test]
    fn action_switch_to_workspace_without_name_creates_random_workspace_name() {
        let mut shell = AppShell::new(PaneLaunch::local("pwsh"));

        shell
            .apply_action(AppAction::SwitchToWorkspace {
                name: None,
                launch: None,
            })
            .unwrap();

        let first_name = shell.active_workspace().name().to_owned();
        assert_ne!(first_name, "workspace-2");
        assert_eq!(shell.active_workspace_id(), WorkspaceId::new(2));
        assert_eq!(shell.active_pane().launch().program(), "pwsh");

        shell
            .apply_action(AppAction::SwitchToWorkspace {
                name: None,
                launch: Some(PaneLaunch::local("top")),
            })
            .unwrap();

        assert_eq!(shell.workspaces().len(), 3);
        assert_ne!(shell.active_workspace().name(), first_name);
        assert_ne!(shell.active_workspace().name(), "workspace-3");
        assert_eq!(shell.active_workspace_id(), WorkspaceId::new(3));
        assert_eq!(shell.active_pane().launch().program(), "top");
    }

    #[test]
    fn action_switch_workspace_relative_wraps_and_sorts_by_name() {
        let mut shell = AppShell::new(PaneLaunch::local("pwsh"));
        shell
            .apply_action(AppAction::NewWorkspace {
                name: "omega".to_owned(),
                launch: None,
            })
            .unwrap();
        shell
            .apply_action(AppAction::NewWorkspace {
                name: "alpha".to_owned(),
                launch: None,
            })
            .unwrap();
        shell
            .apply_action(AppAction::SwitchWorkspace {
                workspace: WorkspaceId::new(1),
            })
            .unwrap();

        shell
            .apply_action(AppAction::SwitchWorkspaceRelative { offset: 1 })
            .unwrap();
        assert_eq!(shell.active_workspace().name(), "omega");
        assert_eq!(shell.active_workspace_id(), WorkspaceId::new(2));

        shell
            .apply_action(AppAction::SwitchWorkspaceRelative { offset: 1 })
            .unwrap();
        assert_eq!(shell.active_workspace().name(), "alpha");

        shell
            .apply_action(AppAction::SwitchWorkspaceRelative { offset: -2 })
            .unwrap();
        assert_eq!(shell.active_workspace().name(), "default");

        shell
            .apply_action(AppAction::SwitchWorkspaceRelative { offset: 0 })
            .unwrap();
        assert_eq!(shell.active_workspace().name(), "default");
    }

    #[test]
    fn action_rename_workspace_updates_name() {
        let mut shell = AppShell::new(PaneLaunch::local("pwsh"));

        shell
            .apply_action(AppAction::RenameWorkspace {
                workspace: WorkspaceId::new(1),
                name: "main".to_owned(),
            })
            .unwrap();

        assert_eq!(shell.active_workspace().name(), "main");
    }

    #[test]
    fn action_rename_workspace_rejects_invalid_workspace() {
        let mut shell = AppShell::new(PaneLaunch::local("pwsh"));

        let error = shell
            .apply_action(AppAction::RenameWorkspace {
                workspace: WorkspaceId::new(999),
                name: "none".to_owned(),
            })
            .unwrap_err();

        assert_eq!(
            error,
            AppShellError::InvalidWorkspace(WorkspaceId::new(999))
        );
    }

    #[test]
    fn action_close_workspace_removes_active_workspace() {
        let mut shell = AppShell::new(PaneLaunch::local("pwsh"));

        shell
            .apply_action(AppAction::NewWorkspace {
                name: "ops".to_owned(),
                launch: None,
            })
            .unwrap();
        shell
            .apply_action(AppAction::NewWorkspace {
                name: "staging".to_owned(),
                launch: None,
            })
            .unwrap();

        assert_eq!(shell.workspaces().len(), 3);
        assert_eq!(shell.active_workspace().name(), "staging");

        shell
            .apply_action(AppAction::CloseWorkspace {
                workspace: shell.active_workspace_id(),
            })
            .unwrap();

        assert_eq!(shell.workspaces().len(), 2);
        assert_eq!(shell.active_workspace_id(), WorkspaceId::new(2));
        assert_eq!(shell.active_workspace().name(), "ops");
    }

    #[test]
    fn action_close_workspace_rejects_last_workspace() {
        let mut shell = AppShell::new(PaneLaunch::local("pwsh"));

        let error = shell
            .apply_action(AppAction::CloseWorkspace {
                workspace: WorkspaceId::new(1),
            })
            .unwrap_err();

        assert_eq!(error, AppShellError::CannotCloseLastWorkspace);
    }

    #[test]
    fn ssh_pane_launch_exposes_typed_domain_without_retaining_secrets() {
        let launch = PaneLaunch::ssh(
            SshPaneLaunch::new(
                "ops@example.com",
                SshAuthDescription::PasswordPrompt,
                SshKnownHostsPolicy::Prompt,
            )
            .with_remote_command(["uname", "-a"]),
        );

        let PaneLaunchDomain::Ssh(ssh) = launch.domain() else {
            panic!("expected SSH pane launch domain");
        };

        assert_eq!(ssh.target(), "ops@example.com");
        assert_eq!(ssh.auth(), &SshAuthDescription::PasswordPrompt);
        assert_eq!(ssh.known_hosts_policy(), SshKnownHostsPolicy::Prompt);
        assert_eq!(ssh.remote_command(), ["uname", "-a"]);
        assert!(!format!("{launch:?}").contains("password"));
    }

    #[test]
    fn local_pane_launch_remains_compatible_with_existing_accessors() {
        let launch = PaneLaunch::local("pwsh")
            .with_args(["-NoLogo"])
            .with_cwd("C:/work")
            .with_environment([(String::from("MODE"), String::from("test"))]);

        assert!(matches!(launch.domain(), PaneLaunchDomain::Local));
        assert_eq!(launch.program(), "pwsh");
        assert_eq!(launch.args(), ["-NoLogo"]);
        assert_eq!(launch.cwd(), Some("C:/work"));
        assert_eq!(launch.environment().get("MODE"), Some(&"test".to_owned()));
    }

    #[test]
    fn ssh_child_launch_preserves_target_auth_policy_but_defaults_to_shell() {
        let launch = PaneLaunch::ssh(
            SshPaneLaunch::openssh(
                "prod",
                SshAuthDescription::Agent,
                SshKnownHostsPolicy::Prompt,
            )
            .with_target_overrides(Some("ops".to_owned()), Some(2222))
            .with_remote_command(["uname", "-a"]),
        );

        let child = launch.for_child_pane();
        let PaneLaunchDomain::Ssh(child) = child.domain() else {
            panic!("expected SSH child launch domain");
        };
        assert_eq!(child.target(), "prod");
        assert_eq!(child.target_kind(), SshTargetKind::OpenSsh);
        assert_eq!(child.username(), Some("ops"));
        assert_eq!(child.port(), Some(2222));
        assert_eq!(child.auth(), &SshAuthDescription::Agent);
        assert_eq!(child.known_hosts_policy(), SshKnownHostsPolicy::Prompt);
        assert!(child.remote_command().is_empty());
    }
}
