impl NativeWindowApp {
    fn dispatch_tab_command(&mut self, command: &WindowCommand) -> Result<bool, AppShellError> {
        match command {
            WindowCommand::DuplicateTab => {
                self.dispatch_duplicate_tab()?;
            }
            WindowCommand::ReopenClosedTab => {
                self.dispatch_reopen_closed_tab()?;
            }
            WindowCommand::CloseOtherTabs => {
                self.dispatch_close_other_tabs()?;
            }
            WindowCommand::CloseTabsToRight => {
                self.dispatch_close_tabs_to_right()?;
            }
            WindowCommand::MoveTabToWindow(target_window_id) => {
                let Some(event_proxy) = self.event_proxy.as_ref() else {
                    return Err(AppShellError::UnsupportedAction);
                };
                event_proxy
                    .send_event(WindowUserEvent::MoveTabToWindow {
                        source_window_id: self.app_window_id,
                        target_window_id: *target_window_id,
                        tab: self.app_shell.active_tab_id(),
                        target_index: usize::MAX,
                    })
                    .map_err(|_| AppShellError::UnsupportedAction)?;
            }
            WindowCommand::MoveTabToNewWindow => {
                self.dispatch_app_action(AppAction::MoveTabToNewWindow {
                    tab: self.app_shell.active_tab_id(),
                })?;
            }
            _ => return Ok(false),
        }
        Ok(true)
    }

    fn dispatch_close_tab_action(
        &mut self,
        tab: rterm_core::TabId,
        switch_to_last_active: bool,
    ) -> Result<(), AppShellError> {
        let selection = if switch_to_last_active {
            CloseTabSelection::LastActive
        } else {
            self.close_tab_selection
        };
        self.dispatch_close_tab_with_selection(tab, selection)
    }

    fn dispatch_close_tab_with_selection(
        &mut self,
        tab: rterm_core::TabId,
        selection: CloseTabSelection,
    ) -> Result<(), AppShellError> {
        let snapshot = self.app_shell.tab_reconnect_snapshot(tab).ok();
        let origin_workspace_id = self.app_shell.active_workspace_id();
        let origin_window_id = self.app_window_id;
        let origin_index = self
            .app_shell
            .active_workspace()
            .tabs()
            .iter()
            .position(|candidate| candidate.id() == tab)
            .unwrap_or_default();
        match self.dispatch_shell_action(AppAction::CloseTabWithSelection { tab, selection }) {
            Ok(()) => {
                if let Some(snapshot) = snapshot {
                    self.closed_tab_history
                        .lock()
                        .expect("closed-tab history lock is not poisoned")
                        .push(ClosedTabEntry::new(
                        snapshot,
                        origin_window_id,
                        origin_workspace_id,
                        origin_index,
                    ));
                }
                Ok(())
            }
            Err(AppShellError::CannotCloseLastTab) => {
                self.request_window_close();
                Ok(())
            }
            Err(error) => Err(error),
        }
    }

    fn dispatch_duplicate_tab(&mut self) -> Result<(), AppShellError> {
        let previous_active_pane = self.app_shell.active_pane_id();
        let active_tab = self.app_shell.active_tab_id();
        let previous_shell = self.app_shell.clone();
        let pointer_transient = self.pointer_transient_state();
        self.end_pointer_modes_for_pane_change();
        let previous_runtime = self.take_active_runtime();
        if let Err(error) = self.app_shell.duplicate_tab(active_tab) {
            self.app_shell = previous_shell;
            self.install_active_runtime(previous_runtime);
            self.restore_pointer_transient_state(pointer_transient);
            self.apply_window_title();
            return Err(error);
        }
        self.sync_pane_runtimes(previous_active_pane, previous_runtime);
        self.apply_window_title();
        Ok(())
    }

    fn dispatch_reopen_closed_tab(&mut self) -> Result<(), AppShellError> {
        let Some(entry) = self
            .closed_tab_history
            .lock()
            .expect("closed-tab history lock is not poisoned")
            .pop()
        else {
            return Ok(());
        };
        if self
            .app_shell
            .workspaces()
            .iter()
            .any(|workspace| workspace.id() == entry.origin_workspace_id())
        {
            self.app_shell.apply_action(AppAction::SwitchWorkspace {
                workspace: entry.origin_workspace_id(),
            })?;
        }

        let previous_active_pane = self.app_shell.active_pane_id();
        let previous_shell = self.app_shell.clone();
        let pointer_transient = self.pointer_transient_state();
        self.end_pointer_modes_for_pane_change();
        let previous_runtime = self.take_active_runtime();
        if let Err(error) = self
            .app_shell
            .restore_tab_snapshot(entry.snapshot().clone(), entry.origin_index())
        {
            self.app_shell = previous_shell;
            self.install_active_runtime(previous_runtime);
            self.restore_pointer_transient_state(pointer_transient);
            self.closed_tab_history
                .lock()
                .expect("closed-tab history lock is not poisoned")
                .push(entry);
            self.apply_window_title();
            return Err(error);
        }
        self.sync_pane_runtimes(previous_active_pane, previous_runtime);
        self.apply_window_title();
        Ok(())
    }

    fn dispatch_close_other_tabs(&mut self) -> Result<(), AppShellError> {
        let active = self.app_shell.active_tab_id();
        let tabs = self
            .app_shell
            .active_workspace()
            .tabs()
            .iter()
            .map(rterm_core::app_shell::Tab::id)
            .filter(|tab| *tab != active)
            .collect::<Vec<_>>();
        self.dispatch_close_tab_set(tabs)
    }

    fn dispatch_close_tabs_to_right(&mut self) -> Result<(), AppShellError> {
        let active = self.app_shell.active_tab_id();
        let tabs = self
            .app_shell
            .active_workspace()
            .tabs()
            .iter()
            .skip_while(|tab| tab.id() != active)
            .skip(1)
            .map(rterm_core::app_shell::Tab::id)
            .collect::<Vec<_>>();
        self.dispatch_close_tab_set(tabs)
    }

    fn dispatch_close_tab_set(
        &mut self,
        tabs: Vec<rterm_core::TabId>,
    ) -> Result<(), AppShellError> {
        if tabs.is_empty() {
            return Ok(());
        }

        let target = WindowCloseTarget::Tabs(tabs);
        if self.should_skip_close_confirmation(&target) {
            if let WindowCloseTarget::Tabs(tabs) = target {
                self.dispatch_close_tab_set_without_confirmation(tabs)?;
            }
        } else {
            self.enter_close_confirmation_mode(target);
        }
        Ok(())
    }

    fn dispatch_close_tab_set_without_confirmation(
        &mut self,
        tabs: Vec<rterm_core::TabId>,
    ) -> Result<(), AppShellError> {
        for tab in tabs {
            self.dispatch_close_tab_with_selection(tab, self.close_tab_selection)?;
        }
        Ok(())
    }

    fn dispatch_close_pane_action(&mut self, pane: rterm_core::PaneId) -> Result<(), AppShellError> {
        if let Some(runtime) = self.runtime.worker_mut() {
            let _ = runtime.begin_close_by_pane(pane, Duration::from_millis(250));
        }
        match self.dispatch_shell_action(AppAction::ClosePane { pane }) {
            Ok(()) => {
                self.sync_pane_runtimes_to_layout_logged();
                Ok(())
            }
            Err(AppShellError::CannotCloseLastPane | AppShellError::CannotCloseLastTab) => {
                self.request_window_close();
                Ok(())
            }
            Err(error) => Err(error),
        }
    }
}
