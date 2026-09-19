use std::{fs, path::PathBuf};

fn repo_file(path: &str) -> String {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    fs::read_to_string(root.join(path)).unwrap()
}

#[test]
fn playwright_runs_all_three_engines_without_semantic_retries() {
    let config = repo_file("web/playwright.config.ts");
    assert!(config.contains("testMatch: 'terminal.spec.ts'"));
    assert!(!config.contains("production.spec.ts"));
    assert!(config.contains("retries: 0"));
    for engine in ["chromium", "firefox", "webkit"] {
        assert!(config.contains(&format!("name: '{engine}'")));
        assert!(config.contains(&format!("browserName: '{engine}'")));
    }
    assert!(config.contains("web.playwright.json"));
    assert!(config.contains("process.env.RSSH_PLAYWRIGHT_EVIDENCE"));
}

#[test]
fn production_web_spec_is_isolated_and_cleanup_handles_spawn_failure() {
    let config = repo_file("web/playwright.production.config.ts");
    let spec = repo_file("web/tests/production.spec.ts");
    assert!(config.contains("testMatch: 'production.spec.ts'"));
    assert!(spec.contains("if (!server.pid)"));
    assert!(spec.contains("server.kill('SIGTERM')"));
}

#[test]
fn web_scenario_leaves_browser_selection_to_the_fixed_matrix() {
    let scenario = repo_file("functional-tests/scenarios/web.local-pty.toml");
    assert!(scenario.contains("capabilities = []"));
    assert!(!scenario.contains("browser_chromium"));
}

#[test]
fn every_browser_writes_a_distinct_evidence_artifact() {
    let workflow = repo_file("docs/trial/legacy-workflows/functional.yml");
    assert!(workflow.contains("web.${{ matrix.browser }}.playwright.json"));
    assert!(workflow.contains("RSSH_PLAYWRIGHT_EVIDENCE"));
}

#[test]
fn web_functional_test_still_uses_browser_keyboard_pointer_and_clipboard_apis() {
    let spec = repo_file("web/tests/terminal.spec.ts");
    for required in [
        "page.keyboard",
        "page.mouse",
        "navigator.clipboard",
        "setViewportSize",
    ] {
        assert!(
            spec.contains(required),
            "missing real browser input path {required}"
        );
    }
    assert!(
        !spec.contains("socket.send("),
        "test bypasses the browser UI"
    );
    assert!(spec.contains("async ({ page, request, browserName })"));
    assert!(spec.contains("if (browserName === 'chromium')"));
    assert_eq!(
        spec.matches("grantPermissions(['clipboard-read', 'clipboard-write']")
            .count(),
        1
    );
    let client = repo_file("web/src/main.ts");
    assert!(client.contains("attachCustomKeyEventHandler"));
    assert!(client.contains("event.key.toLowerCase() === 'v'"));
    assert!(client.contains("return !pasteShortcut"));
}

#[test]
fn baseline_web_ci_builds_functional_assets_for_its_installed_chromium_project() {
    let workflow = repo_file("docs/trial/legacy-workflows/ci.yml");
    assert!(workflow.contains("npm --prefix web run build:functional"));
    assert_eq!(
        workflow
            .matches("npm --prefix web run test:e2e -- --project=chromium")
            .count(),
        2,
        "the executed and recorded Playwright commands must match exactly"
    );
}
