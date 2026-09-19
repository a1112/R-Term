use std::{fs, path::PathBuf};

use rssh_functional_tests::{Capability, CheckpointV1, FunctionalSuite};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn catalog_assigns_stable_ids_to_public_commands_actions_effects_and_lifecycle_results() {
    let root = root();
    let suite = FunctionalSuite::load(root.join("functional-tests")).unwrap();
    let ids: Vec<_> = suite
        .catalog
        .behaviors
        .iter()
        .map(|behavior| behavior.id.as_ref())
        .collect();
    for id in [
        "BHV-CMD-BENCH",
        "BHV-CMD-HELP",
        "BHV-CMD-DOCTOR",
        "BHV-CMD-LOCAL",
        "BHV-CMD-WINDOW",
        "BHV-CMD-SSH",
        "BHV-CMD-SFTP",
        "BHV-CMD-SCP",
        "BHV-CMD-PROFILE-INIT",
        "BHV-CMD-PROFILE-CHECK",
        "BHV-CMD-PROFILE-LIST",
        "BHV-CMD-PROFILE-SHOW",
        "BHV-CMD-PROFILE-RUN",
        "BHV-ACTION-TYPE-TEXT",
        "BHV-ACTION-KEY",
        "BHV-ACTION-MOUSE-CLICK",
        "BHV-ACTION-MOUSE-DRAG",
        "BHV-ACTION-MOUSE-WHEEL",
        "BHV-ACTION-CLIPBOARD-PASTE",
        "BHV-ACTION-RESIZE",
        "BHV-ACTION-FOCUS",
        "BHV-ACTION-WINDOW-CONTROL",
        "BHV-ACTION-PTY-INPUT",
        "BHV-ACTION-FIXTURE-DISCONNECT",
        "BHV-ACTION-FIXTURE-RECONNECT",
        "BHV-ACTION-FINISH",
        "BHV-EFFECT-TRANSPORT-WRITE",
        "BHV-EFFECT-HOST-STREAM",
        "BHV-EFFECT-VISIBLE-OUTPUT",
        "BHV-EFFECT-MODE-CHANGE",
        "BHV-EFFECT-CLIPBOARD-WRITE",
        "BHV-EFFECT-CLIPBOARD-READ",
        "BHV-EFFECT-NOTIFICATION",
        "BHV-EFFECT-BELL",
        "BHV-EFFECT-DIAGNOSTIC",
        "BHV-LIFECYCLE-STARTED",
        "BHV-LIFECYCLE-CONNECTED",
        "BHV-LIFECYCLE-DISCONNECTED",
        "BHV-LIFECYCLE-RECONNECTED",
        "BHV-LIFECYCLE-EXITED",
        "BHV-LIFECYCLE-ERROR",
        "BHV-LIFECYCLE-CLEANUP",
        "BHV-CONSOLE-RESIZE",
        "BHV-CONSOLE-INPUT-MODES",
        "BHV-CONSOLE-OSC52",
        "BHV-CONSOLE-SESSION-LOG",
        "BHV-WINDOW-TABS-PANES-SPLITS",
        "BHV-WINDOW-TAB-SESSION-LIFECYCLE",
        "BHV-WINDOW-LIVE-TAB-TRANSFER",
        "BHV-WINDOW-SELECTION-COPY-MODE",
        "BHV-WINDOW-SCROLL-SEARCH",
        "BHV-WINDOW-COMMAND-PALETTE",
        "BHV-WINDOW-TITLE-PROGRESS-CWD",
        "BHV-WINDOW-ALTERNATE-SCREEN",
        "BHV-WINDOW-IMAGE",
        "BHV-WINDOW-DPI",
        "BHV-SSH-PASSWORD-KEY-AGENT",
        "BHV-SSH-SHELL-EXEC-RESIZE",
        "BHV-TRANSFER-ERROR-PATH",
        "BHV-TRANSFER-INTERRUPT-CLEANUP",
        "BHV-WEB-INPUT-PASTE-RESIZE-RESTART",
        "BHV-WEB-SECURITY-HEADERS",
        "BHV-WEB-BACKPRESSURE",
        "BHV-PACKAGE-OBSERVER-ABSENT",
        "BHV-WINDOW-SYNCHRONIZED-OUTPUT",
        "BHV-WINDOW-PANE-RESTART",
        "BHV-FAULT-LISTENER-CONFLICT",
        "BHV-FAULT-PTY-DISCONNECT",
    ] {
        assert!(ids.contains(&id), "missing stable behavior {id}");
    }
}

#[test]
fn source_variants_are_bound_to_stable_behavior_ids() {
    let root = root();
    let catalog = fs::read_to_string(root.join("functional-tests/behaviors.toml")).unwrap();
    let sources = [
        root.join("crates/rssh-app/src/cli.rs"),
        root.join("crates/rssh-functional-tests/src/scenario.rs"),
        root.join("crates/rssh-runtime/src/api.rs"),
    ];
    let combined = sources
        .iter()
        .map(|path| fs::read_to_string(path).unwrap())
        .collect::<Vec<_>>()
        .join("\n");
    for (variant, id) in [
        ("Bench(BenchOptions)", "BHV-CMD-BENCH"),
        ("Local(LocalOptions)", "BHV-CMD-LOCAL"),
        ("Ssh(SshOptions)", "BHV-CMD-SSH"),
        ("Sftp(SftpOptions)", "BHV-CMD-SFTP"),
        ("Scp(ScpOptions)", "BHV-CMD-SCP"),
        ("Window(WindowOptions)", "BHV-CMD-WINDOW"),
        ("FixtureDisconnect", "BHV-ACTION-FIXTURE-DISCONNECT"),
        ("FixtureReconnect", "BHV-ACTION-FIXTURE-RECONNECT"),
        ("HostStream(Vec<u8>)", "BHV-EFFECT-HOST-STREAM"),
        ("VisibleOutput(Vec<u8>)", "BHV-EFFECT-VISIBLE-OUTPUT"),
        (
            "ModeChange(crate::TerminalModeChange)",
            "BHV-EFFECT-MODE-CHANGE",
        ),
        ("Diagnostic {", "BHV-EFFECT-DIAGNOSTIC"),
    ] {
        assert!(
            combined.contains(variant),
            "source variant disappeared: {variant}"
        );
        assert!(catalog.contains(id), "source variant {variant} lacks {id}");
    }
}

#[test]
fn catalog_is_machine_versioned_and_not_derived_from_test_names() {
    let catalog = fs::read_to_string(root().join("functional-tests/behaviors.toml")).unwrap();
    assert!(catalog.starts_with("schema = 1"));
    assert!(!catalog.contains("::tests::"));
    assert!(!catalog.contains(": test"));
}

#[test]
fn mapped_protocol_evidence_is_executed_by_the_contract_job() {
    let root = root();
    let workflow =
        fs::read_to_string(root.join("docs/trial/legacy-workflows/functional.yml")).unwrap();
    let evidence = fs::read_to_string(root.join("functional-tests/evidence-map.toml")).unwrap();
    for identity in [
        "local_adapter_spawns_reads_resizes_and_preserves_exit_status",
        "window_app_dispatches_native_split_pane_action_payload",
        "window_app_pending_runtime_survives_sync_and_continues_output_until_materialized",
        "window_app_restart_pane_installs_fresh_runtime_without_touching_other_owner",
        "native_authentication_matrix_accepts_password_and_encrypted_ed25519_and_rejects_bad_password",
        "real_ssh_authenticates_through_the_in_memory_agent_protocol",
        "real_sftp_subsystem_rejects_traversal_absolute_and_directory_redirect_escape",
        "real_scp_sink_rejects_incomplete_control_record_at_eof",
        "http_responses_emit_security_and_cache_headers",
        "bounded_input_queue_reports_backpressure_and_cleans_up",
        "authenticated_websocket_reports_real_input_backpressure_and_cleans_up",
        "pixel_renderer_draws_kitty_rgb_direct_inline_image",
        "headless_gpu_readback_matches_cpu_layering_invariants_with_tolerance",
    ] {
        assert!(evidence.contains(identity), "unmapped evidence {identity}");
    }
    for identity in [
        "window_app_dispatches_native_split_pane_action_payload",
        "window_app_pending_runtime_survives_sync_and_continues_output_until_materialized",
        "window_app_restart_pane_installs_fresh_runtime_without_touching_other_owner",
        "real_ssh_authenticates_through_the_in_memory_agent_protocol",
        "real_sftp_subsystem_rejects_traversal_absolute_and_directory_redirect_escape",
        "real_scp_sink_rejects_incomplete_control_record_at_eof",
        "pixel_renderer_draws_kitty_rgb_direct_inline_image",
        "headless_gpu_readback_matches_cpu_layering_invariants_with_tolerance",
    ] {
        assert!(
            workflow.contains(identity),
            "unexecuted focused evidence {identity}"
        );
    }
    assert!(workflow.contains("-p rterm-runtime --features test-support --test pane_worker"));
    assert!(workflow.contains("-p rssh-pty --features runtime-adapter --test runtime_adapter"));
    assert!(workflow.contains("-p rssh-ssh --features runtime-adapter --test runtime_adapter"));
    assert!(workflow.contains("-p rssh-ssh --test loopback_native"));
    assert!(workflow.contains("-p rssh-web --lib"));
}

#[test]
fn tab_session_behaviors_have_explicit_executed_evidence() {
    let root = root();
    let catalog = fs::read_to_string(root.join("functional-tests/behaviors.toml")).unwrap();
    let evidence = fs::read_to_string(root.join("functional-tests/evidence-map.toml")).unwrap();
    let workflow =
        fs::read_to_string(root.join("docs/trial/legacy-workflows/functional.yml")).unwrap();

    for behavior in [
        "BHV-WINDOW-TAB-SESSION-LIFECYCLE",
        "BHV-WINDOW-LIVE-TAB-TRANSFER",
    ] {
        assert!(
            catalog.contains(behavior),
            "missing tab-session behavior {behavior}"
        );
    }

    for identity in [
        "window_app_duplicate_and_reopen_closed_tab_restore_the_full_tab_layout",
        "window_app_pending_windows_share_recently_closed_tab_history",
        "window_app_batch_tab_close_uses_one_confirmation_for_the_whole_set",
        "window_app_tab_context_menu_exposes_browser_tab_actions",
        "window_app_browser_tab_shortcuts_open_launcher_reopen_and_activate_tabs",
        "window_app_tab_bar_wheel_scrolls_headers_without_switching_sessions",
        "window_app_tab_session_config_prefers_new_values_and_maps_legacy_values",
        "window_app_move_tab_to_new_window_transfers_every_pane_runtime",
        "window_manager_transfers_a_live_tab_between_windows_and_remaps_events",
        "window_manager_transfers_its_final_tab_and_retires_the_source_window",
    ] {
        assert!(
            evidence.contains(identity),
            "unmapped tab-session evidence {identity}"
        );
        assert!(
            workflow.contains(identity),
            "unexecuted tab-session evidence {identity}"
        );
    }
}

#[test]
fn gpu_readback_journeys_gate_a_stable_render_region_and_catalog_the_behavior() {
    let suite = FunctionalSuite::load(root().join("functional-tests")).unwrap();
    let behavior = "BHV-RENDER-STABLE-REGION";
    assert!(
        suite
            .catalog
            .behaviors
            .iter()
            .any(|entry| entry.id.as_ref() == behavior),
        "stable render probes need their own public behavior"
    );

    let scenarios = suite
        .scenarios
        .iter()
        .filter(|scenario| scenario.capabilities.contains(&Capability::GpuReadback))
        .collect::<Vec<_>>();
    assert!(
        !scenarios.is_empty(),
        "the fixed matrix needs a GPU journey"
    );
    for scenario in scenarios {
        assert!(
            scenario
                .behavior_ids
                .iter()
                .any(|id| id.as_ref() == behavior)
        );
        assert!(
            scenario
                .checkpoints
                .iter()
                .any(|checkpoint| matches!(checkpoint, CheckpointV1::RenderProbe { region, digest } if region == "terminal:first-row-16-cells" && digest.starts_with("sha256:"))),
            "{} must hard-gate its stable CPU pixel region",
            scenario.id
        );
    }
}
