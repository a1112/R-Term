use std::{fs, path::PathBuf};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn tauri_test_build_projects_web_state_into_the_read_only_observer() {
    let root = root();
    let manifest = fs::read_to_string(root.join("tauri/src-tauri/Cargo.toml")).unwrap();
    let backend = fs::read_to_string(root.join("tauri/src-tauri/src/lib.rs")).unwrap();
    let web = fs::read_to_string(root.join("web/src/main.ts")).unwrap();

    assert!(manifest.contains("functional-test-observer"));
    assert!(manifest.contains("rterm-functional-tests"));
    assert!(backend.contains("functional_observer_publish"));
    assert!(backend.contains("ObserverServer::bind"));
    assert!(backend.contains("wait_until_delivered"));
    assert!(web.contains("invoke('functional_observer_publish'"));
    assert!(web.contains("terminal.onWriteParsed"));
}

#[test]
fn tauri_close_snapshot_is_serialized_with_web_snapshot_publication() {
    let backend =
        fs::read_to_string(root().join("tauri/src-tauri/src/lib.rs")).expect("Tauri backend");
    assert!(backend.contains("publication: Mutex<()>"));
    assert!(backend.contains("publication: Mutex::new(())"));

    let web_publish = backend
        .split("fn functional_observer_publish(")
        .nth(1)
        .expect("functional Web publication")
        .split("pub fn run()")
        .next()
        .expect("bounded Web publication");
    let web_lock = web_publish
        .find(".publication")
        .expect("serialize Web publication");
    assert!(web_publish[web_lock..].contains(".lock()"));
    let web_snapshot = web_publish
        .find("state.observer.snapshot()")
        .expect("read Web observer snapshot");
    assert!(web_lock < web_snapshot);

    let close_publish = backend
        .split("let mut final_observation_delivery = None;")
        .nth(1)
        .expect("functional close publication")
        .split("if defer_main_window_close")
        .next()
        .expect("bounded close publication");
    let close_lock = close_publish
        .find(".publication")
        .expect("serialize close publication");
    assert!(close_publish[close_lock..].contains(".lock()"));
    let close_snapshot = close_publish
        .find("state.observer.snapshot()")
        .expect("read close observer snapshot");
    assert!(close_lock < close_snapshot);
}

#[test]
fn tauri_close_snapshot_cannot_be_overwritten_by_late_web_publication() {
    let backend =
        fs::read_to_string(root().join("tauri/src-tauri/src/lib.rs")).expect("Tauri backend");
    assert!(backend.contains("closing: AtomicBool"));
    assert!(backend.contains("closing: AtomicBool::new(false)"));

    let web_publish = backend
        .split("fn functional_observer_publish(")
        .nth(1)
        .expect("functional Web publication")
        .split("pub fn run()")
        .next()
        .expect("bounded Web publication");
    let web_lock = web_publish
        .find(".publication")
        .expect("serialize Web publication");
    let closing_guard = web_publish
        .find("state.closing.load(Ordering::Acquire)")
        .expect("reject publications after close starts");
    assert!(web_lock < closing_guard);

    let close_publish = backend
        .split("let mut final_observation_delivery = None;")
        .nth(1)
        .expect("functional close publication")
        .split("if defer_main_window_close")
        .next()
        .expect("bounded close publication");
    let close_lock = close_publish
        .find(".publication")
        .expect("serialize close publication");
    let closing_store = close_publish
        .find("state.closing.store(true, Ordering::Release)")
        .expect("close publication fence");
    let close_snapshot = close_publish
        .find("state.observer.snapshot()")
        .expect("read final observer snapshot");
    assert!(close_lock < closing_store);
    assert!(closing_store < close_snapshot);
}

#[test]
fn closing_the_main_window_drives_backend_cleanup_and_app_exit() {
    let backend =
        fs::read_to_string(root().join("tauri/src-tauri/src/lib.rs")).expect("Tauri backend");
    let normalized = backend.split_whitespace().collect::<Vec<_>>().join(" ");

    assert!(backend.contains("tauri::WindowEvent::CloseRequested"));
    assert!(backend.contains("label == \"main\""));
    assert!(normalized.contains("let (main_window_close_requested, defer_main_window_close)"));
    assert!(normalized.contains(
        "let defer_main_window_close = app_handle.try_state::<FunctionalObserverState>().is_some()"
    ));
    assert!(backend.contains("if defer_main_window_close"));
    assert!(backend.contains("_api.prevent_close()"));
    assert!(backend.contains("exit_after_final_observation_delivery"));
    assert!(backend.contains("Duration::from_secs(2)"));
    assert!(backend.contains("thread::sleep(Duration::from_secs(1))"));
    assert!(backend.contains("destroy_main_window_or_exit"));
    assert!(backend.contains("window.destroy()"));
    assert!(backend.contains("app_handle.exit(0)"));
}

#[test]
fn production_close_uses_tauri_default_window_destruction() {
    let backend =
        fs::read_to_string(root().join("tauri/src-tauri/src/lib.rs")).expect("Tauri backend");
    let close_handler = backend
        .split(".run(|app_handle, event|")
        .nth(1)
        .expect("Tauri run-event handler")
        .split("fn destroy_main_window_or_exit")
        .next()
        .expect("bounded Tauri run-event handler");
    let normalized = close_handler
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");

    let feature_guard = close_handler
        .find("#[cfg(feature = \"functional-test-observer\")]")
        .expect("observer-only close deferral");
    let prevent_close = close_handler
        .find("_api.prevent_close()")
        .expect("deferred observer close");
    let deferred_destroy = close_handler
        .rfind("if defer_main_window_close")
        .expect("observer-only explicit destruction");
    assert!(feature_guard < prevent_close);
    assert!(prevent_close < deferred_destroy);
    assert!(normalized.contains(
        "#[cfg(not(feature = \"functional-test-observer\"))] let defer_main_window_close = false;"
    ));
    assert!(!close_handler.contains("if main_window_close_requested {"));
}

#[test]
fn production_tauri_build_does_not_enable_the_observer_feature() {
    let package = fs::read_to_string(root().join("tauri/package.json")).unwrap();
    assert!(package.contains("--features functional-test-observer"));
    assert!(package.contains("build:functional"));
    assert!(
        !package.lines().any(|line| {
            line.contains("\"build\"") && line.contains("functional-test-observer")
        })
    );
}

#[test]
fn tauri_remote_ipc_is_scoped_to_the_functional_feature_and_loopback() {
    let root = root();
    let build = fs::read_to_string(root.join("tauri/src-tauri/build.rs")).unwrap();
    let production =
        fs::read_to_string(root.join("tauri/src-tauri/capabilities/production/default.json"))
            .unwrap();
    let functional =
        fs::read_to_string(root.join("tauri/src-tauri/capabilities/functional/observer.json"))
            .unwrap();
    let functional_permission =
        fs::read_to_string(root.join("tauri/src-tauri/permissions/functional/observer.toml"))
            .unwrap();

    assert!(build.contains("CARGO_FEATURE_FUNCTIONAL_TEST_OBSERVER"));
    assert!(build.contains("AppManifest::new"));
    assert!(build.contains("capabilities/production"));
    assert!(build.contains("capabilities/functional"));

    assert!(production.contains("http://127.0.0.1:*"));
    assert!(!production.contains("functional_observer_publish"));
    assert!(!production.contains("allow-functional-observer-publish"));
    assert!(!production.contains("core:default"));
    assert!(functional.contains("http://127.0.0.1:*"));
    assert!(functional.contains("allow-functional-observer-publish"));
    assert!(!functional.contains("core:default"));
    assert!(functional_permission.contains("functional_observer_publish"));
    assert!(!functional.contains("https://"));
    assert!(!functional.contains("http://*"));
}
