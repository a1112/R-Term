use std::{
    ffi::OsStr,
    fmt,
    fs::File,
    io::{self, Read, Seek, SeekFrom},
    process::{Child, Command, ExitStatus, Stdio},
    sync::{Arc, Condvar, Mutex, OnceLock},
    thread,
    time::{Duration, Instant},
};

const POLL_INTERVAL: Duration = Duration::from_millis(10);
const CLEANUP_GRACE: Duration = Duration::from_millis(500);
const EXPLICIT_TERMINATION_GRACE: Duration = Duration::from_secs(2);
const REAPER_POLL_INTERVAL: Duration = Duration::from_millis(50);
const CAPTURE_LIMIT: usize = 256 * 1024;
const OMISSION_RESERVE: usize = 96;

/// The bounded stdout, stderr, and status collected from a child process.
#[derive(Debug)]
pub struct ChildOutput {
    pub status: ExitStatus,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

/// A failure to launch, observe, or terminate a deadline-bound child process.
#[derive(Debug)]
pub enum ChildGuardError {
    /// An operating-system operation failed.
    Io {
        operation: &'static str,
        source: io::Error,
        secondary: Option<io::Error>,
        output: Option<ChildOutput>,
    },
    /// The child exceeded its deadline and was killed and reaped.
    TimedOut {
        timeout: Duration,
        output: ChildOutput,
        cleanup_error: Option<io::Error>,
        capture_error: Option<io::Error>,
    },
    /// Synchronous cleanup reached its deadline; ownership moved to a reaper.
    CleanupDeferred {
        operation: &'static str,
        source: io::Error,
        secondary: Option<io::Error>,
        stdout: Vec<u8>,
        stderr: Vec<u8>,
    },
}

impl ChildGuardError {
    /// Returns any bounded, redacted child output preserved with the error.
    #[must_use]
    pub fn output(&self) -> Option<&ChildOutput> {
        match self {
            Self::Io { output, .. } => output.as_ref(),
            Self::TimedOut { output, .. } => Some(output),
            Self::CleanupDeferred { .. } => None,
        }
    }

    /// Returns bounded, redacted stdout preserved with an error, if any.
    #[must_use]
    pub fn stdout(&self) -> Option<&[u8]> {
        match self {
            Self::Io { output, .. } => output.as_ref().map(|output| output.stdout.as_slice()),
            Self::TimedOut { output, .. } => Some(&output.stdout),
            Self::CleanupDeferred { stdout, .. } => Some(stdout),
        }
    }

    /// Returns bounded, redacted stderr preserved with an error, if any.
    #[must_use]
    pub fn stderr(&self) -> Option<&[u8]> {
        match self {
            Self::Io { output, .. } => output.as_ref().map(|output| output.stderr.as_slice()),
            Self::TimedOut { output, .. } => Some(&output.stderr),
            Self::CleanupDeferred { stderr, .. } => Some(stderr),
        }
    }
}

impl fmt::Display for ChildGuardError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io {
                operation,
                source,
                secondary,
                output,
            } => {
                write!(formatter, "child process {operation} failed: {source}")?;
                if let Some(secondary) = secondary {
                    write!(formatter, "; secondary error: {secondary}")?;
                }
                if let Some(output) = output {
                    write_diagnostics(formatter, output)?;
                }
                Ok(())
            }
            Self::TimedOut {
                timeout,
                output,
                cleanup_error,
                capture_error,
            } => {
                write!(
                    formatter,
                    "child process exceeded its {timeout:?} deadline and was killed and reaped"
                )?;
                if let Some(cleanup_error) = cleanup_error {
                    write!(formatter, "; cleanup warning: {cleanup_error}")?;
                }
                if let Some(capture_error) = capture_error {
                    write!(formatter, "; capture warning: {capture_error}")?;
                }
                write_diagnostics(formatter, output)
            }
            Self::CleanupDeferred {
                operation,
                source,
                secondary,
                stdout,
                stderr,
            } => {
                write!(
                    formatter,
                    "child process {operation}; cleanup ownership was retained for asynchronous \
                     reaping: {source}"
                )?;
                if let Some(secondary) = secondary {
                    write!(formatter, "; secondary error: {secondary}")?;
                }
                write!(
                    formatter,
                    "; stdout={:?}; stderr={:?}",
                    String::from_utf8_lossy(stdout),
                    String::from_utf8_lossy(stderr)
                )
            }
        }
    }
}

impl std::error::Error for ChildGuardError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } | Self::CleanupDeferred { source, .. } => Some(source),
            Self::TimedOut {
                cleanup_error,
                capture_error,
                ..
            } => capture_error
                .as_ref()
                .or(cleanup_error.as_ref())
                .map(|error| error as &(dyn std::error::Error + 'static)),
        }
    }
}

trait CleanupTarget {
    type Status;

    fn kill(&mut self) -> io::Result<()>;
    fn try_wait(&mut self) -> io::Result<Option<Self::Status>>;
}

impl CleanupTarget for Child {
    type Status = ExitStatus;

    fn kill(&mut self) -> io::Result<()> {
        Child::kill(self)
    }

    fn try_wait(&mut self) -> io::Result<Option<Self::Status>> {
        Child::try_wait(self)
    }
}

trait CleanupClock {
    fn now(&self) -> Instant;
    fn sleep(&mut self, duration: Duration);
}

struct SystemCleanupClock;

impl CleanupClock for SystemCleanupClock {
    fn now(&self) -> Instant {
        Instant::now()
    }

    fn sleep(&mut self, duration: Duration) {
        thread::sleep(duration);
    }
}

enum CleanupOutcome<Status> {
    Reaped {
        status: Status,
        last_error: Option<io::Error>,
    },
    Deferred {
        last_error: Option<io::Error>,
    },
}

fn bounded_cleanup<Target, Clock>(
    target: &mut Target,
    grace: Duration,
    clock: &mut Clock,
) -> CleanupOutcome<Target::Status>
where
    Target: CleanupTarget,
    Clock: CleanupClock,
{
    let started = clock.now();
    let deadline = started.checked_add(grace).unwrap_or(started);
    let (mut killed, mut last_error) = match target.kill() {
        Ok(()) => (true, None),
        Err(source) => (false, Some(source)),
    };

    loop {
        match target.try_wait() {
            Ok(Some(status)) => return CleanupOutcome::Reaped { status, last_error },
            Ok(None) => {}
            Err(source) => last_error = Some(source),
        }

        let now = clock.now();
        if now >= deadline {
            return CleanupOutcome::Deferred { last_error };
        }

        if !killed {
            match target.kill() {
                Ok(()) => killed = true,
                Err(source) => last_error = Some(source),
            }
        }
        clock.sleep(
            deadline
                .saturating_duration_since(clock.now())
                .min(POLL_INTERVAL),
        );
    }
}

fn explicit_termination_grace(deadline: Instant, now: Instant) -> Duration {
    deadline
        .saturating_duration_since(now)
        .min(EXPLICIT_TERMINATION_GRACE)
}

struct CaptureFile {
    file: tempfile::NamedTempFile,
}

impl CaptureFile {
    fn new() -> io::Result<Self> {
        tempfile::NamedTempFile::new().map(|file| Self { file })
    }

    fn reopen_writer(&self) -> io::Result<File> {
        self.file.reopen()
    }

    fn snapshot(&mut self) -> io::Result<Vec<u8>> {
        read_bounded(self.file.as_file_mut())
    }
}

/// Owns a native child process and guarantees bounded waiting plus deferred cleanup.
///
/// Dropping a live guard performs at most one cleanup grace period of synchronous
/// polling, then transfers ownership to the permanent background reaper that is
/// initialized before any guarded child is spawned. If that worker cannot start,
/// [`ChildGuard::spawn`] returns an error before creating the child.
pub struct ChildGuard {
    child: Option<Child>,
    reaper: Arc<ReaperQueue>,
    deadline: Instant,
    timeout: Duration,
    stdout: CaptureFile,
    stderr: CaptureFile,
    redactions: Vec<Vec<u8>>,
}

impl ChildGuard {
    /// Return the guarded child process identifier while it is owned.
    #[must_use]
    pub fn process_id(&self) -> Option<u32> {
        self.child.as_ref().map(Child::id)
    }

    /// Caps the remaining child deadline from the current instant.
    ///
    /// This can only shorten the original bound. It is intended for callers
    /// that have completed their interaction phase and are entering a tighter
    /// cleanup budget.
    pub fn cap_remaining_timeout(&mut self, timeout: Duration) {
        let now = Instant::now();
        let requested = now.checked_add(timeout).unwrap_or(now);
        self.deadline = self.deadline.min(requested);
        self.timeout = self.deadline.saturating_duration_since(now);
    }

    /// Spawns `command` with stdout and stderr redirected to bounded diagnostics.
    ///
    /// The child inherits no stdin. Values of explicitly configured environment
    /// keys whose names indicate credentials, keys, tokens, passwords, or home
    /// paths, plus the current directory, are redacted from error output.
    ///
    /// # Errors
    ///
    /// Returns an error if the permanent reaper cannot start, diagnostic files
    /// cannot be created or independently reopened for child writers, or the
    /// operating system cannot spawn the child.
    pub fn spawn(mut command: Command, timeout: Duration) -> Result<Self, ChildGuardError> {
        let reaper = global_reaper().map_err(|source| ChildGuardError::Io {
            operation: "initialize background child reaper",
            source,
            secondary: None,
            output: None,
        })?;
        let deadline = Instant::now()
            .checked_add(timeout)
            .ok_or_else(|| ChildGuardError::Io {
                operation: "deadline setup",
                source: io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "child timeout exceeds the platform Instant range",
                ),
                secondary: None,
                output: None,
            })?;
        let stdout = CaptureFile::new().map_err(|source| ChildGuardError::Io {
            operation: "stdout capture setup",
            source,
            secondary: None,
            output: None,
        })?;
        let stderr = CaptureFile::new().map_err(|source| ChildGuardError::Io {
            operation: "stderr capture setup",
            source,
            secondary: None,
            output: None,
        })?;
        let stdout_writer = stdout
            .reopen_writer()
            .map_err(|source| ChildGuardError::Io {
                operation: "stdout capture reopen",
                source,
                secondary: None,
                output: None,
            })?;
        let stderr_writer = stderr
            .reopen_writer()
            .map_err(|source| ChildGuardError::Io {
                operation: "stderr capture reopen",
                source,
                secondary: None,
                output: None,
            })?;
        let redactions = command_redactions(&command);

        command
            .stdin(Stdio::null())
            .stdout(Stdio::from(stdout_writer))
            .stderr(Stdio::from(stderr_writer));
        let child = command.spawn().map_err(|source| ChildGuardError::Io {
            operation: "spawn",
            source,
            secondary: None,
            output: None,
        })?;

        Ok(Self {
            child: Some(child),
            reaper,
            deadline,
            timeout,
            stdout,
            stderr,
            redactions,
        })
    }

    /// Terminates the child and waits for synchronous reaping within the cleanup grace period.
    ///
    /// # Errors
    ///
    /// Returns an I/O or capture error after reaping, or
    /// [`ChildGuardError::CleanupDeferred`] if ownership had to move to the background reaper.
    pub fn terminate(mut self) -> Result<ChildOutput, ChildGuardError> {
        let grace = explicit_termination_grace(self.deadline, Instant::now());
        let Some(child) = self.child.as_mut() else {
            return Err(missing_child_error("terminate and reap"));
        };
        match bounded_cleanup(child, grace, &mut SystemCleanupClock) {
            CleanupOutcome::Reaped { status, last_error } => {
                self.child.take();
                let (output, capture_error) = self.capture_output(status, true);
                match (last_error, capture_error) {
                    (None, None) => Ok(output),
                    (Some(source), secondary) => Err(ChildGuardError::Io {
                        operation: "terminate and reap",
                        source,
                        secondary,
                        output: Some(output),
                    }),
                    (None, Some(source)) => Err(build_completed_capture_error(
                        output,
                        source,
                        &self.redactions,
                    )),
                }
            }
            CleanupOutcome::Deferred { last_error } => {
                let (stdout, stderr, capture_error) = self.capture_diagnostics(true);
                let child = self
                    .child
                    .take()
                    .ok_or_else(|| missing_child_error("defer explicit termination cleanup"))?;
                self.reaper.enqueue(child);
                Err(ChildGuardError::CleanupDeferred {
                    operation: "explicit termination cleanup delegated to background reaper",
                    source: last_error.unwrap_or_else(|| {
                        io::Error::new(
                            io::ErrorKind::TimedOut,
                            "child remained unreaped after explicit termination",
                        )
                    }),
                    secondary: capture_error,
                    stdout,
                    stderr,
                })
            }
        }
    }

    /// Waits until the child exits or its deadline expires.
    ///
    /// # Errors
    ///
    /// Returns [`ChildGuardError::TimedOut`] only after killing and reaping the
    /// child. Operating-system polling or capture failures are returned as
    /// [`ChildGuardError::Io`]. If synchronous cleanup reaches its own bounded
    /// deadline, ownership is retained by a reaper and
    /// [`ChildGuardError::CleanupDeferred`] is returned.
    pub fn wait(mut self) -> Result<ChildOutput, ChildGuardError> {
        loop {
            let Some(child) = self.child.as_mut() else {
                return Err(missing_child_error("poll"));
            };
            let try_wait = child.try_wait();
            match try_wait {
                Ok(Some(status)) => {
                    self.child.take();
                    let (output, capture_error) = self.capture_output(status, false);
                    return match capture_error {
                        Some(source) => Err(build_completed_capture_error(
                            output,
                            source,
                            &self.redactions,
                        )),
                        None => Ok(output),
                    };
                }
                Ok(None) if Instant::now() < self.deadline => {
                    thread::sleep(
                        self.deadline
                            .saturating_duration_since(Instant::now())
                            .min(POLL_INTERVAL),
                    );
                }
                Ok(None) => return self.terminate_after_timeout(),
                Err(source) => return self.terminate_after_observation_error(source),
            }
        }
    }

    /// Polls the child once without blocking, returning captured output after
    /// it exits and `None` while it remains active.
    ///
    /// # Errors
    ///
    /// Returns an error if child observation or output capture fails. An
    /// observation failure also performs bounded cleanup before returning.
    pub fn try_wait(&mut self) -> Result<Option<ChildOutput>, ChildGuardError> {
        let Some(child) = self.child.as_mut() else {
            return Err(missing_child_error("poll"));
        };
        match child.try_wait() {
            Ok(Some(status)) => {
                self.child.take();
                let (output, capture_error) = self.capture_output(status, false);
                match capture_error {
                    Some(source) => Err(build_completed_capture_error(
                        output,
                        source,
                        &self.redactions,
                    )),
                    None => Ok(Some(output)),
                }
            }
            Ok(None) => Ok(None),
            Err(source) => match self.terminate_after_observation_error(source) {
                Ok(_) => unreachable!("observation cleanup cannot succeed as a child result"),
                Err(error) => Err(error),
            },
        }
    }

    fn terminate_after_timeout(&mut self) -> Result<ChildOutput, ChildGuardError> {
        let Some(child) = self.child.as_mut() else {
            return Err(missing_child_error("terminate after timeout"));
        };
        let outcome = bounded_cleanup(child, CLEANUP_GRACE, &mut SystemCleanupClock);
        match outcome {
            CleanupOutcome::Reaped { status, last_error } => {
                self.child.take();
                let (output, capture_error) = self.capture_output(status, true);
                Err(build_timeout_error(
                    self.timeout,
                    output,
                    last_error,
                    capture_error,
                ))
            }
            CleanupOutcome::Deferred { last_error } => {
                let (stdout, stderr, capture_error) = self.capture_diagnostics(true);
                let child = self
                    .child
                    .take()
                    .ok_or_else(|| missing_child_error("defer timeout cleanup"))?;
                self.reaper.enqueue(child);
                Err(ChildGuardError::CleanupDeferred {
                    operation: "cleanup deadline expired; delegated to background reaper",
                    source: last_error.unwrap_or_else(|| {
                        io::Error::new(
                            io::ErrorKind::TimedOut,
                            "child remained unreaped after kill and cleanup grace",
                        )
                    }),
                    secondary: capture_error,
                    stdout,
                    stderr,
                })
            }
        }
    }

    fn terminate_after_observation_error(
        &mut self,
        observation_error: io::Error,
    ) -> Result<ChildOutput, ChildGuardError> {
        let Some(child) = self.child.as_mut() else {
            return Err(missing_child_error("clean up after observation error"));
        };
        let outcome = bounded_cleanup(child, CLEANUP_GRACE, &mut SystemCleanupClock);
        match outcome {
            CleanupOutcome::Reaped { status, last_error } => {
                self.child.take();
                let (output, capture_error) = self.capture_output(status, true);
                Err(build_observation_error(
                    output,
                    observation_error,
                    last_error,
                    capture_error,
                ))
            }
            CleanupOutcome::Deferred { last_error } => {
                let (stdout, stderr, capture_error) = self.capture_diagnostics(true);
                let child = self
                    .child
                    .take()
                    .ok_or_else(|| missing_child_error("defer observation-error cleanup"))?;
                self.reaper.enqueue(child);
                Err(ChildGuardError::CleanupDeferred {
                    operation: "observation cleanup deferred to background reaper",
                    source: observation_error,
                    secondary: combine_secondary_errors(last_error, capture_error),
                    stdout,
                    stderr,
                })
            }
        }
    }

    fn capture_output(
        &mut self,
        status: ExitStatus,
        redact: bool,
    ) -> (ChildOutput, Option<io::Error>) {
        let (stdout, stderr, capture_error) = self.capture_diagnostics(redact);
        (
            ChildOutput {
                status,
                stdout,
                stderr,
            },
            capture_error,
        )
    }

    fn capture_diagnostics(&mut self, redact: bool) -> (Vec<u8>, Vec<u8>, Option<io::Error>) {
        let (mut stdout, stdout_error) = match self.stdout.snapshot() {
            Ok(stdout) => (stdout, None),
            Err(source) => (Vec::new(), Some(source)),
        };
        let (mut stderr, stderr_error) = match self.stderr.snapshot() {
            Ok(stderr) => (stderr, None),
            Err(source) => (Vec::new(), Some(source)),
        };
        if redact {
            redact_values(&mut stdout, &self.redactions);
            redact_values(&mut stderr, &self.redactions);
        }
        (stdout, stderr, stdout_error.or(stderr_error))
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let Some(child) = self.child.as_mut() else {
            return;
        };
        let outcome = bounded_cleanup(child, CLEANUP_GRACE, &mut SystemCleanupClock);
        match outcome {
            CleanupOutcome::Reaped { .. } => {
                self.child.take();
            }
            CleanupOutcome::Deferred { .. } => {
                if let Some(child) = self.child.take() {
                    self.reaper.enqueue(child);
                }
            }
        }
    }
}

struct ReaperEntry {
    child: Child,
    #[cfg(test)]
    notification: Option<std::sync::mpsc::Sender<ExitStatus>>,
}

struct ReaperQueue {
    pending: Mutex<Vec<ReaperEntry>>,
    wake: Condvar,
}

impl ReaperQueue {
    fn enqueue(&self, child: Child) {
        self.pending
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(ReaperEntry {
                child,
                #[cfg(test)]
                notification: None,
            });
        self.wake.notify_one();
    }

    #[cfg(test)]
    fn enqueue_with_notification(&self, child: Child) -> std::sync::mpsc::Receiver<ExitStatus> {
        let (sender, receiver) = std::sync::mpsc::channel();
        self.pending
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(ReaperEntry {
                child,
                notification: Some(sender),
            });
        self.wake.notify_one();
        receiver
    }
}

enum ReaperInitialization {
    Ready(Arc<ReaperQueue>),
    Failed(String),
}

static GLOBAL_REAPER: OnceLock<ReaperInitialization> = OnceLock::new();

fn global_reaper() -> io::Result<Arc<ReaperQueue>> {
    match GLOBAL_REAPER.get_or_init(initialize_reaper) {
        ReaperInitialization::Ready(reaper) => Ok(Arc::clone(reaper)),
        ReaperInitialization::Failed(message) => Err(io::Error::other(message.clone())),
    }
}

fn initialize_reaper() -> ReaperInitialization {
    let reaper = Arc::new(ReaperQueue {
        pending: Mutex::new(Vec::new()),
        wake: Condvar::new(),
    });
    let worker_reaper = Arc::clone(&reaper);
    match thread::Builder::new()
        .name("rterm-child-reaper".to_owned())
        .spawn(move || deferred_reaper_loop(&worker_reaper))
    {
        Ok(_) => ReaperInitialization::Ready(reaper),
        Err(error) => ReaperInitialization::Failed(error.to_string()),
    }
}

// `try_wait() == Some(_)` has already reaped the child. Calling `wait()` again
// would be both redundant and contrary to this module's bounded-wait contract.
#[allow(clippy::zombie_processes)]
fn deferred_reaper_loop(reaper: &ReaperQueue) -> ! {
    let mut active = Vec::new();
    loop {
        let mut pending = reaper
            .pending
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if active.is_empty() {
            while pending.is_empty() {
                pending = reaper
                    .wake
                    .wait(pending)
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
            }
        } else if pending.is_empty() {
            let (guard, _) = reaper
                .wake
                .wait_timeout(pending, REAPER_POLL_INTERVAL)
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            pending = guard;
        }
        let batch = std::mem::take(&mut *pending);
        drop(pending);
        active.extend(batch);

        let mut index = active.len();
        while index > 0 {
            index -= 1;
            let status = active[index].child.try_wait().ok().flatten();
            if let Some(status) = status {
                #[cfg(test)]
                if let Some(notification) = active[index].notification.take() {
                    let _ = notification.send(status);
                }
                #[cfg(not(test))]
                let _ = status;
                active.swap_remove(index);
            } else {
                let _ = active[index].child.kill();
            }
        }
    }
}

fn build_timeout_error(
    timeout: Duration,
    output: ChildOutput,
    cleanup_error: Option<io::Error>,
    capture_error: Option<io::Error>,
) -> ChildGuardError {
    ChildGuardError::TimedOut {
        timeout,
        output,
        cleanup_error,
        capture_error,
    }
}

fn build_completed_capture_error(
    mut output: ChildOutput,
    source: io::Error,
    redactions: &[Vec<u8>],
) -> ChildGuardError {
    redact_values(&mut output.stdout, redactions);
    redact_values(&mut output.stderr, redactions);
    ChildGuardError::Io {
        operation: "capture completed child output",
        source,
        secondary: None,
        output: Some(output),
    }
}

fn build_observation_error(
    output: ChildOutput,
    observation_error: io::Error,
    cleanup_error: Option<io::Error>,
    capture_error: Option<io::Error>,
) -> ChildGuardError {
    ChildGuardError::Io {
        operation: "observe and clean up",
        source: observation_error,
        secondary: combine_secondary_errors(cleanup_error, capture_error),
        output: Some(output),
    }
}

fn combine_secondary_errors(
    cleanup_error: Option<io::Error>,
    capture_error: Option<io::Error>,
) -> Option<io::Error> {
    match (cleanup_error, capture_error) {
        (Some(cleanup), Some(capture)) => Some(io::Error::other(format!(
            "cleanup warning: {cleanup}; capture warning: {capture}"
        ))),
        (Some(error), None) | (None, Some(error)) => Some(error),
        (None, None) => None,
    }
}

fn missing_child_error(operation: &'static str) -> ChildGuardError {
    ChildGuardError::Io {
        operation,
        source: io::Error::other("child process ownership was already released"),
        secondary: None,
        output: None,
    }
}

fn write_diagnostics(formatter: &mut fmt::Formatter<'_>, output: &ChildOutput) -> fmt::Result {
    write!(
        formatter,
        "; status={:?}; stdout={:?}; stderr={:?}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn command_redactions(command: &Command) -> Vec<Vec<u8>> {
    let mut redactions = command
        .get_envs()
        .filter(|(key, _)| sensitive_environment_name(key))
        .filter_map(|(_, value)| value)
        .map(OsStr::to_string_lossy)
        .filter(|value| !value.is_empty())
        .map(|value| value.as_bytes().to_vec())
        .collect::<Vec<_>>();
    if let Some(path) = command.get_current_dir() {
        let value = path.as_os_str().to_string_lossy();
        if !value.is_empty() {
            redactions.push(value.as_bytes().to_vec());
        }
    }
    redactions
        .sort_unstable_by(|left, right| right.len().cmp(&left.len()).then_with(|| left.cmp(right)));
    redactions.dedup();
    redactions
}

fn sensitive_environment_name(name: &OsStr) -> bool {
    let name = name.to_string_lossy().to_ascii_uppercase();
    [
        "CREDENTIAL",
        "HOME",
        "KEY",
        "PASSWORD",
        "PROFILE",
        "SECRET",
        "TOKEN",
    ]
    .iter()
    .any(|sensitive| name.contains(sensitive))
}

fn read_bounded(file: &mut File) -> io::Result<Vec<u8>> {
    let total = file.metadata()?.len();
    let retained = CAPTURE_LIMIT - OMISSION_RESERVE;
    let head_limit = retained / 2;
    let tail_limit = retained - head_limit;
    if total <= CAPTURE_LIMIT as u64 {
        file.seek(SeekFrom::Start(0))?;
        let mut output = Vec::with_capacity(usize::try_from(total).unwrap_or(CAPTURE_LIMIT));
        file.take(total).read_to_end(&mut output)?;
        return Ok(output);
    }

    file.seek(SeekFrom::Start(0))?;
    let mut head = Vec::with_capacity(head_limit);
    file.take(head_limit as u64).read_to_end(&mut head)?;
    file.seek(SeekFrom::Start(total.saturating_sub(tail_limit as u64)))?;
    let mut tail = Vec::with_capacity(tail_limit);
    file.take(tail_limit as u64).read_to_end(&mut tail)?;
    let omitted = total.saturating_sub((head.len() + tail.len()) as u64);
    head.extend_from_slice(format!("\n... <{omitted} bytes omitted> ...\n").as_bytes());
    head.extend_from_slice(&tail);
    Ok(head)
}

fn redact_values(bytes: &mut Vec<u8>, redactions: &[Vec<u8>]) {
    for redaction in redactions {
        if redaction.is_empty() {
            continue;
        }
        *bytes = replace_all(bytes, redaction, b"<redacted>");
    }
    if bytes.len() > CAPTURE_LIMIT {
        bytes.truncate(CAPTURE_LIMIT);
    }
}

fn replace_all(haystack: &[u8], needle: &[u8], replacement: &[u8]) -> Vec<u8> {
    let mut result = Vec::with_capacity(haystack.len());
    let mut rest = haystack;
    while let Some(index) = rest
        .windows(needle.len())
        .position(|candidate| candidate == needle)
    {
        result.extend_from_slice(&rest[..index]);
        result.extend_from_slice(replacement);
        rest = &rest[index + needle.len()..];
    }
    result.extend_from_slice(rest);
    result
}

#[cfg(test)]
mod tests {
    use std::{
        collections::VecDeque,
        env,
        ffi::OsStr,
        io::{self, Seek, SeekFrom, Write},
        process::{Command, Stdio},
        sync::{Arc, Barrier},
        thread,
        time::{Duration, Instant},
    };

    use super::{
        CAPTURE_LIMIT, CaptureFile, ChildGuard, ChildGuardError, CleanupClock, CleanupOutcome,
        CleanupTarget, EXPLICIT_TERMINATION_GRACE, bounded_cleanup, build_completed_capture_error,
        build_observation_error, build_timeout_error, command_redactions,
        explicit_termination_grace, global_reaper, read_bounded,
    };
    use crate::{TempHome, platform_marker_command};

    const HELPER_MODE: &str = "RTERM_TEST_HELPER_MODE";

    #[derive(Clone, Copy)]
    enum WaitAction {
        Pending,
        Reaped(u8),
        Error,
    }

    struct FakeCleanupTarget {
        failed_kills_remaining: usize,
        kill_calls: usize,
        waits: VecDeque<WaitAction>,
        default_wait: WaitAction,
    }

    impl CleanupTarget for FakeCleanupTarget {
        type Status = u8;

        fn kill(&mut self) -> io::Result<()> {
            self.kill_calls += 1;
            if self.failed_kills_remaining > 0 {
                self.failed_kills_remaining -= 1;
                Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "injected kill failure",
                ))
            } else {
                Ok(())
            }
        }

        fn try_wait(&mut self) -> io::Result<Option<Self::Status>> {
            let action = self.waits.pop_front().unwrap_or(self.default_wait);
            match action {
                WaitAction::Pending => Ok(None),
                WaitAction::Reaped(status) => Ok(Some(status)),
                WaitAction::Error => Err(io::Error::other("injected try_wait failure")),
            }
        }
    }

    struct FakeCleanupClock {
        now: Instant,
        slept: Duration,
    }

    impl CleanupClock for FakeCleanupClock {
        fn now(&self) -> Instant {
            self.now
        }

        fn sleep(&mut self, duration: Duration) {
            self.now += duration;
            self.slept += duration;
        }
    }

    fn helper_command(mode: &str) -> Command {
        let mut command = Command::new(env::current_exe().expect("current test executable"));
        command
            .args([
                "--exact",
                "process::tests::child_process_helper",
                "--nocapture",
            ])
            .env(HELPER_MODE, mode);
        command
    }

    #[cfg(windows)]
    fn timeout_command() -> Command {
        let mut command = Command::new("cmd.exe");
        command.args([
            "/D",
            "/Q",
            "/C",
            "(echo child-started)& \
             (echo diagnostic-before-timeout 1>&2)& \
             (echo %RTERM_TEST_SECRET% 1>&2)& \
             for /L %i in (0,0,1) do @rem",
        ]);
        command
    }

    #[cfg(not(windows))]
    fn timeout_command() -> Command {
        let mut command = Command::new("/bin/sh");
        command.args([
            "-c",
            "printf 'child-started\\n'; \
             printf 'diagnostic-before-timeout\\n' >&2; \
             printf '%s\\n' \"$RTERM_TEST_SECRET\" >&2; \
             while :; do :; done",
        ]);
        command
    }

    #[test]
    fn child_process_helper() {
        if let Ok("environment") = env::var(HELPER_MODE).as_deref() {
            let home = env::var_os("HOME").expect("HOME is set");
            let userprofile = env::var_os("USERPROFILE").expect("USERPROFILE is set");
            println!("HOME={}", home.to_string_lossy());
            println!("USERPROFILE={}", userprofile.to_string_lossy());
        }
    }

    #[test]
    fn child_guard_returns_output_before_deadline() {
        let output = ChildGuard::spawn(
            platform_marker_command("ready-before-deadline"),
            Duration::from_secs(5),
        )
        .expect("spawn marker command")
        .wait()
        .expect("marker command completes");

        assert!(output.status.success());
        assert_eq!(output.stdout, b"ready-before-deadline");
        assert!(output.stderr.is_empty());
    }

    #[test]
    fn child_guard_cleanup_cap_only_shortens_the_original_deadline() {
        let mut guard =
            ChildGuard::spawn(timeout_command(), Duration::from_secs(30)).expect("spawn helper");
        let original = guard.deadline;
        let before_cap = Instant::now();

        guard.cap_remaining_timeout(Duration::from_millis(100));

        assert!(guard.deadline < original);
        assert!(guard.deadline <= before_cap + Duration::from_millis(150));
        let shortened = guard.deadline;
        guard.cap_remaining_timeout(Duration::from_secs(60));
        assert_eq!(guard.deadline, shortened);
    }

    #[test]
    fn explicit_termination_grace_uses_remaining_budget_with_a_two_second_ceiling() {
        let now = Instant::now();

        assert_eq!(
            explicit_termination_grace(now + Duration::from_millis(100), now),
            Duration::from_millis(100)
        );
        assert_eq!(
            explicit_termination_grace(now + Duration::from_secs(60), now),
            Duration::from_secs(2)
        );
        assert_eq!(
            explicit_termination_grace(
                now.checked_sub(Duration::from_millis(1))
                    .expect("one millisecond before now is representable"),
                now,
            ),
            Duration::ZERO
        );
    }

    #[test]
    fn child_guard_explicit_termination_reaps_a_live_child() {
        let guard =
            ChildGuard::spawn(timeout_command(), Duration::from_secs(30)).expect("spawn helper");
        let started = Instant::now();

        let output = guard.terminate().expect("terminate and reap helper");

        assert!(started.elapsed() < EXPLICIT_TERMINATION_GRACE + Duration::from_secs(1));
        assert!(!output.status.success());
    }

    #[test]
    fn child_guard_kills_and_reaps_timed_out_child() {
        let secret = "super-sensitive-test-value";
        let mut command = timeout_command();
        command.env("RTERM_TEST_SECRET", secret);
        let mut guard =
            ChildGuard::spawn(command, Duration::from_secs(5)).expect("spawn timeout helper");
        let readiness_deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let stdout = guard
                .stdout
                .snapshot()
                .expect("snapshot timeout-helper stdout readiness");
            let stderr = guard
                .stderr
                .snapshot()
                .expect("snapshot timeout-helper stderr readiness");
            let stdout_ready = stdout
                .windows(b"child-started".len())
                .any(|part| part == b"child-started");
            let stderr_ready = stderr
                .windows(secret.len())
                .any(|part| part == secret.as_bytes());
            if stdout_ready && stderr_ready {
                break;
            }
            assert!(
                Instant::now() < readiness_deadline,
                "timeout helper readiness deadline"
            );
            thread::yield_now();
        }
        let timeout = Duration::from_millis(250);
        guard.timeout = timeout;
        guard.deadline = Instant::now() + timeout;
        let started = Instant::now();
        let error = guard.wait().expect_err("helper must time out");

        assert!(started.elapsed() < Duration::from_secs(5));
        assert!(!error.to_string().contains(secret));
        assert!(error.to_string().contains("<redacted>"));
        let ChildGuardError::TimedOut { output, .. } = error else {
            panic!("expected timeout error");
        };
        assert!(!output.status.success());
        assert!(
            output
                .stdout
                .windows(b"child-started".len())
                .any(|part| part == b"child-started")
        );
        assert!(
            output
                .stderr
                .windows(b"diagnostic-before-timeout".len())
                .any(|part| part == b"diagnostic-before-timeout")
        );
        assert!(
            output
                .stderr
                .windows(b"<redacted>".len())
                .any(|part| part == b"<redacted>")
        );
        assert!(
            !output
                .stderr
                .windows(secret.len())
                .any(|part| part == secret.as_bytes())
        );
    }

    #[test]
    fn cleanup_retries_after_kill_failure_without_blocking() {
        let mut target = FakeCleanupTarget {
            failed_kills_remaining: 1,
            kill_calls: 0,
            waits: VecDeque::from([WaitAction::Pending, WaitAction::Reaped(7)]),
            default_wait: WaitAction::Pending,
        };
        let mut clock = FakeCleanupClock {
            now: Instant::now(),
            slept: Duration::ZERO,
        };

        let outcome = bounded_cleanup(&mut target, Duration::from_millis(50), &mut clock);

        let CleanupOutcome::Reaped {
            status: 7,
            last_error: Some(error),
        } = outcome
        else {
            panic!("expected reaped status with retained kill error");
        };
        assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
        assert_eq!(target.kill_calls, 2);
        assert!(clock.slept <= Duration::from_millis(50));
    }

    #[test]
    fn cleanup_recovers_after_try_wait_error_without_blocking() {
        let mut target = FakeCleanupTarget {
            failed_kills_remaining: 0,
            kill_calls: 0,
            waits: VecDeque::from([WaitAction::Error, WaitAction::Reaped(9)]),
            default_wait: WaitAction::Pending,
        };
        let mut clock = FakeCleanupClock {
            now: Instant::now(),
            slept: Duration::ZERO,
        };

        let outcome = bounded_cleanup(&mut target, Duration::from_millis(50), &mut clock);

        let CleanupOutcome::Reaped {
            status: 9,
            last_error: Some(error),
        } = outcome
        else {
            panic!("expected reaped status with retained try_wait error");
        };
        assert_eq!(error.kind(), io::ErrorKind::Other);
        assert_eq!(target.kill_calls, 1);
        assert!(clock.slept <= Duration::from_millis(50));
    }

    #[test]
    fn cleanup_defers_unreaped_child_at_deadline() {
        let mut target = FakeCleanupTarget {
            failed_kills_remaining: usize::MAX,
            kill_calls: 0,
            waits: VecDeque::new(),
            default_wait: WaitAction::Pending,
        };
        let mut clock = FakeCleanupClock {
            now: Instant::now(),
            slept: Duration::ZERO,
        };
        let grace = Duration::from_millis(25);

        let outcome = bounded_cleanup(&mut target, grace, &mut clock);

        let CleanupOutcome::Deferred {
            last_error: Some(error),
        } = outcome
        else {
            panic!("expected deferred cleanup with retained kill error");
        };
        assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
        assert_eq!(clock.slept, grace);
        assert!(target.kill_calls > 1);
    }

    #[test]
    fn cleanup_deferred_error_exposes_preserved_diagnostics() {
        let error = ChildGuardError::CleanupDeferred {
            operation: "injected deferred cleanup",
            source: io::Error::new(io::ErrorKind::TimedOut, "injected cleanup deadline"),
            secondary: None,
            stdout: b"stdout-before-error".to_vec(),
            stderr: b"stderr-with-<redacted>".to_vec(),
        };

        assert_eq!(error.stdout(), Some(b"stdout-before-error".as_slice()));
        assert_eq!(error.stderr(), Some(b"stderr-with-<redacted>".as_slice()));
        let display = error.to_string();
        assert!(display.contains("injected deferred cleanup"));
        assert!(display.contains("stdout-before-error"));
        assert!(display.contains("<redacted>"));
    }

    #[test]
    fn concurrent_capture_snapshot_does_not_move_writer_cursor() {
        let mut capture = CaptureFile::new().expect("create independent capture");
        let mut writer = capture.reopen_writer().expect("reopen capture writer");
        writer.write_all(b"abcdef").unwrap();
        writer.seek(SeekFrom::Start(3)).unwrap();
        let ready = Arc::new(Barrier::new(2));
        let resume = Arc::new(Barrier::new(2));
        let writer_ready = Arc::clone(&ready);
        let writer_resume = Arc::clone(&resume);
        let writer_thread = thread::spawn(move || {
            writer_ready.wait();
            writer_resume.wait();
            writer.write_all(b"X").unwrap();
            writer.flush().unwrap();
        });

        ready.wait();
        assert_eq!(capture.snapshot().unwrap(), b"abcdef");
        resume.wait();
        writer_thread.join().unwrap();

        assert_eq!(capture.snapshot().unwrap(), b"abcXef");
    }

    #[test]
    fn reaper_worker_reaps_a_real_handed_off_child_before_deadline() {
        let reaper = global_reaper().expect("global reaper initialized");
        let mut command = timeout_command();
        command
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        let child = command.spawn().expect("spawn long-running reaper child");
        let notification = reaper.enqueue_with_notification(child);

        let status = notification
            .recv_timeout(Duration::from_secs(2))
            .expect("background worker reaps handed-off child");

        assert!(!status.success());
    }

    #[test]
    fn timeout_classification_survives_transient_cleanup_errors() {
        let output = ChildGuard::spawn(platform_marker_command("status"), Duration::from_secs(5))
            .unwrap()
            .wait()
            .unwrap();
        let error = build_timeout_error(
            Duration::from_millis(25),
            output,
            Some(io::Error::other("transient kill failure")),
            None,
        );

        assert!(matches!(error, ChildGuardError::TimedOut { .. }));
        assert!(error.to_string().contains("transient kill failure"));
    }

    #[test]
    fn nonblocking_poll_observes_child_completion_and_output() {
        let mut guard =
            ChildGuard::spawn(platform_marker_command("polled"), Duration::from_secs(5))
                .expect("spawn polled child");
        let deadline = Instant::now() + Duration::from_secs(2);
        let output = loop {
            if let Some(output) = guard.try_wait().expect("poll child") {
                break output;
            }
            assert!(Instant::now() < deadline, "poll completion deadline");
            thread::yield_now();
        };

        assert!(output.status.success());
        assert_eq!(output.stdout, b"polled");
    }

    #[test]
    fn observation_error_remains_primary_after_cleanup_and_capture_errors() {
        let output = ChildGuard::spawn(platform_marker_command("status"), Duration::from_secs(5))
            .unwrap()
            .wait()
            .unwrap();
        let error = build_observation_error(
            output,
            io::Error::new(io::ErrorKind::BrokenPipe, "primary observation failure"),
            Some(io::Error::other("secondary cleanup failure")),
            Some(io::Error::other("secondary capture failure")),
        );

        assert!(matches!(
            error,
            ChildGuardError::Io {
                ref source,
                ..
            } if source.kind() == io::ErrorKind::BrokenPipe
        ));
        assert!(error.to_string().contains("primary observation failure"));
        assert!(error.to_string().contains("secondary cleanup failure"));
        assert!(error.to_string().contains("secondary capture failure"));
    }

    #[test]
    fn completed_capture_error_redacts_the_other_stream() {
        let mut output =
            ChildGuard::spawn(platform_marker_command("status"), Duration::from_secs(5))
                .unwrap()
                .wait()
                .unwrap();
        output.stdout = b"safe-prefix super-secret-value".to_vec();
        output.stderr = b"super-secret-value in other stream".to_vec();
        let error = build_completed_capture_error(
            output,
            io::Error::other("injected stdout capture failure"),
            &[b"super-secret-value".to_vec()],
        );

        assert!(!error.to_string().contains("super-secret-value"));
        assert!(error.to_string().contains("<redacted>"));
    }

    #[test]
    fn automatic_redaction_ignores_tiny_non_sensitive_environment_values() {
        let mut command = Command::new("unused");
        command
            .env("RTERM_MODE", "x")
            .env("RTERM_API_TOKEN", "token-value");

        let redactions = command_redactions(&command);

        assert!(!redactions.iter().any(|value| value == b"x"));
        assert!(redactions.iter().any(|value| value == b"token-value"));
    }

    #[test]
    fn dropping_child_guard_returns_before_outer_deadline() {
        let started = Instant::now();
        let guard = ChildGuard::spawn(timeout_command(), Duration::from_secs(30))
            .expect("spawn child for Drop cleanup");

        drop(guard);

        assert!(started.elapsed() < Duration::from_secs(2));
    }

    #[test]
    fn temp_home_isolates_home_and_userprofile() {
        let original_home = env::var_os("HOME");
        let original_userprofile = env::var_os("USERPROFILE");
        let temp_home = TempHome::new().expect("create isolated home");
        let mut command = helper_command("environment");
        temp_home.apply_to(&mut command);

        let output = ChildGuard::spawn(command, Duration::from_secs(5))
            .expect("spawn environment helper")
            .wait()
            .expect("environment helper completes");
        let stdout = String::from_utf8(output.stdout).expect("UTF-8 helper output");
        let path = temp_home.path().to_string_lossy();

        assert!(stdout.contains(&format!("HOME={path}")));
        assert!(stdout.contains(&format!("USERPROFILE={path}")));
        assert_eq!(env::var_os("HOME"), original_home);
        assert_eq!(env::var_os("USERPROFILE"), original_userprofile);
        assert_eq!(
            temp_home
                .environment()
                .get(OsStr::new("HOME"))
                .map(AsRef::as_ref),
            Some(temp_home.path().as_os_str())
        );
    }

    #[test]
    fn marker_command_emits_exact_utf8_marker() {
        let marker = "R-Term-终端-🦀";
        let output = ChildGuard::spawn(platform_marker_command(marker), Duration::from_secs(5))
            .expect("spawn UTF-8 marker command")
            .wait()
            .expect("UTF-8 marker command completes");

        assert!(output.status.success());
        assert_eq!(output.stdout, marker.as_bytes());
        assert!(output.stderr.is_empty());
    }

    #[test]
    fn bounded_capture_retains_the_start_and_end() {
        let mut file = tempfile::tempfile().unwrap();
        file.write_all(b"start").unwrap();
        file.write_all(&vec![b'x'; CAPTURE_LIMIT]).unwrap();
        file.write_all(b"end").unwrap();

        let captured = read_bounded(&mut file).unwrap();

        assert!(captured.starts_with(b"start"));
        assert!(captured.ends_with(b"end"));
        assert!(captured.len() <= CAPTURE_LIMIT);
        assert!(
            captured
                .windows(b"bytes omitted".len())
                .any(|part| part == b"bytes omitted")
        );
    }
}
