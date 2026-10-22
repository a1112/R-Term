use std::sync::Arc;

use rterm_domain::{PaneId, TabId};
use rterm_native::{
    CellMetrics, CursorPresentation, FrameRevision, OverlayPresentation, PaneFrameCandidate,
    PaneLayoutPane, PaneLayoutSpec, PaneRenderRect, PaneSplitDirection, PaneSplitSpec,
    PresentationInput, RenderMode, ScaleFactor, ScrollbarPresentation, SelectionPresentation,
    SurfacePresentation, TabPresentation, build_pane_layout, build_presentation,
};
use rterm_runtime::{PaneToken, PaneTokenAllocator, RuntimeRevision};
use rterm_types::{DamageRegion, TerminalSize};

fn token(pane: u64) -> PaneToken {
    PaneTokenAllocator::new()
        .issue(PaneId::new(pane))
        .expect("pane token")
}

fn next_revision(revision: RuntimeRevision) -> RuntimeRevision {
    revision.next().expect("revision")
}

#[test]
fn layout_preserves_tab_pane_order_splits_zoom_and_titlebar_offset() {
    let one = PaneId::new(1);
    let two = PaneId::new(2);
    let three = PaneId::new(3);
    let panes = vec![
        PaneLayoutPane::root(one),
        PaneLayoutPane::split(two, PaneSplitSpec::new(one, PaneSplitDirection::Right, 0)),
        PaneLayoutPane::split(three, PaneSplitSpec::new(two, PaneSplitDirection::Down, 1)),
    ];
    let layout = build_pane_layout(&PaneLayoutSpec::new(
        TerminalSize::new(80, 24),
        2,
        panes.clone(),
        None,
    ));

    assert_eq!(
        layout
            .panes
            .iter()
            .map(|pane| pane.pane)
            .collect::<Vec<_>>(),
        [one, two, three]
    );
    assert_eq!(layout.panes[0].rect, PaneRenderRect::new(2, 0, 24, 39));
    assert_eq!(layout.panes[1].rect, PaneRenderRect::new(2, 40, 12, 40));
    assert_eq!(layout.panes[2].rect, PaneRenderRect::new(15, 40, 11, 40));
    assert_eq!(layout.separators.len(), 2);

    let zoomed = build_pane_layout(&PaneLayoutSpec::new(
        TerminalSize::new(80, 24),
        2,
        panes,
        Some(three),
    ));
    assert_eq!(zoomed.panes.len(), 1);
    assert_eq!(zoomed.panes[0].pane, three);
    assert_eq!(zoomed.panes[0].rect, PaneRenderRect::new(2, 0, 24, 80));
    assert!(zoomed.separators.is_empty());
}

#[test]
fn surface_scale_keeps_logical_cells_and_derives_physical_metrics() {
    let surface = SurfacePresentation::new(
        1920,
        1080,
        ScaleFactor::from_milli(1500).expect("valid scale"),
        CellMetrics::new(8, 16),
        1,
        1,
    );

    assert_eq!(surface.physical_cell_width(), 12);
    assert_eq!(surface.physical_cell_height(), 24);
    assert_eq!(surface.reserved_top_rows(), 2);
}

#[test]
fn presentation_selects_current_generation_latest_revision_and_offsets_damage() {
    let pane = PaneId::new(7);
    let stale = token(7);
    let current = token(7);
    let first = RuntimeRevision::FIRST;
    let second = next_revision(first);
    let layout = build_pane_layout(&PaneLayoutSpec::new(
        TerminalSize::new(20, 8),
        2,
        vec![PaneLayoutPane::root(pane)],
        None,
    ));
    let candidates = vec![
        PaneFrameCandidate::new(stale, second, Arc::new("stale"), vec![]),
        PaneFrameCandidate::new(
            current,
            first,
            Arc::new("first"),
            vec![DamageRegion::new(1, 2, 3, 1)],
        ),
        PaneFrameCandidate::new(
            current,
            second,
            Arc::new("latest"),
            vec![DamageRegion::new(2, 3, 4, 2)],
        )
        .with_cursor(CursorPresentation::new(4, 5, true))
        .with_selection(SelectionPresentation::new(1, 2, 6, 7))
        .with_scrollbar(ScrollbarPresentation::new(10, 100, 8)),
    ];
    let input = PresentationInput::new(
        FrameRevision::new(2).expect("frame revision"),
        FrameRevision::new(1),
        SurfacePresentation::new(800, 600, ScaleFactor::ONE, CellMetrics::new(8, 16), 1, 1),
        layout,
        vec![current],
        candidates,
    );
    let frame = build_presentation(input).expect("presentation");

    assert_eq!(*frame.panes[0].snapshot, "latest");
    assert_eq!(frame.panes[0].revision, second);
    assert_eq!(
        frame.panes[0].cursor,
        Some(CursorPresentation::new(4, 5, true))
    );
    assert_eq!(
        frame.panes[0].selection,
        Some(SelectionPresentation::new(1, 2, 6, 7))
    );
    assert_eq!(
        frame.panes[0].scrollbar,
        Some(ScrollbarPresentation::new(10, 100, 8))
    );
    assert_eq!(frame.damage, [DamageRegion::new(2, 5, 4, 2)]);
    assert!(frame.separators.is_empty());
    assert_eq!(frame.render_mode, RenderMode::Damage);
}

#[test]
fn damage_is_clipped_to_its_pane_and_empty_regions_are_discarded() {
    let pane = PaneId::new(23);
    let owner = token(23);
    let layout = build_pane_layout(&PaneLayoutSpec::new(
        TerminalSize::new(10, 5),
        1,
        vec![PaneLayoutPane::root(pane)],
        None,
    ));
    let frame = build_presentation(PresentationInput::new(
        FrameRevision::new(2).expect("frame revision"),
        FrameRevision::new(1),
        SurfacePresentation::new(80, 80, ScaleFactor::ONE, CellMetrics::new(8, 16), 1, 0),
        layout,
        vec![owner],
        vec![PaneFrameCandidate::new(
            owner,
            RuntimeRevision::FIRST,
            Arc::new(()),
            vec![
                DamageRegion::new(8, 4, 9, 9),
                DamageRegion::new(10, 0, 1, 1),
            ],
        )],
    ))
    .expect("presentation");

    assert_eq!(frame.damage, [DamageRegion::new(8, 5, 2, 1)]);
}

#[test]
fn tabs_search_command_palette_and_titlebar_are_immutable_frame_data() {
    let pane = PaneId::new(11);
    let owner = token(11);
    let layout = build_pane_layout(&PaneLayoutSpec::new(
        TerminalSize::new(80, 24),
        2,
        vec![PaneLayoutPane::root(pane)],
        None,
    ));
    let mut input = PresentationInput::new(
        FrameRevision::new(9).expect("frame revision"),
        None,
        SurfacePresentation::new(1280, 720, ScaleFactor::ONE, CellMetrics::new(8, 16), 1, 1),
        layout,
        vec![owner],
        vec![PaneFrameCandidate::new(
            owner,
            RuntimeRevision::FIRST,
            Arc::new(42_u64),
            vec![],
        )],
    );
    input.tabs = vec![
        TabPresentation::new(TabId::new(1), "one", true),
        TabPresentation::new(TabId::new(2), "two", false),
    ];
    input.overlays = vec![
        OverlayPresentation::Search {
            query: "needle".to_owned(),
            matches: 3,
        },
        OverlayPresentation::CommandPalette {
            query: "open".to_owned(),
            selected: Some(2),
        },
    ];
    let frame = build_presentation(input).expect("presentation");

    assert_eq!(frame.tabs.len(), 2);
    assert_eq!(frame.overlays.len(), 2);
    assert_eq!(frame.surface.reserved_top_rows(), 2);
    assert_eq!(frame.render_mode, RenderMode::Full);
}

#[test]
fn stale_frame_revision_is_rejected_without_selecting_snapshots() {
    let pane = PaneId::new(19);
    let owner = token(19);
    let layout = build_pane_layout(&PaneLayoutSpec::new(
        TerminalSize::new(10, 5),
        0,
        vec![PaneLayoutPane::root(pane)],
        None,
    ));
    let error = build_presentation(PresentationInput::new(
        FrameRevision::new(4).expect("frame revision"),
        FrameRevision::new(4),
        SurfacePresentation::new(80, 80, ScaleFactor::ONE, CellMetrics::new(8, 16), 0, 0),
        layout,
        vec![owner],
        vec![PaneFrameCandidate::new(
            owner,
            RuntimeRevision::FIRST,
            Arc::new(()),
            vec![],
        )],
    ))
    .expect_err("stale frame must be rejected");

    assert_eq!(error.frame_revision(), Some(FrameRevision::new(4).unwrap()));
}
