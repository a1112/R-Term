use std::sync::Arc;

use rterm_config::{
    ConfigDiff, ConfigPatch, FontConfigPatch, Patch, ValidatedConfigStore, resolve_layers,
};

#[test]
fn reload_reuses_every_unchanged_domain_subtree() {
    let mut store = ValidatedConfigStore::new(rterm_config::EffectiveConfig::default())
        .expect("default config is valid");
    let before = store.current();
    let patch = ConfigPatch {
        font: FontConfigPatch {
            family: Patch::Set("Iosevka".to_owned()),
            ..FontConfigPatch::default()
        },
        ..ConfigPatch::default()
    };

    let update = store
        .replace(resolve_layers([&patch]))
        .expect("patched config is valid");

    assert!(!Arc::ptr_eq(&before.font, &update.snapshot.font));
    assert!(Arc::ptr_eq(&before.terminal, &update.snapshot.terminal));
    assert!(Arc::ptr_eq(&before.input, &update.snapshot.input));
    assert!(Arc::ptr_eq(&before.window, &update.snapshot.window));
    assert!(Arc::ptr_eq(&before.render, &update.snapshot.render));
    assert!(Arc::ptr_eq(&before.domain, &update.snapshot.domain));
    assert!(Arc::ptr_eq(&before.lifecycle, &update.snapshot.lifecycle));
}

#[test]
fn one_domain_change_only_notifies_that_domain() {
    let before = resolve_layers(std::iter::empty::<&ConfigPatch>());
    let patch = ConfigPatch {
        font: FontConfigPatch {
            size_milli_points: Patch::Set(14_000),
            ..FontConfigPatch::default()
        },
        ..ConfigPatch::default()
    };
    let after = resolve_layers([&patch]);

    let diff = ConfigDiff::between(&before, &after);

    assert!(diff.font.is_some());
    assert!(diff.terminal.is_none());
    assert!(diff.input.is_none());
    assert!(diff.window.is_none());
    assert!(diff.render.is_none());
    assert!(diff.domain.is_none());
    assert!(diff.lifecycle.is_none());
}
