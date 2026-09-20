fn tab_bar_tab_label_segments(
    position: usize,
    tab_id: rssh_core::TabId,
    pane_count: usize,
    active: bool,
    title: Option<&str>,
    progress: PaneProgress,
    options: TabBarTabLabelOptions,
) -> TabBarTabLabelSegments {
    let marker = if active { "*" } else { "" };
    let progress = pane_progress_label(progress);
    let prefix = if options.show_tab_index {
        let display_index = if options.zero_based_tab_index {
            position
        } else {
            position + 1
        };
        format!("{display_index}:{}{}", tab_id.get(), marker)
    } else {
        format!("{}{}", tab_id.get(), marker)
    };
    let suffix = if options.show_close_button {
        " x "
    } else {
        " "
    };
    match title {
        Some(title) if !progress.is_empty() => TabBarTabLabelSegments {
            prefix: format!(" {prefix} panes:{pane_count} {progress} "),
            title: title.to_owned(),
            suffix: suffix.to_owned(),
        },
        Some(title) => TabBarTabLabelSegments {
            prefix: format!(" {prefix} panes:{pane_count} "),
            title: title.to_owned(),
            suffix: suffix.to_owned(),
        },
        None if !progress.is_empty() => TabBarTabLabelSegments {
            prefix: format!(" {prefix} panes:{pane_count} "),
            title: progress,
            suffix: suffix.to_owned(),
        },
        None => TabBarTabLabelSegments {
            prefix: format!(
                " {prefix} panes:{pane_count}{}",
                if options.show_close_button {
                    " x "
                } else {
                    " "
                }
            ),
            title: String::new(),
            suffix: String::new(),
        },
    }
}

struct BadgeInterpolationContext<'a> {
    user_vars: &'a std::collections::HashMap<String, String>,
    session_id: u64,
    session_termid: &'a str,
    session_process_id: Option<u32>,
    session_tty_name: Option<&'a str>,
    session_name: Option<&'a str>,
    session_job_name: &'a str,
    session_command_line: &'a str,
    session_last_command: Option<&'a str>,
    session_home_directory: Option<&'a str>,
    profile_name: Option<&'a str>,
    session_username: Option<&'a str>,
    session_hostname: Option<&'a str>,
    session_shell: Option<&'a str>,
    session_uname: &'a str,
    session_path: Option<&'a str>,
    terminal_icon_name: Option<&'a str>,
    terminal_window_name: Option<&'a str>,
    iterm2_pid: u32,
    iterm2_localhost_name: Option<&'a str>,
    iterm2_effective_theme: &'a str,
    window_id: u64,
    window_style: &'a str,
    window_frame: NativeWindowFrame,
    window_is_hotkey_window: bool,
    window_title_override: Option<&'a str>,
    tab_id: u64,
    tab_current_session_id: u64,
    tab_current_session_process_id: Option<u32>,
    tab_current_session_tty_name: Option<&'a str>,
    tab_current_session_name: Option<&'a str>,
    tab_current_session_job_name: Option<&'a str>,
    tab_current_session_command_line: Option<&'a str>,
    tab_current_session_last_command: Option<&'a str>,
    tab_current_session_home_directory: Option<&'a str>,
    tab_current_session_username: Option<&'a str>,
    tab_current_session_hostname: Option<&'a str>,
    tab_current_session_shell: Option<&'a str>,
    tab_current_session_uname: &'a str,
    tab_current_session_terminal_icon_name: Option<&'a str>,
    tab_current_session_terminal_window_name: Option<&'a str>,
    tab_current_session_path: Option<&'a str>,
    tab_current_session_profile_name: Option<&'a str>,
    tab_current_session_mouse_reporting_mode: i16,
    tab_current_session_mouse_info: Option<ItermMouseInfo>,
    tab_current_session_application_keypad: bool,
    tab_current_session_bell_count: u64,
    tab_current_session_columns: u16,
    tab_current_session_rows: u16,
    tab_current_session_selection: Option<&'a str>,
    tab_title: Option<&'a str>,
    tab_title_override: Option<&'a str>,
    session_selection: Option<&'a str>,
    session_mouse_reporting_mode: i16,
    session_mouse_info: Option<ItermMouseInfo>,
    session_application_keypad: bool,
    session_bell_count: u64,
    session_columns: u16,
    session_rows: u16,
}

const BADGE_USER_VAR_PREFIX: &str = "\\(user.";
const PROFILE_NAME_ENV: &str = "RSSH_PROFILE";
const SSH_AUTH_SOCK_ENV: &str = "SSH_AUTH_SOCK";
const ITERM2_EFFECTIVE_THEME: &str = "dark";
const ITERM_MOUSE_REPORT_SIDE_EFFECT: u16 = 8;
const ITERM_MOUSE_DRAG_SIDE_EFFECT: u16 = 128;
const ITERM_MOUSE_CONTROL_MODIFIER: u8 = 1 << 0;
const ITERM_MOUSE_OPTION_MODIFIER: u8 = 1 << 1;
const ITERM_MOUSE_COMMAND_MODIFIER: u8 = 1 << 2;
const ITERM_MOUSE_SHIFT_MODIFIER: u8 = 1 << 3;
const BADGE_SESSION_ID_VARIABLE: &str = "\\(session.id)";
const BADGE_SESSION_TERMID_VARIABLE: &str = "\\(session.termid)";
const BADGE_SESSION_PID_VARIABLE: &str = "\\(session.pid)";
const BADGE_SESSION_JOB_PID_VARIABLE: &str = "\\(session.jobPid)";
const BADGE_SESSION_TTY_VARIABLE: &str = "\\(session.tty)";
const BADGE_SESSION_AUTO_NAME_VARIABLE: &str = "\\(session.autoName)";
const BADGE_SESSION_AUTO_NAME_FORMAT_VARIABLE: &str = "\\(session.autoNameFormat)";
const BADGE_SESSION_NAME_VARIABLE: &str = "\\(session.name)";
const BADGE_SESSION_PRESENTATION_NAME_VARIABLE: &str = "\\(session.presentationName)";
const BADGE_SESSION_JOB_NAME_VARIABLE: &str = "\\(session.jobName)";
const BADGE_SESSION_PROCESS_TITLE_VARIABLE: &str = "\\(session.processTitle)";
const BADGE_SESSION_COMMAND_LINE_VARIABLE: &str = "\\(session.commandLine)";
const BADGE_SESSION_LAST_COMMAND_VARIABLE: &str = "\\(session.lastCommand)";
const BADGE_SESSION_HOME_DIRECTORY_VARIABLE: &str = "\\(session.homeDirectory)";
const BADGE_SESSION_PROFILE_NAME_VARIABLE: &str = "\\(session.profileName)";
const BADGE_SESSION_SSH_INTEGRATION_LEVEL_VARIABLE: &str = "\\(session.sshIntegrationLevel)";
const BADGE_SESSION_USERNAME_VARIABLE: &str = "\\(session.username)";
const BADGE_SESSION_HOSTNAME_VARIABLE: &str = "\\(session.hostname)";
const BADGE_SESSION_SHELL_VARIABLE: &str = "\\(session.shell)";
const BADGE_SESSION_UNAME_VARIABLE: &str = "\\(session.uname)";
const BADGE_SESSION_PATH_VARIABLE: &str = "\\(session.path)";
const BADGE_TERMINAL_ICON_NAME_VARIABLE: &str = "\\(session.terminalIconName)";
const BADGE_TERMINAL_WINDOW_NAME_VARIABLE: &str = "\\(session.terminalWindowName)";
const BADGE_ITERM2_PID_VARIABLE: &str = "\\(iterm2.pid)";
const BADGE_ITERM2_LOCALHOST_NAME_VARIABLE: &str = "\\(iterm2.localhostName)";
const BADGE_ITERM2_EFFECTIVE_THEME_VARIABLE: &str = "\\(iterm2.effectiveTheme)";
const BADGE_TAB_WINDOW_ID_VARIABLE: &str = "\\(tab.window.id)";
const BADGE_TAB_WINDOW_NUMBER_VARIABLE: &str = "\\(tab.window.number)";
const BADGE_TAB_WINDOW_FRAME_VARIABLE: &str = "\\(tab.window.frame)";
const BADGE_TAB_WINDOW_STYLE_VARIABLE: &str = "\\(tab.window.style)";
const BADGE_TAB_WINDOW_IS_HOTKEY_WINDOW_VARIABLE: &str = "\\(tab.window.isHotkeyWindow)";
const BADGE_TAB_WINDOW_ITERM2_EFFECTIVE_THEME_VARIABLE: &str =
    "\\(tab.window.iterm2.effectiveTheme)";
const BADGE_TAB_WINDOW_TITLE_OVERRIDE_FORMAT_VARIABLE: &str = "\\(tab.window.titleOverrideFormat)";
const BADGE_TAB_WINDOW_TITLE_OVERRIDE_VARIABLE: &str = "\\(tab.window.titleOverride)";
const BADGE_TAB_WINDOW_CURRENT_TAB_ID_VARIABLE: &str = "\\(tab.window.currentTab.id)";
const BADGE_TAB_WINDOW_CURRENT_TAB_TITLE_VARIABLE: &str = "\\(tab.window.currentTab.title)";
const BADGE_TAB_WINDOW_CURRENT_TAB_ITERM2_EFFECTIVE_THEME_VARIABLE: &str =
    "\\(tab.window.currentTab.iterm2.effectiveTheme)";
const BADGE_TAB_WINDOW_CURRENT_TAB_TITLE_OVERRIDE_FORMAT_VARIABLE: &str =
    "\\(tab.window.currentTab.titleOverrideFormat)";
const BADGE_TAB_WINDOW_CURRENT_TAB_TITLE_OVERRIDE_VARIABLE: &str =
    "\\(tab.window.currentTab.titleOverride)";
const BADGE_TAB_WINDOW_CURRENT_TAB_CURRENT_SESSION_ID_VARIABLE: &str =
    "\\(tab.window.currentTab.currentSession.id)";
const BADGE_TAB_WINDOW_CURRENT_TAB_CURRENT_SESSION_PID_VARIABLE: &str =
    "\\(tab.window.currentTab.currentSession.pid)";
const BADGE_TAB_WINDOW_CURRENT_TAB_CURRENT_SESSION_JOB_PID_VARIABLE: &str =
    "\\(tab.window.currentTab.currentSession.jobPid)";
const BADGE_TAB_WINDOW_CURRENT_TAB_CURRENT_SESSION_TTY_VARIABLE: &str =
    "\\(tab.window.currentTab.currentSession.tty)";
const BADGE_TAB_WINDOW_CURRENT_TAB_CURRENT_SESSION_AUTO_NAME_VARIABLE: &str =
    "\\(tab.window.currentTab.currentSession.autoName)";
const BADGE_TAB_WINDOW_CURRENT_TAB_CURRENT_SESSION_AUTO_NAME_FORMAT_VARIABLE: &str =
    "\\(tab.window.currentTab.currentSession.autoNameFormat)";
const BADGE_TAB_WINDOW_CURRENT_TAB_CURRENT_SESSION_NAME_VARIABLE: &str =
    "\\(tab.window.currentTab.currentSession.name)";
const BADGE_TAB_WINDOW_CURRENT_TAB_CURRENT_SESSION_PRESENTATION_NAME_VARIABLE: &str =
    "\\(tab.window.currentTab.currentSession.presentationName)";
const BADGE_TAB_WINDOW_CURRENT_TAB_CURRENT_SESSION_JOB_NAME_VARIABLE: &str =
    "\\(tab.window.currentTab.currentSession.jobName)";
const BADGE_TAB_WINDOW_CURRENT_TAB_CURRENT_SESSION_PROCESS_TITLE_VARIABLE: &str =
    "\\(tab.window.currentTab.currentSession.processTitle)";
const BADGE_TAB_WINDOW_CURRENT_TAB_CURRENT_SESSION_COMMAND_LINE_VARIABLE: &str =
    "\\(tab.window.currentTab.currentSession.commandLine)";
const BADGE_TAB_WINDOW_CURRENT_TAB_CURRENT_SESSION_LAST_COMMAND_VARIABLE: &str =
    "\\(tab.window.currentTab.currentSession.lastCommand)";
const BADGE_TAB_WINDOW_CURRENT_TAB_CURRENT_SESSION_HOME_DIRECTORY_VARIABLE: &str =
    "\\(tab.window.currentTab.currentSession.homeDirectory)";
const BADGE_TAB_WINDOW_CURRENT_TAB_CURRENT_SESSION_SSH_INTEGRATION_LEVEL_VARIABLE: &str =
    "\\(tab.window.currentTab.currentSession.sshIntegrationLevel)";
const BADGE_TAB_WINDOW_CURRENT_TAB_CURRENT_SESSION_USERNAME_VARIABLE: &str =
    "\\(tab.window.currentTab.currentSession.username)";
const BADGE_TAB_WINDOW_CURRENT_TAB_CURRENT_SESSION_HOSTNAME_VARIABLE: &str =
    "\\(tab.window.currentTab.currentSession.hostname)";
const BADGE_TAB_WINDOW_CURRENT_TAB_CURRENT_SESSION_SHELL_VARIABLE: &str =
    "\\(tab.window.currentTab.currentSession.shell)";
const BADGE_TAB_WINDOW_CURRENT_TAB_CURRENT_SESSION_UNAME_VARIABLE: &str =
    "\\(tab.window.currentTab.currentSession.uname)";
const BADGE_TAB_WINDOW_CURRENT_TAB_CURRENT_SESSION_PATH_VARIABLE: &str =
    "\\(tab.window.currentTab.currentSession.path)";
const BADGE_TAB_WINDOW_CURRENT_TAB_CURRENT_SESSION_PROFILE_NAME_VARIABLE: &str =
    "\\(tab.window.currentTab.currentSession.profileName)";
const BADGE_TAB_WINDOW_CURRENT_TAB_CURRENT_SESSION_TERMINAL_ICON_NAME_VARIABLE: &str =
    "\\(tab.window.currentTab.currentSession.terminalIconName)";
const BADGE_TAB_WINDOW_CURRENT_TAB_CURRENT_SESSION_TERMINAL_WINDOW_NAME_VARIABLE: &str =
    "\\(tab.window.currentTab.currentSession.terminalWindowName)";
const BADGE_TAB_WINDOW_CURRENT_TAB_CURRENT_SESSION_APPLICATION_KEYPAD_VARIABLE: &str =
    "\\(tab.window.currentTab.currentSession.applicationKeypad)";
const BADGE_TAB_WINDOW_CURRENT_TAB_CURRENT_SESSION_BELL_COUNT_VARIABLE: &str =
    "\\(tab.window.currentTab.currentSession.bellCount)";
const BADGE_TAB_WINDOW_CURRENT_TAB_CURRENT_SESSION_MOUSE_REPORTING_MODE_VARIABLE: &str =
    "\\(tab.window.currentTab.currentSession.mouseReportingMode)";
const BADGE_TAB_WINDOW_CURRENT_TAB_CURRENT_SESSION_MOUSE_INFO_VARIABLE: &str =
    "\\(tab.window.currentTab.currentSession.mouseInfo)";
const BADGE_TAB_WINDOW_CURRENT_TAB_CURRENT_SESSION_MOUSE_INFO_PREFIX: &str =
    "\\(tab.window.currentTab.currentSession.mouseInfo[";
const BADGE_TAB_WINDOW_CURRENT_TAB_CURRENT_SESSION_COLUMNS_VARIABLE: &str =
    "\\(tab.window.currentTab.currentSession.columns)";
const BADGE_TAB_WINDOW_CURRENT_TAB_CURRENT_SESSION_ROWS_VARIABLE: &str =
    "\\(tab.window.currentTab.currentSession.rows)";
const BADGE_TAB_WINDOW_CURRENT_TAB_CURRENT_SESSION_SELECTION_VARIABLE: &str =
    "\\(tab.window.currentTab.currentSession.selection)";
const BADGE_TAB_WINDOW_CURRENT_TAB_CURRENT_SESSION_SELECTION_LENGTH_VARIABLE: &str =
    "\\(tab.window.currentTab.currentSession.selectionLength)";
const BADGE_TAB_CURRENT_SESSION_ID_VARIABLE: &str = "\\(tab.currentSession.id)";
const BADGE_TAB_CURRENT_SESSION_PID_VARIABLE: &str = "\\(tab.currentSession.pid)";
const BADGE_TAB_CURRENT_SESSION_JOB_PID_VARIABLE: &str = "\\(tab.currentSession.jobPid)";
const BADGE_TAB_CURRENT_SESSION_TTY_VARIABLE: &str = "\\(tab.currentSession.tty)";
const BADGE_TAB_CURRENT_SESSION_AUTO_NAME_VARIABLE: &str = "\\(tab.currentSession.autoName)";
const BADGE_TAB_CURRENT_SESSION_AUTO_NAME_FORMAT_VARIABLE: &str =
    "\\(tab.currentSession.autoNameFormat)";
const BADGE_TAB_CURRENT_SESSION_NAME_VARIABLE: &str = "\\(tab.currentSession.name)";
const BADGE_TAB_CURRENT_SESSION_PRESENTATION_NAME_VARIABLE: &str =
    "\\(tab.currentSession.presentationName)";
const BADGE_TAB_CURRENT_SESSION_JOB_NAME_VARIABLE: &str = "\\(tab.currentSession.jobName)";
const BADGE_TAB_CURRENT_SESSION_PROCESS_TITLE_VARIABLE: &str =
    "\\(tab.currentSession.processTitle)";
const BADGE_TAB_CURRENT_SESSION_COMMAND_LINE_VARIABLE: &str = "\\(tab.currentSession.commandLine)";
const BADGE_TAB_CURRENT_SESSION_LAST_COMMAND_VARIABLE: &str = "\\(tab.currentSession.lastCommand)";
const BADGE_TAB_CURRENT_SESSION_HOME_DIRECTORY_VARIABLE: &str =
    "\\(tab.currentSession.homeDirectory)";
const BADGE_TAB_CURRENT_SESSION_SSH_INTEGRATION_LEVEL_VARIABLE: &str =
    "\\(tab.currentSession.sshIntegrationLevel)";
const BADGE_TAB_CURRENT_SESSION_USERNAME_VARIABLE: &str = "\\(tab.currentSession.username)";
const BADGE_TAB_CURRENT_SESSION_HOSTNAME_VARIABLE: &str = "\\(tab.currentSession.hostname)";
const BADGE_TAB_CURRENT_SESSION_SHELL_VARIABLE: &str = "\\(tab.currentSession.shell)";
const BADGE_TAB_CURRENT_SESSION_UNAME_VARIABLE: &str = "\\(tab.currentSession.uname)";
const BADGE_TAB_CURRENT_SESSION_PATH_VARIABLE: &str = "\\(tab.currentSession.path)";
const BADGE_TAB_CURRENT_SESSION_PROFILE_NAME_VARIABLE: &str = "\\(tab.currentSession.profileName)";
const BADGE_TAB_CURRENT_SESSION_TERMINAL_ICON_NAME_VARIABLE: &str =
    "\\(tab.currentSession.terminalIconName)";
const BADGE_TAB_CURRENT_SESSION_TERMINAL_WINDOW_NAME_VARIABLE: &str =
    "\\(tab.currentSession.terminalWindowName)";
const BADGE_TAB_CURRENT_SESSION_APPLICATION_KEYPAD_VARIABLE: &str =
    "\\(tab.currentSession.applicationKeypad)";
const BADGE_TAB_CURRENT_SESSION_BELL_COUNT_VARIABLE: &str = "\\(tab.currentSession.bellCount)";
const BADGE_TAB_CURRENT_SESSION_MOUSE_REPORTING_MODE_VARIABLE: &str =
    "\\(tab.currentSession.mouseReportingMode)";
const BADGE_TAB_CURRENT_SESSION_MOUSE_INFO_VARIABLE: &str = "\\(tab.currentSession.mouseInfo)";
const BADGE_TAB_CURRENT_SESSION_MOUSE_INFO_PREFIX: &str = "\\(tab.currentSession.mouseInfo[";
const BADGE_TAB_CURRENT_SESSION_COLUMNS_VARIABLE: &str = "\\(tab.currentSession.columns)";
const BADGE_TAB_CURRENT_SESSION_ROWS_VARIABLE: &str = "\\(tab.currentSession.rows)";
const BADGE_TAB_CURRENT_SESSION_SELECTION_VARIABLE: &str = "\\(tab.currentSession.selection)";
const BADGE_TAB_CURRENT_SESSION_SELECTION_LENGTH_VARIABLE: &str =
    "\\(tab.currentSession.selectionLength)";
const BADGE_TAB_ID_VARIABLE: &str = "\\(tab.id)";
const BADGE_TAB_ITERM2_EFFECTIVE_THEME_VARIABLE: &str = "\\(tab.iterm2.effectiveTheme)";
const BADGE_TAB_TITLE_VARIABLE: &str = "\\(tab.title)";
const BADGE_TAB_TITLE_OVERRIDE_FORMAT_VARIABLE: &str = "\\(tab.titleOverrideFormat)";
const BADGE_TAB_TITLE_OVERRIDE_VARIABLE: &str = "\\(tab.titleOverride)";
const BADGE_SESSION_APPLICATION_KEYPAD_VARIABLE: &str = "\\(session.applicationKeypad)";
const BADGE_SESSION_BELL_COUNT_VARIABLE: &str = "\\(session.bellCount)";
const BADGE_SESSION_MOUSE_REPORTING_MODE_VARIABLE: &str = "\\(session.mouseReportingMode)";
const BADGE_SESSION_MOUSE_INFO_VARIABLE: &str = "\\(session.mouseInfo)";
const BADGE_SESSION_MOUSE_INFO_PREFIX: &str = "\\(session.mouseInfo[";
const BADGE_SESSION_COLUMNS_VARIABLE: &str = "\\(session.columns)";
const BADGE_SESSION_ROWS_VARIABLE: &str = "\\(session.rows)";
const BADGE_SESSION_SELECTION_VARIABLE: &str = "\\(session.selection)";
const BADGE_SESSION_SELECTION_LENGTH_VARIABLE: &str = "\\(session.selectionLength)";

fn iterm_mouse_reporting_mode_value(mode: MouseReportingMode) -> i16 {
    match mode {
        MouseReportingMode::None => -1,
        MouseReportingMode::Normal => 0,
        MouseReportingMode::ButtonEvent => 2,
        MouseReportingMode::AnyEvent => 3,
    }
}

fn iterm_window_style_value(full_screen: bool) -> &'static str {
    if full_screen {
        "native full screen"
    } else {
        "normal"
    }
}

fn tab_current_session_path(tab: &rssh_core::app_shell::Tab) -> Option<&str> {
    tab.panes()
        .iter()
        .find(|pane| pane.id() == tab.active_pane_id())
        .and_then(|pane| pane.launch().cwd())
}

fn tab_current_session_profile_name(tab: &rssh_core::app_shell::Tab) -> Option<&str> {
    tab.panes()
        .iter()
        .find(|pane| pane.id() == tab.active_pane_id())
        .and_then(|pane| pane_launch_profile_name(pane.launch()))
}

fn tab_current_session_launch(tab: &rssh_core::app_shell::Tab) -> Option<&PaneLaunch> {
    tab.panes()
        .iter()
        .find(|pane| pane.id() == tab.active_pane_id())
        .map(rssh_core::app_shell::Pane::launch)
}

fn tab_title_override(tab: &rssh_core::app_shell::Tab) -> Option<&str> {
    tab.title().map(str::trim).filter(|title| !title.is_empty())
}

fn pane_launch_command_line(launch: &PaneLaunch) -> String {
    let (program, args): (&str, &[String]) = match launch.domain() {
        PaneLaunchDomain::Local => (launch.program(), launch.args()),
        PaneLaunchDomain::Ssh(ssh) => ("ssh", ssh.remote_command()),
    };
    let mut command_line = program.to_owned();
    for arg in args {
        command_line.push(' ');
        command_line.push_str(arg);
    }
    command_line
}

fn pane_launch_display_program(launch: &PaneLaunch) -> &str {
    match launch.domain() {
        PaneLaunchDomain::Local => launch.program(),
        PaneLaunchDomain::Ssh(_) => "ssh",
    }
}

fn pane_launch_domain_name(launch: &PaneLaunch) -> &'static str {
    match launch.domain() {
        PaneLaunchDomain::Local => "local",
        PaneLaunchDomain::Ssh(_) => "ssh",
    }
}

fn pane_launch_profile_name(launch: &PaneLaunch) -> Option<&str> {
    launch
        .environment()
        .get(PROFILE_NAME_ENV)
        .map(String::as_str)
}

fn iterm_session_auto_name_format<'a>(
    terminal_icon_name: Option<&'a str>,
    profile_name: Option<&'a str>,
) -> Option<&'a str> {
    terminal_icon_name.or(profile_name)
}

fn iterm_session_termid(window_id: u64, tab_id: u64, pane_id: u64) -> String {
    format!("w{window_id}t{tab_id}p{pane_id}")
}

fn local_home_directory() -> Option<String> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(|home| home.to_string_lossy().into_owned())
        .filter(|home| !home.is_empty())
}

fn local_user_name() -> Option<String> {
    std::env::var_os("USER")
        .or_else(|| std::env::var_os("USERNAME"))
        .or_else(|| std::env::var_os("LOGNAME"))
        .map(|user| user.to_string_lossy().into_owned())
        .filter(|user| !user.is_empty())
}

fn local_host_name() -> Option<String> {
    std::env::var_os("COMPUTERNAME")
        .or_else(|| std::env::var_os("HOSTNAME"))
        .map(|host| host.to_string_lossy().into_owned())
        .filter(|host| !host.is_empty())
}

fn local_shell() -> Option<String> {
    let shell = if cfg!(windows) {
        std::env::var_os("COMSPEC")
            .or_else(|| std::env::var_os("SHELL"))
            .or_else(|| Some("cmd.exe".into()))
    } else {
        std::env::var_os("SHELL")
            .or_else(|| std::env::var_os("COMSPEC"))
            .or_else(|| Some("/bin/sh".into()))
    };
    shell
        .map(|shell| shell.to_string_lossy().into_owned())
        .filter(|shell| !shell.is_empty())
}

fn local_uname() -> String {
    format!("{} {}", std::env::consts::OS, std::env::consts::ARCH)
}

struct BadgeHostContext {
    home_directory: Option<String>,
    username: Option<String>,
    hostname: Option<String>,
    shell: Option<String>,
    uname: String,
}

fn local_badge_host_context() -> BadgeHostContext {
    BadgeHostContext {
        home_directory: local_home_directory(),
        username: local_user_name(),
        hostname: local_host_name(),
        shell: local_shell(),
        uname: local_uname(),
    }
}

fn badge_window_title_override<'a>(
    window_title: &'a str,
    tab_title: Option<&'a str>,
) -> Option<&'a str> {
    let window_title = window_title.trim();
    if window_title.is_empty() {
        tab_title
    } else {
        Some(window_title)
    }
}

fn last_command_from_terminal(terminal: &Terminal) -> Option<String> {
    let latest_exit_row = terminal
        .stable_semantic_command_exits()
        .last()
        .map(|exit| exit.row);
    let mut input_zones = terminal
        .stable_semantic_zones()
        .into_iter()
        .rev()
        .filter(|zone| zone.semantic_type == SemanticType::Input);
    let zone = match latest_exit_row {
        Some(row) => input_zones.find(|zone| zone.start_y <= row)?,
        None => input_zones.next()?,
    };
    let domain = terminal.stable_dimensions().domain;
    let command = terminal.text_from_stable_selection(StableSelectionRange {
        start: StableSelectionCoordinate {
            domain,
            row: zone.start_y,
            column: zone.start_x,
        },
        end: StableSelectionCoordinate {
            domain,
            row: zone.end_y,
            column: zone.end_x,
        },
        rectangular: false,
    })?;
    let command = command.trim();
    (!command.is_empty()).then(|| command.to_owned())
}

fn trimmed_badge_text(badge: &str) -> Option<String> {
    let badge = badge.trim();
    (!badge.is_empty()).then(|| badge.to_owned())
}

fn push_optional_badge_value(output: &mut String, value: Option<&str>) {
    if let Some(value) = value {
        output.push_str(value);
    }
}

fn push_iterm_window_frame_array(output: &mut String, frame: NativeWindowFrame) {
    output.push('[');
    output.push_str(&frame.x.to_string());
    output.push_str(", ");
    output.push_str(&frame.y.to_string());
    output.push_str(", ");
    output.push_str(&frame.width.to_string());
    output.push_str(", ");
    output.push_str(&frame.height.to_string());
    output.push(']');
}

fn interpolate_mouse_info_array_badge_variable<'a>(
    output: &mut String,
    rest: &'a str,
    start: usize,
    variable: &str,
    mouse_info: Option<ItermMouseInfo>,
) -> Option<&'a str> {
    if !rest[start..].starts_with(variable) {
        return None;
    }
    if let Some(info) = mouse_info {
        push_iterm_mouse_info_array(output, info);
    }
    Some(&rest[start + variable.len()..])
}

fn interpolate_mouse_info_badge_variable<'a>(
    output: &mut String,
    rest: &'a str,
    start: usize,
    prefix: &str,
    mouse_info: Option<ItermMouseInfo>,
) -> Option<&'a str> {
    let candidate = &rest[start..];
    let index_text = candidate.strip_prefix(prefix)?;
    let end = index_text.find("])")?;
    let index = index_text[..end].parse::<u8>().ok()?;
    if let Some(info) = mouse_info {
        if index == 4 {
            push_iterm_mouse_modifier_array(output, info.modifier_mask);
        } else if let Some(value) = iterm_mouse_info_scalar_value(info, index) {
            output.push_str(&value.to_string());
        }
    }
    Some(&rest[start + prefix.len() + end + 2..])
}

fn push_iterm_mouse_info_array(output: &mut String, info: ItermMouseInfo) {
    output.push('[');
    output.push_str(&info.x.to_string());
    output.push_str(", ");
    output.push_str(&info.y.to_string());
    output.push_str(", ");
    output.push_str(&info.button.to_string());
    output.push_str(", ");
    output.push_str(&info.click_count.to_string());
    output.push_str(", ");
    push_iterm_mouse_modifier_array(output, info.modifier_mask);
    output.push_str(", ");
    output.push_str(&info.side_effects.to_string());
    output.push_str(", ");
    output.push_str(&info.event_type.to_string());
    output.push(']');
}

fn push_iterm_mouse_modifier_array(output: &mut String, modifier_mask: u8) {
    let modifiers = [
        (ITERM_MOUSE_CONTROL_MODIFIER, 1),
        (ITERM_MOUSE_OPTION_MODIFIER, 2),
        (ITERM_MOUSE_COMMAND_MODIFIER, 3),
        (ITERM_MOUSE_SHIFT_MODIFIER, 4),
    ];
    output.push('[');
    let mut needs_separator = false;
    for (flag, value) in modifiers {
        if modifier_mask & flag == 0 {
            continue;
        }
        if needs_separator {
            output.push_str(", ");
        }
        output.push_str(&value.to_string());
        needs_separator = true;
    }
    output.push(']');
}

const fn iterm_mouse_info_scalar_value(info: ItermMouseInfo, index: u8) -> Option<u64> {
    match index {
        0 => Some(info.x as u64),
        1 => Some(info.y as u64),
        2 => Some(info.button as u64),
        3 => Some(info.click_count as u64),
        5 => Some(info.side_effects as u64),
        6 => Some(info.event_type as u64),
        _ => None,
    }
}

fn interpolate_session_badge_variable<'a>(
    output: &mut String,
    rest: &'a str,
    start: usize,
    context: &BadgeInterpolationContext<'_>,
) -> Option<&'a str> {
    if rest[start..].starts_with(BADGE_SESSION_ID_VARIABLE) {
        output.push_str(&context.session_id.to_string());
        Some(&rest[start + BADGE_SESSION_ID_VARIABLE.len()..])
    } else if rest[start..].starts_with(BADGE_SESSION_TERMID_VARIABLE) {
        output.push_str(context.session_termid);
        Some(&rest[start + BADGE_SESSION_TERMID_VARIABLE.len()..])
    } else if rest[start..].starts_with(BADGE_SESSION_PID_VARIABLE) {
        if let Some(process_id) = context.session_process_id {
            output.push_str(&process_id.to_string());
        }
        Some(&rest[start + BADGE_SESSION_PID_VARIABLE.len()..])
    } else if rest[start..].starts_with(BADGE_SESSION_JOB_PID_VARIABLE) {
        if let Some(process_id) = context.session_process_id {
            output.push_str(&process_id.to_string());
        }
        Some(&rest[start + BADGE_SESSION_JOB_PID_VARIABLE.len()..])
    } else if rest[start..].starts_with(BADGE_SESSION_TTY_VARIABLE) {
        push_optional_badge_value(output, context.session_tty_name);
        Some(&rest[start + BADGE_SESSION_TTY_VARIABLE.len()..])
    } else if rest[start..].starts_with(BADGE_SESSION_AUTO_NAME_VARIABLE) {
        push_optional_badge_value(
            output,
            iterm_session_auto_name_format(context.terminal_icon_name, context.profile_name),
        );
        Some(&rest[start + BADGE_SESSION_AUTO_NAME_VARIABLE.len()..])
    } else if rest[start..].starts_with(BADGE_SESSION_AUTO_NAME_FORMAT_VARIABLE) {
        push_optional_badge_value(
            output,
            iterm_session_auto_name_format(context.terminal_icon_name, context.profile_name),
        );
        Some(&rest[start + BADGE_SESSION_AUTO_NAME_FORMAT_VARIABLE.len()..])
    } else if rest[start..].starts_with(BADGE_SESSION_NAME_VARIABLE) {
        push_optional_badge_value(output, context.session_name);
        Some(&rest[start + BADGE_SESSION_NAME_VARIABLE.len()..])
    } else if rest[start..].starts_with(BADGE_SESSION_PRESENTATION_NAME_VARIABLE) {
        push_optional_badge_value(output, context.session_name);
        Some(&rest[start + BADGE_SESSION_PRESENTATION_NAME_VARIABLE.len()..])
    } else if let Some(next) =
        interpolate_session_process_badge_variable(output, rest, start, context)
    {
        Some(next)
    } else if rest[start..].starts_with(BADGE_SESSION_HOME_DIRECTORY_VARIABLE) {
        push_optional_badge_value(output, context.session_home_directory);
        Some(&rest[start + BADGE_SESSION_HOME_DIRECTORY_VARIABLE.len()..])
    } else if rest[start..].starts_with(BADGE_SESSION_PROFILE_NAME_VARIABLE) {
        push_optional_badge_value(output, context.profile_name);
        Some(&rest[start + BADGE_SESSION_PROFILE_NAME_VARIABLE.len()..])
    } else if rest[start..].starts_with(BADGE_SESSION_SSH_INTEGRATION_LEVEL_VARIABLE) {
        output.push('0');
        Some(&rest[start + BADGE_SESSION_SSH_INTEGRATION_LEVEL_VARIABLE.len()..])
    } else if rest[start..].starts_with(BADGE_SESSION_USERNAME_VARIABLE) {
        push_optional_badge_value(output, context.session_username);
        Some(&rest[start + BADGE_SESSION_USERNAME_VARIABLE.len()..])
    } else if rest[start..].starts_with(BADGE_SESSION_HOSTNAME_VARIABLE) {
        push_optional_badge_value(output, context.session_hostname);
        Some(&rest[start + BADGE_SESSION_HOSTNAME_VARIABLE.len()..])
    } else if rest[start..].starts_with(BADGE_SESSION_SHELL_VARIABLE) {
        push_optional_badge_value(output, context.session_shell);
        Some(&rest[start + BADGE_SESSION_SHELL_VARIABLE.len()..])
    } else if rest[start..].starts_with(BADGE_SESSION_UNAME_VARIABLE) {
        output.push_str(context.session_uname);
        Some(&rest[start + BADGE_SESSION_UNAME_VARIABLE.len()..])
    } else if rest[start..].starts_with(BADGE_SESSION_PATH_VARIABLE) {
        push_optional_badge_value(output, context.session_path);
        Some(&rest[start + BADGE_SESSION_PATH_VARIABLE.len()..])
    } else if rest[start..].starts_with(BADGE_TERMINAL_ICON_NAME_VARIABLE) {
        push_optional_badge_value(output, context.terminal_icon_name);
        Some(&rest[start + BADGE_TERMINAL_ICON_NAME_VARIABLE.len()..])
    } else if rest[start..].starts_with(BADGE_TERMINAL_WINDOW_NAME_VARIABLE) {
        push_optional_badge_value(output, context.terminal_window_name);
        Some(&rest[start + BADGE_TERMINAL_WINDOW_NAME_VARIABLE.len()..])
    } else {
        interpolate_session_runtime_badge_variable(output, rest, start, context)
    }
}

fn interpolate_session_runtime_badge_variable<'a>(
    output: &mut String,
    rest: &'a str,
    start: usize,
    context: &BadgeInterpolationContext<'_>,
) -> Option<&'a str> {
    if rest[start..].starts_with(BADGE_SESSION_APPLICATION_KEYPAD_VARIABLE) {
        output.push_str(if context.session_application_keypad {
            "true"
        } else {
            "false"
        });
        Some(&rest[start + BADGE_SESSION_APPLICATION_KEYPAD_VARIABLE.len()..])
    } else if rest[start..].starts_with(BADGE_SESSION_BELL_COUNT_VARIABLE) {
        output.push_str(&context.session_bell_count.to_string());
        Some(&rest[start + BADGE_SESSION_BELL_COUNT_VARIABLE.len()..])
    } else if rest[start..].starts_with(BADGE_SESSION_MOUSE_REPORTING_MODE_VARIABLE) {
        output.push_str(&context.session_mouse_reporting_mode.to_string());
        Some(&rest[start + BADGE_SESSION_MOUSE_REPORTING_MODE_VARIABLE.len()..])
    } else if let Some(next) = interpolate_mouse_info_array_badge_variable(
        output,
        rest,
        start,
        BADGE_SESSION_MOUSE_INFO_VARIABLE,
        context.session_mouse_info,
    ) {
        Some(next)
    } else if let Some(next) = interpolate_mouse_info_badge_variable(
        output,
        rest,
        start,
        BADGE_SESSION_MOUSE_INFO_PREFIX,
        context.session_mouse_info,
    ) {
        Some(next)
    } else if rest[start..].starts_with(BADGE_SESSION_COLUMNS_VARIABLE) {
        output.push_str(&context.session_columns.to_string());
        Some(&rest[start + BADGE_SESSION_COLUMNS_VARIABLE.len()..])
    } else if rest[start..].starts_with(BADGE_SESSION_ROWS_VARIABLE) {
        output.push_str(&context.session_rows.to_string());
        Some(&rest[start + BADGE_SESSION_ROWS_VARIABLE.len()..])
    } else if rest[start..].starts_with(BADGE_SESSION_SELECTION_VARIABLE) {
        push_optional_badge_value(output, context.session_selection);
        Some(&rest[start + BADGE_SESSION_SELECTION_VARIABLE.len()..])
    } else if rest[start..].starts_with(BADGE_SESSION_SELECTION_LENGTH_VARIABLE) {
        output.push_str(&context.session_selection.map_or(0, str::len).to_string());
        Some(&rest[start + BADGE_SESSION_SELECTION_LENGTH_VARIABLE.len()..])
    } else {
        None
    }
}

fn interpolate_session_process_badge_variable<'a>(
    output: &mut String,
    rest: &'a str,
    start: usize,
    context: &BadgeInterpolationContext<'_>,
) -> Option<&'a str> {
    if rest[start..].starts_with(BADGE_SESSION_JOB_NAME_VARIABLE) {
        output.push_str(context.session_job_name);
        Some(&rest[start + BADGE_SESSION_JOB_NAME_VARIABLE.len()..])
    } else if rest[start..].starts_with(BADGE_SESSION_PROCESS_TITLE_VARIABLE) {
        output.push_str(context.session_job_name);
        Some(&rest[start + BADGE_SESSION_PROCESS_TITLE_VARIABLE.len()..])
    } else if rest[start..].starts_with(BADGE_SESSION_COMMAND_LINE_VARIABLE) {
        output.push_str(context.session_command_line);
        Some(&rest[start + BADGE_SESSION_COMMAND_LINE_VARIABLE.len()..])
    } else if rest[start..].starts_with(BADGE_SESSION_LAST_COMMAND_VARIABLE) {
        push_optional_badge_value(output, context.session_last_command);
        Some(&rest[start + BADGE_SESSION_LAST_COMMAND_VARIABLE.len()..])
    } else {
        None
    }
}

fn interpolate_tab_window_current_tab_current_session_badge_variable<'a>(
    output: &mut String,
    rest: &'a str,
    start: usize,
    context: &BadgeInterpolationContext<'_>,
) -> Option<&'a str> {
    if rest[start..].starts_with(BADGE_TAB_WINDOW_CURRENT_TAB_CURRENT_SESSION_ID_VARIABLE) {
        output.push_str(&context.tab_current_session_id.to_string());
        Some(&rest[start + BADGE_TAB_WINDOW_CURRENT_TAB_CURRENT_SESSION_ID_VARIABLE.len()..])
    } else if rest[start..].starts_with(BADGE_TAB_WINDOW_CURRENT_TAB_CURRENT_SESSION_PID_VARIABLE) {
        if let Some(process_id) = context.tab_current_session_process_id {
            output.push_str(&process_id.to_string());
        }
        Some(&rest[start + BADGE_TAB_WINDOW_CURRENT_TAB_CURRENT_SESSION_PID_VARIABLE.len()..])
    } else if rest[start..]
        .starts_with(BADGE_TAB_WINDOW_CURRENT_TAB_CURRENT_SESSION_JOB_PID_VARIABLE)
    {
        if let Some(process_id) = context.tab_current_session_process_id {
            output.push_str(&process_id.to_string());
        }
        Some(&rest[start + BADGE_TAB_WINDOW_CURRENT_TAB_CURRENT_SESSION_JOB_PID_VARIABLE.len()..])
    } else if rest[start..].starts_with(BADGE_TAB_WINDOW_CURRENT_TAB_CURRENT_SESSION_TTY_VARIABLE) {
        push_optional_badge_value(output, context.tab_current_session_tty_name);
        Some(&rest[start + BADGE_TAB_WINDOW_CURRENT_TAB_CURRENT_SESSION_TTY_VARIABLE.len()..])
    } else if let Some(next) =
        interpolate_tab_window_current_tab_current_session_name_badge_variable(
            output, rest, start, context,
        )
    {
        Some(next)
    } else if rest[start..]
        .starts_with(BADGE_TAB_WINDOW_CURRENT_TAB_CURRENT_SESSION_JOB_NAME_VARIABLE)
    {
        push_optional_badge_value(output, context.tab_current_session_job_name);
        Some(&rest[start + BADGE_TAB_WINDOW_CURRENT_TAB_CURRENT_SESSION_JOB_NAME_VARIABLE.len()..])
    } else if rest[start..]
        .starts_with(BADGE_TAB_WINDOW_CURRENT_TAB_CURRENT_SESSION_PROCESS_TITLE_VARIABLE)
    {
        push_optional_badge_value(output, context.tab_current_session_job_name);
        Some(
            &rest[start
                + BADGE_TAB_WINDOW_CURRENT_TAB_CURRENT_SESSION_PROCESS_TITLE_VARIABLE.len()..],
        )
    } else if rest[start..]
        .starts_with(BADGE_TAB_WINDOW_CURRENT_TAB_CURRENT_SESSION_COMMAND_LINE_VARIABLE)
    {
        push_optional_badge_value(output, context.tab_current_session_command_line);
        Some(
            &rest[start
                + BADGE_TAB_WINDOW_CURRENT_TAB_CURRENT_SESSION_COMMAND_LINE_VARIABLE.len()..],
        )
    } else if rest[start..]
        .starts_with(BADGE_TAB_WINDOW_CURRENT_TAB_CURRENT_SESSION_LAST_COMMAND_VARIABLE)
    {
        push_optional_badge_value(output, context.tab_current_session_last_command);
        Some(
            &rest[start
                + BADGE_TAB_WINDOW_CURRENT_TAB_CURRENT_SESSION_LAST_COMMAND_VARIABLE.len()..],
        )
    } else if let Some(next) =
        interpolate_tab_window_current_tab_current_session_host_badge_variable(
            output, rest, start, context,
        )
    {
        Some(next)
    } else if rest[start..].starts_with(BADGE_TAB_WINDOW_CURRENT_TAB_CURRENT_SESSION_PATH_VARIABLE)
    {
        push_optional_badge_value(output, context.tab_current_session_path);
        Some(&rest[start + BADGE_TAB_WINDOW_CURRENT_TAB_CURRENT_SESSION_PATH_VARIABLE.len()..])
    } else if rest[start..]
        .starts_with(BADGE_TAB_WINDOW_CURRENT_TAB_CURRENT_SESSION_PROFILE_NAME_VARIABLE)
    {
        push_optional_badge_value(output, context.tab_current_session_profile_name);
        Some(
            &rest[start
                + BADGE_TAB_WINDOW_CURRENT_TAB_CURRENT_SESSION_PROFILE_NAME_VARIABLE.len()..],
        )
    } else if rest[start..]
        .starts_with(BADGE_TAB_WINDOW_CURRENT_TAB_CURRENT_SESSION_TERMINAL_ICON_NAME_VARIABLE)
    {
        push_optional_badge_value(output, context.tab_current_session_terminal_icon_name);
        Some(
            &rest[start
                + BADGE_TAB_WINDOW_CURRENT_TAB_CURRENT_SESSION_TERMINAL_ICON_NAME_VARIABLE.len()..],
        )
    } else if rest[start..]
        .starts_with(BADGE_TAB_WINDOW_CURRENT_TAB_CURRENT_SESSION_TERMINAL_WINDOW_NAME_VARIABLE)
    {
        push_optional_badge_value(output, context.tab_current_session_terminal_window_name);
        Some(
            &rest[start
                + BADGE_TAB_WINDOW_CURRENT_TAB_CURRENT_SESSION_TERMINAL_WINDOW_NAME_VARIABLE
                    .len()..],
        )
    } else {
        interpolate_tab_window_current_tab_current_session_runtime_badge_variable(
            output, rest, start, context,
        )
    }
}

fn interpolate_tab_window_current_tab_current_session_name_badge_variable<'a>(
    output: &mut String,
    rest: &'a str,
    start: usize,
    context: &BadgeInterpolationContext<'_>,
) -> Option<&'a str> {
    if rest[start..].starts_with(BADGE_TAB_WINDOW_CURRENT_TAB_CURRENT_SESSION_AUTO_NAME_VARIABLE) {
        push_optional_badge_value(
            output,
            iterm_session_auto_name_format(
                context.tab_current_session_terminal_icon_name,
                context.tab_current_session_profile_name,
            ),
        );
        Some(&rest[start + BADGE_TAB_WINDOW_CURRENT_TAB_CURRENT_SESSION_AUTO_NAME_VARIABLE.len()..])
    } else if rest[start..]
        .starts_with(BADGE_TAB_WINDOW_CURRENT_TAB_CURRENT_SESSION_AUTO_NAME_FORMAT_VARIABLE)
    {
        push_optional_badge_value(
            output,
            iterm_session_auto_name_format(
                context.tab_current_session_terminal_icon_name,
                context.tab_current_session_profile_name,
            ),
        );
        Some(
            &rest[start
                + BADGE_TAB_WINDOW_CURRENT_TAB_CURRENT_SESSION_AUTO_NAME_FORMAT_VARIABLE.len()..],
        )
    } else if rest[start..].starts_with(BADGE_TAB_WINDOW_CURRENT_TAB_CURRENT_SESSION_NAME_VARIABLE)
    {
        push_optional_badge_value(output, context.tab_current_session_name);
        Some(&rest[start + BADGE_TAB_WINDOW_CURRENT_TAB_CURRENT_SESSION_NAME_VARIABLE.len()..])
    } else if rest[start..]
        .starts_with(BADGE_TAB_WINDOW_CURRENT_TAB_CURRENT_SESSION_PRESENTATION_NAME_VARIABLE)
    {
        push_optional_badge_value(output, context.tab_current_session_name);
        Some(
            &rest[start
                + BADGE_TAB_WINDOW_CURRENT_TAB_CURRENT_SESSION_PRESENTATION_NAME_VARIABLE.len()..],
        )
    } else {
        None
    }
}

fn interpolate_tab_window_current_tab_current_session_host_badge_variable<'a>(
    output: &mut String,
    rest: &'a str,
    start: usize,
    context: &BadgeInterpolationContext<'_>,
) -> Option<&'a str> {
    if rest[start..]
        .starts_with(BADGE_TAB_WINDOW_CURRENT_TAB_CURRENT_SESSION_HOME_DIRECTORY_VARIABLE)
    {
        push_optional_badge_value(output, context.tab_current_session_home_directory);
        Some(
            &rest[start
                + BADGE_TAB_WINDOW_CURRENT_TAB_CURRENT_SESSION_HOME_DIRECTORY_VARIABLE.len()..],
        )
    } else if rest[start..]
        .starts_with(BADGE_TAB_WINDOW_CURRENT_TAB_CURRENT_SESSION_SSH_INTEGRATION_LEVEL_VARIABLE)
    {
        output.push('0');
        Some(
            &rest[start
                + BADGE_TAB_WINDOW_CURRENT_TAB_CURRENT_SESSION_SSH_INTEGRATION_LEVEL_VARIABLE
                    .len()..],
        )
    } else if rest[start..]
        .starts_with(BADGE_TAB_WINDOW_CURRENT_TAB_CURRENT_SESSION_USERNAME_VARIABLE)
    {
        push_optional_badge_value(output, context.tab_current_session_username);
        Some(&rest[start + BADGE_TAB_WINDOW_CURRENT_TAB_CURRENT_SESSION_USERNAME_VARIABLE.len()..])
    } else if rest[start..]
        .starts_with(BADGE_TAB_WINDOW_CURRENT_TAB_CURRENT_SESSION_HOSTNAME_VARIABLE)
    {
        push_optional_badge_value(output, context.tab_current_session_hostname);
        Some(&rest[start + BADGE_TAB_WINDOW_CURRENT_TAB_CURRENT_SESSION_HOSTNAME_VARIABLE.len()..])
    } else if rest[start..].starts_with(BADGE_TAB_WINDOW_CURRENT_TAB_CURRENT_SESSION_SHELL_VARIABLE)
    {
        push_optional_badge_value(output, context.tab_current_session_shell);
        Some(&rest[start + BADGE_TAB_WINDOW_CURRENT_TAB_CURRENT_SESSION_SHELL_VARIABLE.len()..])
    } else if rest[start..].starts_with(BADGE_TAB_WINDOW_CURRENT_TAB_CURRENT_SESSION_UNAME_VARIABLE)
    {
        output.push_str(context.tab_current_session_uname);
        Some(&rest[start + BADGE_TAB_WINDOW_CURRENT_TAB_CURRENT_SESSION_UNAME_VARIABLE.len()..])
    } else {
        None
    }
}

fn interpolate_tab_window_current_tab_current_session_runtime_badge_variable<'a>(
    output: &mut String,
    rest: &'a str,
    start: usize,
    context: &BadgeInterpolationContext<'_>,
) -> Option<&'a str> {
    if rest[start..]
        .starts_with(BADGE_TAB_WINDOW_CURRENT_TAB_CURRENT_SESSION_APPLICATION_KEYPAD_VARIABLE)
    {
        output.push_str(if context.tab_current_session_application_keypad {
            "true"
        } else {
            "false"
        });
        Some(
            &rest[start
                + BADGE_TAB_WINDOW_CURRENT_TAB_CURRENT_SESSION_APPLICATION_KEYPAD_VARIABLE.len()..],
        )
    } else if rest[start..]
        .starts_with(BADGE_TAB_WINDOW_CURRENT_TAB_CURRENT_SESSION_BELL_COUNT_VARIABLE)
    {
        output.push_str(&context.tab_current_session_bell_count.to_string());
        Some(
            &rest[start + BADGE_TAB_WINDOW_CURRENT_TAB_CURRENT_SESSION_BELL_COUNT_VARIABLE.len()..],
        )
    } else if rest[start..]
        .starts_with(BADGE_TAB_WINDOW_CURRENT_TAB_CURRENT_SESSION_MOUSE_REPORTING_MODE_VARIABLE)
    {
        output.push_str(&context.tab_current_session_mouse_reporting_mode.to_string());
        Some(
            &rest[start
                + BADGE_TAB_WINDOW_CURRENT_TAB_CURRENT_SESSION_MOUSE_REPORTING_MODE_VARIABLE
                    .len()..],
        )
    } else if let Some(next) = interpolate_mouse_info_array_badge_variable(
        output,
        rest,
        start,
        BADGE_TAB_WINDOW_CURRENT_TAB_CURRENT_SESSION_MOUSE_INFO_VARIABLE,
        context.tab_current_session_mouse_info,
    ) {
        Some(next)
    } else if let Some(next) = interpolate_mouse_info_badge_variable(
        output,
        rest,
        start,
        BADGE_TAB_WINDOW_CURRENT_TAB_CURRENT_SESSION_MOUSE_INFO_PREFIX,
        context.tab_current_session_mouse_info,
    ) {
        Some(next)
    } else if rest[start..]
        .starts_with(BADGE_TAB_WINDOW_CURRENT_TAB_CURRENT_SESSION_COLUMNS_VARIABLE)
    {
        output.push_str(&context.tab_current_session_columns.to_string());
        Some(&rest[start + BADGE_TAB_WINDOW_CURRENT_TAB_CURRENT_SESSION_COLUMNS_VARIABLE.len()..])
    } else if rest[start..].starts_with(BADGE_TAB_WINDOW_CURRENT_TAB_CURRENT_SESSION_ROWS_VARIABLE)
    {
        output.push_str(&context.tab_current_session_rows.to_string());
        Some(&rest[start + BADGE_TAB_WINDOW_CURRENT_TAB_CURRENT_SESSION_ROWS_VARIABLE.len()..])
    } else if rest[start..]
        .starts_with(BADGE_TAB_WINDOW_CURRENT_TAB_CURRENT_SESSION_SELECTION_VARIABLE)
    {
        push_optional_badge_value(output, context.tab_current_session_selection);
        Some(&rest[start + BADGE_TAB_WINDOW_CURRENT_TAB_CURRENT_SESSION_SELECTION_VARIABLE.len()..])
    } else if rest[start..]
        .starts_with(BADGE_TAB_WINDOW_CURRENT_TAB_CURRENT_SESSION_SELECTION_LENGTH_VARIABLE)
    {
        output.push_str(
            &context
                .tab_current_session_selection
                .map_or(0, str::len)
                .to_string(),
        );
        Some(
            &rest[start
                + BADGE_TAB_WINDOW_CURRENT_TAB_CURRENT_SESSION_SELECTION_LENGTH_VARIABLE.len()..],
        )
    } else {
        None
    }
}

fn interpolate_tab_current_session_badge_variable<'a>(
    output: &mut String,
    rest: &'a str,
    start: usize,
    context: &BadgeInterpolationContext<'_>,
) -> Option<&'a str> {
    if rest[start..].starts_with(BADGE_TAB_CURRENT_SESSION_ID_VARIABLE) {
        output.push_str(&context.tab_current_session_id.to_string());
        Some(&rest[start + BADGE_TAB_CURRENT_SESSION_ID_VARIABLE.len()..])
    } else if rest[start..].starts_with(BADGE_TAB_CURRENT_SESSION_PID_VARIABLE) {
        if let Some(process_id) = context.tab_current_session_process_id {
            output.push_str(&process_id.to_string());
        }
        Some(&rest[start + BADGE_TAB_CURRENT_SESSION_PID_VARIABLE.len()..])
    } else if rest[start..].starts_with(BADGE_TAB_CURRENT_SESSION_JOB_PID_VARIABLE) {
        if let Some(process_id) = context.tab_current_session_process_id {
            output.push_str(&process_id.to_string());
        }
        Some(&rest[start + BADGE_TAB_CURRENT_SESSION_JOB_PID_VARIABLE.len()..])
    } else if rest[start..].starts_with(BADGE_TAB_CURRENT_SESSION_TTY_VARIABLE) {
        push_optional_badge_value(output, context.tab_current_session_tty_name);
        Some(&rest[start + BADGE_TAB_CURRENT_SESSION_TTY_VARIABLE.len()..])
    } else if rest[start..].starts_with(BADGE_TAB_CURRENT_SESSION_AUTO_NAME_VARIABLE) {
        push_optional_badge_value(
            output,
            iterm_session_auto_name_format(
                context.tab_current_session_terminal_icon_name,
                context.tab_current_session_profile_name,
            ),
        );
        Some(&rest[start + BADGE_TAB_CURRENT_SESSION_AUTO_NAME_VARIABLE.len()..])
    } else if rest[start..].starts_with(BADGE_TAB_CURRENT_SESSION_AUTO_NAME_FORMAT_VARIABLE) {
        push_optional_badge_value(
            output,
            iterm_session_auto_name_format(
                context.tab_current_session_terminal_icon_name,
                context.tab_current_session_profile_name,
            ),
        );
        Some(&rest[start + BADGE_TAB_CURRENT_SESSION_AUTO_NAME_FORMAT_VARIABLE.len()..])
    } else if rest[start..].starts_with(BADGE_TAB_CURRENT_SESSION_NAME_VARIABLE) {
        push_optional_badge_value(output, context.tab_current_session_name);
        Some(&rest[start + BADGE_TAB_CURRENT_SESSION_NAME_VARIABLE.len()..])
    } else if rest[start..].starts_with(BADGE_TAB_CURRENT_SESSION_PRESENTATION_NAME_VARIABLE) {
        push_optional_badge_value(output, context.tab_current_session_name);
        Some(&rest[start + BADGE_TAB_CURRENT_SESSION_PRESENTATION_NAME_VARIABLE.len()..])
    } else if let Some(next) =
        interpolate_tab_current_session_process_badge_variable(output, rest, start, context)
    {
        Some(next)
    } else if rest[start..].starts_with(BADGE_TAB_CURRENT_SESSION_HOME_DIRECTORY_VARIABLE) {
        push_optional_badge_value(output, context.tab_current_session_home_directory);
        Some(&rest[start + BADGE_TAB_CURRENT_SESSION_HOME_DIRECTORY_VARIABLE.len()..])
    } else if rest[start..].starts_with(BADGE_TAB_CURRENT_SESSION_SSH_INTEGRATION_LEVEL_VARIABLE) {
        output.push('0');
        Some(&rest[start + BADGE_TAB_CURRENT_SESSION_SSH_INTEGRATION_LEVEL_VARIABLE.len()..])
    } else if rest[start..].starts_with(BADGE_TAB_CURRENT_SESSION_USERNAME_VARIABLE) {
        push_optional_badge_value(output, context.tab_current_session_username);
        Some(&rest[start + BADGE_TAB_CURRENT_SESSION_USERNAME_VARIABLE.len()..])
    } else if rest[start..].starts_with(BADGE_TAB_CURRENT_SESSION_HOSTNAME_VARIABLE) {
        push_optional_badge_value(output, context.tab_current_session_hostname);
        Some(&rest[start + BADGE_TAB_CURRENT_SESSION_HOSTNAME_VARIABLE.len()..])
    } else if rest[start..].starts_with(BADGE_TAB_CURRENT_SESSION_SHELL_VARIABLE) {
        push_optional_badge_value(output, context.tab_current_session_shell);
        Some(&rest[start + BADGE_TAB_CURRENT_SESSION_SHELL_VARIABLE.len()..])
    } else if rest[start..].starts_with(BADGE_TAB_CURRENT_SESSION_UNAME_VARIABLE) {
        output.push_str(context.tab_current_session_uname);
        Some(&rest[start + BADGE_TAB_CURRENT_SESSION_UNAME_VARIABLE.len()..])
    } else if rest[start..].starts_with(BADGE_TAB_CURRENT_SESSION_PATH_VARIABLE) {
        push_optional_badge_value(output, context.tab_current_session_path);
        Some(&rest[start + BADGE_TAB_CURRENT_SESSION_PATH_VARIABLE.len()..])
    } else if rest[start..].starts_with(BADGE_TAB_CURRENT_SESSION_PROFILE_NAME_VARIABLE) {
        push_optional_badge_value(output, context.tab_current_session_profile_name);
        Some(&rest[start + BADGE_TAB_CURRENT_SESSION_PROFILE_NAME_VARIABLE.len()..])
    } else if rest[start..].starts_with(BADGE_TAB_CURRENT_SESSION_TERMINAL_ICON_NAME_VARIABLE) {
        push_optional_badge_value(output, context.tab_current_session_terminal_icon_name);
        Some(&rest[start + BADGE_TAB_CURRENT_SESSION_TERMINAL_ICON_NAME_VARIABLE.len()..])
    } else if rest[start..].starts_with(BADGE_TAB_CURRENT_SESSION_TERMINAL_WINDOW_NAME_VARIABLE) {
        push_optional_badge_value(output, context.tab_current_session_terminal_window_name);
        Some(&rest[start + BADGE_TAB_CURRENT_SESSION_TERMINAL_WINDOW_NAME_VARIABLE.len()..])
    } else {
        interpolate_tab_current_session_runtime_badge_variable(output, rest, start, context)
    }
}

fn interpolate_tab_current_session_runtime_badge_variable<'a>(
    output: &mut String,
    rest: &'a str,
    start: usize,
    context: &BadgeInterpolationContext<'_>,
) -> Option<&'a str> {
    if rest[start..].starts_with(BADGE_TAB_CURRENT_SESSION_APPLICATION_KEYPAD_VARIABLE) {
        output.push_str(if context.tab_current_session_application_keypad {
            "true"
        } else {
            "false"
        });
        Some(&rest[start + BADGE_TAB_CURRENT_SESSION_APPLICATION_KEYPAD_VARIABLE.len()..])
    } else if rest[start..].starts_with(BADGE_TAB_CURRENT_SESSION_BELL_COUNT_VARIABLE) {
        output.push_str(&context.tab_current_session_bell_count.to_string());
        Some(&rest[start + BADGE_TAB_CURRENT_SESSION_BELL_COUNT_VARIABLE.len()..])
    } else if rest[start..].starts_with(BADGE_TAB_CURRENT_SESSION_MOUSE_REPORTING_MODE_VARIABLE) {
        output.push_str(&context.tab_current_session_mouse_reporting_mode.to_string());
        Some(&rest[start + BADGE_TAB_CURRENT_SESSION_MOUSE_REPORTING_MODE_VARIABLE.len()..])
    } else if let Some(next) = interpolate_mouse_info_array_badge_variable(
        output,
        rest,
        start,
        BADGE_TAB_CURRENT_SESSION_MOUSE_INFO_VARIABLE,
        context.tab_current_session_mouse_info,
    ) {
        Some(next)
    } else if let Some(next) = interpolate_mouse_info_badge_variable(
        output,
        rest,
        start,
        BADGE_TAB_CURRENT_SESSION_MOUSE_INFO_PREFIX,
        context.tab_current_session_mouse_info,
    ) {
        Some(next)
    } else if rest[start..].starts_with(BADGE_TAB_CURRENT_SESSION_COLUMNS_VARIABLE) {
        output.push_str(&context.tab_current_session_columns.to_string());
        Some(&rest[start + BADGE_TAB_CURRENT_SESSION_COLUMNS_VARIABLE.len()..])
    } else if rest[start..].starts_with(BADGE_TAB_CURRENT_SESSION_ROWS_VARIABLE) {
        output.push_str(&context.tab_current_session_rows.to_string());
        Some(&rest[start + BADGE_TAB_CURRENT_SESSION_ROWS_VARIABLE.len()..])
    } else if rest[start..].starts_with(BADGE_TAB_CURRENT_SESSION_SELECTION_VARIABLE) {
        push_optional_badge_value(output, context.tab_current_session_selection);
        Some(&rest[start + BADGE_TAB_CURRENT_SESSION_SELECTION_VARIABLE.len()..])
    } else if rest[start..].starts_with(BADGE_TAB_CURRENT_SESSION_SELECTION_LENGTH_VARIABLE) {
        output.push_str(
            &context
                .tab_current_session_selection
                .map_or(0, str::len)
                .to_string(),
        );
        Some(&rest[start + BADGE_TAB_CURRENT_SESSION_SELECTION_LENGTH_VARIABLE.len()..])
    } else {
        None
    }
}

fn interpolate_tab_current_session_process_badge_variable<'a>(
    output: &mut String,
    rest: &'a str,
    start: usize,
    context: &BadgeInterpolationContext<'_>,
) -> Option<&'a str> {
    if rest[start..].starts_with(BADGE_TAB_CURRENT_SESSION_JOB_NAME_VARIABLE) {
        push_optional_badge_value(output, context.tab_current_session_job_name);
        Some(&rest[start + BADGE_TAB_CURRENT_SESSION_JOB_NAME_VARIABLE.len()..])
    } else if rest[start..].starts_with(BADGE_TAB_CURRENT_SESSION_PROCESS_TITLE_VARIABLE) {
        push_optional_badge_value(output, context.tab_current_session_job_name);
        Some(&rest[start + BADGE_TAB_CURRENT_SESSION_PROCESS_TITLE_VARIABLE.len()..])
    } else if rest[start..].starts_with(BADGE_TAB_CURRENT_SESSION_COMMAND_LINE_VARIABLE) {
        push_optional_badge_value(output, context.tab_current_session_command_line);
        Some(&rest[start + BADGE_TAB_CURRENT_SESSION_COMMAND_LINE_VARIABLE.len()..])
    } else if rest[start..].starts_with(BADGE_TAB_CURRENT_SESSION_LAST_COMMAND_VARIABLE) {
        push_optional_badge_value(output, context.tab_current_session_last_command);
        Some(&rest[start + BADGE_TAB_CURRENT_SESSION_LAST_COMMAND_VARIABLE.len()..])
    } else {
        None
    }
}

fn interpolate_known_badge_variable<'a>(
    output: &mut String,
    rest: &'a str,
    start: usize,
    context: &BadgeInterpolationContext<'_>,
) -> Option<&'a str> {
    if let Some(next) = interpolate_session_badge_variable(output, rest, start, context) {
        Some(next)
    } else if rest[start..].starts_with(BADGE_ITERM2_PID_VARIABLE) {
        output.push_str(&context.iterm2_pid.to_string());
        Some(&rest[start + BADGE_ITERM2_PID_VARIABLE.len()..])
    } else if rest[start..].starts_with(BADGE_ITERM2_LOCALHOST_NAME_VARIABLE) {
        push_optional_badge_value(output, context.iterm2_localhost_name);
        Some(&rest[start + BADGE_ITERM2_LOCALHOST_NAME_VARIABLE.len()..])
    } else if rest[start..].starts_with(BADGE_ITERM2_EFFECTIVE_THEME_VARIABLE) {
        output.push_str(context.iterm2_effective_theme);
        Some(&rest[start + BADGE_ITERM2_EFFECTIVE_THEME_VARIABLE.len()..])
    } else if rest[start..].starts_with(BADGE_TAB_WINDOW_ID_VARIABLE) {
        output.push_str(&context.window_id.to_string());
        Some(&rest[start + BADGE_TAB_WINDOW_ID_VARIABLE.len()..])
    } else if rest[start..].starts_with(BADGE_TAB_WINDOW_NUMBER_VARIABLE) {
        output.push_str(&context.window_id.to_string());
        Some(&rest[start + BADGE_TAB_WINDOW_NUMBER_VARIABLE.len()..])
    } else if rest[start..].starts_with(BADGE_TAB_WINDOW_FRAME_VARIABLE) {
        push_iterm_window_frame_array(output, context.window_frame);
        Some(&rest[start + BADGE_TAB_WINDOW_FRAME_VARIABLE.len()..])
    } else if rest[start..].starts_with(BADGE_TAB_WINDOW_STYLE_VARIABLE) {
        output.push_str(context.window_style);
        Some(&rest[start + BADGE_TAB_WINDOW_STYLE_VARIABLE.len()..])
    } else if rest[start..].starts_with(BADGE_TAB_WINDOW_IS_HOTKEY_WINDOW_VARIABLE) {
        output.push_str(if context.window_is_hotkey_window {
            "true"
        } else {
            "false"
        });
        Some(&rest[start + BADGE_TAB_WINDOW_IS_HOTKEY_WINDOW_VARIABLE.len()..])
    } else if rest[start..].starts_with(BADGE_TAB_WINDOW_ITERM2_EFFECTIVE_THEME_VARIABLE) {
        output.push_str(context.iterm2_effective_theme);
        Some(&rest[start + BADGE_TAB_WINDOW_ITERM2_EFFECTIVE_THEME_VARIABLE.len()..])
    } else if rest[start..].starts_with(BADGE_TAB_WINDOW_TITLE_OVERRIDE_FORMAT_VARIABLE) {
        push_optional_badge_value(output, context.window_title_override);
        Some(&rest[start + BADGE_TAB_WINDOW_TITLE_OVERRIDE_FORMAT_VARIABLE.len()..])
    } else if rest[start..].starts_with(BADGE_TAB_WINDOW_TITLE_OVERRIDE_VARIABLE) {
        push_optional_badge_value(output, context.window_title_override);
        Some(&rest[start + BADGE_TAB_WINDOW_TITLE_OVERRIDE_VARIABLE.len()..])
    } else if rest[start..].starts_with(BADGE_TAB_WINDOW_CURRENT_TAB_ID_VARIABLE) {
        output.push_str(&context.tab_id.to_string());
        Some(&rest[start + BADGE_TAB_WINDOW_CURRENT_TAB_ID_VARIABLE.len()..])
    } else if rest[start..].starts_with(BADGE_TAB_WINDOW_CURRENT_TAB_TITLE_OVERRIDE_FORMAT_VARIABLE)
    {
        push_optional_badge_value(output, context.tab_title_override);
        Some(&rest[start + BADGE_TAB_WINDOW_CURRENT_TAB_TITLE_OVERRIDE_FORMAT_VARIABLE.len()..])
    } else if rest[start..].starts_with(BADGE_TAB_WINDOW_CURRENT_TAB_TITLE_OVERRIDE_VARIABLE) {
        push_optional_badge_value(output, context.tab_title_override);
        Some(&rest[start + BADGE_TAB_WINDOW_CURRENT_TAB_TITLE_OVERRIDE_VARIABLE.len()..])
    } else if rest[start..].starts_with(BADGE_TAB_WINDOW_CURRENT_TAB_TITLE_VARIABLE) {
        push_optional_badge_value(output, context.tab_title);
        Some(&rest[start + BADGE_TAB_WINDOW_CURRENT_TAB_TITLE_VARIABLE.len()..])
    } else if rest[start..]
        .starts_with(BADGE_TAB_WINDOW_CURRENT_TAB_ITERM2_EFFECTIVE_THEME_VARIABLE)
    {
        output.push_str(context.iterm2_effective_theme);
        Some(&rest[start + BADGE_TAB_WINDOW_CURRENT_TAB_ITERM2_EFFECTIVE_THEME_VARIABLE.len()..])
    } else if let Some(next) = interpolate_tab_window_current_tab_current_session_badge_variable(
        output, rest, start, context,
    ) {
        Some(next)
    } else if let Some(next) =
        interpolate_tab_current_session_badge_variable(output, rest, start, context)
    {
        Some(next)
    } else if rest[start..].starts_with(BADGE_TAB_ID_VARIABLE) {
        output.push_str(&context.tab_id.to_string());
        Some(&rest[start + BADGE_TAB_ID_VARIABLE.len()..])
    } else if rest[start..].starts_with(BADGE_TAB_ITERM2_EFFECTIVE_THEME_VARIABLE) {
        output.push_str(context.iterm2_effective_theme);
        Some(&rest[start + BADGE_TAB_ITERM2_EFFECTIVE_THEME_VARIABLE.len()..])
    } else if rest[start..].starts_with(BADGE_TAB_TITLE_OVERRIDE_FORMAT_VARIABLE) {
        push_optional_badge_value(output, context.tab_title_override);
        Some(&rest[start + BADGE_TAB_TITLE_OVERRIDE_FORMAT_VARIABLE.len()..])
    } else if rest[start..].starts_with(BADGE_TAB_TITLE_OVERRIDE_VARIABLE) {
        push_optional_badge_value(output, context.tab_title_override);
        Some(&rest[start + BADGE_TAB_TITLE_OVERRIDE_VARIABLE.len()..])
    } else if rest[start..].starts_with(BADGE_TAB_TITLE_VARIABLE) {
        push_optional_badge_value(output, context.tab_title);
        Some(&rest[start + BADGE_TAB_TITLE_VARIABLE.len()..])
    } else {
        None
    }
}

fn interpolate_badge_format(badge_format: &str, context: &BadgeInterpolationContext<'_>) -> String {
    let mut output = String::new();
    let mut rest = badge_format;
    while let Some(start) = rest.find("\\(") {
        output.push_str(&rest[..start]);
        if let Some(variable) = rest[start..].strip_prefix(BADGE_USER_VAR_PREFIX) {
            let Some(end) = variable.find(')') else {
                output.push_str(&rest[start..]);
                return output;
            };

            let name = &variable[..end];
            if let Some(value) = context.user_vars.get(name) {
                output.push_str(value);
            }
            rest = &variable[end + 1..];
        } else if let Some(next) =
            interpolate_known_badge_variable(&mut output, rest, start, context)
        {
            rest = next;
        } else if let Some(end) = rest[start + 2..].find(')') {
            rest = &rest[start + 2 + end + 1..];
        } else {
            output.push_str("\\(");
            rest = &rest[start + 2..];
        }
    }
    output.push_str(rest);
    output
}

fn pane_progress_label(progress: PaneProgress) -> String {
    match progress {
        PaneProgress::None => String::new(),
        PaneProgress::Percentage(value) => format!("{value}%"),
        PaneProgress::Error(value) => format!("err:{value}%"),
        PaneProgress::Indeterminate => "~".to_owned(),
    }
}

/// Keep terminal-provided executable paths from consuming the compact tab
/// title while preserving explicit user-assigned tab titles verbatim.
fn compact_terminal_tab_title(title: &str) -> String {
    let basename = title
        .trim()
        .rsplit(['\\', '/'])
        .next()
        .unwrap_or(title.trim());
    match basename.to_ascii_lowercase().as_str() {
        "powershell.exe" | "pwsh.exe" => "PowerShell".to_owned(),
        "cmd.exe" => "Command Prompt".to_owned(),
        _ => basename.to_owned(),
    }
}

const fn tab_bar_new_tab_label() -> &'static str {
    " + "
}

fn integrated_title_button_default_tab_bar_label(
    button: NativeIntegratedTitleButton,
) -> &'static str {
    match button {
        NativeIntegratedTitleButton::Hide => " — ",
        NativeIntegratedTitleButton::Maximize => " □ ",
        NativeIntegratedTitleButton::Close => " × ",
    }
}

const fn native_pane_split_direction(direction: SplitDirection) -> rssh_native::PaneSplitDirection {
    match direction {
        SplitDirection::Left => rssh_native::PaneSplitDirection::Left,
        SplitDirection::Right => rssh_native::PaneSplitDirection::Right,
        SplitDirection::Up => rssh_native::PaneSplitDirection::Up,
        SplitDirection::Down => rssh_native::PaneSplitDirection::Down,
    }
}

const fn app_pane_split_direction(direction: rssh_native::PaneSplitDirection) -> SplitDirection {
    match direction {
        rssh_native::PaneSplitDirection::Left => SplitDirection::Left,
        rssh_native::PaneSplitDirection::Right => SplitDirection::Right,
        rssh_native::PaneSplitDirection::Up => SplitDirection::Up,
        rssh_native::PaneSplitDirection::Down => SplitDirection::Down,
    }
}

fn split_pane_source_size_delta(total_cells: u16, size: WindowSplitPaneSize) -> i16 {
    if total_cells < 3 {
        return 0;
    }

    let available_cells = total_cells.saturating_sub(1);
    let max_new_cells = total_cells.saturating_sub(2).max(1);
    let requested_new_cells = match size {
        WindowSplitPaneSize::Cells(cells) => cells,
        WindowSplitPaneSize::Percent(percent) => {
            let rounded = (u32::from(available_cells) * u32::from(percent) + 50) / 100;
            u16::try_from(rounded).unwrap_or(u16::MAX)
        }
    }
    .clamp(1, max_new_cells);
    let desired_source_cells = total_cells
        .saturating_sub(requested_new_cells)
        .saturating_sub(1);
    let default_source_cells = total_cells.saturating_sub(1) / 2;
    let delta = i32::from(desired_source_cells) - i32::from(default_source_cells);
    i16::try_from(delta).unwrap_or(if delta.is_negative() {
        i16::MIN
    } else {
        i16::MAX
    })
}

fn pane_mouse_cell(rect: PaneRenderRect, row: u16, column: u16) -> Option<PaneMouseCell> {
    if row < rect.row
        || row >= rect.row.saturating_add(rect.rows)
        || column < rect.column
        || column >= rect.column.saturating_add(rect.columns)
    {
        return None;
    }

    Some(PaneMouseCell {
        pane_id: rect.pane_id,
        row: row.saturating_sub(rect.row),
        column: column.saturating_sub(rect.column),
    })
}

fn pane_close_button_position(rect: PaneRenderRect) -> Option<(u16, u16)> {
    let column_offset = rect.columns.checked_sub(1)?;
    let row_end = rect.row.checked_add(rect.rows)?;
    let column_end = rect.column.checked_add(rect.columns)?;
    let column = rect.column.checked_add(column_offset)?;
    (rect.row < row_end && column < column_end).then_some((rect.row, column))
}

#[cfg_attr(not(test), allow(dead_code))]
fn pane_local_overlay_snapshot(
    base: TerminalRenderSnapshot,
    rect: PaneRenderRect,
    cells: &[RenderCell],
) -> TerminalRenderSnapshot {
    base.with_overlay_cells(cells.iter().filter_map(|cell| {
        let local = pane_mouse_cell(rect, cell.row, cell.column)?;
        let mut cell = cell.clone();
        cell.row = local.row;
        cell.column = local.column;
        Some(cell)
    }))
}

fn pane_inspection_cells_for_rect(lines: &[String], rect: PaneRenderRect) -> Vec<RenderCell> {
    if rect.rows == 0 || rect.columns == 0 {
        return Vec::new();
    }

    let mut cells = Vec::new();
    for (row_offset, line) in lines.iter().take(usize::from(rect.rows)).enumerate() {
        let row_offset = u16::try_from(row_offset).unwrap_or(u16::MAX);
        let row = rect.row.saturating_add(row_offset);
        let mut column_offset = 0_u16;
        for grapheme in line.graphemes(true) {
            let width = UnicodeWidthStr::width(grapheme);
            if width == 0 {
                continue;
            }
            let width = u16::try_from(width).unwrap_or(u16::MAX);
            let Some(end_offset) = column_offset.checked_add(width) else {
                break;
            };
            if end_offset > rect.columns {
                break;
            }
            let column = rect.column.saturating_add(column_offset);
            cells.push(ui_render_cell(
                row,
                column,
                grapheme.nfc().next().unwrap_or(' '),
                PANE_INSPECTION_FOREGROUND,
                PANE_INSPECTION_BACKGROUND,
                true,
            ));
            for continuation_offset in column_offset.saturating_add(1)..end_offset {
                cells.push(ui_render_cell(
                    row,
                    rect.column.saturating_add(continuation_offset),
                    ' ',
                    PANE_INSPECTION_FOREGROUND,
                    PANE_INSPECTION_BACKGROUND,
                    true,
                ));
            }
            column_offset = end_offset;
        }
        for trailing_offset in column_offset..rect.columns {
            cells.push(ui_render_cell(
                row,
                rect.column.saturating_add(trailing_offset),
                ' ',
                PANE_INSPECTION_FOREGROUND,
                PANE_INSPECTION_BACKGROUND,
                true,
            ));
        }
    }
    cells
}

fn split_resize_drag(
    separator: PaneSeparator,
    row: u16,
    column: u16,
) -> Option<PaneSplitResizeDrag> {
    if row < separator.row
        || row >= separator.row.saturating_add(separator.rows)
        || column < separator.column
        || column >= separator.column.saturating_add(separator.columns)
    {
        return None;
    }

    Some(PaneSplitResizeDrag {
        pane_id: separator.new_pane,
        direction: separator.direction,
        last_row: row.saturating_sub(TAB_BAR_ROWS),
        last_column: column,
    })
}

fn split_resize_cursor_icon(direction: SplitDirection) -> CursorIcon {
    match direction {
        SplitDirection::Left | SplitDirection::Right => CursorIcon::EwResize,
        SplitDirection::Up | SplitDirection::Down => CursorIcon::NsResize,
    }
}

fn write_tab_bar_segment(
    cells: &mut [RenderCell],
    column: &mut u16,
    text: &str,
    foreground: Color,
    background: Color,
    bold: bool,
) {
    for ch in text.chars() {
        let index = usize::from(*column);
        let Some(cell) = cells.get_mut(index) else {
            return;
        };

        *cell = tab_bar_render_cell(*column, ch, foreground, background, bold);
        *column = column.saturating_add(1);
    }
}

fn write_tab_bar_ansi_segment(
    cells: &mut [RenderCell],
    column: &mut u16,
    text: &str,
    base_style: TabBarSegmentStyle,
) {
    let mut style = base_style;
    write_tab_bar_ansi_styled_segment(cells, column, text, base_style, &mut style);
}

fn write_tab_bar_ansi_styled_segment(
    cells: &mut [RenderCell],
    column: &mut u16,
    text: &str,
    base_style: TabBarSegmentStyle,
    style: &mut TabBarSegmentStyle,
) {
    for (ch, cell_style) in tab_bar_ansi_text_cells_with_style(text, base_style, style) {
        let index = usize::from(*column);
        let Some(cell) = cells.get_mut(index) else {
            return;
        };

        *cell = tab_bar_styled_render_cell(*column, ch, cell_style);
        *column = column.saturating_add(1);
    }
}

fn tab_bar_ansi_visible_width(text: &str) -> usize {
    tab_bar_ansi_text_cells(
        text,
        tab_bar_segment_style(Color::Default, Color::Default, false),
    )
    .len()
}

fn tab_bar_ansi_plain_text(text: &str) -> String {
    tab_bar_ansi_text_cells(
        text,
        tab_bar_segment_style(Color::Default, Color::Default, false),
    )
    .into_iter()
    .map(|(ch, _)| ch)
    .collect()
}

fn tab_bar_truncate_right(text: &str, max_width: usize) -> String {
    let cells = tab_bar_ansi_text_cells(
        text,
        tab_bar_segment_style(Color::Default, Color::Default, false),
    );
    if cells.len() <= max_width {
        return text.to_owned();
    }

    cells
        .into_iter()
        .take(max_width)
        .map(|(ch, _)| ch)
        .collect()
}

fn tab_bar_truncate_left(text: &str, max_width: usize) -> String {
    let cells = tab_bar_ansi_text_cells(
        text,
        tab_bar_segment_style(Color::Default, Color::Default, false),
    );
    if cells.len() <= max_width {
        return text.to_owned();
    }

    cells
        .into_iter()
        .rev()
        .take(max_width)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .map(|(ch, _)| ch)
        .collect()
}

fn tab_bar_ansi_text_cells(
    text: &str,
    base_style: TabBarSegmentStyle,
) -> Vec<(char, TabBarSegmentStyle)> {
    let mut style = base_style;
    tab_bar_ansi_text_cells_with_style(text, base_style, &mut style)
}

fn tab_bar_ansi_text_cells_with_style(
    text: &str,
    base_style: TabBarSegmentStyle,
    style: &mut TabBarSegmentStyle,
) -> Vec<(char, TabBarSegmentStyle)> {
    let mut cells = Vec::new();
    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\x1b' && chars.peek() == Some(&'[') {
            chars.next();
            let mut parameters = String::new();
            for control in chars.by_ref() {
                if control == 'm' {
                    apply_tab_bar_sgr(style, base_style, &parameters);
                    break;
                }
                if control.is_ascii_alphabetic() {
                    break;
                }
                parameters.push(control);
            }
            continue;
        }

        cells.push((ch, *style));
    }

    cells
}

fn apply_tab_bar_sgr(
    style: &mut TabBarSegmentStyle,
    base_style: TabBarSegmentStyle,
    parameters: &str,
) {
    if parameters.is_empty() {
        *style = base_style;
        return;
    }

    let parameters = tab_bar_sgr_parameters(parameters);
    let mut index = 0;
    while index < parameters.len() {
        let code = parameters[index].value;
        index += 1;

        match code {
            0 => *style = base_style,
            1 => {
                style.bold = true;
                style.faint = false;
            }
            2 => {
                style.bold = false;
                style.faint = true;
            }
            3 => style.italic = true,
            4 => {
                style.underline_style = tab_bar_sgr_underline_style(&parameters, &mut index)
                    .unwrap_or(UnderlineStyle::Single);
            }
            5 => {
                style.blink = true;
                style.rapid_blink = false;
            }
            6 => {
                style.blink = true;
                style.rapid_blink = true;
            }
            7 => style.inverse = true,
            8 => style.conceal = true,
            9 => style.strikethrough = true,
            21 => style.underline_style = UnderlineStyle::Double,
            22 => {
                style.bold = false;
                style.faint = false;
            }
            23 => style.italic = false,
            24 => style.underline_style = UnderlineStyle::None,
            25 => {
                style.blink = false;
                style.rapid_blink = false;
            }
            27 => style.inverse = false,
            28 => style.conceal = false,
            29 => style.strikethrough = false,
            30..=37 => style.foreground = Color::Indexed(u8::try_from(code - 30).unwrap_or(0)),
            39 => style.foreground = base_style.foreground,
            40..=47 => style.background = Color::Indexed(u8::try_from(code - 40).unwrap_or(0)),
            49 => style.background = base_style.background,
            53 => style.overline = true,
            55 => style.overline = false,
            58 => {
                if let Some(color) = tab_bar_sgr_extended_color(&parameters, &mut index) {
                    style.underline_color = color;
                }
            }
            59 => style.underline_color = base_style.underline_color,
            38 | 48 => {
                if let Some(color) = tab_bar_sgr_extended_color(&parameters, &mut index) {
                    if code == 38 {
                        style.foreground = color;
                    } else {
                        style.background = color;
                    }
                }
            }
            90..=97 => {
                style.foreground = Color::Indexed(u8::try_from(code - 90 + 8).unwrap_or(0));
            }
            100..=107 => {
                style.background = Color::Indexed(u8::try_from(code - 100 + 8).unwrap_or(0));
            }
            _ => {}
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TabBarSgrParameter {
    separator: Option<char>,
    value: u16,
}

fn tab_bar_sgr_parameters(parameters: &str) -> Vec<TabBarSgrParameter> {
    let mut parsed = Vec::new();
    let mut current = String::new();
    let mut separator = None;
    for ch in parameters.chars() {
        if matches!(ch, ';' | ':') {
            parsed.push(TabBarSgrParameter {
                separator,
                value: if current.is_empty() {
                    0
                } else if let Ok(value) = current.parse::<u16>() {
                    value
                } else {
                    current.clear();
                    separator = Some(ch);
                    continue;
                },
            });
            current.clear();
            separator = Some(ch);
        } else {
            current.push(ch);
        }
    }
    parsed.push(TabBarSgrParameter {
        separator,
        value: if current.is_empty() {
            0
        } else if let Ok(value) = current.parse::<u16>() {
            value
        } else {
            return parsed;
        },
    });
    parsed
}

fn tab_bar_sgr_underline_style(
    parameters: &[TabBarSgrParameter],
    index: &mut usize,
) -> Option<UnderlineStyle> {
    if parameters.get(*index)?.separator != Some(':') {
        return None;
    }

    let style = match parameters.get(*index)?.value {
        0 => UnderlineStyle::None,
        1 => UnderlineStyle::Single,
        2 => UnderlineStyle::Double,
        3 => UnderlineStyle::Curly,
        4 => UnderlineStyle::Dotted,
        5 => UnderlineStyle::Dashed,
        _ => return None,
    };
    *index += 1;
    Some(style)
}

fn tab_bar_sgr_extended_color(
    parameters: &[TabBarSgrParameter],
    index: &mut usize,
) -> Option<Color> {
    let mode = parameters.get(*index)?.value;
    *index += 1;

    match mode {
        5 => {
            let color = Color::Indexed(u8::try_from(parameters.get(*index)?.value).ok()?);
            *index += 1;
            Some(color)
        }
        2 => {
            if parameters.len().saturating_sub(*index) >= 4
                && parameters.get(*index).map(|parameter| parameter.value) == Some(0)
            {
                *index += 1;
            }
            let red = u8::try_from(parameters.get(*index)?.value).ok()?;
            let green = u8::try_from(parameters.get(*index + 1)?.value).ok()?;
            let blue = u8::try_from(parameters.get(*index + 2)?.value).ok()?;
            *index += 3;
            Some(Color::Rgb(red, green, blue))
        }
        _ => None,
    }
}

fn write_tab_bar_format_items(
    cells: &mut [RenderCell],
    column: &mut u16,
    items: &[NativeFormatItem],
    base_style: TabBarSegmentStyle,
) {
    let mut style = base_style;
    for item in items {
        match item {
            NativeFormatItem::Text(text) => {
                write_tab_bar_ansi_styled_segment(cells, column, text, base_style, &mut style);
            }
            NativeFormatItem::Foreground(color) => style.foreground = *color,
            NativeFormatItem::Background(color) => style.background = *color,
            NativeFormatItem::Attribute(attribute) => match attribute {
                NativeFormatAttribute::Intensity(NativeFormatIntensity::Bold) => {
                    style.bold = true;
                    style.faint = false;
                }
                NativeFormatAttribute::Intensity(NativeFormatIntensity::Normal) => {
                    style.bold = false;
                    style.faint = false;
                }
                NativeFormatAttribute::Intensity(NativeFormatIntensity::Half) => {
                    style.bold = false;
                    style.faint = true;
                }
                NativeFormatAttribute::Italic(italic) => style.italic = *italic,
                NativeFormatAttribute::Underline(underline) => {
                    style.underline_style = underline_style_for_native_format(*underline);
                }
            },
            NativeFormatItem::ResetAttributes => style = base_style,
        }
    }
}

fn write_tab_bar_format_items_if_configured(
    cells: &mut [RenderCell],
    column: &mut u16,
    items: Option<&[NativeFormatItem]>,
    base_style: TabBarSegmentStyle,
) {
    if let Some(items) = items {
        write_tab_bar_format_items(cells, column, items, base_style);
    }
}

fn native_format_items_visible_width(items: &[NativeFormatItem]) -> usize {
    items
        .iter()
        .filter_map(|item| match item {
            NativeFormatItem::Text(text) => Some(tab_bar_ansi_visible_width(text)),
            NativeFormatItem::Foreground(_)
            | NativeFormatItem::Background(_)
            | NativeFormatItem::Attribute(_)
            | NativeFormatItem::ResetAttributes => None,
        })
        .sum()
}

fn native_tab_title_visible_width(title: &NativeTabTitle) -> usize {
    match title {
        NativeTabTitle::Text(text) => tab_bar_ansi_visible_width(text),
        NativeTabTitle::Format(items) => native_format_items_visible_width(items),
    }
}

fn write_tab_bar_title_with_max_width(
    cells: &mut [RenderCell],
    column: &mut u16,
    title: &NativeTabTitle,
    base_style: TabBarSegmentStyle,
    max_width: usize,
) -> usize {
    let mut remaining = max_width;
    let mut written = 0usize;
    let mut style = base_style;

    let write_text = |text: &str,
                      style: &mut TabBarSegmentStyle,
                      column: &mut u16,
                      cells: &mut [RenderCell],
                      written: &mut usize,
                      remaining: &mut usize| {
        let text_cells = tab_bar_ansi_text_cells_with_style(text, base_style, style);
        let max_len = (*remaining).min(text_cells.len());
        for (ch, cell_style) in text_cells.into_iter().take(max_len) {
            if *remaining == 0 {
                return;
            }
            let index = usize::from(*column);
            let Some(cell) = cells.get_mut(index) else {
                return;
            };

            *cell = tab_bar_styled_render_cell(*column, ch, cell_style);
            *column = column.saturating_add(1);
            *written = written.saturating_add(1);
            *remaining = remaining.saturating_sub(1);
        }
    };

    match title {
        NativeTabTitle::Text(text) => {
            write_text(
                text,
                &mut style,
                column,
                cells,
                &mut written,
                &mut remaining,
            );
        }
        NativeTabTitle::Format(items) => {
            for item in items {
                if remaining == 0 {
                    break;
                }

                match item {
                    NativeFormatItem::Attribute(attribute) => match attribute {
                        NativeFormatAttribute::Intensity(NativeFormatIntensity::Bold) => {
                            style.bold = true;
                            style.faint = false;
                        }
                        NativeFormatAttribute::Intensity(NativeFormatIntensity::Normal) => {
                            style.bold = false;
                            style.faint = false;
                        }
                        NativeFormatAttribute::Intensity(NativeFormatIntensity::Half) => {
                            style.bold = false;
                            style.faint = true;
                        }
                        NativeFormatAttribute::Italic(italic) => style.italic = *italic,
                        NativeFormatAttribute::Underline(underline) => {
                            style.underline_style = underline_style_for_native_format(*underline);
                        }
                    },
                    NativeFormatItem::ResetAttributes => {
                        style = base_style;
                    }
                    NativeFormatItem::Text(text) => {
                        write_text(
                            text,
                            &mut style,
                            column,
                            cells,
                            &mut written,
                            &mut remaining,
                        );
                    }
                    NativeFormatItem::Foreground(color) => style.foreground = *color,
                    NativeFormatItem::Background(color) => style.background = *color,
                }
            }
        }
    }

    written
}

fn write_right_aligned_tab_bar_segment_with_reserved(
    cells: &mut [RenderCell],
    text: &str,
    base_style: TabBarSegmentStyle,
    reserved_right: usize,
) {
    let text_cells = tab_bar_ansi_text_cells(text, base_style);
    let width = text_cells.len();
    let available_width = cells.len().saturating_sub(reserved_right);
    if width == 0 || available_width == 0 {
        return;
    }

    let visible_width = width.min(available_width);
    let start = available_width - visible_width;
    let skip = width.saturating_sub(visible_width);
    for (offset, (ch, style)) in text_cells.into_iter().skip(skip).enumerate() {
        let Ok(column) = u16::try_from(start + offset) else {
            return;
        };
        if let Some(cell) = cells.get_mut(start + offset) {
            *cell = tab_bar_styled_render_cell(column, ch, style);
        }
    }
}

fn tab_bar_render_cell(
    column: u16,
    ch: char,
    foreground: Color,
    background: Color,
    bold: bool,
) -> RenderCell {
    ui_render_cell(0, column, ch, foreground, background, bold)
}

fn tab_bar_styled_render_cell(column: u16, ch: char, style: TabBarSegmentStyle) -> RenderCell {
    let mut cell = tab_bar_render_cell(column, ch, style.foreground, style.background, style.bold);
    cell.faint = style.faint;
    cell.italic = style.italic;
    cell.blink = style.blink;
    cell.rapid_blink = style.rapid_blink;
    cell.inverse = style.inverse;
    cell.conceal = style.conceal;
    cell.strikethrough = style.strikethrough;
    cell.overline = style.overline;
    cell.underline_color = style.underline_color;
    cell.underline_style = style.underline_style;
    cell.underline = style.underline_style != UnderlineStyle::None;
    cell.double_underline = style.underline_style == UnderlineStyle::Double;
    cell
}

const fn underline_style_for_native_format(underline: NativeFormatUnderline) -> UnderlineStyle {
    match underline {
        NativeFormatUnderline::None => UnderlineStyle::None,
        NativeFormatUnderline::Single => UnderlineStyle::Single,
        NativeFormatUnderline::Double => UnderlineStyle::Double,
        NativeFormatUnderline::Curly => UnderlineStyle::Curly,
        NativeFormatUnderline::Dotted => UnderlineStyle::Dotted,
        NativeFormatUnderline::Dashed => UnderlineStyle::Dashed,
    }
}

fn ui_render_cell(
    row: u16,
    column: u16,
    ch: char,
    foreground: Color,
    background: Color,
    bold: bool,
) -> RenderCell {
    RenderCell::new(row, column, ch.to_string()).with_style(RenderStyle {
        foreground,
        background,
        bold,
        ..RenderStyle::default()
    })
}

fn encode_modified_window_key(key: &Key, modifiers: ModifiersState) -> Option<Vec<u8>> {
    let modifier = xterm_window_modifier(modifiers)?;

    let Key::Named(named) = key else {
        return None;
    };

    match named {
        NamedKey::ArrowLeft => Some(format!("\x1b[1;{modifier}D").into_bytes()),
        NamedKey::ArrowRight => Some(format!("\x1b[1;{modifier}C").into_bytes()),
        NamedKey::ArrowUp => Some(format!("\x1b[1;{modifier}A").into_bytes()),
        NamedKey::ArrowDown => Some(format!("\x1b[1;{modifier}B").into_bytes()),
        NamedKey::Home => Some(format!("\x1b[1;{modifier}H").into_bytes()),
        NamedKey::End => Some(format!("\x1b[1;{modifier}F").into_bytes()),
        NamedKey::Insert => Some(format!("\x1b[2;{modifier}~").into_bytes()),
        NamedKey::Delete => Some(format!("\x1b[3;{modifier}~").into_bytes()),
        NamedKey::PageUp => Some(format!("\x1b[5;{modifier}~").into_bytes()),
        NamedKey::PageDown => Some(format!("\x1b[6;{modifier}~").into_bytes()),
        NamedKey::F1 => Some(format!("\x1b[1;{modifier}P").into_bytes()),
        NamedKey::F2 => Some(format!("\x1b[1;{modifier}Q").into_bytes()),
        NamedKey::F3 => Some(format!("\x1b[1;{modifier}R").into_bytes()),
        NamedKey::F4 => Some(format!("\x1b[1;{modifier}S").into_bytes()),
        NamedKey::F5 => Some(format!("\x1b[15;{modifier}~").into_bytes()),
        NamedKey::F6 => Some(format!("\x1b[17;{modifier}~").into_bytes()),
        NamedKey::F7 => Some(format!("\x1b[18;{modifier}~").into_bytes()),
        NamedKey::F8 => Some(format!("\x1b[19;{modifier}~").into_bytes()),
        NamedKey::F9 => Some(format!("\x1b[20;{modifier}~").into_bytes()),
        NamedKey::F10 => Some(format!("\x1b[21;{modifier}~").into_bytes()),
        NamedKey::F11 => Some(format!("\x1b[23;{modifier}~").into_bytes()),
        NamedKey::F12 => Some(format!("\x1b[24;{modifier}~").into_bytes()),
        _ => None,
    }
}

fn xterm_window_modifier(modifiers: ModifiersState) -> Option<u8> {
    let shift = modifiers.shift_key();
    let alt = modifiers.alt_key();
    let control = modifiers.control_key();
    if !(shift || alt || control) {
        return None;
    }

    Some(1 + u8::from(shift) + u8::from(alt) * 2 + u8::from(control) * 4)
}

fn kitty_window_modifier(modifiers: ModifiersState) -> Option<u8> {
    let shift = modifiers.shift_key();
    let alt = modifiers.alt_key();
    let control = modifiers.control_key();
    let super_key = modifiers.super_key();
    if !(shift || alt || control || super_key) {
        return None;
    }

    Some(1 + u8::from(shift) + u8::from(alt) * 2 + u8::from(control) * 4 + u8::from(super_key) * 8)
}

fn encode_control_window_key(
    key: &Key,
    physical_key: PhysicalKey,
    modifiers: ModifiersState,
    application_keypad: bool,
) -> Option<Vec<u8>> {
    if !modifiers.control_key() {
        return None;
    }

    let Key::Character(character) = key.as_ref() else {
        return None;
    };

    if application_keypad && encode_application_keypad_key(physical_key).is_some() {
        let mut bytes = Vec::new();
        if modifiers.alt_key() {
            bytes.push(0x1b);
        }
        return Some(bytes);
    }

    let character = character.chars().next()?;
    let mut bytes = encode_terminal_key(TerminalKey::Control(character)).unwrap_or_default();
    if modifiers.alt_key() {
        bytes.insert(0, 0x1b);
    }
    Some(bytes)
}

fn encode_application_keypad_key(physical_key: PhysicalKey) -> Option<Vec<u8>> {
    let PhysicalKey::Code(code) = physical_key else {
        return None;
    };

    let final_byte = match code {
        WinitKeyCode::NumpadEnter => b'M',
        WinitKeyCode::NumpadMultiply => b'j',
        WinitKeyCode::NumpadAdd => b'k',
        WinitKeyCode::NumpadComma => b'l',
        WinitKeyCode::NumpadSubtract => b'm',
        WinitKeyCode::NumpadDecimal => b'n',
        WinitKeyCode::NumpadDivide => b'o',
        WinitKeyCode::Numpad0 => b'p',
        WinitKeyCode::Numpad1 => b'q',
        WinitKeyCode::Numpad2 => b'r',
        WinitKeyCode::Numpad3 => b's',
        WinitKeyCode::Numpad4 => b't',
        WinitKeyCode::Numpad5 => b'u',
        WinitKeyCode::Numpad6 => b'v',
        WinitKeyCode::Numpad7 => b'w',
        WinitKeyCode::Numpad8 => b'x',
        WinitKeyCode::Numpad9 => b'y',
        WinitKeyCode::NumpadEqual => b'X',
        _ => return None,
    };

    Some(vec![0x1b, b'O', final_byte])
}

fn encode_application_cursor_key(key: &Key) -> Option<Vec<u8>> {
    let Key::Named(named) = key else {
        return None;
    };

    match named {
        NamedKey::ArrowUp => Some(b"\x1bOA".to_vec()),
        NamedKey::ArrowDown => Some(b"\x1bOB".to_vec()),
        NamedKey::ArrowRight => Some(b"\x1bOC".to_vec()),
        NamedKey::ArrowLeft => Some(b"\x1bOD".to_vec()),
        _ => None,
    }
}

fn encode_window_focus_event(focused: bool, focus_reporting: bool) -> Option<Vec<u8>> {
    if !focus_reporting {
        return None;
    }

    Some(if focused {
        b"\x1b[I".to_vec()
    } else {
        b"\x1b[O".to_vec()
    })
}

fn quote_dropped_file_name(path: &str, quote: NativeQuoteDroppedFiles) -> String {
    match quote {
        NativeQuoteDroppedFiles::None => path.to_owned(),
        NativeQuoteDroppedFiles::SpacesOnly => path.replace(' ', "\\ "),
        NativeQuoteDroppedFiles::Posix => quote_dropped_file_posix(path),
        NativeQuoteDroppedFiles::Windows => {
            if path.contains(' ') {
                quote_dropped_file_windows(path)
            } else {
                path.to_owned()
            }
        }
        NativeQuoteDroppedFiles::WindowsAlwaysQuoted => quote_dropped_file_windows(path),
    }
}

fn quote_dropped_file_posix(path: &str) -> String {
    if path.chars().all(|character| {
        character.is_ascii_alphanumeric()
            || matches!(
                character,
                '/' | '.' | '_' | '-' | '+' | ':' | ',' | '@' | '%'
            )
    }) {
        return path.to_owned();
    }

    let mut quoted = String::with_capacity(path.len() + 2);
    quoted.push('"');
    for character in path.chars() {
        match character {
            '\\' | '"' | '$' | '`' => {
                quoted.push('\\');
                quoted.push(character);
            }
            '\n' => quoted.push_str("\\n"),
            _ => quoted.push(character),
        }
    }
    quoted.push('"');
    quoted
}

fn quote_dropped_file_windows(path: &str) -> String {
    let mut quoted = String::with_capacity(path.len() + 2);
    quoted.push('"');
    for character in path.chars() {
        if character == '"' {
            quoted.push('\\');
        }
        quoted.push(character);
    }
    quoted.push('"');
    quoted
}

fn encode_osc52_clipboard_response(selection: &str, text: &str) -> Vec<u8> {
    format!(
        "\x1b]52;{};{}\x07",
        selection,
        STANDARD.encode(text.as_bytes())
    )
    .into_bytes()
}

fn pane_progress_from_terminal_progress(progress: TerminalProgress) -> PaneProgress {
    match progress {
        TerminalProgress::None => PaneProgress::None,
        TerminalProgress::Percentage(value) => PaneProgress::Percentage(value),
        TerminalProgress::Error(value) => PaneProgress::Error(value),
        TerminalProgress::Indeterminate => PaneProgress::Indeterminate,
    }
}

const fn terminal_progress_from_runtime(
    progress: rterm_runtime::RuntimeProgress,
) -> TerminalProgress {
    match progress {
        rterm_runtime::RuntimeProgress::None => TerminalProgress::None,
        rterm_runtime::RuntimeProgress::Percentage(value) => TerminalProgress::Percentage(value),
        rterm_runtime::RuntimeProgress::Error(value) => TerminalProgress::Error(value),
        rterm_runtime::RuntimeProgress::Indeterminate => TerminalProgress::Indeterminate,
    }
}

fn window_paste_source_for_shortcut(
    key: &Key,
    modifiers: ModifiersState,
) -> Option<WindowPasteSource> {
    let ctrl_shift_v = modifiers.control_key()
        && modifiers.shift_key()
        && !modifiers.alt_key()
        && matches!(key.as_ref(), Key::Character(character) if character.eq_ignore_ascii_case("v"));
    let super_v = modifiers == ModifiersState::SUPER
        && matches!(key.as_ref(), Key::Character(character) if character.eq_ignore_ascii_case("v"));
    let paste_key = modifiers.is_empty() && matches!(key, Key::Named(NamedKey::Paste));
    if ctrl_shift_v || super_v || paste_key {
        return Some(WindowPasteSource::Clipboard);
    }

    if modifiers == ModifiersState::SHIFT && matches!(key, Key::Named(NamedKey::Insert)) {
        return Some(WindowPasteSource::PrimarySelection);
    }

    None
}

fn window_copy_destination_for_shortcut(
    key: &Key,
    modifiers: ModifiersState,
) -> Option<WindowCopyDestination> {
    let ctrl_shift_c = modifiers.control_key()
        && modifiers.shift_key()
        && !modifiers.alt_key()
        && matches!(key.as_ref(), Key::Character(character) if character.eq_ignore_ascii_case("c"));
    let super_c = modifiers == ModifiersState::SUPER
        && matches!(key.as_ref(), Key::Character(character) if character.eq_ignore_ascii_case("c"));
    let copy_key = modifiers.is_empty() && matches!(key, Key::Named(NamedKey::Copy));
    if ctrl_shift_c || super_c || copy_key {
        return Some(WindowCopyDestination::Clipboard);
    }

    if modifiers == ModifiersState::CONTROL && matches!(key, Key::Named(NamedKey::Insert)) {
        return Some(WindowCopyDestination::PrimarySelection);
    }

    None
}

fn window_copy_mode_shortcut(key: &Key, modifiers: ModifiersState) -> bool {
    modifiers.control_key()
        && modifiers.shift_key()
        && !modifiers.alt_key()
        && matches!(key.as_ref(), Key::Character(character) if character.eq_ignore_ascii_case("x"))
}

fn window_quick_select_shortcut(key: &Key, modifiers: ModifiersState) -> bool {
    modifiers.control_key()
        && modifiers.shift_key()
        && !modifiers.alt_key()
        && matches!(key, Key::Named(NamedKey::Space))
}

#[cfg(test)]
fn window_char_select_shortcut(key: &Key, modifiers: ModifiersState) -> bool {
    window_char_select_shortcut_with_preference(
        key,
        None,
        modifiers,
        NativeKeyMapPreference::Mapped,
    )
}

fn window_char_select_shortcut_with_preference(
    key: &Key,
    physical_key: Option<PhysicalKey>,
    modifiers: ModifiersState,
    key_map_preference: NativeKeyMapPreference,
) -> bool {
    modifiers.control_key()
        && modifiers.shift_key()
        && !modifiers.alt_key()
        && window_key_assignment_key_matches("U", key, physical_key, key_map_preference)
}

fn window_search_shortcut(key: &Key, modifiers: ModifiersState) -> bool {
    let ctrl_shift_f = modifiers.control_key()
        && modifiers.shift_key()
        && !modifiers.alt_key()
        && matches!(key.as_ref(), Key::Character(character) if character.eq_ignore_ascii_case("f"));
    let super_f = modifiers == ModifiersState::SUPER
        && matches!(key.as_ref(), Key::Character(character) if character.eq_ignore_ascii_case("f"));
    ctrl_shift_f || super_f
}

#[cfg(test)]
fn window_reload_configuration_shortcut(key: &Key, modifiers: ModifiersState) -> bool {
    window_reload_configuration_shortcut_with_preference(
        key,
        None,
        modifiers,
        NativeKeyMapPreference::Mapped,
    )
}

fn window_reload_configuration_shortcut_with_preference(
    key: &Key,
    physical_key: Option<PhysicalKey>,
    modifiers: ModifiersState,
    key_map_preference: NativeKeyMapPreference,
) -> bool {
    let ctrl_shift_r = modifiers.control_key()
        && modifiers.shift_key()
        && !modifiers.alt_key()
        && window_key_assignment_key_matches("R", key, physical_key, key_map_preference);
    let super_r = modifiers == ModifiersState::SUPER
        && window_key_assignment_key_matches("R", key, physical_key, key_map_preference);
    ctrl_shift_r || super_r
}

fn window_toggle_full_screen_shortcut(key: &Key, modifiers: ModifiersState) -> bool {
    modifiers == ModifiersState::ALT && matches!(key, Key::Named(NamedKey::Enter))
}

#[cfg(test)]
fn window_hide_shortcut(key: &Key, modifiers: ModifiersState) -> bool {
    window_hide_shortcut_with_preference(key, None, modifiers, NativeKeyMapPreference::Mapped)
}

fn window_hide_shortcut_with_preference(
    key: &Key,
    physical_key: Option<PhysicalKey>,
    modifiers: ModifiersState,
    key_map_preference: NativeKeyMapPreference,
) -> bool {
    modifiers == ModifiersState::SUPER
        && window_key_assignment_key_matches("M", key, physical_key, key_map_preference)
}

#[cfg(test)]
fn window_application_hide_shortcut(key: &Key, modifiers: ModifiersState) -> bool {
    window_application_hide_shortcut_with_preference(
        key,
        None,
        modifiers,
        NativeKeyMapPreference::Mapped,
    )
}

fn window_application_hide_shortcut_with_preference(
    key: &Key,
    physical_key: Option<PhysicalKey>,
    modifiers: ModifiersState,
    key_map_preference: NativeKeyMapPreference,
) -> bool {
    if !cfg!(target_os = "macos") {
        return false;
    }

    modifiers == ModifiersState::SUPER
        && window_key_assignment_key_matches("H", key, physical_key, key_map_preference)
}

fn window_start_drag_mouse_binding(button: MouseButton, modifiers: ModifiersState) -> bool {
    button == MouseButton::Left
        && (modifiers == ModifiersState::SUPER
            || modifiers == (ModifiersState::CONTROL | ModifiersState::SHIFT))
}

fn window_font_size_shortcut(key: &Key, modifiers: ModifiersState) -> Option<WindowFontSizeAction> {
    if modifiers != ModifiersState::CONTROL && modifiers != ModifiersState::SUPER {
        return None;
    }

    match key.as_ref() {
        Key::Character("-") => Some(WindowFontSizeAction::Decrease),
        Key::Character("=") => Some(WindowFontSizeAction::Increase),
        Key::Character("0") => Some(WindowFontSizeAction::Reset),
        _ => None,
    }
}

#[cfg(test)]
fn window_show_debug_overlay_shortcut(key: &Key, modifiers: ModifiersState) -> bool {
    window_show_debug_overlay_shortcut_with_preference(
        key,
        None,
        modifiers,
        NativeKeyMapPreference::Mapped,
    )
}

fn window_show_debug_overlay_shortcut_with_preference(
    key: &Key,
    physical_key: Option<PhysicalKey>,
    modifiers: ModifiersState,
    key_map_preference: NativeKeyMapPreference,
) -> bool {
    modifiers.control_key()
        && modifiers.shift_key()
        && !modifiers.alt_key()
        && window_key_assignment_key_matches("L", key, physical_key, key_map_preference)
}

fn window_clear_scrollback_shortcut(key: &Key, modifiers: ModifiersState) -> bool {
    let ctrl_shift_k = modifiers.control_key()
        && modifiers.shift_key()
        && !modifiers.alt_key()
        && matches!(key.as_ref(), Key::Character(character) if character.eq_ignore_ascii_case("k"));
    let super_k = modifiers == ModifiersState::SUPER
        && matches!(key.as_ref(), Key::Character(character) if character.eq_ignore_ascii_case("k"));
    ctrl_shift_k || super_k
}

fn window_hyperlink_activation_modifiers(modifiers: ModifiersState) -> bool {
    modifiers.control_key() && !modifiers.shift_key() && !modifiers.alt_key()
}

fn read_window_clipboard_text() -> Option<String> {
    arboard::Clipboard::new().ok()?.get_text().ok()
}

fn write_window_clipboard_text(text: &str) -> bool {
    arboard::Clipboard::new()
        .and_then(|mut clipboard| clipboard.set_text(text.to_owned()))
        .is_ok()
}

fn read_window_primary_selection_text() -> Option<String> {
    None
}

fn write_window_primary_selection_text(_text: &str) -> bool {
    false
}

fn show_window_notification(_notification: &TerminalNotification) -> bool {
    false
}

#[cfg(not(test))]
fn ring_window_audible_bell(_bell: &NativeWindowBell) -> bool {
    io::stderr()
        .write_all(b"\x07")
        .and_then(|()| io::stderr().flush())
        .is_ok()
}

#[cfg(test)]
fn ring_window_audible_bell(_bell: &NativeWindowBell) -> bool {
    false
}

fn dispatch_window_open_uri(_event: &NativeWindowOpenUri) -> bool {
    true
}

fn dispatch_window_new_tab_button_click(_event: &NativeWindowNewTabButtonClick) -> bool {
    true
}

fn format_tab_title(_event: &NativeTabTitleFormat) -> Option<NativeTabTitle> {
    None
}

fn format_window_title(_event: &NativeWindowTitleFormat) -> Option<String> {
    None
}

fn native_lua_keyboard_modifiers_text(modifiers: ModifiersState) -> String {
    [
        (ModifiersState::CONTROL, "CTRL"),
        (ModifiersState::SHIFT, "SHIFT"),
        (ModifiersState::ALT, "ALT"),
        (ModifiersState::SUPER, "SUPER"),
    ]
    .into_iter()
    .filter_map(|(flag, name)| modifiers.contains(flag).then_some(name))
    .collect::<Vec<_>>()
    .join("|")
}

fn native_lua_cursor_shape_text(shape: rssh_terminal::CursorShape) -> &'static str {
    match shape {
        rssh_terminal::CursorShape::Block => "Block",
        rssh_terminal::CursorShape::Underline => "Underline",
        rssh_terminal::CursorShape::Bar => "Bar",
    }
}

fn dispatch_window_update_status(
    _event: &NativeWindowStatusUpdateEvent,
) -> NativeWindowStatusUpdate {
    NativeWindowStatusUpdate {
        left_status: None,
        right_status: None,
    }
}

fn dispatch_window_update_right_status(_event: &NativeWindowStatusUpdateEvent) -> Option<String> {
    None
}

#[allow(clippy::trivially_copy_pass_by_ref)]
fn dispatch_window_bell(_bell: &NativeWindowBell) -> bool {
    false
}

fn dispatch_window_focus_change(_change: &NativeWindowFocusChange) -> bool {
    false
}

fn dispatch_window_resize(_resize: &NativeWindowResize) -> bool {
    false
}

fn cursor_blink_opacity_alpha(
    elapsed: Duration,
    cursor_blink_rate: Duration,
    cursor_blink_ease_in: NativeEasingFunction,
    cursor_blink_ease_out: NativeEasingFunction,
) -> u8 {
    if cursor_blink_rate.is_zero() {
        return u8::MAX;
    }

    let phase_ms = u32::try_from(cursor_blink_rate.as_millis()).unwrap_or(u32::MAX);
    if phase_ms == 0 {
        return u8::MAX;
    }
    let cycle_ms = phase_ms.saturating_mul(2);
    let elapsed_ms = u32::try_from(elapsed.as_millis()).unwrap_or(u32::MAX);
    let cycle_elapsed_ms = elapsed_ms % cycle_ms;

    if cycle_elapsed_ms < phase_ms {
        let progress = f64::from(cycle_elapsed_ms) / f64::from(phase_ms);
        let eased = easing_value(cursor_blink_ease_out, progress);
        opacity_alpha(1.0 - eased)
    } else {
        let progress = f64::from(cycle_elapsed_ms.saturating_sub(phase_ms)) / f64::from(phase_ms);
        opacity_alpha(easing_value(cursor_blink_ease_in, progress))
    }
}

fn blink_opacity_alpha_if_changed(
    now: Instant,
    last_blink_at: &mut Option<Instant>,
    blink_rate: Duration,
    blink_ease_in: NativeEasingFunction,
    blink_ease_out: NativeEasingFunction,
    current_alpha: u8,
) -> Option<u8> {
    if blink_rate.is_zero() {
        *last_blink_at = None;
        return (current_alpha != u8::MAX).then_some(u8::MAX);
    }

    let Some(last) = *last_blink_at else {
        *last_blink_at = Some(now);
        return None;
    };

    let elapsed = now.checked_duration_since(last)?;
    let alpha = cursor_blink_opacity_alpha(elapsed, blink_rate, blink_ease_in, blink_ease_out);
    (alpha != current_alpha).then_some(alpha)
}

fn easing_value(easing: NativeEasingFunction, progress: f64) -> f64 {
    let progress = progress.clamp(0.0, 1.0);
    match easing {
        NativeEasingFunction::Constant => {
            if progress >= 1.0 {
                1.0
            } else {
                0.0
            }
        }
        NativeEasingFunction::Linear => progress,
        NativeEasingFunction::EaseIn => progress * progress * progress,
        NativeEasingFunction::EaseOut => 1.0 - (1.0 - progress).powi(3),
        NativeEasingFunction::EaseInOut => {
            if progress < 0.5 {
                4.0 * progress * progress * progress
            } else {
                1.0 - (-2.0 * progress + 2.0).powi(3) / 2.0
            }
        }
        NativeEasingFunction::Ease => {
            if progress < 0.5 {
                4.0 * progress * progress * progress
            } else {
                1.0 - (1.0 - progress).powi(2)
            }
        }
        NativeEasingFunction::CubicBezier(bezier) => cubic_bezier_easing_value(bezier, progress),
    }
}

fn cubic_bezier_easing_value(bezier: NativeCubicBezier, progress: f64) -> f64 {
    let x1 = cubic_bezier_unit_x(bezier.x1_per_mille);
    let y1 = cubic_bezier_coordinate(bezier.y1_per_mille);
    let x2 = cubic_bezier_unit_x(bezier.x2_per_mille);
    let y2 = cubic_bezier_coordinate(bezier.y2_per_mille);
    let parameter = cubic_bezier_parameter_for_x(progress, x1, x2);
    cubic_bezier_axis_value(parameter, y1, y2)
}

fn cubic_bezier_parameter_for_x(progress: f64, x1: f64, x2: f64) -> f64 {
    let mut low = 0.0;
    let mut high = 1.0;
    let mut parameter = progress.clamp(0.0, 1.0);

    for _ in 0..32 {
        let value = cubic_bezier_axis_value(parameter, x1, x2);
        if (value - progress).abs() <= 0.000_001 {
            break;
        }

        if value < progress {
            low = parameter;
        } else {
            high = parameter;
        }
        parameter = f64::midpoint(low, high);
    }

    parameter
}

fn cubic_bezier_axis_value(parameter: f64, control1: f64, control2: f64) -> f64 {
    let inverse = 1.0 - parameter;
    3.0 * inverse * inverse * parameter * control1
        + 3.0 * inverse * parameter * parameter * control2
        + parameter * parameter * parameter
}

fn cubic_bezier_unit_x(value_per_mille: i32) -> f64 {
    cubic_bezier_coordinate(value_per_mille).clamp(0.0, 1.0)
}

fn cubic_bezier_coordinate(value_per_mille: i32) -> f64 {
    f64::from(value_per_mille) / 1_000.0
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn opacity_alpha(opacity: f64) -> u8 {
    (opacity.clamp(0.0, 1.0) * f64::from(u8::MAX)) as u8
}

fn dispatch_window_user_var_change(_change: &NativeWindowUserVarChange) -> bool {
    false
}

fn dispatch_window_config_reloaded(_event: &NativeWindowConfigReloaded) -> bool {
    false
}

fn dispatch_command_palette_augment(
    _event: &NativeCommandPaletteAugment,
) -> Vec<NativeCommandPaletteEntry> {
    Vec::new()
}

fn dispatch_prompt_input_line(_event: &NativePromptInputLine) -> bool {
    false
}

fn dispatch_input_selector(_event: &NativeInputSelector) -> bool {
    false
}

fn dispatch_confirmation(_event: &NativeConfirmation) -> bool {
    false
}

fn dispatch_emit_event(_event: &NativeWindowEmitEvent) -> bool {
    false
}

#[cfg(target_os = "windows")]
fn open_window_hyperlink(url: &str) -> bool {
    Command::new("rundll32")
        .arg("url.dll,FileProtocolHandler")
        .arg(url)
        .spawn()
        .is_ok()
}

#[cfg(target_os = "macos")]
fn open_window_hyperlink(url: &str) -> bool {
    Command::new("open").arg(url).spawn().is_ok()
}

#[cfg(all(unix, not(target_os = "macos")))]
fn open_window_hyperlink(url: &str) -> bool {
    Command::new("xdg-open").arg(url).spawn().is_ok()
}

#[cfg(not(any(target_os = "windows", target_os = "macos", unix)))]
fn open_window_hyperlink(_url: &str) -> bool {
    false
}

fn named_terminal_key(key: &Key) -> Option<TerminalKey> {
    let Key::Named(named) = key else {
        return None;
    };

    match named {
        NamedKey::Enter => Some(TerminalKey::Enter),
        NamedKey::Backspace => Some(TerminalKey::Backspace),
        NamedKey::Tab => Some(TerminalKey::Tab),
        NamedKey::Escape => Some(TerminalKey::Escape),
        NamedKey::ArrowLeft => Some(TerminalKey::Left),
        NamedKey::ArrowRight => Some(TerminalKey::Right),
        NamedKey::ArrowUp => Some(TerminalKey::Up),
        NamedKey::ArrowDown => Some(TerminalKey::Down),
        NamedKey::Home => Some(TerminalKey::Home),
        NamedKey::End => Some(TerminalKey::End),
        NamedKey::Delete => Some(TerminalKey::Delete),
        NamedKey::Insert => Some(TerminalKey::Insert),
        NamedKey::PageUp => Some(TerminalKey::PageUp),
        NamedKey::PageDown => Some(TerminalKey::PageDown),
        NamedKey::ContextMenu => Some(TerminalKey::Menu),
        NamedKey::F1 => Some(TerminalKey::Function(1)),
        NamedKey::F2 => Some(TerminalKey::Function(2)),
        NamedKey::F3 => Some(TerminalKey::Function(3)),
        NamedKey::F4 => Some(TerminalKey::Function(4)),
        NamedKey::F5 => Some(TerminalKey::Function(5)),
        NamedKey::F6 => Some(TerminalKey::Function(6)),
        NamedKey::F7 => Some(TerminalKey::Function(7)),
        NamedKey::F8 => Some(TerminalKey::Function(8)),
        NamedKey::F9 => Some(TerminalKey::Function(9)),
        NamedKey::F10 => Some(TerminalKey::Function(10)),
        NamedKey::F11 => Some(TerminalKey::Function(11)),
        NamedKey::F12 => Some(TerminalKey::Function(12)),
        _ => None,
    }
}

#[cfg(test)]
fn terminal_size_from_window_pixels(width: u32, height: u32) -> TerminalSize {
    terminal_size_from_window_pixels_with_cell_size(width, height, CELL_WIDTH, CELL_HEIGHT)
}

fn missing_glyph_warning(glyph: char) -> String {
    format!(
        "CONFIG ERROR missing glyph for codepoint U+{:04X} ('{glyph}')",
        u32::from(glyph)
    )
}

#[cfg(test)]
fn terminal_size_from_window_pixels_with_cell_size(
    width: u32,
    height: u32,
    cell_width: u32,
    cell_height: u32,
) -> TerminalSize {
    let cell_width = cell_width.max(1);
    let cell_height = cell_height.max(1);
    let columns = u16::try_from((width / cell_width).clamp(1, u32::from(u16::MAX)))
        .expect("column count is clamped to u16");
    let total_rows = height / cell_height;
    let terminal_rows = total_rows.saturating_sub(u32::from(TAB_BAR_ROWS));
    let rows = u16::try_from(terminal_rows.clamp(1, u32::from(u16::MAX)))
        .expect("row count is clamped to u16");

    TerminalSize::new(columns, rows)
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn scaled_cell_dimension(base: u32, scale: f64) -> u32 {
    if !scale.is_finite() || scale <= 0.0 {
        return base.max(1);
    }

    (f64::from(base) * scale)
        .round()
        .clamp(1.0, f64::from(u32::MAX)) as u32
}

fn native_window_resize_increments_supported() -> bool {
    cfg!(any(
        target_os = "macos",
        target_os = "linux",
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "netbsd",
        target_os = "dragonfly"
    ))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NativeFullscreenPlatform {
    Macos,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NativeFullscreenRequest {
    Windowed,
    Borderless,
    MacosSimple,
    MacosSimpleExtendBehindNotch,
}

fn current_native_fullscreen_platform() -> NativeFullscreenPlatform {
    if cfg!(target_os = "macos") {
        NativeFullscreenPlatform::Macos
    } else {
        NativeFullscreenPlatform::Other
    }
}

fn native_fullscreen_request(
    full_screen: bool,
    native_macos_fullscreen_mode: bool,
    macos_fullscreen_extend_behind_notch: bool,
    platform: NativeFullscreenPlatform,
) -> NativeFullscreenRequest {
    if !full_screen {
        return NativeFullscreenRequest::Windowed;
    }

    if platform == NativeFullscreenPlatform::Macos && !native_macos_fullscreen_mode {
        if macos_fullscreen_extend_behind_notch {
            NativeFullscreenRequest::MacosSimpleExtendBehindNotch
        } else {
            NativeFullscreenRequest::MacosSimple
        }
    } else {
        NativeFullscreenRequest::Borderless
    }
}

fn apply_native_fullscreen(window: &Window, request: NativeFullscreenRequest) {
    match request {
        NativeFullscreenRequest::Windowed => {
            let _ = set_native_simple_fullscreen(window, false, false);
            window.set_fullscreen(None);
        }
        NativeFullscreenRequest::Borderless => {
            let _ = set_native_simple_fullscreen(window, false, false);
            window.set_fullscreen(Some(Fullscreen::Borderless(window.current_monitor())));
        }
        NativeFullscreenRequest::MacosSimple
        | NativeFullscreenRequest::MacosSimpleExtendBehindNotch => {
            if !set_native_simple_fullscreen(
                window,
                true,
                request == NativeFullscreenRequest::MacosSimpleExtendBehindNotch,
            ) {
                window.set_fullscreen(Some(Fullscreen::Borderless(window.current_monitor())));
            }
        }
    }
}

#[cfg(target_os = "macos")]
fn set_native_simple_fullscreen(
    window: &Window,
    full_screen: bool,
    _extend_behind_notch: bool,
) -> bool {
    window.set_simple_fullscreen(full_screen)
}

#[cfg(not(target_os = "macos"))]
fn set_native_simple_fullscreen(
    _window: &Window,
    _full_screen: bool,
    _extend_behind_notch: bool,
) -> bool {
    false
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn window_dpi_from_scale_factor(scale_factor: f64) -> u32 {
    if !scale_factor.is_finite() || scale_factor <= 0.0 {
        return DEFAULT_WINDOW_DPI;
    }

    (scale_factor * f64::from(DEFAULT_WINDOW_DPI))
        .round()
        .clamp(1.0, f64::from(u32::MAX)) as u32
}

fn parse_test_window_scale_factor(value: &str) -> Option<f64> {
    let scale_factor = value.parse::<f64>().ok()?;
    (scale_factor.is_finite() && (0.5..=4.0).contains(&scale_factor)).then_some(scale_factor)
}

fn benchmark_window_scale_factor(benchmark_startup: bool) -> Option<f64> {
    if !benchmark_startup {
        return None;
    }
    std::env::var("RSSH_BENCHMARK_WINDOW_SCALE_FACTOR")
        .ok()
        .and_then(|value| parse_test_window_scale_factor(&value))
}

fn startup_window_scale_factor(benchmark_startup: bool, detected_scale_factor: f64) -> f64 {
    benchmark_window_scale_factor(benchmark_startup)
        .or_else(test_window_scale_factor)
        .unwrap_or(detected_scale_factor)
}

#[cfg(debug_assertions)]
fn test_window_scale_factor() -> Option<f64> {
    std::env::var("RSSH_TEST_WINDOW_SCALE_FACTOR")
        .ok()
        .and_then(|value| parse_test_window_scale_factor(&value))
}

#[cfg(not(debug_assertions))]
const fn test_window_scale_factor() -> Option<f64> {
    None
}

#[cfg(debug_assertions)]
fn test_resize_after_first_present() -> Option<PhysicalSize<u32>> {
    const MAX_TEST_WINDOW_DIMENSION: u32 = 16_384;

    let value = std::env::var("RSSH_TEST_RESIZE_AFTER_FIRST_PRESENT").ok()?;
    let (width, height) = value.split_once('x')?;
    let width = width.parse().ok()?;
    let height = height.parse().ok()?;
    (width > 0
        && height > 0
        && width <= MAX_TEST_WINDOW_DIMENSION
        && height <= MAX_TEST_WINDOW_DIMENSION)
        .then_some(PhysicalSize::new(width, height))
}

#[cfg(not(debug_assertions))]
const fn test_resize_after_first_present() -> Option<PhysicalSize<u32>> {
    None
}

#[cfg(debug_assertions)]
fn test_ssh_gui_frame_limit() -> Option<u64> {
    std::env::var("RSSH_TEST_SSH_GUI_FRAME_LIMIT")
        .ok()?
        .parse()
        .ok()
        .filter(|limit| *limit > 0)
}

#[cfg(not(debug_assertions))]
const fn test_ssh_gui_frame_limit() -> Option<u64> {
    None
}

#[cfg(debug_assertions)]
fn test_deferred_gpu_init_failure() -> bool {
    std::env::var_os("RSSH_TEST_DEFERRED_GPU_INIT_FAILURE").as_deref()
        == Some(std::ffi::OsStr::new("1"))
}

#[cfg(not(debug_assertions))]
const fn test_deferred_gpu_init_failure() -> bool {
    false
}

impl ApplicationHandler<WindowUserEvent> for NativeWindowManager {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if let Err(error) = self.materialize_startup_app(event_loop) {
            eprintln!("window error: {error}");
            event_loop.exit();
            return;
        }

        if let Err(error) = self.materialize_pending_apps(event_loop) {
            eprintln!("window error: {error}");
            event_loop.exit();
            return;
        }
        self.refresh_tab_transfer_targets();
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: WindowUserEvent) {
        if matches!(event, WindowUserEvent::DiagnosticShutdownRequested) {
            event_loop.exit();
            return;
        }
        if self.handle_manager_user_event(&event) {
            return;
        }
        let Some(close_window) = self.dispatch_user_event_to_owner(event) else {
            return;
        };
        if close_window && self.should_exit_when_idle() {
            event_loop.exit();
            return;
        }

        if let Err(error) = self.materialize_pending_apps(event_loop) {
            eprintln!("window error: {error}");
            event_loop.exit();
        }
    }

    #[allow(clippy::too_many_lines)]
    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        if matches!(event, WindowEvent::CloseRequested) {
            if self.close_window(window_id) {
                event_loop.exit();
            }
            return;
        }

        if let WindowEvent::Focused(focused) = &event {
            let event = rssh_native::PlatformEvent::Focused(*focused);
            if let Err(error) = self.handle_window_platform_event(window_id, &event) {
                eprintln!("PTY focus error: {error}");
                event_loop.exit();
            }
            return;
        }

        self.refresh_tab_transfer_targets();
        let Some(mut app) = self.windows.remove(&window_id) else {
            return;
        };
        app.window_event(event_loop, window_id, event);
        self.collect_pending_window_apps_from_app(&mut app);
        let activate_window_request = app.take_activate_window_request();
        let application_hide_requested = app.take_application_hide_request();
        if app.take_window_close_request() {
            self.windows.insert(window_id, app);
            self.finalize_app_close_at_location(ManagedWindowAppLocation::Window(window_id))
                .expect("event window remains manager-owned until final removal");
            if self.should_exit_when_idle() {
                event_loop.exit();
                return;
            }
        } else if app.take_application_quit_request() {
            self.focus.remove(window_id);
            self.quit_application_from_app(app);
            event_loop.exit();
            return;
        } else {
            self.windows.insert(window_id, app);
            if let Some(request) = activate_window_request {
                self.activate_window_relative_from(window_id, request);
            }
            if application_hide_requested {
                hide_native_application(event_loop);
            }
        }

        if let Err(error) = self.materialize_pending_apps(event_loop) {
            eprintln!("window error: {error}");
            event_loop.exit();
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.finish_deferred_config_if_ready();
        self.start_deferred_config_if_ready();
        self.reap_retired_apps();
        if let Err(error) = self.materialize_pending_apps(event_loop) {
            eprintln!("window error: {error}");
            event_loop.exit();
            return;
        }

        let mut closed_windows = Vec::new();
        for (window_id, app) in &mut self.windows {
            match app.poll_active_v2_runtime() {
                Ok(Some(true)) => closed_windows.push(*window_id),
                Ok(Some(false) | None) => {}
                Err(error) => {
                    eprintln!("runtime V2 host error: {error}");
                    event_loop.exit();
                    return;
                }
            }
        }
        for window_id in closed_windows {
            self.finalize_app_close_at_location(ManagedWindowAppLocation::Window(window_id));
        }
        if self.should_exit_when_idle() {
            event_loop.exit();
            return;
        }

        let now = Instant::now();
        let mut next_frame_limit_redraw = None;
        for app in self.windows.values_mut() {
            if let Some(deadline) = app.diagnostic_hold_deadline() {
                if now >= deadline {
                    event_loop.exit();
                    return;
                }
                next_frame_limit_redraw = Some(
                    next_frame_limit_redraw
                        .map_or(deadline, |earliest: Instant| earliest.min(deadline)),
                );
            }
            let mut regular_redraw_needed =
                app.frame_needs_full_repaint || !app.pending_frame_damage.is_empty();
            regular_redraw_needed |= app.frame_limit_refresh_pending();
            regular_redraw_needed |= app.dispatch_update_status_if_due(now);
            let cursor_animation_changed = app.update_cursor_blink_phase_if_due(now);
            let text_animation_changed = app.update_text_blink_phase_if_due(now);
            let animation_changed = cursor_animation_changed || text_animation_changed;
            regular_redraw_needed |= app.expire_visual_bells_if_due(now);
            regular_redraw_needed |= app.expire_key_table_stack_if_due(now);
            regular_redraw_needed |= app.expire_leader_key_if_due(now);

            let animation_active = app.has_active_animation_at(now);
            if regular_redraw_needed || (animation_changed && !animation_active) {
                app.request_redraw_if_due(now);
            }
            if animation_active {
                app.request_animation_redraw_if_due(now);
            }
            if let Some(deadline) = app.frame_limit_redraw_deadline(now) {
                next_frame_limit_redraw = Some(
                    next_frame_limit_redraw
                        .map_or(deadline, |earliest: Instant| earliest.min(deadline)),
                );
            }
            if app
                .runtime
                .worker()
                .is_some_and(WindowPaneRuntime::needs_poll)
            {
                let deadline = now + Duration::from_millis(1);
                next_frame_limit_redraw = Some(
                    next_frame_limit_redraw
                        .map_or(deadline, |earliest: Instant| earliest.min(deadline)),
                );
            }
        }
        #[cfg(feature = "functional-test-observer")]
        if let Some(app) = self.windows.values().next() {
            crate::functional_observer::publish(app.functional_observer_snapshot());
        }
        if let Some(deadline) = next_frame_limit_redraw {
            event_loop.set_control_flow(ControlFlow::WaitUntil(deadline));
        } else {
            event_loop.set_control_flow(ControlFlow::Wait);
        }
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        self.shutdown_gpu_for_application_exit();
    }
}

impl ApplicationHandler<WindowUserEvent> for NativeWindowApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        if let Err(error) = self.create_window(event_loop) {
            eprintln!("window error: {error}");
            event_loop.exit();
            return;
        }

        if self.renderer_mode == RendererMode::Gpu
            && !self.is_benchmark_startup()
            && !self.suppresses_transport_start()
            && let Err(error) = self.spawn_pty()
        {
            eprintln!("PTY error: {error}");
            event_loop.exit();
            return;
        }

        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    #[allow(clippy::too_many_lines)]
    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: WindowUserEvent) {
        match event {
            WindowUserEvent::ReloadConfigurationRequested | WindowUserEvent::ConfigFileChanged => {
                self.reload_configuration();
            }
            WindowUserEvent::DeferredConfigReady | WindowUserEvent::MoveTabToWindow { .. } => {}
            WindowUserEvent::DiagnosticShutdownRequested => event_loop.exit(),
            WindowUserEvent::RuntimeWakeWindow { .. } => {
                self.handle_runtime_wake_window(event_loop);
            }
            WindowUserEvent::DeferredGpuInitialized {
                generation,
                outcome,
                ..
            } => {
                self.handle_deferred_gpu_initialized(generation, outcome);
            }
            WindowUserEvent::Output {
                pane_id,
                runtime_generation,
                bytes,
                ..
            } => {
                if !self.pane_runtime_generation_matches(pane_id, runtime_generation) {
                    return;
                }
                if let Err(error) = self.handle_pane_pty_output(pane_id, &bytes) {
                    eprintln!("PTY write error: {error}");
                    event_loop.exit();
                    return;
                }

                if pane_id == self.app_shell.active_pane_id()
                    && self.window.is_some()
                {
                    self.update_ime_cursor_area();
                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
                }
            }
            WindowUserEvent::Exited {
                pane_id,
                runtime_generation,
                ..
            } => {
                if !self.pane_runtime_generation_matches(pane_id, runtime_generation) {
                    return;
                }
                let status = self.finish_pane_runtime_after_exit(pane_id, runtime_generation);
                #[cfg(feature = "functional-test-observer")]
                {
                    crate::functional_observer::publish(self.functional_observer_snapshot());
                    let _ = crate::functional_observer::wait_until_current_revision_delivered(
                        Duration::from_millis(250),
                    );
                }
                let close_window = self.apply_pane_exit_behavior_after_exit(pane_id, status);
                if self.defer_automatic_close_for_frame_limit(close_window) {
                    event_loop.exit();
                }
            }
            WindowUserEvent::ReadError {
                pane_id,
                runtime_generation,
                error,
                ..
            } => {
                if !self.pane_runtime_generation_matches(pane_id, runtime_generation) {
                    return;
                }
                if self.handle_pane_runtime_read_error(pane_id, &error) {
                    event_loop.exit();
                }
            }
            WindowUserEvent::WriteCompleted {
                pane_id,
                runtime_generation,
                byte_count,
                elapsed,
                ..
            } => {
                if !self.pane_runtime_generation_matches(pane_id, runtime_generation) {
                    return;
                }
                self.handle_pane_input_write_completed(byte_count, elapsed);
            }
            WindowUserEvent::WriteError {
                pane_id,
                runtime_generation,
                error,
                ..
            } => {
                if !self.pane_runtime_generation_matches(pane_id, runtime_generation) {
                    return;
                }
                if self.handle_pane_runtime_write_error(pane_id, &error) {
                    event_loop.exit();
                }
            }
            #[cfg(feature = "ssh")]
            WindowUserEvent::SshState {
                pane_id,
                runtime_generation,
                state,
                ..
            } => {
                if !self.pane_runtime_generation_matches(pane_id, runtime_generation) {
                    return;
                }
                self.handle_ssh_state(pane_id, state);
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            #[cfg(feature = "ssh")]
            WindowUserEvent::HostKeyPrompt {
                pane_id,
                runtime_generation,
                challenge,
                decision,
                ..
            } => {
                if !self.pane_runtime_generation_matches(pane_id, runtime_generation) {
                    let _ = decision.send(HostKeyDecision::Cancel);
                    return;
                }
                self.handle_host_key_prompt(pane_id, challenge, decision);
                self.handle_ssh_state(pane_id, ConnectionState::AwaitingHostKey);
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            #[cfg(feature = "ssh")]
            WindowUserEvent::SecretPrompt {
                pane_id,
                runtime_generation,
                prompt,
                response,
                ..
            } => {
                if !self.pane_runtime_generation_matches(pane_id, runtime_generation) {
                    let _ = response.send(None);
                    return;
                }
                self.handle_secret_prompt(pane_id, prompt, response);
                self.handle_ssh_state(pane_id, ConnectionState::AwaitingSecret);
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
        }
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        self.shutdown_gpu_after_native_window_close();
    }

    #[allow(clippy::too_many_lines)]
    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                self.handle_window_close_requested();
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if let Err(error) = self.handle_keyboard_input(&event) {
                    eprintln!("PTY input error: {error}");
                    event_loop.exit();
                    return;
                }
                self.update_ime_cursor_area();
            }
            WindowEvent::Ime(winit::event::Ime::Commit(text)) => {
                if let Err(error) = self.handle_ime_commit(&text) {
                    eprintln!("PTY IME input error: {error}");
                    event_loop.exit();
                }
            }
            WindowEvent::Ime(winit::event::Ime::Preedit(text, _cursor)) => {
                self.handle_ime_preedit(&text);
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                self.modifiers = modifiers.state();
                if !self.modifiers.contains(ModifiersState::ALT) {
                    self.left_alt_pressed = false;
                    self.right_alt_pressed = false;
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                let previous_tab_bar_hover = self.tab_bar_hover_column();
                if let Err(error) = self.handle_cursor_moved(position) {
                    eprintln!("PTY mouse error: {error}");
                    event_loop.exit();
                } else if previous_tab_bar_hover != self.tab_bar_hover_column()
                    && let Some(window) = &self.window
                {
                    // The tab bar is rendered into the terminal snapshot, so
                    // AppKit does not invalidate it automatically on pointer
                    // motion.  Repaint only when the hover target changes;
                    // this keeps close/new-tab feedback responsive without
                    // turning every mouse move into a full frame request.
                    window.request_redraw();
                }
            }
            WindowEvent::CursorLeft { .. } => {
                let was_tab_bar_hovered = self.tab_bar_hover_column().is_some();
                self.handle_cursor_left();
                if was_tab_bar_hovered && let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if let Err(error) = self.handle_mouse_input(state, button) {
                    eprintln!("PTY mouse error: {error}");
                    event_loop.exit();
                }
            }
            WindowEvent::MouseWheel { delta, .. } => match self.handle_window_mouse_wheel(delta) {
                Ok(true) => {
                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
                }
                Ok(false) => {}
                Err(error) => {
                    eprintln!("PTY mouse error: {error}");
                    event_loop.exit();
                }
            },
            WindowEvent::DroppedFile(path) => {
                if let Err(error) = self.handle_dropped_file_path(&path) {
                    eprintln!("PTY dropped file error: {error}");
                    event_loop.exit();
                }
            }
            WindowEvent::Moved(position) => {
                self.handle_window_moved(position);
            }
            WindowEvent::Resized(size) => {
                if let Err(error) = self.handle_window_resize(size) {
                    eprintln!("resize error: {error}");
                    event_loop.exit();
                } else {
                    self.update_ime_cursor_area();
                }
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.apply_window_scale_factor(startup_window_scale_factor(
                    self.diagnostic_scale_override_enabled(),
                    scale_factor,
                ));
                self.update_ime_cursor_area();
            }
            WindowEvent::RedrawRequested => {
                self.draw_frame(event_loop);
            }
            _ => {}
        }
        if self.event_loop_exit_requested() {
            event_loop.exit();
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let now = Instant::now();
        let mut regular_redraw_needed =
            self.frame_needs_full_repaint || !self.pending_frame_damage.is_empty();
        regular_redraw_needed |= self.frame_limit_refresh_pending();
        regular_redraw_needed |= self.dispatch_update_status_if_due(now);
        let cursor_animation_changed = self.update_cursor_blink_phase_if_due(now);
        let text_animation_changed = self.update_text_blink_phase_if_due(now);
        let animation_changed = cursor_animation_changed || text_animation_changed;
        regular_redraw_needed |= self.expire_visual_bells_if_due(now);
        regular_redraw_needed |= self.expire_key_table_stack_if_due(now);
        regular_redraw_needed |= self.expire_leader_key_if_due(now);

        #[cfg(feature = "functional-test-observer")]
        crate::functional_observer::publish(self.functional_observer_snapshot());

        let animation_active = self.has_active_animation_at(now);
        if regular_redraw_needed || (animation_changed && !animation_active) {
            self.request_redraw_if_due(now);
        }
        if animation_active {
            self.request_animation_redraw_if_due(now);
        }
        if let Some(deadline) = self.frame_limit_redraw_deadline(now) {
            event_loop.set_control_flow(ControlFlow::WaitUntil(deadline));
        } else {
            event_loop.set_control_flow(ControlFlow::Wait);
        }
    }
}

impl NativeWindowApp {
    fn handle_runtime_wake_window(&mut self, event_loop: &ActiveEventLoop) {
        match self.poll_active_v2_runtime() {
            Ok(Some(true)) => {
                #[cfg(feature = "functional-test-observer")]
                {
                    crate::functional_observer::publish(self.functional_observer_snapshot());
                    let _ = crate::functional_observer::wait_until_current_revision_delivered(
                        Duration::from_millis(250),
                    );
                }
                event_loop.exit();
            }
            Ok(Some(false) | None) => {}
            Err(error) => {
                eprintln!("runtime V2 host error: {error}");
                event_loop.exit();
            }
        }
    }
}

fn app_shell_from_pty_command(
    startup_command: &PtyCommand,
    startup_workspace: Option<&str>,
) -> AppShell {
    let mut launch = PaneLaunch::local(startup_command.program())
        .with_args(startup_command.args().iter().cloned());
    if let Some(cwd) = startup_command.cwd() {
        launch = launch.with_cwd(cwd.to_string_lossy());
    }
    if let Some(profile_name) = startup_command.env_value(PROFILE_NAME_ENV) {
        launch = launch.with_environment([(PROFILE_NAME_ENV, profile_name)]);
    }
    if let Some(ssh_auth_sock) = startup_command.env_value(SSH_AUTH_SOCK_ENV) {
        launch = launch.with_environment([(SSH_AUTH_SOCK_ENV, ssh_auth_sock)]);
    }
    match startup_workspace {
        Some(workspace) => AppShell::new_with_workspace_name(launch, workspace),
        None => AppShell::new(launch),
    }
}

#[cfg(feature = "ssh")]
fn ssh_request_from_pane_launch(
    launch: &SshPaneLaunch,
    pty_size: PtySize,
) -> Result<SshConnectRequest, Box<dyn Error>> {
    let (username, host, port) = match launch.target_kind() {
        SshTargetKind::Direct => parse_ssh_gui_target(launch.target())?,
        SshTargetKind::OpenSsh => resolve_ssh_gui_openssh_target(launch)?,
    };
    let initial_size = TerminalSize::new(pty_size.columns(), pty_size.rows());
    let config = SshSessionConfig::try_new(host, port, username, initial_size.into())?;
    let auth = match launch.auth() {
        SshAuthDescription::Agent => SshAuthMethod::Agent,
        SshAuthDescription::PasswordPrompt => SshAuthMethod::PasswordPrompt,
        SshAuthDescription::PrivateKey { path } => {
            SshAuthMethod::private_key(path, None::<String>)?
        }
    };
    let startup = if launch.remote_command().is_empty() {
        SshSessionStartup::Shell
    } else {
        SshSessionStartup::command(launch.remote_command().iter().cloned())?
    };
    Ok(SshConnectRequest::new(config, auth).with_startup(startup))
}

fn resolve_ssh_gui_openssh_target(
    launch: &SshPaneLaunch,
) -> Result<(String, String, u16), Box<dyn Error>> {
    let mut command = Command::new("ssh");
    command.arg("-G");
    if let Some(username) = launch.username() {
        command.arg("-l").arg(username);
    }
    if let Some(port) = launch.port() {
        command.arg("-p").arg(port.to_string());
    }
    command.arg(launch.target());
    let output = command.output()?;
    if !output.status.success() {
        return Err(format!(
            "OpenSSH config resolution failed for target {}",
            launch.target()
        )
        .into());
    }

    let config = String::from_utf8(output.stdout)?;
    let mut host = None;
    let mut username = None;
    let mut port = None;
    for line in config.lines() {
        let mut parts = line.splitn(2, char::is_whitespace);
        let Some(key) = parts.next() else {
            continue;
        };
        let Some(value) = parts.next().map(str::trim).filter(|value| !value.is_empty()) else {
            continue;
        };
        match key.to_ascii_lowercase().as_str() {
            "hostname" => host = Some(value.to_owned()),
            "user" => username = Some(value.to_owned()),
            "port" => port = Some(value.parse::<u16>()?),
            _ => {}
        }
    }
    let host = host.unwrap_or_else(|| launch.target().to_owned());
    let username = launch
        .username()
        .map(ToOwned::to_owned)
        .or(username)
        .or_else(|| {
            std::env::var("USERNAME")
                .ok()
                .or_else(|| std::env::var("USER").ok())
        })
        .filter(|username| !username.trim().is_empty())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "OpenSSH target has no username"))?;
    let port = launch.port().or(port).unwrap_or(22);
    if host.trim().is_empty() || port == 0 {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "invalid OpenSSH host or port").into());
    }
    Ok((username, host, port))
}

fn parse_ssh_gui_target(target: &str) -> Result<(String, String, u16), Box<dyn Error>> {
    let target = target.trim();
    if target.is_empty() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "SSH target is empty").into());
    }
    let (username, authority) = target
        .split_once('@')
        .map_or((None, target), |(user, authority)| (Some(user), authority));
    let username = username
        .filter(|user| !user.trim().is_empty())
        .map(str::trim)
        .map(ToOwned::to_owned)
        .or_else(|| {
            std::env::var("USERNAME")
                .ok()
                .or_else(|| std::env::var("USER").ok())
                .filter(|user| !user.trim().is_empty())
        })
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "SSH target has no username"))?;

    let (host, port) = if let Some(bracketed) = authority.strip_prefix('[') {
        let Some((host, suffix)) = bracketed.split_once(']') else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid bracketed SSH host",
            )
            .into());
        };
        let port = suffix
            .strip_prefix(':')
            .map(str::parse::<u16>)
            .transpose()?
            .unwrap_or(22);
        (host.to_owned(), port)
    } else if authority.matches(':').count() == 1 {
        if let Some((host, port)) = authority.rsplit_once(':')
            && let Ok(port) = port.parse::<u16>()
        {
            (host.to_owned(), port)
        } else {
            (authority.to_owned(), 22)
        }
    } else {
        (authority.to_owned(), 22)
    };
    if host.trim().is_empty() || port == 0 {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "invalid SSH host or port").into());
    }
    Ok((username, host, port))
}

#[cfg(feature = "ssh")]
fn russh_host_key_policy(policy: SshKnownHostsPolicy) -> RusshHostKeyPolicy {
    match policy {
        SshKnownHostsPolicy::RejectUnknown => RusshHostKeyPolicy::RejectUnknown,
        SshKnownHostsPolicy::Prompt => RusshHostKeyPolicy::Prompt,
        SshKnownHostsPolicy::TrustOnFirstUse => RusshHostKeyPolicy::TrustOnFirstUse,
        SshKnownHostsPolicy::AcceptUnknown => RusshHostKeyPolicy::AcceptUnknown,
    }
}

#[cfg(feature = "ssh")]
fn ssh_known_hosts_path() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    let home = std::env::var_os("USERPROFILE");
    #[cfg(not(target_os = "windows"))]
    let home = std::env::var_os("HOME");
    home.map(PathBuf::from).map(|home| home.join(".ssh").join("known_hosts"))
}

fn pty_command_matches_default_shell(command: &PtyCommand) -> bool {
    let default_shell = PtyCommand::default_shell();
    command.program() == default_shell.program() && command.args() == default_shell.args()
}

fn startup_command_from_default_prog(
    default_prog: Option<&[String]>,
    cwd: Option<&Path>,
) -> Option<PtyCommand> {
    let (program, args) = default_prog?.split_first()?;
    if program.is_empty() {
        return None;
    }
    let mut command = PtyCommand::new(program).with_args(args.iter());
    if let Some(cwd) = cwd {
        command = command.with_cwd(cwd);
    }
    Some(command)
}

fn default_domain_from_override(default_domain: Option<String>) -> String {
    default_domain
        .filter(|domain| !domain.is_empty())
        .unwrap_or_else(|| DEFAULT_DOMAIN_NAME.to_owned())
}

fn default_mux_env_remove() -> Vec<String> {
    DEFAULT_MUX_ENV_REMOVE
        .iter()
        .map(ToString::to_string)
        .collect()
}

fn default_gui_startup_args() -> Vec<String> {
    DEFAULT_GUI_STARTUP_ARGS
        .iter()
        .map(ToString::to_string)
        .collect()
}

fn default_native_unix_domains() -> Vec<NativeUnixDomain> {
    vec![NativeUnixDomain {
        name: "unix".to_owned(),
        socket_path: None,
        connect_automatically: false,
        no_serve_automatically: false,
        serve_command: None,
        proxy_command: None,
        skip_permissions_check: false,
        read_timeout_ms: DEFAULT_UNIX_DOMAIN_TIMEOUT_MS,
        write_timeout_ms: DEFAULT_UNIX_DOMAIN_TIMEOUT_MS,
        local_echo_threshold_ms: None,
        overlay_lag_indicator: false,
    }]
}

fn default_tiling_desktop_environments() -> Vec<String> {
    DEFAULT_TILING_DESKTOP_ENVIRONMENTS
        .iter()
        .map(ToString::to_string)
        .collect()
}

fn is_local_domain_name(domain: &str) -> bool {
    domain.eq_ignore_ascii_case(DEFAULT_DOMAIN_NAME)
}

fn is_attach_domain_supported_locally(domain: &str, default_domain: &str) -> bool {
    let normalized = domain
        .chars()
        .filter(|character| !character.is_whitespace() && *character != '-' && *character != '_')
        .collect::<String>()
        .to_ascii_lowercase();
    if is_local_domain_name(&normalized) {
        return true;
    }
    match normalized.as_str() {
        "currentpanedomain" | "currentpane" | "current" => true,
        "defaultdomain" | "default" => is_local_domain_name(default_domain),
        _ => false,
    }
}

#[cfg(test)]
fn pty_command_from_pane_launch(launch: &PaneLaunch) -> PtyCommand {
    pty_command_from_pane_launch_with_term(launch, DEFAULT_TERM)
}

#[cfg(test)]
fn pty_command_from_pane_launch_with_term(launch: &PaneLaunch, term: &str) -> PtyCommand {
    pty_command_from_pane_launch_with_environment(launch, term, &BTreeMap::new(), None)
}

#[cfg(test)]
fn pty_command_from_pane_launch_with_default_cwd(
    launch: &PaneLaunch,
    term: &str,
    environment: &BTreeMap<String, String>,
    default_cwd: Option<&str>,
) -> PtyCommand {
    pty_command_from_pane_launch_with_environment(launch, term, environment, default_cwd)
}

#[cfg(test)]
fn pty_command_from_pane_launch_with_environment(
    launch: &PaneLaunch,
    term: &str,
    environment: &BTreeMap<String, String>,
    default_cwd: Option<&str>,
) -> PtyCommand {
    pty_command_from_pane_launch_with_optional_term_session_id(
        launch,
        term,
        environment,
        default_cwd,
        None,
    )
}

fn pty_command_from_pane_launch_with_term_session_id(
    launch: &PaneLaunch,
    term: &str,
    environment: &BTreeMap<String, String>,
    default_cwd: Option<&str>,
    term_session_id: &str,
) -> PtyCommand {
    pty_command_from_pane_launch_with_optional_term_session_id(
        launch,
        term,
        environment,
        default_cwd,
        Some(term_session_id),
    )
}

fn pty_command_from_pane_launch_with_optional_term_session_id(
    launch: &PaneLaunch,
    term: &str,
    environment: &BTreeMap<String, String>,
    default_cwd: Option<&str>,
    term_session_id: Option<&str>,
) -> PtyCommand {
    if matches!(launch.domain(), PaneLaunchDomain::Ssh(_)) {
        // SSH panes are materialized by the native channel worker. Returning
        // a local shell here is a defensive fallback for legacy call sites;
        // it prevents an empty program from ever reaching PtySession::spawn.
        return PtyCommand::default_shell();
    }
    let mut command = PtyCommand::new(launch.program()).with_args(launch.args().iter());
    let cwd = launch
        .cwd()
        .and_then(pane_launch_cwd_to_path)
        .or_else(|| default_cwd.map(PathBuf::from))
        .or_else(user_home_dir);
    if let Some(cwd) = cwd {
        command = command.with_cwd(cwd);
    }
    command = command.with_env("TERM", term);
    for (key, value) in environment {
        command = command.with_env(key, value);
    }
    for (key, value) in launch.environment() {
        command = command.with_env(key, value);
    }
    if let Some(term_session_id) = term_session_id {
        command = command.with_env("TERM_SESSION_ID", term_session_id);
    }
    command
}

fn pane_launch_cwd_to_path(cwd: &str) -> Option<PathBuf> {
    if cwd.is_empty() {
        return None;
    }

    if let Some(rest) = cwd.strip_prefix("file://") {
        let path_start = rest.find('/')?;
        let mut path = percent_decode_path_component(&rest[path_start..]);
        if cfg!(windows)
            && path.len() >= 3
            && path.as_bytes().first() == Some(&b'/')
            && path.as_bytes().get(2) == Some(&b':')
        {
            path.remove(0);
        }
        return Some(PathBuf::from(path));
    }

    Some(PathBuf::from(percent_decode_path_component(cwd)))
}

fn user_home_dir() -> Option<PathBuf> {
    if cfg!(windows) {
        windows_user_home_dir().or_else(|| env_path("HOME"))
    } else {
        env_path("HOME").or_else(windows_user_home_dir)
    }
}

fn windows_user_home_dir() -> Option<PathBuf> {
    env_path("USERPROFILE").or_else(|| {
        let drive = std::env::var_os("HOMEDRIVE")?;
        let path = std::env::var_os("HOMEPATH")?;
        if drive.is_empty() || path.is_empty() {
            return None;
        }
        let mut home = drive;
        home.push(path);
        Some(PathBuf::from(home))
    })
}

fn env_path(name: &str) -> Option<PathBuf> {
    std::env::var_os(name)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

fn percent_decode_path_component(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%'
            && index + 2 < bytes.len()
            && let (Some(high), Some(low)) =
                (hex_value(bytes[index + 1]), hex_value(bytes[index + 2]))
        {
            decoded.push(high << 4 | low);
            index += 3;
            continue;
        }

        decoded.push(bytes[index]);
        index += 1;
    }

    String::from_utf8_lossy(&decoded).into_owned()
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
#[path = "../window_terminal_delta_tests.rs"]
mod window_terminal_delta_tests;

#[cfg(test)]
#[allow(
    clippy::needless_raw_string_hashes,
    reason = "embedded Lua fixtures preserve upstream text and delimiter conventions"
)]
#[allow(
    clippy::too_many_lines,
    reason = "integration scenarios intentionally cover complete compatibility lifecycles"
)]
#[path = "../window_compat_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "../window_restart_pane_tests.rs"]
mod window_restart_pane_tests;

#[cfg(test)]
#[path = "../window_runtime_hub_tests.rs"]
mod window_runtime_hub_tests;

#[cfg(test)]
#[path = "../window_inspect_pane_tests.rs"]
mod window_inspect_pane_tests;

#[path = "../window_runtime_v2.rs"]
mod window_runtime_v2;
#[cfg(feature = "ssh")]
#[path = "../window_ssh_gui.rs"]
mod window_ssh_gui;
#[path = "../window_runtime_exit.rs"]
mod window_runtime_exit;

#[path = "../window_state_report.rs"]
mod window_state_report;

#[cfg(not(feature = "ssh"))]
#[path = "../window_without_ssh.rs"]
mod window_without_ssh;
#[cfg(not(feature = "ssh"))]
pub(crate) use window_without_ssh::run_ssh_gui;
