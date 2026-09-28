use std::fmt;
use std::io::{self, Read, Write};
use std::num::NonZeroU64;
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use rssh_terminal::Terminal;
use rterm_types::TerminalSize;

use crate::{
    BatchAdmission, BatchPolicy, BatchWindow, Clock, EffectSequence, MailboxItem, MailboxLimits,
    MailboxReceiver, MailboxSender, MetadataChange, MetadataChangeRef, PaneMetadataDelta,
    PaneToken, PresentationFrame, PublishedEffect, RecvTimeoutError, RuntimeBatchMetrics,
    RuntimeBuffers, RuntimeEffect, RuntimeEffectKind, RuntimeRevision, SessionExit,
    SessionInterrupt, SessionTransport, SubmitResult, TerminalRuntime, TerminalStateSummary,
    TryRecvError, TrySendError, UserVarDelta, batch::PanePublication, bounded_mailbox,
    delta::RuntimeEffectRef,
};

const BACKPRESSURE_RETRY: Duration = Duration::from_millis(1);
const EXIT_POLL_INTERVAL: Duration = Duration::from_millis(100);
const READER_CHUNK_BYTES: usize = 8 * 1024;

/// Bounded resources and initial terminal state for one pane worker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PaneWorkerConfig {
    /// Initial terminal grid size.
    pub size: TerminalSize,
    /// Shared bounded inbox used by controller commands and reader events.
    pub inbox_limits: MailboxLimits,
    /// Byte/item budget for ordered host effects awaiting a drain.
    pub effect_limits: MailboxLimits,
    /// Byte/item/time coalescing policy for transport output.
    pub batch_policy: BatchPolicy,
    /// Whether to retain ordered host-stream bytes for an explicitly enabled
    /// diagnostic or linkage consumer.
    pub capture_host_stream: bool,
    /// Whether to retain filtered terminal-visible bytes for a session log.
    pub capture_visible_output: bool,
}

impl Default for PaneWorkerConfig {
    fn default() -> Self {
        Self {
            size: TerminalSize::new(80, 24),
            inbox_limits: MailboxLimits::try_new(256, 2 * 1024 * 1024)
                .expect("default pane mailbox limits are nonzero"),
            effect_limits: MailboxLimits::try_new(1024, 4 * 1024 * 1024)
                .expect("default effect mailbox limits are nonzero"),
            batch_policy: BatchPolicy::default(),
            capture_host_stream: false,
            capture_visible_output: false,
        }
    }
}

/// Lossless lifecycle or terminal-progress notice emitted by a pane worker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaneNotice {
    /// The worker owns all session resources and accepts commands.
    Ready(PaneToken),
    /// The pane transitioned from idle to publication-ready.
    Wake(PaneToken),
    /// A user-input or terminal-response write reached the transport.
    InputWriteCompleted {
        /// Pane generation that owns the transport.
        pane: PaneToken,
        /// Bytes accepted by the transport write.
        byte_count: usize,
        /// Time spent completing the write.
        elapsed: Duration,
    },
    /// The reader observed the first non-empty PTY chunk.
    FirstPtyByte {
        /// Pane generation that owns the reader.
        pane: PaneToken,
        /// Monotonic time at which the reader returned the first bytes.
        observed_at: Instant,
    },
    /// The worker and its blocking reader have stopped.
    Closed {
        /// Pane generation that stopped.
        pane: PaneToken,
        /// Transport exit status when it was available.
        exit: Option<SessionExit>,
    },
}

impl PaneNotice {
    /// Returns the pane generation that emitted this notice.
    #[must_use]
    pub const fn pane(&self) -> PaneToken {
        match self {
            Self::Ready(pane)
            | Self::Wake(pane)
            | Self::InputWriteCompleted { pane, .. }
            | Self::FirstPtyByte { pane, .. }
            | Self::Closed { pane, .. } => *pane,
        }
    }
}

impl MailboxItem for PaneNotice {
    fn retained_bytes(&self) -> usize {
        match self {
            Self::Ready(_)
            | Self::Wake(_)
            | Self::InputWriteCompleted { .. }
            | Self::FirstPtyByte { .. } => 0,
            Self::Closed { exit, .. } => exit.as_ref().map_or(0, |exit| {
                exit.signal.as_ref().map_or(0, |signal| {
                    signal
                        .name
                        .capacity()
                        .saturating_add(signal.error_message.capacity())
                        .saturating_add(signal.lang_tag.capacity())
                })
            }),
        }
    }
}

#[derive(Debug)]
enum PaneMessage {
    Input(Vec<u8>),
    Resize(TerminalSize),
    Output(Vec<u8>),
    ReaderEof,
    ReaderError,
    Close,
}

impl MailboxItem for PaneMessage {
    fn retained_bytes(&self) -> usize {
        match self {
            Self::Input(bytes) | Self::Output(bytes) => bytes.capacity(),
            Self::Resize(_) | Self::ReaderEof | Self::ReaderError | Self::Close => 0,
        }
    }
}

#[derive(Clone)]
pub(crate) struct ErasedInterrupt {
    call: Arc<dyn Fn() -> io::Result<()> + Send + Sync>,
}

impl fmt::Debug for ErasedInterrupt {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ErasedInterrupt(..)")
    }
}

impl ErasedInterrupt {
    fn new<I: SessionInterrupt>(interrupt: I) -> Self {
        Self {
            call: Arc::new(move || interrupt.interrupt()),
        }
    }

    pub(crate) fn interrupt(&self) -> io::Result<()> {
        (self.call)()
    }
}

/// Cloneable controller endpoint for one pane generation.
#[derive(Clone)]
pub struct PaneHandle {
    token: PaneToken,
    sender: MailboxSender<PaneMessage>,
    accepting: Arc<AtomicBool>,
}

impl fmt::Debug for PaneHandle {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PaneHandle")
            .field("token", &self.token)
            .field("accepting", &self.accepting.load(Ordering::Acquire))
            .finish_non_exhaustive()
    }
}

impl PaneHandle {
    /// Returns the exact pane generation owned by this handle.
    #[must_use]
    pub const fn token(&self) -> PaneToken {
        self.token
    }

    /// Submits ordered user input without silently dropping it.
    pub fn submit_input(&self, bytes: Vec<u8>) -> SubmitResult {
        self.submit(PaneMessage::Input(bytes))
    }

    /// Submits a terminal resize to the same one-owner worker.
    pub fn resize(&self, size: TerminalSize) -> SubmitResult {
        self.submit(PaneMessage::Resize(size))
    }

    /// Returns bounded inbox occupancy and high-water accounting.
    #[must_use]
    pub fn inbox_metrics(&self) -> crate::MailboxMetrics {
        self.sender.metrics()
    }

    fn submit(&self, message: PaneMessage) -> SubmitResult {
        if !self.accepting.load(Ordering::Acquire) {
            return SubmitResult::Closed;
        }
        match self.sender.try_send(message) {
            Ok(()) => SubmitResult::Accepted,
            Err(TrySendError::Closed(_)) => SubmitResult::Closed,
            Err(TrySendError::Full { .. } | TrySendError::Oversize { .. }) => {
                SubmitResult::Backpressured {
                    retry_after: BACKPRESSURE_RETRY,
                }
            }
        }
    }

    pub(crate) fn stop_accepting(&self) {
        self.accepting.store(false, Ordering::Release);
    }

    pub(crate) fn request_close(&self) {
        self.stop_accepting();
        let _ = self.sender.try_send(PaneMessage::Close);
    }
}

pub(crate) struct SpawnedPane {
    pub handle: PaneHandle,
    pub interrupt: ErasedInterrupt,
    pub publication: Arc<PanePublication>,
    pub join: JoinHandle<()>,
}

struct LiveThreadGuard {
    count: Arc<AtomicUsize>,
}

impl LiveThreadGuard {
    fn new(count: Arc<AtomicUsize>) -> Self {
        count.fetch_add(1, Ordering::AcqRel);
        Self { count }
    }
}

impl Drop for LiveThreadGuard {
    fn drop(&mut self) {
        self.count.fetch_sub(1, Ordering::AcqRel);
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "pane spawn transfers each independently owned worker resource exactly once"
)]
pub(crate) fn spawn_pane<T: SessionTransport, C: Clock>(
    token: PaneToken,
    transport: T,
    config: PaneWorkerConfig,
    runtime: TerminalRuntime,
    clock: C,
    notices: MailboxSender<PaneNotice>,
    notice_waker: Arc<dyn Fn() + Send + Sync>,
    live_threads: &Arc<AtomicUsize>,
) -> io::Result<SpawnedPane> {
    let parts = transport.split();
    let interrupt = ErasedInterrupt::new(parts.interrupt.clone());
    let (sender, receiver) = bounded_mailbox(config.inbox_limits);
    let accepting = Arc::new(AtomicBool::new(true));
    let publication = Arc::new(PanePublication::new(config.effect_limits));

    let reader_sender = sender.clone();
    let reader_notices = notices.clone();
    let reader_notice_waker = Arc::clone(&notice_waker);
    let reader_interrupt = interrupt.clone();
    let reader_live_threads = Arc::clone(live_threads);
    let reader_join = thread::Builder::new()
        .name(format!("rssh-pane-reader-{}", token.pane().get()))
        .spawn(move || {
            let _guard = LiveThreadGuard::new(reader_live_threads);
            run_reader(
                parts.reader,
                &reader_sender,
                token,
                &reader_notices,
                &reader_notice_waker,
            );
        })?;

    let worker_interrupt = interrupt.clone();
    let worker_live_threads = Arc::clone(live_threads);
    let worker_accepting = Arc::clone(&accepting);
    let worker_publication = Arc::clone(&publication);
    let join = match thread::Builder::new()
        .name(format!("rssh-pane-worker-{}", token.pane().get()))
        .spawn(move || {
            let _guard = LiveThreadGuard::new(worker_live_threads);
            run_worker(
                token,
                config,
                runtime,
                &clock,
                receiver,
                parts.writer,
                parts.control,
                &worker_interrupt,
                &worker_accepting,
                reader_join,
                &notices,
                &notice_waker,
                &worker_publication,
            );
        }) {
        Ok(join) => join,
        Err(error) => {
            let _ = reader_interrupt.interrupt();
            return Err(error);
        }
    };

    Ok(SpawnedPane {
        handle: PaneHandle {
            token,
            sender,
            accepting,
        },
        interrupt,
        publication,
        join,
    })
}

fn run_reader<R: Read>(
    mut reader: R,
    sender: &MailboxSender<PaneMessage>,
    token: PaneToken,
    notices: &MailboxSender<PaneNotice>,
    notice_waker: &Arc<dyn Fn() + Send + Sync>,
) {
    let mut buffer = vec![0; READER_CHUNK_BYTES];
    let mut first_byte_observed = false;
    loop {
        let message = match reader.read(&mut buffer) {
            Ok(0) => PaneMessage::ReaderEof,
            Ok(count) => {
                if !first_byte_observed {
                    first_byte_observed = true;
                    if send_notice(
                        notices,
                        notice_waker,
                        PaneNotice::FirstPtyByte {
                            pane: token,
                            observed_at: Instant::now(),
                        },
                    )
                    .is_err()
                    {
                        break;
                    }
                }
                PaneMessage::Output(buffer[..count].to_vec())
            }
            Err(_) => PaneMessage::ReaderError,
        };
        let terminal = matches!(message, PaneMessage::ReaderEof | PaneMessage::ReaderError);
        if sender.send(message).is_err() || terminal {
            break;
        }
    }
}

fn next_worker_message<S: crate::SessionControl>(
    receiver: &mut MailboxReceiver<PaneMessage>,
    pending_message: &mut Option<PaneMessage>,
    control: &mut S,
    observed_exit: &mut Option<crate::SessionExit>,
) -> Option<PaneMessage> {
    if let Some(message) = pending_message.take() {
        return Some(message);
    }
    loop {
        match receiver.recv_timeout(EXIT_POLL_INTERVAL) {
            Ok(message) => return Some(message),
            Err(RecvTimeoutError::Timeout) => {
                if observed_exit.is_some() {
                    continue;
                }
                match control.poll_exit() {
                    Ok(Some(exit)) => {
                        *observed_exit = Some(exit);
                        // Child exit and PTY EOF are separate observations.
                        // Begin the platform close so its reader can flush any
                        // bytes buffered behind the exit notification, then
                        // keep consuming until ReaderEof/ReaderError arrives.
                        if control.begin_close().is_err() {
                            return None;
                        }
                    }
                    Ok(None) => {}
                    Err(_) => return None,
                }
            }
            Err(RecvTimeoutError::Closed) => return None,
        }
    }
}

// Closing the slave side is observable before waitpid/remote exit metadata on
// some transports. Preserve a natural exit before any out-of-band termination,
// while bounding the case where a program closes its output but keeps running.
fn poll_exit_after_reader_eof<S: crate::SessionControl>(control: &mut S) -> Option<SessionExit> {
    let deadline = Instant::now() + Duration::from_millis(100);
    loop {
        match control.poll_exit() {
            Ok(Some(exit)) => return Some(exit),
            Err(_) => return None,
            Ok(None) => {}
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return None;
        }
        thread::sleep(remaining.min(Duration::from_millis(5)));
    }
}

fn configure_runtime(runtime: &mut TerminalRuntime, capture_host_stream: bool) -> RuntimeBuffers {
    runtime.set_capture_host_stream(capture_host_stream);
    RuntimeBuffers::default()
}

const fn initial_sequences() -> (RuntimeRevision, Option<EffectSequence>) {
    (RuntimeRevision::FIRST, Some(EffectSequence::FIRST))
}

#[expect(
    clippy::too_many_arguments,
    reason = "worker entry receives each uniquely owned session resource exactly once"
)]
fn run_worker<W: Write, S: crate::SessionControl, C: Clock>(
    token: PaneToken,
    config: PaneWorkerConfig,
    mut runtime: TerminalRuntime,
    clock: &C,
    mut receiver: MailboxReceiver<PaneMessage>,
    mut writer: W,
    mut control: S,
    interrupt: &ErasedInterrupt,
    accepting: &Arc<AtomicBool>,
    reader_join: JoinHandle<()>,
    notices: &MailboxSender<PaneNotice>,
    notice_waker: &Arc<dyn Fn() + Send + Sync>,
    publication: &Arc<PanePublication>,
) {
    let mut buffers = configure_runtime(&mut runtime, config.capture_host_stream);
    let (mut revision, mut next_effect_sequence) = initial_sequences();
    let (mut pending_message, mut observed_exit) = (None, None);
    let mut batch_bytes = Vec::with_capacity(config.batch_policy.max_bytes());
    if send_notice(notices, notice_waker, PaneNotice::Ready(token)).is_err() {
        accepting.store(false, Ordering::Release);
        let _ = interrupt.interrupt();
        let _ = reader_join.join();
        return;
    }

    loop {
        let Some(message) = next_worker_message(
            &mut receiver,
            &mut pending_message,
            &mut control,
            &mut observed_exit,
        ) else {
            break;
        };
        let result = match message {
            PaneMessage::Input(bytes) => {
                report_write(&mut writer, &bytes, token, notices, notice_waker)
            }
            PaneMessage::Resize(new_size) => control.resize(new_size).and_then(|()| {
                let (_, delta) = runtime.resize_into(new_size, &mut buffers);
                let (snapshot, state, metrics) =
                    capture_presentation(&runtime, clock, RuntimeBatchMetrics::default());
                DeltaPublisher {
                    token,
                    writer: &mut writer,
                    notices,
                    notice_waker,
                    publication,
                    next_effect_sequence: &mut next_effect_sequence,
                    capture_visible_output: config.capture_visible_output,
                }
                .publish(revision, delta, snapshot, state, metrics, true)?;
                revision = next_revision(revision)?;
                Ok(())
            }),
            PaneMessage::Output(bytes) => {
                let batch_metrics = collect_output_batch(
                    &bytes,
                    &mut receiver,
                    &mut pending_message,
                    &mut batch_bytes,
                    config.batch_policy,
                    clock,
                );
                let parse_started = clock.now();
                let delta = runtime.feed_into(&batch_bytes, &mut buffers);
                let mut batch_metrics = batch_metrics;
                batch_metrics.parse_duration = clock.now().saturating_duration_since(parse_started);
                let (snapshot, state, batch_metrics) =
                    capture_presentation(&runtime, clock, batch_metrics);
                DeltaPublisher {
                    token,
                    writer: &mut writer,
                    notices,
                    notice_waker,
                    publication,
                    next_effect_sequence: &mut next_effect_sequence,
                    capture_visible_output: config.capture_visible_output,
                }
                .publish(revision, delta, snapshot, state, batch_metrics, true)
                .and_then(|()| {
                    revision = next_revision(revision)?;
                    Ok(())
                })
            }
            PaneMessage::ReaderEof => {
                if observed_exit.is_none() {
                    observed_exit = poll_exit_after_reader_eof(&mut control);
                }
                break;
            }
            PaneMessage::ReaderError | PaneMessage::Close => break,
        };
        if result.is_err() {
            break;
        }
    }

    accepting.store(false, Ordering::Release);
    let _ = receiver.close();
    if observed_exit.is_none() {
        let _ = interrupt.interrupt();
    }
    let _ = control.begin_close();
    let delta = runtime.finish_into(&mut buffers);
    let (snapshot, state, metrics) =
        capture_presentation(&runtime, clock, RuntimeBatchMetrics::default());
    let _ = DeltaPublisher {
        token,
        writer: &mut writer,
        notices,
        notice_waker,
        publication,
        next_effect_sequence: &mut next_effect_sequence,
        capture_visible_output: config.capture_visible_output,
    }
    .publish(revision, delta, snapshot, state, metrics, false);
    let _ = reader_join.join();
    let exit = observed_exit.or_else(|| control.poll_exit().ok().flatten());
    let _ = send_notice(
        notices,
        notice_waker,
        PaneNotice::Closed { pane: token, exit },
    );
}

struct DeltaPublisher<'a, W> {
    token: PaneToken,
    writer: &'a mut W,
    notices: &'a MailboxSender<PaneNotice>,
    notice_waker: &'a Arc<dyn Fn() + Send + Sync>,
    publication: &'a PanePublication,
    next_effect_sequence: &'a mut Option<EffectSequence>,
    capture_visible_output: bool,
}

impl<W: Write> DeltaPublisher<'_, W> {
    fn publish(
        &mut self,
        revision: RuntimeRevision,
        delta: crate::RuntimeDelta<'_>,
        snapshot: Arc<Terminal>,
        state: TerminalStateSummary,
        metrics: RuntimeBatchMetrics,
        always_publish: bool,
    ) -> io::Result<()> {
        let transport_result = write_transport_effects(
            delta,
            self.writer,
            self.token,
            self.notices,
            self.notice_waker,
        );
        let damage = delta.damage().to_vec();
        let metadata = owned_metadata(delta.metadata());
        let effects = owned_host_effects(
            self.token,
            revision,
            delta,
            self.next_effect_sequence,
            self.capture_visible_output,
        )?;
        let should_publish = always_publish
            || delta.snapshot_changed()
            || !damage.is_empty()
            || metadata != PaneMetadataDelta::default()
            || !effects.is_empty();
        if !should_publish {
            return transport_result;
        }
        let frame = PresentationFrame {
            pane: self.token,
            revision,
            snapshot,
            state,
            snapshot_changed: delta.snapshot_changed(),
            full_repaint: false,
            damage,
            metadata,
            metrics,
        };
        let publish_result = self
            .publication
            .publish(frame, effects, || {
                send_notice(
                    self.notices,
                    self.notice_waker,
                    PaneNotice::Wake(self.token),
                )
                .map_err(|_| ())
            })
            .map_err(|()| io::Error::from(io::ErrorKind::BrokenPipe));
        transport_result.and(publish_result)
    }
}

fn send_notice(
    notices: &MailboxSender<PaneNotice>,
    notice_waker: &Arc<dyn Fn() + Send + Sync>,
    notice: PaneNotice,
) -> Result<(), crate::SendError<PaneNotice>> {
    notices.send(notice)?;
    notice_waker();
    Ok(())
}

fn capture_presentation<C: Clock>(
    runtime: &TerminalRuntime,
    clock: &C,
    mut metrics: RuntimeBatchMetrics,
) -> (Arc<Terminal>, TerminalStateSummary, RuntimeBatchMetrics) {
    let started = clock.now();
    let snapshot = Arc::new(runtime.terminal().clone());
    let state = TerminalStateSummary::capture_terminal(&snapshot);
    metrics.snapshot_duration = clock.now().saturating_duration_since(started);
    (snapshot, state, metrics)
}

fn collect_output_batch<C: Clock>(
    first: &[u8],
    receiver: &mut MailboxReceiver<PaneMessage>,
    pending_message: &mut Option<PaneMessage>,
    batch_bytes: &mut Vec<u8>,
    policy: BatchPolicy,
    clock: &C,
) -> RuntimeBatchMetrics {
    batch_bytes.clear();
    let mut window = BatchWindow::new(policy);
    let mut admission = window.try_push(clock.now(), first.len());
    batch_bytes.extend_from_slice(first);
    while admission != BatchAdmission::AcceptedAndFull {
        match receiver.try_recv() {
            Ok(PaneMessage::Output(bytes)) => {
                admission = window.try_push(clock.now(), bytes.len());
                if admission == BatchAdmission::Rejected {
                    *pending_message = Some(PaneMessage::Output(bytes));
                    break;
                }
                batch_bytes.extend_from_slice(&bytes);
            }
            Ok(message) => {
                *pending_message = Some(message);
                break;
            }
            Err(TryRecvError::Empty | TryRecvError::Closed) => break,
        }
    }
    RuntimeBatchMetrics {
        transport_bytes: window.bytes(),
        coalesced_reads: u32::try_from(window.items()).unwrap_or(u32::MAX),
        parse_duration: Duration::ZERO,
        snapshot_duration: Duration::ZERO,
    }
}

fn next_revision(revision: RuntimeRevision) -> io::Result<RuntimeRevision> {
    revision
        .next()
        .map_err(|_| io::Error::other("pane runtime revision exhausted"))
}

fn write_transport_effects<W: Write>(
    delta: crate::RuntimeDelta<'_>,
    writer: &mut W,
    pane: PaneToken,
    notices: &MailboxSender<PaneNotice>,
    notice_waker: &Arc<dyn Fn() + Send + Sync>,
) -> io::Result<()> {
    for effect in delta.effects() {
        if let RuntimeEffectRef::TransportWrite(bytes) = effect {
            report_write(writer, bytes, pane, notices, notice_waker)?;
        }
    }
    Ok(())
}

fn report_write<W: Write>(
    writer: &mut W,
    bytes: &[u8],
    pane: PaneToken,
    notices: &MailboxSender<PaneNotice>,
    notice_waker: &Arc<dyn Fn() + Send + Sync>,
) -> io::Result<()> {
    let started = Instant::now();
    writer.write_all(bytes)?;
    send_notice(
        notices,
        notice_waker,
        PaneNotice::InputWriteCompleted {
            pane,
            byte_count: bytes.len(),
            elapsed: started.elapsed(),
        },
    )
    .map_err(|_| io::Error::from(io::ErrorKind::BrokenPipe))
}

fn owned_host_effects(
    pane: PaneToken,
    revision: RuntimeRevision,
    delta: crate::RuntimeDelta<'_>,
    next_sequence: &mut Option<EffectSequence>,
    capture_visible_output: bool,
) -> io::Result<Vec<PublishedEffect>> {
    let visible_output = (capture_visible_output && !delta.visible_bytes().is_empty())
        .then(|| RuntimeEffectKind::VisibleOutput(delta.visible_bytes().to_vec()));
    visible_output
        .into_iter()
        .chain(delta.effects().filter_map(|effect| match effect {
            RuntimeEffectRef::TransportWrite(_) => None,
            RuntimeEffectRef::ModeChange(change) => Some(RuntimeEffectKind::ModeChange(change)),
            RuntimeEffectRef::ConsoleWrite(bytes) => {
                Some(RuntimeEffectKind::HostStream(bytes.to_vec()))
            }
            RuntimeEffectRef::Bell { count } => {
                NonZeroU64::new(count).map(|count| RuntimeEffectKind::Bell { count })
            }
            RuntimeEffectRef::ClipboardWrite {
                selection,
                contents,
            } => Some(RuntimeEffectKind::ClipboardWrite {
                selection: selection.map(str::to_owned),
                contents: contents.to_owned(),
            }),
            RuntimeEffectRef::ClipboardRead { selection } => {
                Some(RuntimeEffectKind::ClipboardRead {
                    selection: selection.to_owned(),
                })
            }
            RuntimeEffectRef::Notification { title, body } => {
                Some(RuntimeEffectKind::Notification {
                    title: title.map(str::to_owned),
                    body: body.to_owned(),
                })
            }
            RuntimeEffectRef::Diagnostic { message } => Some(RuntimeEffectKind::Diagnostic {
                message: message.to_owned(),
            }),
        }))
        .map(|kind| {
            let sequence = next_sequence
                .take()
                .ok_or_else(|| io::Error::other("pane effect sequence exhausted"))?;
            *next_sequence = sequence.next().ok();
            Ok(PublishedEffect {
                pane,
                revision,
                effect: RuntimeEffect::new(sequence, kind),
            })
        })
        .collect()
}

fn owned_metadata(metadata: crate::RuntimeMetadataDeltaRef<'_>) -> PaneMetadataDelta {
    PaneMetadataDelta {
        title: owned_change(metadata.title()),
        working_directory: owned_change(metadata.working_directory()),
        badge_format: owned_change(metadata.badge_format()),
        progress: metadata.progress().map(MetadataChange::Set),
        user_vars: metadata
            .user_vars()
            .map(|(name, value)| UserVarDelta {
                name: name.to_owned(),
                value: owned_change(Some(value)).expect("user variable always has a change"),
            })
            .collect(),
    }
}

fn owned_change(change: Option<MetadataChangeRef<'_>>) -> Option<MetadataChange<String>> {
    change.map(|change| match change {
        MetadataChangeRef::Set(value) => MetadataChange::Set(value.to_owned()),
        MetadataChangeRef::Clear => MetadataChange::Clear,
    })
}
