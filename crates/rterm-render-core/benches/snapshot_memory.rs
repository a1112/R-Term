use std::time::Instant;

use rterm_render_core::{SnapshotCacheConfig, TerminalRenderSnapshot, TerminalSnapshotCache};
use rterm_terminal::Terminal;
use rterm_types::TerminalSize;

const FULL_ITERATIONS: u32 = 200;
const DAMAGE_ITERATIONS: u32 = 1_000;

fn main() {
    for size in [TerminalSize::new(80, 24), TerminalSize::new(200, 60)] {
        run_case(size);
    }
}

fn run_case(size: TerminalSize) {
    let mut terminal = seeded_terminal(size);
    let mut cache = TerminalSnapshotCache::new(SnapshotCacheConfig::default());
    let _ = cache.build(&terminal);

    let full_started = Instant::now();
    let mut full = TerminalRenderSnapshot::from_terminal(&terminal);
    for _ in 1..FULL_ITERATIONS {
        full = cache.build(&terminal);
    }
    let full_ns = full_started.elapsed().as_nanos() / u128::from(FULL_ITERATIONS);

    terminal.take_damage();
    let mut incremental = cache.build(&terminal);
    let damage_started = Instant::now();
    for iteration in 0..DAMAGE_ITERATIONS {
        let row = iteration % u32::from(size.rows);
        terminal.feed(format!("\x1b[{};1H中🙂-{iteration:04}", row + 1).as_bytes());
        let damage = terminal.take_damage();
        incremental = cache.update(&incremental, &terminal, &damage);
    }
    let damage_ns = damage_started.elapsed().as_nanos() / u128::from(DAMAGE_ITERATIONS);
    let rebuilt = TerminalRenderSnapshot::from_terminal(&terminal);
    assert_eq!(
        incremental, rebuilt,
        "damage snapshot must match full rebuild"
    );

    let metrics = cache.metrics();
    let reuse_total = metrics.row_hits.saturating_add(metrics.row_misses);
    let reuse_permille = if reuse_total == 0 {
        0
    } else {
        metrics.row_hits.saturating_mul(1_000) / reuse_total
    };
    println!(
        "{{\"schema_version\":1,\"columns\":{},\"rows\":{},\"full_iterations\":{},\"damage_iterations\":{},\"full_mean_ns\":{},\"damage_mean_ns\":{},\"active_snapshot_bytes\":{},\"retained_snapshot_bytes\":{},\"retained_image_bytes\":{},\"row_reuse_permille\":{}}}",
        size.columns,
        size.rows,
        FULL_ITERATIONS,
        DAMAGE_ITERATIONS,
        full_ns,
        damage_ns,
        metrics.active_snapshot_bytes,
        metrics.retained_snapshot_bytes,
        metrics.retained_image_bytes,
        reuse_permille,
    );

    std::hint::black_box(full);
    std::hint::black_box(incremental);
}

fn seeded_terminal(size: TerminalSize) -> Terminal {
    let mut terminal = Terminal::new(size);
    terminal.feed(b"\x1b[?25l");
    for row in 0..size.rows {
        terminal.feed(
            format!(
                "\x1b[{};1H\x1b[38;5;{}mrow-{row:03}-ASCII-中-🙂-e\u{301}\x1b[0m",
                row + 1,
                row % 16,
            )
            .as_bytes(),
        );
    }
    terminal.feed(b"\x1b[1;40H\x1b]8;;https://example.invalid/stage4\x1b\\link\x1b]8;;\x1b\\");
    terminal.feed(b"\x1b[2;40H\x1b]1337;File=inline=1:YWJjZA==\x07");
    terminal
}
