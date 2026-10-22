use rterm_domain::TabId;
use rterm_native::tabs::TabPresentation;

#[test]
fn tab_presentation_is_immutable_renderer_neutral_frame_data() {
    let active = TabPresentation::new(TabId::new(1), "one", true);
    let inactive = TabPresentation::new(TabId::new(2), "two", false);

    assert_eq!(active.tab, TabId::new(1));
    assert_eq!(active.title, "one");
    assert!(active.active);
    assert!(!inactive.active);
}
