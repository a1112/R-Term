use rterm_native::overlays::OverlayPresentation;

#[test]
fn overlays_are_immutable_renderer_neutral_frame_data() {
    let overlays = [
        OverlayPresentation::Search {
            query: "needle".to_owned(),
            matches: 3,
        },
        OverlayPresentation::CommandPalette {
            query: "open".to_owned(),
            selected: Some(2),
        },
    ];

    assert!(matches!(
        &overlays[0],
        OverlayPresentation::Search { query, matches: 3 } if query == "needle"
    ));
    assert!(matches!(
        &overlays[1],
        OverlayPresentation::CommandPalette { query, selected: Some(2) } if query == "open"
    ));
}
