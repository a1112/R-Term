use std::sync::Arc;

use rterm_config::{
    ConfigPatch, DomainConfigPatch, EffectiveConfig, FontConfigPatch, InputConfigPatch,
    LifecycleConfigPatch, Patch, RenderConfigPatch, TerminalConfigPatch, WindowConfigPatch,
    resolve_layers,
};

#[test]
fn patch_truth_table_distinguishes_inherit_clear_and_set() {
    let defaults = EffectiveConfig::default();
    let lower = ConfigPatch {
        font: FontConfigPatch {
            family: Patch::Set("Fira Code".to_owned()),
            ..FontConfigPatch::default()
        },
        ..ConfigPatch::default()
    };

    for (upper, expected) in [
        (Patch::Inherit, "Fira Code"),
        (Patch::Clear, defaults.font.family.as_str()),
        (Patch::Set("Iosevka".to_owned()), "Iosevka"),
    ] {
        let upper = ConfigPatch {
            font: FontConfigPatch {
                family: upper,
                ..FontConfigPatch::default()
            },
            ..ConfigPatch::default()
        };

        let resolved = resolve_layers([&lower, &upper]);

        assert_eq!(resolved.font.family, expected);
    }
}

#[test]
fn later_layers_override_only_their_nested_values() {
    let user_file = ConfigPatch {
        font: FontConfigPatch {
            family: Patch::Set("JetBrains Mono".to_owned()),
            size_milli_points: Patch::Set(13_500),
        },
        terminal: TerminalConfigPatch {
            scrollback_lines: Patch::Set(20_000),
            ..TerminalConfigPatch::default()
        },
        input: InputConfigPatch {
            copy_on_select: Patch::Set(true),
        },
        ..ConfigPatch::default()
    };
    let cli = ConfigPatch {
        font: FontConfigPatch {
            size_milli_points: Patch::Set(15_000),
            ..FontConfigPatch::default()
        },
        window: WindowConfigPatch {
            integrated_titlebar: Patch::Set(false),
            ..WindowConfigPatch::default()
        },
        ..ConfigPatch::default()
    };
    let runtime = ConfigPatch {
        render: RenderConfigPatch {
            max_fps: Patch::Set(144),
        },
        lifecycle: LifecycleConfigPatch {
            reload_on_change: Patch::Set(false),
        },
        ..ConfigPatch::default()
    };
    let per_window = ConfigPatch {
        terminal: TerminalConfigPatch {
            term: Patch::Set("xterm-rterm".to_owned()),
            ..TerminalConfigPatch::default()
        },
        domain: DomainConfigPatch {
            default_domain: Patch::Set(Some("ssh:production".to_owned())),
        },
        ..ConfigPatch::default()
    };

    let resolved = resolve_layers([&user_file, &cli, &runtime, &per_window]);

    assert_eq!(resolved.font.family, "JetBrains Mono");
    assert_eq!(resolved.font.size_milli_points, 15_000);
    assert_eq!(resolved.terminal.scrollback_lines, 20_000);
    assert_eq!(resolved.terminal.term, "xterm-rterm");
    assert!(resolved.input.copy_on_select);
    assert!(!resolved.window.integrated_titlebar);
    assert_eq!(resolved.render.max_fps, 144);
    assert_eq!(
        resolved.domain.default_domain.as_deref(),
        Some("ssh:production")
    );
    assert!(!resolved.lifecycle.reload_on_change);
}

#[test]
fn clear_restores_the_schema_default_without_resetting_sibling_values() {
    let lower = ConfigPatch {
        window: WindowConfigPatch {
            title: Patch::Set("Operations".to_owned()),
            integrated_titlebar: Patch::Set(false),
        },
        ..ConfigPatch::default()
    };
    let upper = ConfigPatch {
        window: WindowConfigPatch {
            title: Patch::Clear,
            ..WindowConfigPatch::default()
        },
        ..ConfigPatch::default()
    };

    let resolved = resolve_layers([&lower, &upper]);

    assert_eq!(
        resolved.window.title,
        EffectiveConfig::default().window.title
    );
    assert!(!resolved.window.integrated_titlebar);
}

#[test]
fn resolution_returns_a_shareable_immutable_snapshot() {
    let snapshot = resolve_layers(std::iter::empty::<&ConfigPatch>());
    let shared = Arc::clone(&snapshot);

    assert!(Arc::ptr_eq(&snapshot, &shared));
    assert_eq!(*snapshot, EffectiveConfig::default());
}
