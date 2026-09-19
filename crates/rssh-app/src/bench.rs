use std::{
    error::Error,
    io, thread,
    time::{Duration, Instant},
};

use rssh_core::TerminalSize;
use rssh_terminal::Terminal;
use rterm_render_core::TerminalRenderSnapshot;
use rterm_render_cpu::PixelRenderer;
use serde::{Deserialize, Serialize, Serializer, ser::SerializeStruct};
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, RefreshKind, System};

use crate::{
    cli::{BenchOptions, BenchThresholds, BenchWorkload},
    terminal_runtime::TerminalRuntime,
};

const BENCH_CELL_WIDTH: u32 = 8;
const BENCH_CELL_HEIGHT: u32 = 16;

#[derive(Debug, PartialEq, Deserialize, Serialize)]
pub struct BenchReport {
    pub ok: bool,
    pub workload: String,
    pub runtime_api: String,
    pub bytes: usize,
    pub chunk_size: usize,
    pub chunks: usize,
    pub columns: u16,
    pub rows: u16,
    pub elapsed_ms: u128,
    pub throughput_bytes_per_sec: u64,
    pub chunk_p95_us: u128,
    pub render_frames: usize,
    pub render_frame_p95_us: u128,
    pub rendered_pixels: u128,
    pub render_pixels_per_sec: u64,
    pub idle_sample_ms: usize,
    pub idle_cpu_usage_percent: f32,
    pub process_memory_bytes: u64,
    pub process_virtual_memory_bytes: u64,
    pub process_accumulated_cpu_ms: u64,
    pub threshold_violations: Vec<BenchThresholdViolation>,
    pub display_bytes: usize,
    pub responses: usize,
    pub response_commits: u64,
    pub response_payload_copies: u64,
    pub owned_response_materializations: u64,
    pub bells: u64,
    pub scrollback_lines: usize,
    pub inspected_query_bytes: u64,
    pub scrolled_survivor_cell_clones: u64,
    pub history_row_relocations: u64,
    pub metadata_rebase_batches: u64,
    pub cursor_row: u16,
    pub cursor_column: u16,
}

#[derive(Debug, PartialEq, Eq, Deserialize)]
pub struct BenchThresholdViolation {
    pub metric: String,
    pub actual: String,
    pub limit: String,
}

impl Serialize for BenchThresholdViolation {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut fields = serializer.serialize_struct("BenchThresholdViolation", 5)?;
        fields.serialize_field("metric", &self.metric)?;
        fields.serialize_field("actual", &self.actual)?;
        fields.serialize_field("limit", &self.limit)?;
        fields.serialize_field("observed", &self.actual)?;
        fields.serialize_field("expected", &self.limit)?;
        fields.end()
    }
}

pub fn print_bench(options: &BenchOptions) -> Result<(), Box<dyn Error>> {
    let report = run_bench(options);

    if options.json {
        println!("{}", bench_json(&report)?);
    } else {
        for line in bench_text_lines(&report) {
            println!("{line}");
        }
    }

    if report.ok {
        Ok(())
    } else {
        Err(bench_threshold_error(&report))
    }
}

pub fn run_bench(options: &BenchOptions) -> BenchReport {
    let workload = build_benchmark_workload(options.workload, options.bytes);
    let mut report = run_benchmark_workload(
        options.workload,
        &workload,
        options.chunk_size,
        options.render_frames,
        options.idle_ms,
        options.size,
    );
    apply_bench_thresholds(&mut report, &options.thresholds);
    report
}

pub fn bench_json(report: &BenchReport) -> Result<String, Box<dyn Error>> {
    Ok(serde_json::to_string(report)?)
}

pub fn bench_text_lines(report: &BenchReport) -> Vec<String> {
    let mut lines = vec![
        format!(
            "ok\tbench\tworkload={} runtime_api={} bytes={} chunks={} chunk_size={} size={}x{}",
            report.workload,
            report.runtime_api,
            report.bytes,
            report.chunks,
            report.chunk_size,
            report.columns,
            report.rows
        ),
        format!(
            "metric\tthroughput_bytes_per_sec={}\tchunk_p95_us={}\telapsed_ms={}",
            report.throughput_bytes_per_sec, report.chunk_p95_us, report.elapsed_ms
        ),
        format!(
            "metric\trender_frames={}\trender_frame_p95_us={}\trendered_pixels={}\trender_pixels_per_sec={}",
            report.render_frames,
            report.render_frame_p95_us,
            report.rendered_pixels,
            report.render_pixels_per_sec
        ),
        format!(
            "metric\tidle_sample_ms={}\tidle_cpu_usage_percent={:.2}\tprocess_memory_bytes={}\tprocess_virtual_memory_bytes={}\tprocess_accumulated_cpu_ms={}",
            report.idle_sample_ms,
            report.idle_cpu_usage_percent,
            report.process_memory_bytes,
            report.process_virtual_memory_bytes,
            report.process_accumulated_cpu_ms
        ),
        format!(
            "metric\tdisplay_bytes={}\tresponses={}\tbells={}\tscrollback_lines={}",
            report.display_bytes, report.responses, report.bells, report.scrollback_lines
        ),
        format!(
            "work\tinspected_query_bytes={}\tscrolled_survivor_cell_clones={}\thistory_row_relocations={}\tmetadata_rebase_batches={}",
            report.inspected_query_bytes,
            report.scrolled_survivor_cell_clones,
            report.history_row_relocations,
            report.metadata_rebase_batches
        ),
        format!(
            "work\tresponse_commits={}\tresponse_payload_copies={}\towned_response_materializations={}",
            report.response_commits,
            report.response_payload_copies,
            report.owned_response_materializations
        ),
    ];

    lines.extend(report.threshold_violations.iter().map(|violation| {
        format!(
            "fail\tthreshold\tmetric={} observed={} expected={}",
            violation.metric, violation.actual, violation.limit
        )
    }));

    lines
}

fn run_benchmark_workload(
    workload_kind: BenchWorkload,
    workload: &[u8],
    chunk_size: usize,
    render_frames: usize,
    idle_ms: usize,
    size: TerminalSize,
) -> BenchReport {
    let mut runtime = BenchmarkRuntime::new(workload_kind, size);
    let inspected_query_bytes_start = runtime.inspected_query_bytes();
    let terminal_work_start = runtime.terminal().work_counters();
    let mut chunk_timings = Vec::new();
    let mut responses = 0_usize;
    let mut response_commits = 0_u64;
    let mut response_payload_copies = 0_u64;
    let mut owned_response_materializations = 0_u64;
    let mut display_bytes = 0_usize;
    let mut bells = 0_u64;

    let started = Instant::now();
    for chunk in workload.chunks(chunk_size) {
        let chunk_started = Instant::now();
        let output = runtime.feed(chunk);
        chunk_timings.push(chunk_started.elapsed().as_micros());
        responses = responses.saturating_add(output.responses);
        response_commits = response_commits.saturating_add(output.response_commits);
        response_payload_copies =
            response_payload_copies.saturating_add(output.response_payload_copies);
        owned_response_materializations =
            owned_response_materializations.saturating_add(output.owned_response_materializations);
        display_bytes = display_bytes.saturating_add(output.display_bytes);
        bells = bells.saturating_add(output.bells);
    }
    let elapsed = started.elapsed();
    let (cursor_row, cursor_column) = runtime.terminal().cursor();
    let work = runtime
        .terminal()
        .work_counters()
        .saturating_delta_since(terminal_work_start);
    let render_report = benchmark_rendering(runtime.terminal(), render_frames, size);
    let resource_report = sample_process_resources(idle_ms);

    BenchReport {
        ok: true,
        workload: workload_kind.as_str().to_owned(),
        runtime_api: runtime.runtime_api().to_owned(),
        bytes: workload.len(),
        chunk_size,
        chunks: chunk_timings.len(),
        columns: size.columns,
        rows: size.rows,
        elapsed_ms: elapsed.as_millis(),
        throughput_bytes_per_sec: bytes_per_second(workload.len(), elapsed.as_nanos()),
        chunk_p95_us: percentile_95(&mut chunk_timings),
        render_frames,
        render_frame_p95_us: render_report.frame_p95_us,
        rendered_pixels: render_report.rendered_pixels,
        render_pixels_per_sec: render_report.pixels_per_sec,
        idle_sample_ms: resource_report.idle_sample_ms,
        idle_cpu_usage_percent: resource_report.idle_cpu_usage_percent,
        process_memory_bytes: resource_report.process_memory_bytes,
        process_virtual_memory_bytes: resource_report.process_virtual_memory_bytes,
        process_accumulated_cpu_ms: resource_report.process_accumulated_cpu_ms,
        threshold_violations: Vec::new(),
        display_bytes,
        responses,
        response_commits,
        response_payload_copies,
        owned_response_materializations,
        bells,
        scrollback_lines: runtime.terminal().scrollback().len(),
        inspected_query_bytes: saturating_counter_delta(
            runtime.inspected_query_bytes(),
            inspected_query_bytes_start,
        ),
        scrolled_survivor_cell_clones: work.scrolled_survivor_cell_clones,
        history_row_relocations: work.history_row_relocations,
        metadata_rebase_batches: work.metadata_rebase_batches,
        cursor_row,
        cursor_column,
    }
}

struct BenchmarkChunkOutput {
    responses: usize,
    display_bytes: usize,
    bells: u64,
    response_commits: u64,
    response_payload_copies: u64,
    owned_response_materializations: u64,
}

enum BenchmarkRuntime {
    Plain(Box<Terminal>),
    Filtered(Box<TerminalRuntime>),
}

impl BenchmarkRuntime {
    fn new(workload: BenchWorkload, size: TerminalSize) -> Self {
        match workload {
            BenchWorkload::PlainScroll => Self::Plain(Box::new(Terminal::new(size))),
            BenchWorkload::AnsiScroll | BenchWorkload::AnsiScrollQuery => {
                Self::Filtered(Box::new(TerminalRuntime::new_with_query_scan_work(size)))
            }
        }
    }

    fn feed(&mut self, bytes: &[u8]) -> BenchmarkChunkOutput {
        match self {
            Self::Plain(terminal) => {
                terminal.feed(bytes);
                BenchmarkChunkOutput {
                    responses: 0,
                    display_bytes: bytes.len(),
                    bells: terminal.take_bell_count(),
                    response_commits: 0,
                    response_payload_copies: 0,
                    owned_response_materializations: 0,
                }
            }
            Self::Filtered(runtime) => {
                let delta = runtime.inner.feed_into(bytes, &mut runtime.storage.buffers);
                let responses = delta.responses().count();
                let display_bytes = delta.visible_bytes().len();
                let bells = delta.bell_count();
                BenchmarkChunkOutput {
                    responses,
                    display_bytes,
                    bells,
                    response_commits: runtime.storage.buffers.response_commits(),
                    response_payload_copies: runtime.storage.buffers.response_payload_copies(),
                    owned_response_materializations: runtime
                        .storage
                        .buffers
                        .owned_response_materializations(),
                }
            }
        }
    }

    fn terminal(&self) -> &Terminal {
        match self {
            Self::Plain(terminal) => terminal,
            Self::Filtered(runtime) => runtime.terminal(),
        }
    }

    fn inspected_query_bytes(&self) -> u64 {
        match self {
            Self::Plain(_) => 0,
            Self::Filtered(runtime) => runtime.inspected_query_bytes(),
        }
    }

    const fn runtime_api(&self) -> &'static str {
        match self {
            Self::Plain(_) => "terminal-feed",
            Self::Filtered(_) => "v2-feed-into",
        }
    }
}

fn apply_bench_thresholds(report: &mut BenchReport, thresholds: &BenchThresholds) {
    report.threshold_violations.clear();

    record_max_violation(
        &mut report.threshold_violations,
        "inspected_query_bytes",
        u128::from(report.inspected_query_bytes),
        usize_to_u128(report.bytes).saturating_mul(4),
    );
    record_max_violation(
        &mut report.threshold_violations,
        "scrolled_survivor_cell_clones",
        u128::from(report.scrolled_survivor_cell_clones),
        0,
    );
    record_max_violation(
        &mut report.threshold_violations,
        "history_row_relocations",
        u128::from(report.history_row_relocations),
        0,
    );

    if let Some(limit) = thresholds.min_throughput_bytes_per_sec {
        record_min_violation(
            &mut report.threshold_violations,
            "throughput_bytes_per_sec",
            u128::from(report.throughput_bytes_per_sec),
            usize_to_u128(limit),
        );
    }

    if let Some(limit) = thresholds.max_chunk_p95_us {
        record_max_violation(
            &mut report.threshold_violations,
            "chunk_p95_us",
            report.chunk_p95_us,
            usize_to_u128(limit),
        );
    }

    if let Some(limit) = thresholds.max_render_frame_p95_us {
        record_max_violation(
            &mut report.threshold_violations,
            "render_frame_p95_us",
            report.render_frame_p95_us,
            usize_to_u128(limit),
        );
    }

    if let Some(limit) = thresholds.max_idle_cpu_percent {
        if process_resource_sampling_available(report) {
            record_max_float_violation(
                &mut report.threshold_violations,
                "idle_cpu_usage_percent",
                report.idle_cpu_usage_percent,
                f32::from(limit),
            );
        } else {
            record_unavailable_violation(
                &mut report.threshold_violations,
                "idle_cpu_usage_percent",
            );
        }
    }

    if let Some(limit) = thresholds.max_process_memory_bytes {
        if process_resource_sampling_available(report) {
            record_max_violation(
                &mut report.threshold_violations,
                "process_memory_bytes",
                u128::from(report.process_memory_bytes),
                usize_to_u128(limit),
            );
        } else {
            record_unavailable_violation(&mut report.threshold_violations, "process_memory_bytes");
        }
    }

    report.ok = report.threshold_violations.is_empty();
}

fn record_min_violation(
    violations: &mut Vec<BenchThresholdViolation>,
    metric: &str,
    observed: u128,
    limit: u128,
) {
    if observed >= limit {
        return;
    }

    violations.push(BenchThresholdViolation {
        metric: metric.to_owned(),
        actual: observed.to_string(),
        limit: format!(">={limit}"),
    });
}

fn record_max_violation(
    violations: &mut Vec<BenchThresholdViolation>,
    metric: &str,
    observed: u128,
    limit: u128,
) {
    if observed <= limit {
        return;
    }

    violations.push(BenchThresholdViolation {
        metric: metric.to_owned(),
        actual: observed.to_string(),
        limit: format!("<={limit}"),
    });
}

fn record_max_float_violation(
    violations: &mut Vec<BenchThresholdViolation>,
    metric: &str,
    observed: f32,
    limit: f32,
) {
    if observed <= limit {
        return;
    }

    violations.push(BenchThresholdViolation {
        metric: metric.to_owned(),
        actual: format!("{observed:.2}"),
        limit: format!("<={limit:.2}"),
    });
}

fn record_unavailable_violation(violations: &mut Vec<BenchThresholdViolation>, metric: &str) {
    violations.push(BenchThresholdViolation {
        metric: metric.to_owned(),
        actual: "unavailable".to_owned(),
        limit: "available".to_owned(),
    });
}

const fn process_resource_sampling_available(report: &BenchReport) -> bool {
    report.process_memory_bytes > 0 && report.process_virtual_memory_bytes > 0
}

fn bench_threshold_error(report: &BenchReport) -> Box<dyn Error> {
    Box::new(io::Error::other(format!(
        "bench thresholds failed: {}",
        report
            .threshold_violations
            .iter()
            .map(|violation| violation.metric.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    )))
}

#[derive(Default)]
struct ProcessResourceReport {
    idle_sample_ms: usize,
    idle_cpu_usage_percent: f32,
    process_memory_bytes: u64,
    process_virtual_memory_bytes: u64,
    process_accumulated_cpu_ms: u64,
}

fn sample_process_resources(idle_ms: usize) -> ProcessResourceReport {
    let Ok(pid) = sysinfo::get_current_pid() else {
        return ProcessResourceReport {
            idle_sample_ms: idle_ms,
            ..ProcessResourceReport::default()
        };
    };

    let refreshes = RefreshKind::nothing().with_processes(
        ProcessRefreshKind::nothing()
            .with_cpu()
            .with_memory()
            .without_tasks(),
    );
    let mut system = System::new_with_specifics(refreshes);
    let _ = system.refresh_processes(ProcessesToUpdate::Some(&[pid]), true);

    thread::sleep(Duration::from_millis(usize_to_u64(idle_ms)));

    let _ = system.refresh_processes(ProcessesToUpdate::Some(&[pid]), true);

    system
        .process(pid)
        .map(|process| ProcessResourceReport {
            idle_sample_ms: idle_ms,
            idle_cpu_usage_percent: process.cpu_usage(),
            process_memory_bytes: process.memory(),
            process_virtual_memory_bytes: process.virtual_memory(),
            process_accumulated_cpu_ms: process.accumulated_cpu_time(),
        })
        .unwrap_or(ProcessResourceReport {
            idle_sample_ms: idle_ms,
            ..ProcessResourceReport::default()
        })
}

struct RenderBenchReport {
    frame_p95_us: u128,
    rendered_pixels: u128,
    pixels_per_sec: u64,
}

fn benchmark_rendering(
    terminal: &rssh_terminal::Terminal,
    render_frames: usize,
    size: TerminalSize,
) -> RenderBenchReport {
    let snapshot = TerminalRenderSnapshot::from_terminal(terminal);
    let renderer = PixelRenderer::new();
    let target_width = u32::from(size.columns).saturating_mul(BENCH_CELL_WIDTH);
    let target_height = u32::from(size.rows).saturating_mul(BENCH_CELL_HEIGHT);
    let buffer_len = usize::try_from(
        u64::from(target_width)
            .saturating_mul(u64::from(target_height))
            .saturating_mul(4),
    )
    .unwrap_or(usize::MAX);
    let mut target = vec![0; buffer_len];
    let mut frame_timings = Vec::with_capacity(render_frames);

    let started = Instant::now();
    for _ in 0..render_frames {
        let frame_started = Instant::now();
        renderer.render(
            &snapshot,
            &mut target,
            target_width,
            target_height,
            BENCH_CELL_WIDTH,
            BENCH_CELL_HEIGHT,
        );
        frame_timings.push(frame_started.elapsed().as_micros());
    }
    let elapsed_nanos = started.elapsed().as_nanos();
    let pixels_per_frame = u128::from(target_width).saturating_mul(u128::from(target_height));
    let rendered_pixels = pixels_per_frame.saturating_mul(usize_to_u128(render_frames));

    RenderBenchReport {
        frame_p95_us: percentile_95(&mut frame_timings),
        rendered_pixels,
        pixels_per_sec: u128_to_u64(
            rendered_pixels
                .saturating_mul(1_000_000_000)
                .checked_div(elapsed_nanos)
                .unwrap_or(u128::from(u64::MAX)),
        ),
    }
}

fn build_benchmark_workload(workload_kind: BenchWorkload, target_bytes: usize) -> Vec<u8> {
    let mut workload = Vec::with_capacity(target_bytes);
    let mut line = 0_u64;

    while workload.len() < target_bytes {
        let record = benchmark_record(workload_kind, line);
        let remaining = target_bytes - workload.len();
        let record_bytes = record.as_bytes();
        workload.extend_from_slice(&record_bytes[..record_bytes.len().min(remaining)]);
        line = line.saturating_add(1);
    }

    workload
}

fn benchmark_record(workload_kind: BenchWorkload, line: u64) -> String {
    const TEXT_SUFFIX: &str = " ABCDEFGHIJKLMNOPQRSTUVWXYZ 0123456789";
    match workload_kind {
        BenchWorkload::PlainScroll => format!("bench line {line:08}{TEXT_SUFFIX}\r\n"),
        BenchWorkload::AnsiScroll => {
            let color = line % 256;
            format!("\x1b[38;5;{color}mbench line {line:08}{TEXT_SUFFIX}\x1b[0m\r\n")
        }
        BenchWorkload::AnsiScrollQuery => {
            let color = line % 256;
            format!(
                "\x1b[38;5;{color}mbench line {line:08}{TEXT_SUFFIX}\x1b[0m\r\n\
                 \x1b[6n\x1b[18t\x1b]0;R-SSH bench {line}\x07"
            )
        }
    }
}

fn percentile_95(values: &mut [u128]) -> u128 {
    if values.is_empty() {
        return 0;
    }

    values.sort_unstable();
    let index = values
        .len()
        .saturating_mul(95)
        .saturating_add(99)
        .saturating_div(100)
        .saturating_sub(1);
    values[index]
}

fn bytes_per_second(bytes: usize, elapsed_nanos: u128) -> u64 {
    if elapsed_nanos == 0 {
        return u64::MAX;
    }

    let rate = usize_to_u128(bytes)
        .saturating_mul(1_000_000_000)
        .saturating_div(elapsed_nanos);
    u128_to_u64(rate)
}

fn usize_to_u128(value: usize) -> u128 {
    u128::try_from(value).expect("usize fits into u128")
}

fn usize_to_u64(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}

fn u128_to_u64(value: u128) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}

const fn saturating_counter_delta(current: u64, earlier: u64) -> u64 {
    current.saturating_sub(earlier)
}

#[cfg(test)]
mod tests {
    use rssh_core::TerminalSize;

    use crate::cli::BenchOptions;

    #[test]
    fn counter_delta_excludes_prior_query_work_and_saturates() {
        assert_eq!(super::saturating_counter_delta(125, 100), 25);
        assert_eq!(super::saturating_counter_delta(35, 40), 0);
    }

    #[test]
    fn builds_exact_sized_benchmark_workload() {
        for workload in [
            crate::cli::BenchWorkload::PlainScroll,
            crate::cli::BenchWorkload::AnsiScroll,
            crate::cli::BenchWorkload::AnsiScrollQuery,
        ] {
            let bytes = super::build_benchmark_workload(workload, 513);
            assert_eq!(bytes.len(), 513);
            assert!(String::from_utf8_lossy(&bytes).contains("bench line"));
        }
    }

    #[test]
    fn ansi_scroll_query_first_record_matches_legacy_payload_byte_for_byte() {
        assert_eq!(
            super::benchmark_record(crate::cli::BenchWorkload::AnsiScrollQuery, 0).as_bytes(),
            b"\x1b[38;5;0mbench line 00000000 ABCDEFGHIJKLMNOPQRSTUVWXYZ 0123456789\x1b[0m\r\n\x1b[6n\x1b[18t\x1b]0;R-SSH bench 0\x07"
        );
    }

    #[test]
    fn exact_target_size_intentionally_keeps_a_truncated_final_record() {
        let full_record = super::benchmark_record(crate::cli::BenchWorkload::AnsiScrollQuery, 0);
        let target_bytes = full_record.len() - 1;

        let workload = super::build_benchmark_workload(
            crate::cli::BenchWorkload::AnsiScrollQuery,
            target_bytes,
        );

        assert_eq!(workload.len(), target_bytes);
        assert_eq!(workload, full_record.as_bytes()[..target_bytes]);
        assert_ne!(workload.last(), full_record.as_bytes().last());
    }

    #[test]
    fn console_benchmark_report_tracks_terminal_runtime_metrics() {
        let report = super::run_bench(&BenchOptions {
            json: false,
            workload: crate::cli::BenchWorkload::AnsiScrollQuery,
            bytes: 2048,
            chunk_size: 256,
            render_frames: 3,
            idle_ms: 1,
            thresholds: crate::cli::BenchThresholds::default(),
            size: TerminalSize::new(40, 10),
        });

        assert!(report.ok);
        assert_eq!(report.workload, "ansi-scroll-query");
        assert_eq!(report.runtime_api, "v2-feed-into");
        assert_eq!(report.bytes, 2048);
        assert_eq!(report.chunk_size, 256);
        assert_eq!(report.chunks, 8);
        assert_eq!(report.columns, 40);
        assert_eq!(report.rows, 10);
        assert!(report.throughput_bytes_per_sec > 0);
        assert_eq!(report.render_frames, 3);
        assert!(report.render_frame_p95_us > 0);
        assert!(report.rendered_pixels > 0);
        assert!(report.render_pixels_per_sec > 0);
        assert_eq!(report.idle_sample_ms, 1);
        assert!(report.process_memory_bytes > 0);
        assert!(report.process_virtual_memory_bytes > 0);
        assert!(report.process_accumulated_cpu_ms > 0);
        assert!(report.idle_cpu_usage_percent >= 0.0);
        assert!(report.display_bytes > 0);
        assert!(report.responses > 0);
        assert_eq!(
            usize::try_from(report.response_commits).unwrap(),
            report.responses
        );
        assert_eq!(report.response_payload_copies, 0);
        assert_eq!(report.owned_response_materializations, 0);
        assert!(report.inspected_query_bytes > 0);
        assert_eq!(report.scrolled_survivor_cell_clones, 0);
        assert!(report.cursor_row < report.rows);
        assert!(report.cursor_column < report.columns);
    }

    #[test]
    fn benchmark_json_report_is_machine_readable() {
        let report = super::BenchReport {
            ok: true,
            workload: "ansi-scroll-query".to_owned(),
            runtime_api: "v2-feed-into".to_owned(),
            bytes: 1024,
            chunk_size: 128,
            chunks: 8,
            columns: 80,
            rows: 24,
            elapsed_ms: 12,
            throughput_bytes_per_sec: 85_333,
            chunk_p95_us: 9,
            render_frames: 3,
            render_frame_p95_us: 11,
            rendered_pixels: 737_280,
            render_pixels_per_sec: 61_440_000,
            idle_sample_ms: 200,
            idle_cpu_usage_percent: 1.25,
            process_memory_bytes: 2_097_152,
            process_virtual_memory_bytes: 67_108_864,
            process_accumulated_cpu_ms: 123,
            threshold_violations: Vec::new(),
            display_bytes: 900,
            responses: 4,
            response_commits: 4,
            response_payload_copies: 0,
            owned_response_materializations: 0,
            bells: 1,
            scrollback_lines: 2,
            inspected_query_bytes: 10_240,
            scrolled_survivor_cell_clones: 800,
            history_row_relocations: 12,
            metadata_rebase_batches: 3,
            cursor_row: 3,
            cursor_column: 7,
        };

        let json = super::bench_json(&report).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(value["ok"], true);
        assert_eq!(value["workload"], "ansi-scroll-query");
        assert_eq!(value["runtime_api"], "v2-feed-into");
        assert_eq!(value["throughput_bytes_per_sec"], 85_333);
        assert_eq!(value["chunk_p95_us"], 9);
        assert_eq!(value["render_frames"], 3);
        assert_eq!(value["render_frame_p95_us"], 11);
        assert_eq!(value["render_pixels_per_sec"], 61_440_000);
        assert_eq!(value["idle_sample_ms"], 200);
        assert_eq!(value["idle_cpu_usage_percent"], 1.25);
        assert_eq!(value["process_memory_bytes"], 2_097_152);
        assert_eq!(value["process_virtual_memory_bytes"], 67_108_864);
        assert_eq!(value["process_accumulated_cpu_ms"], 123);
        assert_eq!(value["inspected_query_bytes"], 10_240);
        assert_eq!(value["scrolled_survivor_cell_clones"], 800);
        assert_eq!(value["history_row_relocations"], 12);
        assert_eq!(value["metadata_rebase_batches"], 3);
        assert_eq!(value["response_commits"], 4);
        assert_eq!(value["response_payload_copies"], 0);
        assert_eq!(value["owned_response_materializations"], 0);
    }

    #[test]
    fn benchmark_text_report_includes_comparison_metrics() {
        let report = super::BenchReport {
            ok: true,
            workload: "ansi-scroll-query".to_owned(),
            runtime_api: "v2-feed-into".to_owned(),
            bytes: 1024,
            chunk_size: 128,
            chunks: 8,
            columns: 80,
            rows: 24,
            elapsed_ms: 12,
            throughput_bytes_per_sec: 85_333,
            chunk_p95_us: 9,
            render_frames: 3,
            render_frame_p95_us: 11,
            rendered_pixels: 737_280,
            render_pixels_per_sec: 61_440_000,
            idle_sample_ms: 200,
            idle_cpu_usage_percent: 1.25,
            process_memory_bytes: 2_097_152,
            process_virtual_memory_bytes: 67_108_864,
            process_accumulated_cpu_ms: 123,
            threshold_violations: Vec::new(),
            display_bytes: 900,
            responses: 4,
            response_commits: 4,
            response_payload_copies: 0,
            owned_response_materializations: 0,
            bells: 1,
            scrollback_lines: 2,
            inspected_query_bytes: 10_240,
            scrolled_survivor_cell_clones: 800,
            history_row_relocations: 12,
            metadata_rebase_batches: 3,
            cursor_row: 3,
            cursor_column: 7,
        };

        let lines = super::bench_text_lines(&report);

        assert!(lines.iter().any(|line| line.contains("ok\tbench")));
        assert!(
            lines
                .iter()
                .any(|line| line.contains("runtime_api=v2-feed-into"))
        );
        assert!(
            lines
                .iter()
                .any(|line| line.contains("throughput_bytes_per_sec=85333"))
        );
        assert!(lines.iter().any(|line| line.contains("chunk_p95_us=9")));
        assert!(
            lines
                .iter()
                .any(|line| line.contains("render_pixels_per_sec=61440000"))
        );
        assert!(
            lines
                .iter()
                .any(|line| line.contains("idle_cpu_usage_percent=1.25"))
        );
        assert!(
            lines
                .iter()
                .any(|line| line.contains("process_memory_bytes=2097152"))
        );
        assert!(lines.iter().any(|line| {
            line.contains("inspected_query_bytes=10240")
                && line.contains("scrolled_survivor_cell_clones=800")
                && line.contains("history_row_relocations=12")
                && line.contains("metadata_rebase_batches=3")
        }));
        assert!(lines.iter().any(|line| {
            line.contains("response_commits=4")
                && line.contains("response_payload_copies=0")
                && line.contains("owned_response_materializations=0")
        }));
    }

    #[test]
    fn benchmark_thresholds_mark_report_failed_with_violation_details() {
        let mut report = super::BenchReport {
            ok: true,
            workload: "ansi-scroll-query".to_owned(),
            runtime_api: "v2-feed-into".to_owned(),
            bytes: 1024,
            chunk_size: 128,
            chunks: 8,
            columns: 80,
            rows: 24,
            elapsed_ms: 12,
            throughput_bytes_per_sec: 85_333,
            chunk_p95_us: 9,
            render_frames: 3,
            render_frame_p95_us: 11,
            rendered_pixels: 737_280,
            render_pixels_per_sec: 61_440_000,
            idle_sample_ms: 200,
            idle_cpu_usage_percent: 1.25,
            process_memory_bytes: 2_097_152,
            process_virtual_memory_bytes: 67_108_864,
            process_accumulated_cpu_ms: 123,
            threshold_violations: Vec::new(),
            display_bytes: 900,
            responses: 4,
            response_commits: 4,
            response_payload_copies: 0,
            owned_response_materializations: 0,
            bells: 1,
            scrollback_lines: 2,
            inspected_query_bytes: 4_096,
            scrolled_survivor_cell_clones: 0,
            history_row_relocations: 0,
            metadata_rebase_batches: 9_999,
            cursor_row: 3,
            cursor_column: 7,
        };
        let thresholds = crate::cli::BenchThresholds {
            min_throughput_bytes_per_sec: Some(100_000),
            max_chunk_p95_us: Some(8),
            max_render_frame_p95_us: Some(10),
            max_idle_cpu_percent: Some(1),
            max_process_memory_bytes: Some(1_048_576),
        };

        super::apply_bench_thresholds(&mut report, &thresholds);

        assert!(!report.ok);
        assert_eq!(report.threshold_violations.len(), 5);
        assert!(
            report
                .threshold_violations
                .iter()
                .any(|violation| violation.metric == "throughput_bytes_per_sec")
        );
        assert!(
            report
                .threshold_violations
                .iter()
                .any(|violation| violation.metric == "process_memory_bytes")
        );

        let lines = super::bench_text_lines(&report);
        assert!(lines.iter().any(|line| line.contains("fail\tthreshold")));

        let json = super::bench_json(&report).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["ok"], false);
        assert_eq!(value["threshold_violations"].as_array().unwrap().len(), 5);
    }

    #[test]
    fn workload_reports_linear_scanner_and_scroll_costs() {
        let options = |workload| BenchOptions {
            json: false,
            workload,
            bytes: 4096,
            chunk_size: 256,
            render_frames: 1,
            idle_ms: 1,
            thresholds: crate::cli::BenchThresholds::default(),
            size: TerminalSize::new(20, 4),
        };

        let plain = super::run_bench(&options(crate::cli::BenchWorkload::PlainScroll));
        let ansi = super::run_bench(&options(crate::cli::BenchWorkload::AnsiScroll));
        let query = super::run_bench(&options(crate::cli::BenchWorkload::AnsiScrollQuery));

        for (report, expected_name) in [
            (&plain, "plain-scroll"),
            (&ansi, "ansi-scroll"),
            (&query, "ansi-scroll-query"),
        ] {
            let json = super::bench_json(report).expect("serialize benchmark report");
            let decoded: super::BenchReport =
                serde_json::from_str(&json).expect("deserialize benchmark report");
            assert_eq!(decoded.workload, expected_name);
            assert_eq!(&decoded, report);
        }

        assert_eq!(plain.inspected_query_bytes, 0);
        assert_eq!(plain.runtime_api, "terminal-feed");
        assert_eq!(plain.scrolled_survivor_cell_clones, 0);
        assert!(ansi.inspected_query_bytes > 0);
        assert!(query.inspected_query_bytes > 0);
        assert!(ansi.inspected_query_bytes >= ansi.bytes as u64);
        assert!(ansi.inspected_query_bytes <= ansi.bytes as u64 * 4);
        assert!(query.inspected_query_bytes >= query.bytes as u64);
        assert!(query.inspected_query_bytes <= query.bytes as u64 * 4);
        assert!(query.inspected_query_bytes > ansi.inspected_query_bytes);
        assert!(query.responses > 0);
        assert_eq!(query.runtime_api, "v2-feed-into");
        assert_eq!(
            usize::try_from(query.response_commits).unwrap(),
            query.responses
        );
        assert_eq!(query.response_payload_copies, 0);
        assert_eq!(query.owned_response_materializations, 0);
        assert_eq!(plain.responses, 0);
        assert_eq!(ansi.responses, 0);
    }

    #[test]
    fn approved_algorithmic_performance_budgets_report_observed_and_expected_values() {
        let mut report = super::BenchReport {
            ok: true,
            workload: "ansi-scroll-query".to_owned(),
            runtime_api: "v2-feed-into".to_owned(),
            bytes: 1_024,
            chunk_size: 512,
            chunks: 2,
            columns: 80,
            rows: 24,
            elapsed_ms: 1,
            throughput_bytes_per_sec: 1_048_576,
            chunk_p95_us: 5_000,
            render_frames: 1,
            render_frame_p95_us: 16_000,
            rendered_pixels: 245_760,
            render_pixels_per_sec: 15_360_000,
            idle_sample_ms: 200,
            idle_cpu_usage_percent: 3.0,
            process_memory_bytes: 268_435_456,
            process_virtual_memory_bytes: 536_870_912,
            process_accumulated_cpu_ms: 1,
            threshold_violations: Vec::new(),
            display_bytes: 1_024,
            responses: 1,
            response_commits: 1,
            response_payload_copies: 0,
            owned_response_materializations: 0,
            bells: 0,
            scrollback_lines: 1,
            inspected_query_bytes: 4_097,
            scrolled_survivor_cell_clones: 1,
            history_row_relocations: 1,
            metadata_rebase_batches: 3,
            cursor_row: 0,
            cursor_column: 0,
        };

        super::apply_bench_thresholds(&mut report, &crate::cli::BenchThresholds::default());

        assert!(!report.ok);
        let json = super::bench_json(&report).expect("serialize performance budget failures");
        let value: serde_json::Value =
            serde_json::from_str(&json).expect("parse performance budget failures");
        let violations = value["threshold_violations"]
            .as_array()
            .expect("threshold violations are an array");
        assert_eq!(violations.len(), 3);
        for metric in [
            "inspected_query_bytes",
            "scrolled_survivor_cell_clones",
            "history_row_relocations",
        ] {
            let violation = violations
                .iter()
                .find(|violation| violation["metric"] == metric)
                .unwrap_or_else(|| panic!("missing {metric} violation"));
            assert!(violation.get("observed").is_some());
            assert!(violation.get("expected").is_some());
            assert_eq!(violation["actual"], violation["observed"]);
            assert_eq!(violation["limit"], violation["expected"]);
        }
    }

    #[test]
    fn unavailable_resource_sampling_fails_requested_cpu_and_rss_thresholds() {
        let mut report = super::BenchReport {
            ok: true,
            workload: "plain-scroll".to_owned(),
            runtime_api: "terminal-feed".to_owned(),
            bytes: 1_024,
            chunk_size: 512,
            chunks: 2,
            columns: 80,
            rows: 24,
            elapsed_ms: 1,
            throughput_bytes_per_sec: 5_242_880,
            chunk_p95_us: 5_000,
            render_frames: 1,
            render_frame_p95_us: 16_000,
            rendered_pixels: 245_760,
            render_pixels_per_sec: 15_360_000,
            idle_sample_ms: 1_000,
            idle_cpu_usage_percent: 0.0,
            process_memory_bytes: 0,
            process_virtual_memory_bytes: 0,
            process_accumulated_cpu_ms: 0,
            threshold_violations: Vec::new(),
            display_bytes: 1_024,
            responses: 0,
            response_commits: 0,
            response_payload_copies: 0,
            owned_response_materializations: 0,
            bells: 0,
            scrollback_lines: 1,
            inspected_query_bytes: 0,
            scrolled_survivor_cell_clones: 0,
            history_row_relocations: 0,
            metadata_rebase_batches: 1,
            cursor_row: 0,
            cursor_column: 0,
        };
        let thresholds = crate::cli::BenchThresholds {
            min_throughput_bytes_per_sec: None,
            max_chunk_p95_us: None,
            max_render_frame_p95_us: None,
            max_idle_cpu_percent: Some(3),
            max_process_memory_bytes: Some(268_435_456),
        };

        for (rss, virtual_memory) in [(0, 0), (0, 1), (1, 0)] {
            report.process_memory_bytes = rss;
            report.process_virtual_memory_bytes = virtual_memory;
            super::apply_bench_thresholds(&mut report, &thresholds);

            assert!(!report.ok);
            assert_eq!(report.threshold_violations.len(), 2);
            for metric in ["idle_cpu_usage_percent", "process_memory_bytes"] {
                let violation = report
                    .threshold_violations
                    .iter()
                    .find(|violation| violation.metric == metric)
                    .unwrap_or_else(|| panic!("missing {metric} availability violation"));
                assert_eq!(violation.actual, "unavailable");
                assert_eq!(violation.limit, "available");
            }
        }
    }

    #[test]
    fn algorithmic_performance_budget_boundaries_are_accepted() {
        let mut report = super::BenchReport {
            ok: true,
            workload: "ansi-scroll-query".to_owned(),
            runtime_api: "v2-feed-into".to_owned(),
            bytes: 1_024,
            chunk_size: 512,
            chunks: 2,
            columns: 80,
            rows: 24,
            elapsed_ms: 1,
            throughput_bytes_per_sec: 1_048_576,
            chunk_p95_us: 5_000,
            render_frames: 1,
            render_frame_p95_us: 16_000,
            rendered_pixels: 245_760,
            render_pixels_per_sec: 15_360_000,
            idle_sample_ms: 200,
            idle_cpu_usage_percent: 3.0,
            process_memory_bytes: 268_435_456,
            process_virtual_memory_bytes: 536_870_912,
            process_accumulated_cpu_ms: 1,
            threshold_violations: Vec::new(),
            display_bytes: 1_024,
            responses: 1,
            response_commits: 1,
            response_payload_copies: 0,
            owned_response_materializations: 0,
            bells: 0,
            scrollback_lines: 1,
            inspected_query_bytes: 4_096,
            scrolled_survivor_cell_clones: 0,
            history_row_relocations: 0,
            metadata_rebase_batches: 9_999,
            cursor_row: 0,
            cursor_column: 0,
        };

        super::apply_bench_thresholds(&mut report, &crate::cli::BenchThresholds::default());

        assert!(report.ok);
        assert!(report.threshold_violations.is_empty());
    }

    #[test]
    fn hosted_workflow_encodes_deterministic_performance_contract() {
        let ci = include_str!("../../../docs/trial/legacy-workflows/ci.yml");
        for required in [
            "Invoke-QueryBench 512",
            "Invoke-QueryBench 16384",
            "inspected_query_bytes",
            "scrolled_survivor_cell_clones",
            "history_row_relocations",
            "metadata_rebase_batches",
            "batched_scroll_prune_matches_incremental_prune",
            "$smallChunk.throughput_bytes_per_sec -le 0",
            "$ratio = [double]$largeChunk.throughput_bytes_per_sec / [double]$smallChunk.throughput_bytes_per_sec",
            "$warmupCount = 1",
            "$sampleCount = 5",
            "$index % 2",
            "$ratios += $ratio",
            "$sortedRatios[2]",
            "ratio_samples",
            "ratio_median",
        ] {
            assert!(
                ci.contains(required),
                "hosted CI performance gate is missing {required:?}"
            );
        }
        for forbidden in [
            "--min-throughput-bytes-per-sec",
            "--max-chunk-p95-us",
            "--max-render-frame-p95-us",
            "--max-idle-cpu-percent",
            "--max-process-memory-bytes",
        ] {
            assert!(
                !ci.contains(forbidden),
                "hosted PR performance gate must not enforce flaky absolute timing {forbidden:?}"
            );
        }
        assert_raw_exit_checked_before_json("hosted CI", ci);
    }

    #[test]
    fn release_workflow_encodes_fixed_runner_performance_contract() {
        let release = include_str!("../../../docs/trial/legacy-workflows/release.yml");
        for required in [
            "runs-on: [self-hosted, Windows, X64, rssh-performance]",
            "environment: performance",
            "cancel-in-progress: false",
            "contents: read",
            "persist-credentials: false",
            "github.event.repository.default_branch",
            "publish-release:",
            "contents: write",
            "RELEASE_TAG: ${{ github.ref_name }}",
            "gh release create \"$RELEASE_TAG\"",
            "--title \"R-SSH $RELEASE_TAG\"",
            "$warmupCount = 2",
            "$sampleCount = 7",
            "$regressionTolerance = 0.10",
            "$idleCpuRegressionNoiseFloor = 0.01",
            "$null = Invoke-Benchmark \"ansi-scroll-query\"",
            "$null = Invoke-Benchmark \"plain-scroll\"",
            "$querySamples += Invoke-Benchmark \"ansi-scroll-query\"",
            "$plainSamples += Invoke-Benchmark \"plain-scroll\"",
            "$sorted[3]",
            "($observed / $baseline) -lt (1.0 - $regressionTolerance)",
            "($observed / $baseline) -gt (1.0 + $regressionTolerance)",
            "($observed - $baseline) -gt $minimumAbsoluteRegression",
            "RSSH_PERF_BASELINE_MACHINE_CLASS",
            "RSSH_PERF_BASELINE_OS",
            "RSSH_PERF_BASELINE_ARCH",
            "RSSH_PERF_BASELINE_CPU",
            "RSSH_PERF_BASELINE_TOOLCHAIN",
            "RSSH_PERF_BASELINE_COMMAND_FINGERPRINT",
            "Get-CimInstance Win32_Processor",
            "$report.workload -ne $workload",
            "$report.bytes -ne 1048576",
            "$report.chunk_size -ne 8192",
            "$report.threshold_violations.Count -ne 0",
            "idle=1000",
            "--idle-ms 1000",
            "process_memory_bytes_available",
            "process_virtual_memory_bytes_available",
            "Test-ValidBaseline 0.0",
            "Test-ValidBaseline ([double]::NaN)",
            "Test-ValidBaseline ([double]::PositiveInfinity)",
            "Test-ValidBaseline ([double]::NegativeInfinity)",
            "Test-ValidBaseline ([double]::MaxValue)",
            "Test-Higher-Is-Regression 90.0 100.0",
            "Test-Lower-Is-Regression 110.0 100.0",
            "Test-Lower-Is-Regression 0.0034488498 0.0006851674 $idleCpuRegressionNoiseFloor",
            "Test-Lower-Is-Regression 0.02 0.0006851674 $idleCpuRegressionNoiseFloor",
            "Test-Lower-Is-BetterRegression \"idle_cpu_regression\" $idleCpu $idleBaseline $idleCpuRegressionNoiseFloor",
            "1048576",
            "5242880",
            "5000",
            "16000",
            "3.0",
            "268435456",
            "observed",
            "expected",
        ] {
            assert!(
                release.contains(required),
                "release performance gate is missing {required:?}"
            );
        }
        let warm_query = release
            .find("$null = Invoke-Benchmark \"ansi-scroll-query\"")
            .expect("query warmup");
        let warm_plain = release
            .find("$null = Invoke-Benchmark \"plain-scroll\"")
            .expect("plain warmup");
        let sample_query = release
            .find("$querySamples += Invoke-Benchmark \"ansi-scroll-query\"")
            .expect("query sample");
        let sample_plain = release
            .find("$plainSamples += Invoke-Benchmark \"plain-scroll\"")
            .expect("plain sample");
        assert!(
            warm_query < warm_plain && warm_plain < sample_query && sample_query < sample_plain,
            "warmups and samples must be interleaved query/plain, with warmups fully discarded"
        );
        assert!(
            !release.contains("idle_cpu_usage_percent_available"),
            "a real zero-percent idle sample must not be mistaken for unavailable resource data"
        );
        assert!(
            !release.contains("$baseline * (1.0 + $regressionTolerance)")
                && !release.contains("$baseline * (1.0 - $regressionTolerance)"),
            "baseline comparisons must not overflow by multiplying attacker-controlled baselines"
        );
        assert_eq!(
            release.matches("${{ github.ref_name }}").count(),
            1,
            "the tag expression must appear only in RELEASE_TAG env, never directly in bash"
        );
        assert_raw_exit_checked_before_json("release", release);
    }

    fn assert_raw_exit_checked_before_json(name: &str, workflow: &str) {
        let exit_check = workflow
            .find("if ($LASTEXITCODE -ne 0)")
            .unwrap_or_else(|| panic!("{name} benchmark must inspect the raw exit code"));
        let json_parse = workflow
            .find("ConvertFrom-Json")
            .unwrap_or_else(|| panic!("{name} benchmark must parse JSON"));
        assert!(
            exit_check < json_parse,
            "{name} benchmark must inspect the raw exit code before parsing JSON"
        );
    }
}
