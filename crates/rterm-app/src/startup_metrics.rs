use std::io::{self, Write};
use std::time::Instant;

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RendererKind {
    Cpu,
    Gpu,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ConnectionState {
    NotStarted,
    Pending,
    Connecting,
    AwaitingSecret,
    AwaitingHostKey,
    Connected,
    Disconnected,
    Failed,
}

impl Default for ConnectionState {
    fn default() -> Self {
        Self::NotStarted
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct StartupMetricsSnapshot {
    pub(crate) process_to_first_present_ms: Option<u128>,
    pub(crate) config_duration_ms: Option<u128>,
    pub(crate) gpu_duration_ms: Option<u128>,
    pub(crate) ssh_duration_ms: Option<u128>,
    pub(crate) first_frame_private_bytes: Option<u64>,
    pub(crate) final_renderer: Option<RendererKind>,
    pub(crate) connection_state: ConnectionState,
}

#[derive(Debug)]
pub(crate) struct StartupTrace {
    process_started_at: Instant,
    config_started_at: Option<Instant>,
    config_finished_at: Option<Instant>,
    gpu_started_at: Option<Instant>,
    gpu_finished_at: Option<Instant>,
    ssh_started_at: Option<Instant>,
    ssh_finished_at: Option<Instant>,
    first_present_at: Option<Instant>,
    first_frame_private_bytes: Option<u64>,
    final_renderer: Option<RendererKind>,
    connection_state: ConnectionState,
}

impl StartupTrace {
    pub(crate) fn new() -> Self {
        Self::from_process_started_at(Instant::now())
    }

    pub(crate) fn from_process_started_at(process_started_at: Instant) -> Self {
        Self {
            process_started_at,
            config_started_at: None,
            config_finished_at: None,
            gpu_started_at: None,
            gpu_finished_at: None,
            ssh_started_at: None,
            ssh_finished_at: None,
            first_present_at: None,
            first_frame_private_bytes: None,
            final_renderer: None,
            connection_state: ConnectionState::NotStarted,
        }
    }

    pub(crate) fn mark_config_started(&mut self) {
        if self.config_started_at.is_none() {
            self.config_started_at = Some(Instant::now());
            eprintln!("startup_stage stage=config phase=started");
        }
    }

    pub(crate) fn mark_config_finished(&mut self) {
        if self.config_finished_at.is_none() {
            let finished = Instant::now();
            self.config_finished_at = Some(finished);
            if let Some(started) = self.config_started_at {
                eprintln!(
                    "startup_stage stage=config phase=finished duration_ms={}",
                    finished.saturating_duration_since(started).as_millis()
                );
            }
        }
    }

    pub(crate) fn mark_gpu_started(&mut self) {
        if self.gpu_started_at.is_none() {
            self.gpu_started_at = Some(Instant::now());
            eprintln!("startup_stage stage=gpu phase=started");
        }
    }

    pub(crate) fn mark_gpu_finished(&mut self) {
        if self.gpu_finished_at.is_none() {
            let finished = Instant::now();
            self.gpu_finished_at = Some(finished);
            if let Some(started) = self.gpu_started_at {
                eprintln!(
                    "startup_stage stage=gpu phase=finished duration_ms={}",
                    finished.saturating_duration_since(started).as_millis()
                );
            }
        }
    }

    pub(crate) fn mark_ssh_started(&mut self) {
        if self.ssh_started_at.is_none() {
            self.ssh_started_at = Some(Instant::now());
            eprintln!("startup_stage stage=ssh phase=started");
        }
        self.connection_state = ConnectionState::Connecting;
    }

    pub(crate) fn mark_ssh_connected(&mut self) {
        if self.ssh_finished_at.is_none() {
            let finished = Instant::now();
            self.ssh_finished_at = Some(finished);
            if let Some(started) = self.ssh_started_at {
                eprintln!(
                    "startup_stage stage=ssh phase=finished duration_ms={} state=connected",
                    finished.saturating_duration_since(started).as_millis()
                );
            }
        }
        self.connection_state = ConnectionState::Connected;
    }

    pub(crate) fn mark_connection_state(&mut self, state: ConnectionState) {
        self.connection_state = state;
        if matches!(
            state,
            ConnectionState::Connected | ConnectionState::Disconnected | ConnectionState::Failed
        ) && self.ssh_finished_at.is_none()
        {
            let finished = Instant::now();
            self.ssh_finished_at = Some(finished);
            if let Some(started) = self.ssh_started_at {
                eprintln!(
                    "startup_stage stage=ssh phase=finished duration_ms={} state={}",
                    finished.saturating_duration_since(started).as_millis(),
                    connection_state_name(state)
                );
            }
        }
    }

    pub(crate) fn mark_renderer(&mut self, renderer: RendererKind) {
        self.final_renderer = Some(renderer);
    }

    #[cfg(test)]
    pub(crate) fn mark_first_present(&mut self, private_bytes: u64) {
        if self.mark_first_present_inner().is_some() {
            self.first_frame_private_bytes = Some(private_bytes);
        }
    }

    pub(crate) fn mark_first_present_to<W: Write>(&mut self, writer: &mut W) -> io::Result<bool> {
        let Some(snapshot) = self.mark_first_present_inner() else {
            return Ok(false);
        };
        writeln!(
            writer,
            "first_present process_to_first_present_ms={} final_renderer={}",
            metric_option(snapshot.process_to_first_present_ms),
            renderer_name(snapshot.final_renderer),
        )?;
        writer.flush()?;
        Ok(true)
    }

    pub(crate) fn mark_first_frame_private_bytes_to<W: Write>(
        &mut self,
        writer: &mut W,
        private_bytes: u64,
    ) -> io::Result<bool> {
        if self.first_present_at.is_none() || self.first_frame_private_bytes.is_some() {
            return Ok(false);
        }
        self.first_frame_private_bytes = Some(private_bytes);
        writeln!(
            writer,
            "first_frame_memory first_frame_private_bytes={private_bytes}"
        )?;
        writer.flush()?;
        Ok(true)
    }

    fn mark_first_present_inner(&mut self) -> Option<StartupMetricsSnapshot> {
        if self.first_present_at.is_some() {
            return None;
        }
        self.first_present_at = Some(Instant::now());
        self.final_renderer.get_or_insert(RendererKind::Cpu);
        Some(self.snapshot())
    }

    pub(crate) fn snapshot(&self) -> StartupMetricsSnapshot {
        StartupMetricsSnapshot {
            process_to_first_present_ms: elapsed_ms(self.process_started_at, self.first_present_at),
            config_duration_ms: interval_ms(self.config_started_at, self.config_finished_at),
            gpu_duration_ms: interval_ms(self.gpu_started_at, self.gpu_finished_at),
            ssh_duration_ms: interval_ms(self.ssh_started_at, self.ssh_finished_at),
            first_frame_private_bytes: self.first_frame_private_bytes,
            final_renderer: self.final_renderer,
            connection_state: self.connection_state,
        }
    }
}

fn elapsed_ms(start: Instant, end: Option<Instant>) -> Option<u128> {
    end.map(|end| end.saturating_duration_since(start).as_millis())
}

fn interval_ms(start: Option<Instant>, end: Option<Instant>) -> Option<u128> {
    start
        .zip(end)
        .map(|(start, end)| end.saturating_duration_since(start).as_millis())
}

fn metric_option(value: Option<u128>) -> u128 {
    value.unwrap_or_default()
}

const fn renderer_name(renderer: Option<RendererKind>) -> &'static str {
    match renderer {
        Some(RendererKind::Cpu) => "cpu",
        Some(RendererKind::Gpu) => "gpu",
        None => "unknown",
    }
}

const fn connection_state_name(state: ConnectionState) -> &'static str {
    match state {
        ConnectionState::NotStarted => "not_started",
        ConnectionState::Pending => "pending",
        ConnectionState::Connecting => "connecting",
        ConnectionState::AwaitingSecret => "awaiting_secret",
        ConnectionState::AwaitingHostKey => "awaiting_host_key",
        ConnectionState::Connected => "connected",
        ConnectionState::Disconnected => "disconnected",
        ConnectionState::Failed => "failed",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn startup_trace_serializes_first_present_and_stage_durations() {
        let mut trace = StartupTrace::new();
        trace.mark_config_started();
        trace.mark_config_finished();
        trace.mark_gpu_started();
        trace.mark_gpu_finished();
        trace.mark_ssh_connected();
        trace.mark_first_present(42);

        let snapshot = trace.snapshot();
        assert_eq!(snapshot.first_frame_private_bytes, Some(42));
        assert!(snapshot.process_to_first_present_ms.is_some());
        assert!(snapshot.config_duration_ms.is_some());
        assert!(snapshot.gpu_duration_ms.is_some());
        assert_eq!(snapshot.final_renderer, Some(RendererKind::Cpu));
        assert_eq!(snapshot.connection_state, ConnectionState::Connected);

        let json = serde_json::to_string(&snapshot).expect("startup metrics must serialize");
        assert!(json.contains("first_frame_private_bytes"));
        assert!(json.contains("process_to_first_present_ms"));
    }

    #[test]
    fn startup_trace_emits_a_single_first_present_marker() {
        let mut trace = StartupTrace::new();
        let mut output = Vec::new();
        trace.mark_first_present_to(&mut output).unwrap();
        trace.mark_first_present_to(&mut output).unwrap();

        let text = String::from_utf8(output).unwrap();
        assert_eq!(
            text.lines()
                .filter(|line| line.starts_with("first_present "))
                .count(),
            1
        );
        assert!(text.contains("process_to_first_present_ms="));
        assert!(!text.contains("first_frame_private_bytes="));
    }

    #[test]
    fn startup_trace_emits_memory_after_the_first_present_marker() {
        let mut trace = StartupTrace::new();
        let mut output = Vec::new();

        trace.mark_first_present_to(&mut output).unwrap();
        trace
            .mark_first_frame_private_bytes_to(&mut output, 11)
            .unwrap();
        trace
            .mark_first_frame_private_bytes_to(&mut output, 12)
            .unwrap();

        let text = String::from_utf8(output).unwrap();
        let lines = text.lines().collect::<Vec<_>>();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].starts_with("first_present "));
        assert!(lines[1].starts_with("first_frame_memory "));
        assert!(lines[1].contains("first_frame_private_bytes=11"));
        assert_eq!(trace.snapshot().first_frame_private_bytes, Some(11));
    }

    #[test]
    fn startup_trace_never_reports_negative_durations() {
        let mut trace = StartupTrace::new();
        trace.mark_config_finished();
        trace.mark_first_present(0);
        let snapshot = trace.snapshot();
        for value in [
            snapshot.process_to_first_present_ms,
            snapshot.config_duration_ms,
            snapshot.gpu_duration_ms,
            snapshot.ssh_duration_ms,
        ] {
            assert!(value.is_none_or(|milliseconds| milliseconds <= Duration::MAX.as_millis()));
        }
    }

    #[test]
    fn startup_trace_can_include_work_before_gui_dispatch() {
        let process_started_at = Instant::now()
            .checked_sub(Duration::from_millis(25))
            .expect("test instant should support a small subtraction");
        let mut trace = StartupTrace::from_process_started_at(process_started_at);

        trace.mark_first_present(0);

        assert!(trace.snapshot().process_to_first_present_ms >= Some(25));
    }
}
