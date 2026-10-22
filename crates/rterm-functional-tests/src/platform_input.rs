use std::{
    collections::BTreeMap, error::Error, fmt, path::PathBuf, process::Command, time::Duration,
};

use rterm_test_support::ChildGuard;

use crate::{ActionV1, KeyModifier, MouseButton, WindowControl};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InputBackend {
    WindowsSendInput,
    X11Xtest,
    WaylandWestonSeat,
    MacosCgEvent,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DriverOperation {
    Command {
        program: String,
        arguments: Vec<String>,
        environment: BTreeMap<String, String>,
    },
}

#[derive(Clone, Debug)]
pub struct PlatformInputDriver {
    backend: InputBackend,
    program: String,
    script: Option<String>,
    target: String,
    windows_client_close_button: bool,
    xtest_paste_key: String,
    xtest_wayland_clipboard: bool,
    xtest_close_key: String,
    xtest_close_confirm_key: Option<String>,
    xtest_web_close_point: Option<(i32, i32)>,
    environment: BTreeMap<String, String>,
}

impl PlatformInputDriver {
    /// Creates an OS-input driver from capability-gated environment state.
    ///
    /// # Errors
    ///
    /// Returns a capability error when a required helper, display, seat, or
    /// Accessibility authorization is absent.
    pub fn from_environment(
        backend: InputBackend,
        environment: &BTreeMap<String, String>,
    ) -> Result<Self, PlatformInputError> {
        let required = |name: &'static str| {
            environment
                .get(name)
                .cloned()
                .ok_or_else(|| PlatformInputError::CapabilityGate {
                    capability: capability_name(backend),
                    detail: format!("required environment variable {name} is absent"),
                })
        };
        let (program, target, mut operation_environment) = match backend {
            InputBackend::WindowsSendInput => (
                environment
                    .get("RTERM_FUNCTIONAL_POWERSHELL")
                    .cloned()
                    .unwrap_or_else(|| "powershell.exe".to_owned()),
                environment
                    .get("RTERM_FUNCTIONAL_WINDOWS_WINDOW_HANDLE")
                    .or_else(|| environment.get("RTERM_FUNCTIONAL_WINDOWS_WINDOW_TITLE"))
                    .cloned()
                    .map_or_else(|| required("RTERM_FUNCTIONAL_APP_PID"), Ok)?,
                BTreeMap::new(),
            ),
            InputBackend::X11Xtest => {
                let mut operation_environment = BTreeMap::new();
                operation_environment.insert("DISPLAY".to_owned(), required("DISPLAY")?);
                (
                    required("RTERM_FUNCTIONAL_XDOTOOL")?,
                    required("RTERM_FUNCTIONAL_X11_WINDOW")?,
                    operation_environment,
                )
            }
            InputBackend::WaylandWestonSeat => {
                if environment
                    .get("RTERM_FUNCTIONAL_WESTON_BACKEND")
                    .map(String::as_str)
                    != Some("x11")
                {
                    return Err(PlatformInputError::CapabilityGate {
                        capability: capability_name(backend),
                        detail: "Weston must use its X11 backend so injected XTEST events traverse the compositor seat".to_owned(),
                    });
                }
                let mut operation_environment = BTreeMap::new();
                operation_environment.insert("DISPLAY".to_owned(), required("DISPLAY")?);
                operation_environment
                    .insert("WAYLAND_DISPLAY".to_owned(), required("WAYLAND_DISPLAY")?);
                (
                    required("RTERM_FUNCTIONAL_XDOTOOL")?,
                    required("RTERM_FUNCTIONAL_WESTON_WINDOW")?,
                    operation_environment,
                )
            }
            InputBackend::MacosCgEvent => {
                if environment
                    .get("RTERM_FUNCTIONAL_MACOS_ACCESSIBILITY")
                    .map(String::as_str)
                    != Some("authorized")
                {
                    return Err(PlatformInputError::CapabilityGate {
                        capability: capability_name(backend),
                        detail:
                            "the self-hosted runner has not confirmed Accessibility authorization"
                                .to_owned(),
                    });
                }
                let mut operation_environment = BTreeMap::new();
                operation_environment.insert(
                    "RTERM_FUNCTIONAL_MACOS_CGEVENT_HELPER".to_owned(),
                    required("RTERM_FUNCTIONAL_MACOS_CGEVENT_HELPER")?,
                );
                (
                    "/usr/bin/xcrun".to_owned(),
                    environment
                        .get("RTERM_FUNCTIONAL_APP_PID")
                        .cloned()
                        .unwrap_or_default(),
                    operation_environment,
                )
            }
        };
        let script = input_script(backend, environment);
        let windows_client_close_button = windows_client_close_button(environment);
        let xtest_paste_key = xtest_paste_key(environment);
        let xtest_wayland_clipboard =
            configure_wayland_clipboard(backend, environment, &mut operation_environment);
        let (xtest_close_key, xtest_close_confirm_key) = xtest_close_keys(environment);
        let xtest_web_close_point = xtest_web_close_point(backend, environment)?;
        Ok(Self {
            backend,
            program,
            script,
            target,
            windows_client_close_button,
            xtest_paste_key,
            xtest_wayland_clipboard,
            xtest_close_key,
            xtest_close_confirm_key,
            xtest_web_close_point,
            environment: operation_environment,
        })
    }

    /// Converts a closed scenario action into platform driver operations.
    ///
    /// # Errors
    ///
    /// Returns an error when this backend cannot represent the requested action.
    pub fn plan(&self, action: &ActionV1) -> Result<Vec<DriverOperation>, PlatformInputError> {
        match self.backend {
            InputBackend::WindowsSendInput => self.plan_windows(action),
            InputBackend::X11Xtest | InputBackend::WaylandWestonSeat => self.plan_xtest(action),
            InputBackend::MacosCgEvent => self.plan_macos(action),
        }
    }

    /// Executes all operations for one action before the supplied deadline.
    ///
    /// # Errors
    ///
    /// Returns an error when planning, spawning, timing, or a helper process fails.
    pub fn execute(&self, action: &ActionV1, deadline: Duration) -> Result<(), PlatformInputError> {
        for operation in self.plan(action)? {
            let DriverOperation::Command {
                program,
                arguments,
                environment,
            } = operation;
            let mut command = Command::new(&program);
            command.args(&arguments).envs(environment);
            let output = ChildGuard::spawn(command, deadline)
                .map_err(|error| PlatformInputError::Execution {
                    operation: format_command(&program, &arguments),
                    detail: error.to_string(),
                })?
                .wait()
                .map_err(|error| PlatformInputError::Execution {
                    operation: format_command(&program, &arguments),
                    detail: error.to_string(),
                })?;
            if !output.status.success() {
                return Err(PlatformInputError::Execution {
                    operation: format_command(&program, &arguments),
                    detail: format!(
                        "exit={:?}; stderr={}",
                        output.status.code(),
                        String::from_utf8_lossy(&output.stderr)
                    ),
                });
            }
        }
        Ok(())
    }

    fn plan_windows(&self, action: &ActionV1) -> Result<Vec<DriverOperation>, PlatformInputError> {
        let mut arguments = vec![
            "-NoLogo".to_owned(),
            "-NoProfile".to_owned(),
            "-NonInteractive".to_owned(),
            "-ExecutionPolicy".to_owned(),
            "Bypass".to_owned(),
            "-File".to_owned(),
            self.script
                .clone()
                .expect("Windows SendInput driver has a script"),
        ];
        if let Some(handle) = self.target.strip_prefix("hwnd:") {
            arguments.extend(["-WindowHandle".to_owned(), handle.to_owned()]);
        } else if self
            .target
            .chars()
            .all(|character| character.is_ascii_digit())
        {
            arguments.extend(["-ProcessId".to_owned(), self.target.clone()]);
        } else {
            arguments.extend(["-WindowTitle".to_owned(), self.target.clone()]);
        }
        let mut planned_action = action_arguments(action)?;
        if self.windows_client_close_button
            && matches!(
                action,
                ActionV1::WindowControl {
                    operation: WindowControl::Close
                }
            )
        {
            "close-client-button".clone_into(&mut planned_action[1]);
        }
        let mut action = planned_action.into_iter();
        arguments.extend([
            "-Action".to_owned(),
            action
                .next()
                .expect("all supported Windows actions have a name"),
        ]);
        let action_arguments: Vec<_> = action.collect();
        if !action_arguments.is_empty() {
            let encoded = serde_json::to_string(&action_arguments).map_err(|error| {
                PlatformInputError::UnsupportedAction(format!(
                    "encode Windows action arguments: {error}"
                ))
            })?;
            arguments.extend(["-ActionArgumentsJson".to_owned(), encoded]);
        }
        Ok(vec![DriverOperation::Command {
            program: self.program.clone(),
            arguments,
            environment: self.environment.clone(),
        }])
    }

    fn plan_xtest(&self, action: &ActionV1) -> Result<Vec<DriverOperation>, PlatformInputError> {
        let command = |arguments: Vec<String>| DriverOperation::Command {
            program: self.program.clone(),
            arguments,
            environment: self.environment.clone(),
        };
        let focus = || {
            command(vec![
                self.xtest_focus_command().to_owned(),
                "--sync".to_owned(),
                self.target.clone(),
            ])
        };
        let mut operations = vec![focus()];
        if self.backend == InputBackend::WaylandWestonSeat
            && !matches!(action, ActionV1::FocusWindow)
        {
            // XTEST focuses the X11 window that hosts nested Weston. Give the
            // compositor one bounded dispatch interval to transfer that focus
            // to its Wayland seat before sending the actual input event.
            operations.push(command(vec!["sleep".to_owned(), "0.2".to_owned()]));
        }
        match action {
            ActionV1::TypeText { text } => operations.push(command(vec![
                "type".to_owned(),
                "--clearmodifiers".to_owned(),
                "--delay".to_owned(),
                "0".to_owned(),
                text.clone(),
            ])),
            ActionV1::Key { key, modifiers } => operations.push(command(vec![
                "key".to_owned(),
                "--clearmodifiers".to_owned(),
                xtest_key(key, modifiers),
            ])),
            ActionV1::MouseClick { x, y, button } => {
                operations.push(command(mouse_move_arguments(&self.target, *x, *y)));
                operations.push(command(vec![
                    "click".to_owned(),
                    mouse_button_number(*button).to_string(),
                ]));
            }
            ActionV1::MouseDrag {
                from_x,
                from_y,
                to_x,
                to_y,
                button,
            } => {
                operations.push(command(mouse_move_arguments(
                    &self.target,
                    *from_x,
                    *from_y,
                )));
                let button = mouse_button_number(*button).to_string();
                operations.push(command(vec!["mousedown".to_owned(), button.clone()]));
                operations.push(command(mouse_move_arguments(&self.target, *to_x, *to_y)));
                operations.push(command(vec!["mouseup".to_owned(), button]));
            }
            ActionV1::MouseWheel { delta_x, delta_y } => {
                for _ in 0..delta_y.unsigned_abs() {
                    operations.push(command(vec![
                        "click".to_owned(),
                        if *delta_y < 0 { "5" } else { "4" }.to_owned(),
                    ]));
                }
                for _ in 0..delta_x.unsigned_abs() {
                    operations.push(command(vec![
                        "click".to_owned(),
                        if *delta_x < 0 { "7" } else { "6" }.to_owned(),
                    ]));
                }
            }
            ActionV1::ClipboardPaste { text } => {
                operations.extend(self.xtest_clipboard_operations(text));
            }
            ActionV1::ResizeWindow { width, height } => operations.push(command(vec![
                "windowsize".to_owned(),
                self.target.clone(),
                width.to_string(),
                height.to_string(),
            ])),
            ActionV1::WindowControl {
                operation: WindowControl::Close,
            } if self.xtest_web_close_point.is_some() => {
                let (x, y) = self.xtest_web_close_point.expect("guarded Web close point");
                operations.push(command(mouse_move_arguments(&self.target, x, y)));
                operations.push(command(vec!["click".to_owned(), "1".to_owned()]));
            }
            ActionV1::WindowControl { operation } => {
                for arguments in xtest_window_control_commands(
                    *operation,
                    &self.xtest_close_key,
                    self.xtest_close_confirm_key.as_deref(),
                ) {
                    operations.push(command(arguments));
                }
            }
            ActionV1::FocusWindow => {}
            action => return Err(PlatformInputError::UnsupportedAction(format!("{action:?}"))),
        }
        operations.push(command(vec!["sleep".to_owned(), "0.1".to_owned()]));
        self.append_xtest_clipboard_cleanup(action, &mut operations);
        Ok(operations)
    }

    fn xtest_clipboard_operations(&self, text: &str) -> [DriverOperation; 2] {
        [
            DriverOperation::Command {
                program: "bash".to_owned(),
                arguments: vec![
                    self.script
                        .clone()
                        .expect("XTEST driver has a clipboard helper"),
                    text.to_owned(),
                ],
                environment: self.environment.clone(),
            },
            DriverOperation::Command {
                program: self.program.clone(),
                arguments: vec![
                    "key".to_owned(),
                    "--clearmodifiers".to_owned(),
                    self.xtest_paste_key.clone(),
                ],
                environment: self.environment.clone(),
            },
        ]
    }

    fn append_xtest_clipboard_cleanup(
        &self,
        action: &ActionV1,
        operations: &mut Vec<DriverOperation>,
    ) {
        if !matches!(action, ActionV1::ClipboardPaste { .. }) || !self.xtest_wayland_clipboard {
            return;
        }
        operations.push(DriverOperation::Command {
            program: "bash".to_owned(),
            arguments: vec![
                self.script
                    .clone()
                    .expect("XTEST driver has a clipboard helper"),
                "--clear".to_owned(),
            ],
            environment: self.environment.clone(),
        });
    }

    fn xtest_focus_command(&self) -> &'static str {
        match self.backend {
            InputBackend::WaylandWestonSeat => "windowfocus",
            InputBackend::X11Xtest => "windowactivate",
            _ => unreachable!("XTEST planning is only used for X11-backed inputs"),
        }
    }

    fn plan_macos(&self, action: &ActionV1) -> Result<Vec<DriverOperation>, PlatformInputError> {
        let mut arguments = vec![
            "swift".to_owned(),
            self.environment
                .get("RTERM_FUNCTIONAL_MACOS_CGEVENT_HELPER")
                .cloned()
                .ok_or(PlatformInputError::CapabilityGate {
                    capability: capability_name(self.backend),
                    detail: "required environment variable RTERM_FUNCTIONAL_MACOS_CGEVENT_HELPER is absent".to_owned(),
                })?,
        ];
        arguments.extend(action_arguments(action)?);
        if !self.target.is_empty() {
            arguments.extend(["--pid".to_owned(), self.target.clone()]);
        }
        Ok(vec![DriverOperation::Command {
            program: self.program.clone(),
            arguments,
            environment: self.environment.clone(),
        }])
    }
}

fn xtest_close_keys(environment: &BTreeMap<String, String>) -> (String, Option<String>) {
    let close = environment
        .get("RTERM_FUNCTIONAL_XTEST_CLOSE_KEY")
        .cloned()
        .unwrap_or_else(|| "alt+F4".to_owned());
    let confirm = environment
        .get("RTERM_FUNCTIONAL_XTEST_CLOSE_CONFIRM_KEY")
        .cloned();
    (close, confirm)
}

fn windows_client_close_button(environment: &BTreeMap<String, String>) -> bool {
    environment
        .get("RTERM_FUNCTIONAL_WINDOWS_CLIENT_CLOSE_BUTTON")
        .is_some_and(|value| value == "1")
}

fn xtest_paste_key(environment: &BTreeMap<String, String>) -> String {
    environment
        .get("RTERM_FUNCTIONAL_XTEST_PASTE_KEY")
        .cloned()
        .unwrap_or_else(|| "ctrl+shift+v".to_owned())
}

fn configure_wayland_clipboard(
    backend: InputBackend,
    environment: &BTreeMap<String, String>,
    operation_environment: &mut BTreeMap<String, String>,
) -> bool {
    let enabled = backend == InputBackend::WaylandWestonSeat
        && environment
            .get("RTERM_FUNCTIONAL_WAYLAND_CLIPBOARD")
            .map(String::as_str)
            == Some("1");
    if enabled {
        operation_environment.insert(
            "RTERM_FUNCTIONAL_WAYLAND_CLIPBOARD".to_owned(),
            "1".to_owned(),
        );
    }
    enabled
}

fn xtest_window_control_commands(
    operation: WindowControl,
    close_key: &str,
    close_confirm_key: Option<&str>,
) -> Vec<Vec<String>> {
    let key = match operation {
        WindowControl::Minimize => "alt+F9",
        WindowControl::Maximize | WindowControl::Restore => "alt+F10",
        WindowControl::Close => close_key,
    };
    let mut commands = vec![vec![
        "key".to_owned(),
        "--clearmodifiers".to_owned(),
        key.to_owned(),
    ]];
    if matches!(operation, WindowControl::Close)
        && let Some(confirm_key) = close_confirm_key
    {
        commands.push(vec!["sleep".to_owned(), "0.1".to_owned()]);
        commands.push(vec![
            "key".to_owned(),
            "--clearmodifiers".to_owned(),
            confirm_key.to_owned(),
        ]);
    }
    commands
}

fn xtest_web_close_point(
    backend: InputBackend,
    environment: &BTreeMap<String, String>,
) -> Result<Option<(i32, i32)>, PlatformInputError> {
    environment
        .get("RTERM_FUNCTIONAL_XTEST_WEB_CLOSE_POINT")
        .map(|value| {
            let invalid = || PlatformInputError::CapabilityGate {
                capability: capability_name(backend),
                detail: format!("invalid Web close point {value:?}"),
            };
            let (x, y) = value.split_once(',').ok_or_else(invalid)?;
            Ok((
                x.parse::<i32>().map_err(|_| invalid())?,
                y.parse::<i32>().map_err(|_| invalid())?,
            ))
        })
        .transpose()
}

fn input_script(backend: InputBackend, environment: &BTreeMap<String, String>) -> Option<String> {
    let (name, default) = match backend {
        InputBackend::WindowsSendInput => (
            "RTERM_FUNCTIONAL_WINDOWS_SENDINPUT",
            "scripts/functional/windows-send-input.ps1",
        ),
        InputBackend::X11Xtest | InputBackend::WaylandWestonSeat => (
            "RTERM_FUNCTIONAL_X11_CLIPBOARD_HELPER",
            "scripts/functional/x11-set-clipboard.sh",
        ),
        InputBackend::MacosCgEvent => return None,
    };
    Some(
        environment
            .get(name)
            .cloned()
            .unwrap_or_else(|| default.to_owned()),
    )
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PlatformInputError {
    CapabilityGate {
        capability: &'static str,
        detail: String,
    },
    UnsupportedAction(String),
    Execution {
        operation: String,
        detail: String,
    },
}

impl fmt::Display for PlatformInputError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CapabilityGate { capability, detail } => {
                write!(formatter, "capability gate {capability} failed: {detail}")
            }
            Self::UnsupportedAction(action) => write!(formatter, "unsupported OS action {action}"),
            Self::Execution { operation, detail } => {
                write!(
                    formatter,
                    "OS input operation `{operation}` failed: {detail}"
                )
            }
        }
    }
}

impl Error for PlatformInputError {}

fn capability_name(backend: InputBackend) -> &'static str {
    match backend {
        InputBackend::WindowsSendInput => "windows_send_input",
        InputBackend::X11Xtest => "x11_xtest",
        InputBackend::WaylandWestonSeat => "wayland_weston_x11_seat",
        InputBackend::MacosCgEvent => "macos_accessibility",
    }
}

fn action_arguments(action: &ActionV1) -> Result<Vec<String>, PlatformInputError> {
    let arguments = match action {
        ActionV1::TypeText { text } => vec!["type".to_owned(), text.clone()],
        ActionV1::Key { key, modifiers } => {
            let mut arguments = vec!["key".to_owned(), key.clone()];
            arguments.extend(
                modifiers
                    .iter()
                    .map(|modifier| format!("{modifier:?}").to_lowercase()),
            );
            arguments
        }
        ActionV1::MouseClick { x, y, button } => vec![
            "click".to_owned(),
            x.to_string(),
            y.to_string(),
            format!("{button:?}").to_lowercase(),
        ],
        ActionV1::MouseDrag {
            from_x,
            from_y,
            to_x,
            to_y,
            button,
        } => vec![
            "drag".to_owned(),
            from_x.to_string(),
            from_y.to_string(),
            to_x.to_string(),
            to_y.to_string(),
            format!("{button:?}").to_lowercase(),
        ],
        ActionV1::MouseWheel { delta_x, delta_y } => {
            vec!["wheel".to_owned(), delta_x.to_string(), delta_y.to_string()]
        }
        ActionV1::ClipboardPaste { text } => vec!["paste".to_owned(), text.clone()],
        ActionV1::ResizeWindow { width, height } => {
            vec!["resize".to_owned(), width.to_string(), height.to_string()]
        }
        ActionV1::WindowControl { operation } => vec![
            "window".to_owned(),
            window_control_name(*operation).to_owned(),
        ],
        ActionV1::FocusWindow => vec!["focus".to_owned()],
        action => return Err(PlatformInputError::UnsupportedAction(format!("{action:?}"))),
    };
    Ok(arguments)
}

const fn window_control_name(operation: WindowControl) -> &'static str {
    match operation {
        WindowControl::Minimize => "minimize",
        WindowControl::Maximize => "maximize",
        WindowControl::Restore => "restore",
        WindowControl::Close => "close",
    }
}

fn xtest_key(key: &str, modifiers: &[KeyModifier]) -> String {
    let key = if key.eq_ignore_ascii_case("enter") {
        "Return"
    } else {
        key
    };
    let mut components: Vec<_> = modifiers
        .iter()
        .map(|modifier| match modifier {
            KeyModifier::Shift => "shift",
            KeyModifier::Ctrl => "ctrl",
            KeyModifier::Alt => "alt",
            KeyModifier::Super => "super",
        })
        .collect();
    components.push(key);
    components.join("+")
}

fn mouse_button_number(button: MouseButton) -> u8 {
    match button {
        MouseButton::Left => 1,
        MouseButton::Middle => 2,
        MouseButton::Right => 3,
    }
}

fn mouse_move_arguments(target: &str, x: i32, y: i32) -> Vec<String> {
    vec![
        "mousemove".to_owned(),
        "--window".to_owned(),
        target.to_owned(),
        x.to_string(),
        y.to_string(),
    ]
}

fn format_command(program: &str, arguments: &[String]) -> String {
    let mut value = PathBuf::from(program).display().to_string();
    for argument in arguments {
        value.push(' ');
        value.push_str(argument);
    }
    value
}
