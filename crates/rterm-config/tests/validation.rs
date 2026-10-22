use std::sync::Arc;

use rterm_config::{ConfigDiff, EffectiveConfig, ValidatedConfigStore};

#[test]
fn invalid_candidate_preserves_last_known_good_and_reports_field_paths() {
    let mut store = ValidatedConfigStore::new(EffectiveConfig::default())
        .expect("schema defaults must be valid");
    let previous = store.current();
    let mut invalid = EffectiveConfig::default();
    Arc::make_mut(&mut invalid.font).family.clear();
    Arc::make_mut(&mut invalid.render).max_fps = 0;

    let diagnostics = store
        .replace(Arc::new(invalid))
        .expect_err("invalid candidate must be rejected");

    assert_eq!(
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic.path.as_str())
            .collect::<Vec<_>>(),
        ["font.family", "render.max_fps"]
    );
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| !diagnostic.message.is_empty())
    );
    assert!(Arc::ptr_eq(&previous, &store.current()));
}

#[test]
fn font_only_candidate_emits_only_a_typed_font_diff() {
    let mut store = ValidatedConfigStore::new(EffectiveConfig::default()).unwrap();
    let mut candidate = EffectiveConfig::default();
    let font = Arc::make_mut(&mut candidate.font);
    font.family = "Iosevka".to_owned();
    font.size_milli_points = 14_500;

    let update = store
        .replace(Arc::new(candidate))
        .expect("valid font candidate");

    let font = update.diff.font.expect("font diff");
    assert!(font.family);
    assert!(font.size_milli_points);
    assert!(update.diff.terminal.is_none());
    assert!(update.diff.input.is_none());
    assert!(update.diff.window.is_none());
    assert!(update.diff.render.is_none());
    assert!(update.diff.domain.is_none());
    assert!(update.diff.lifecycle.is_none());
    assert_eq!(update.snapshot.font.family, "Iosevka");
    assert!(Arc::ptr_eq(&update.snapshot, &store.current()));
}

#[test]
fn config_diff_marks_each_changed_domain_without_copying_unchanged_domains() {
    let before = EffectiveConfig::default();
    let mut after = before.clone();
    Arc::make_mut(&mut after.terminal).scrollback_lines += 1;
    let input = Arc::make_mut(&mut after.input);
    input.copy_on_select = !input.copy_on_select;
    let window = Arc::make_mut(&mut after.window);
    window.integrated_titlebar = !window.integrated_titlebar;
    Arc::make_mut(&mut after.render).max_fps += 1;
    Arc::make_mut(&mut after.domain).default_domain = Some("local:test".to_owned());
    let lifecycle = Arc::make_mut(&mut after.lifecycle);
    lifecycle.reload_on_change = !lifecycle.reload_on_change;

    let diff = ConfigDiff::between(&before, &after);

    assert!(diff.font.is_none());
    assert!(diff.terminal.expect("terminal diff").scrollback_lines);
    assert!(diff.input.expect("input diff").copy_on_select);
    assert!(diff.window.expect("window diff").integrated_titlebar);
    assert!(diff.render.expect("render diff").max_fps);
    assert!(diff.domain.expect("domain diff").default_domain);
    assert!(diff.lifecycle.expect("lifecycle diff").reload_on_change);
}

#[test]
fn identical_candidate_keeps_the_existing_arc_and_empty_diff() {
    let mut store = ValidatedConfigStore::new(EffectiveConfig::default()).unwrap();
    let previous = store.current();

    let update = store
        .replace(Arc::new(EffectiveConfig::default()))
        .expect("identical defaults remain valid");

    assert!(update.diff.is_empty());
    assert!(Arc::ptr_eq(&previous, &update.snapshot));
    assert!(Arc::ptr_eq(&previous, &store.current()));
}
