use std::sync::Arc;

use rterm_render_core::{SnapshotCacheConfig, TerminalRenderSnapshot, TerminalSnapshotCache};
use rterm_terminal::Terminal;
use rterm_types::TerminalSize;

#[test]
fn repeated_cells_share_grapheme_style_and_hyperlink_allocations() {
    let mut terminal = Terminal::new(TerminalSize::new(8, 1));
    terminal.feed(b"\x1b]8;;https://example.invalid/shared\x1b\\aaaa\x1b]8;;\x1b\\");

    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    let cells = snapshot.cells();
    assert_eq!(cells.len(), 4);

    for cell in &cells[1..] {
        assert!(
            Arc::ptr_eq(cells[0].grapheme(), cell.grapheme()),
            "repeated graphemes must use one immutable allocation"
        );
        assert!(
            Arc::ptr_eq(cells[0].style(), cell.style()),
            "identical visual styles must use one immutable allocation"
        );
        assert!(
            Arc::ptr_eq(
                cells[0].hyperlink().expect("first hyperlink"),
                cell.hyperlink().expect("repeated hyperlink"),
            ),
            "terminal-owned hyperlink identity must survive snapshot projection"
        );
    }
}

#[test]
fn repeated_inline_images_share_immutable_payload_allocations() {
    let mut terminal = Terminal::new(TerminalSize::new(4, 2));
    terminal.feed(b"\x1b]1337;File=inline=1:YWJjZA==\x07");
    terminal.feed(b"\r\n\x1b]1337;File=inline=1:YWJjZA==\x07");

    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    let images = snapshot.inline_images();
    assert_eq!(images.len(), 2);
    assert!(Arc::ptr_eq(images[0].payload(), images[1].payload()));

    let _: Arc<[u8]> = images[0].data.clone();
}

#[test]
fn renderer_core_manifest_stays_platform_and_decoder_free() {
    let manifest = std::fs::read_to_string(format!("{}/Cargo.toml", env!("CARGO_MANIFEST_DIR")))
        .expect("read renderer-core manifest");

    for forbidden in [
        "wgpu",
        "winit",
        "raw-window-handle",
        "image =",
        "rterm-app",
        "rterm-ssh",
        "rterm-pty",
    ] {
        assert!(
            !manifest.contains(forbidden),
            "renderer-core manifest must not contain {forbidden}"
        );
    }
}

#[test]
fn standard_and_large_snapshots_expose_immutable_rows() {
    for size in [TerminalSize::new(80, 24), TerminalSize::new(200, 60)] {
        let mut terminal = Terminal::new(size);
        for row in 0..size.rows {
            terminal.feed(format!("\x1b[{};1Hrow-{row}", row + 1).as_bytes());
        }

        let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
        assert_eq!(snapshot.rows().len(), usize::from(size.rows));
        assert!(snapshot.rows().iter().all(|row| {
            row.cells()
                .windows(2)
                .all(|pair| pair[0].column < pair[1].column)
        }));
    }
}

#[test]
fn damage_replaces_only_intersecting_rows_and_matches_full_rebuild() {
    let mut terminal = Terminal::new(TerminalSize::new(12, 3));
    terminal.feed("ascii\r\n中🙂\r\nlast".as_bytes());
    terminal.take_damage();
    let mut incremental = TerminalRenderSnapshot::from_terminal(&terminal);
    let before = incremental.rows().to_vec();

    terminal.feed(b"\x1b[2;1Hchanged");
    let damage = terminal.take_damage();
    incremental.update_from_terminal_damage(&terminal, &damage);

    assert!(Arc::ptr_eq(&before[0], &incremental.rows()[0]));
    assert!(!Arc::ptr_eq(&before[1], &incremental.rows()[1]));
    assert!(Arc::ptr_eq(&before[2], &incremental.rows()[2]));
    assert_eq!(
        incremental,
        TerminalRenderSnapshot::from_terminal(&terminal)
    );
}

#[test]
fn cursor_only_updates_reuse_every_row() {
    let mut terminal = Terminal::new(TerminalSize::new(8, 2));
    terminal.feed(b"first\r\nsecond");
    let mut snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    let before = snapshot.rows().to_vec();

    terminal.feed(b"\x1b[1;2H");
    snapshot.update_cursor_from_terminal(&terminal, 0);

    assert!(
        before
            .iter()
            .zip(snapshot.rows())
            .all(|(before, after)| Arc::ptr_eq(before, after))
    );
}

#[test]
fn overlay_clones_only_the_touched_row() {
    let mut terminal = Terminal::new(TerminalSize::new(8, 2));
    terminal.feed(b"first\r\nsecond");
    let snapshot = TerminalRenderSnapshot::from_terminal(&terminal);
    let before = snapshot.rows().to_vec();

    let overlaid = snapshot.with_overlay_cells([rterm_render_core::RenderCell::new(1, 0, "X")]);

    assert!(Arc::ptr_eq(&before[0], &overlaid.rows()[0]));
    assert!(!Arc::ptr_eq(&before[1], &overlaid.rows()[1]));
}

#[test]
fn bounded_snapshot_cache_reuses_rows_without_exceeding_either_budget() {
    let mut terminal = Terminal::new(TerminalSize::new(16, 2));
    terminal.feed(b"same-row\r\nsame-row");
    let mut cache = TerminalSnapshotCache::new(SnapshotCacheConfig::new(4096, 64));

    let first = cache.build(&terminal);
    let second = cache.build(&terminal);
    assert!(Arc::ptr_eq(&first.rows()[0], &second.rows()[0]));
    assert!(Arc::ptr_eq(&first.rows()[1], &second.rows()[1]));

    let metrics = cache.metrics();
    assert!(metrics.retained_snapshot_bytes <= metrics.snapshot_budget_bytes);
    assert!(metrics.retained_image_bytes <= metrics.image_budget_bytes);
    assert!(metrics.row_hits >= 2);
}

#[test]
fn zero_budget_bypasses_reuse_without_truncating_the_active_frame() {
    let mut terminal = Terminal::new(TerminalSize::new(8, 1));
    terminal.feed(b"complete");
    let mut cache = TerminalSnapshotCache::new(SnapshotCacheConfig::new(0, 0));

    let first = cache.build(&terminal);
    let second = cache.build(&terminal);
    assert_eq!(second.cells().len(), 8);
    assert!(!Arc::ptr_eq(&first.rows()[0], &second.rows()[0]));
    assert_eq!(cache.metrics().retained_snapshot_bytes, 0);
    assert!(cache.metrics().oversize_bypasses >= 1);
}

#[test]
fn shrinking_snapshot_budgets_evicts_retained_state_deterministically() {
    let mut terminal = Terminal::new(TerminalSize::new(16, 2));
    terminal.feed(b"first\r\nsecond");
    let mut cache = TerminalSnapshotCache::new(SnapshotCacheConfig::new(4096, 4096));
    let _ = cache.build(&terminal);
    assert!(cache.metrics().retained_snapshot_bytes > 0);

    cache.set_config(SnapshotCacheConfig::new(0, 0));
    let metrics = cache.metrics();
    assert_eq!(metrics.retained_snapshot_bytes, 0);
    assert_eq!(metrics.retained_image_bytes, 0);
    assert!(metrics.evictions > 0);
}

#[test]
fn oversized_image_payload_is_renderable_but_not_retained_by_the_cache() {
    let mut terminal = Terminal::new(TerminalSize::new(4, 1));
    terminal.feed(b"\x1b]1337;File=inline=1:YWJjZA==\x07");
    let mut cache = TerminalSnapshotCache::new(SnapshotCacheConfig::new(4096, 3));

    let snapshot = cache.build(&terminal);
    assert_eq!(snapshot.inline_images()[0].payload().as_ref(), b"abcd");
    assert_eq!(cache.metrics().retained_image_bytes, 0);
    assert!(cache.metrics().oversize_bypasses >= 1);
}
