mod window_geometry;
use std::{path::PathBuf, sync::Mutex};

#[cfg(feature = "functional-test-observer")]
use std::{
    collections::BTreeMap,
    env, io,
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
    thread,
    time::Duration,
};

#[cfg(feature = "functional-test-observer")]
use rssh_functional_tests::{
    ObserverServer, ObserverSnapshotV1, ObserverState, ObserverToken, RuntimeObservationV1,
    TerminalObservationV1, WindowObservationV1,
};
use rssh_web::{WebServer, WebServerConfig};
#[cfg(not(debug_assertions))]
use tauri::path::BaseDirectory;
use tauri::{Manager, RunEvent, Url};
use tokio::sync::oneshot;

struct BackendState {
    shutdown: Mutex<Option<oneshot::Sender<()>>>,
}

#[cfg(feature = "functional-test-observer")]
struct FunctionalObserverState {
    observer: ObserverState,
    publication: Mutex<()>,
    closing: AtomicBool,
}

#[cfg(feature = "functional-test-observer")]
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct FunctionalWebSnapshot {
    schema: u16,
    terminal_text: String,
    cursor_row: u32,
    cursor_column: u32,
    cols: u32,
    rows: u32,
    window_width: u32,
    window_height: u32,
    connection_state: String,
}

#[cfg(feature = "functional-test-observer")]
#[tauri::command]
fn functional_observer_publish(
    state: tauri::State<'_, FunctionalObserverState>,
    snapshot: FunctionalWebSnapshot,
) -> Result<(), String> {
    if snapshot.schema != 1 {
        return Err(format!(
            "unsupported functional Web snapshot {}",
            snapshot.schema
        ));
    }
    let _publication = state
        .publication
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if state.closing.load(Ordering::Acquire) {
        return Ok(());
    }
    let current = state.observer.snapshot();
    state
        .observer
        .publish(ObserverSnapshotV1 {
            schema: 1,
            revision: current.revision.saturating_add(1),
            config_generation: 0,
            config_diagnostic_present: false,
            terminal: TerminalObservationV1 {
                text: snapshot.terminal_text,
                cursor_row: snapshot.cursor_row,
                cursor_column: snapshot.cursor_column,
                modes: BTreeMap::new(),
            },
            window: WindowObservationV1 {
                width: snapshot.window_width,
                height: snapshot.window_height,
                active_tab_id: Some(1),
                active_pane_id: Some(1),
                overlay: None,
                panes: vec![rssh_functional_tests::PaneObservationV1 {
                    tab_id: 1,
                    pane_id: 1,
                    active: true,
                    row: 0,
                    column: 0,
                    rows: snapshot.rows,
                    columns: snapshot.cols,
                }],
            },
            runtime: RuntimeObservationV1 {
                transport_state: snapshot.connection_state.clone(),
                effects: current.runtime.effects,
                render_digest: None,
                worker_count: u32::from(matches!(
                    snapshot.connection_state.as_str(),
                    "connecting" | "open" | "closing"
                )),
                listener_count: 1,
                child_process_count: u32::from(snapshot.connection_state == "open"),
            },
        })
        .map_err(|error| error.to_string())
}

pub fn run() {
    let builder = tauri::Builder::default().plugin(window_geometry::init());
    #[cfg(feature = "functional-test-observer")]
    let builder = builder.invoke_handler(tauri::generate_handler![functional_observer_publish]);
    builder
        .setup(|app| {
            #[cfg(feature = "functional-test-observer")]
            if let Some(observer) = functional_observer_from_environment()? {
                app.manage(observer);
            }
            let web_root = web_root(app.handle())?;
            let server = tauri::async_runtime::block_on(WebServer::bind(WebServerConfig {
                listen: "127.0.0.1:0".parse().expect("loopback endpoint is valid"),
                web_root,
                max_sessions: rssh_web::server::DEFAULT_MAX_SESSIONS,
                allowed_origin: None,
            }))?;
            let bootstrap_url = server.bootstrap_url();
            let (shutdown_tx, shutdown_rx) = oneshot::channel();

            tauri::async_runtime::spawn(async move {
                if let Err(error) = server
                    .run_until(async move {
                        let _ = shutdown_rx.await;
                    })
                    .await
                {
                    eprintln!("R-SSH Web backend stopped: {error}");
                }
            });
            app.manage(BackendState {
                shutdown: Mutex::new(Some(shutdown_tx)),
            });

            let window = app
                .get_webview_window("main")
                .ok_or_else(|| "main Tauri window is missing".to_owned())?;
            window.navigate(
                bootstrap_url
                    .parse::<Url>()
                    .map_err(|error| format!("invalid backend URL: {error}"))?,
            )?;
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building R-SSH Tauri application")
        .run(|app_handle, event| {
            let (main_window_close_requested, defer_main_window_close) =
                if let RunEvent::WindowEvent {
                    label,
                    event: tauri::WindowEvent::CloseRequested { api: _api, .. },
                    ..
                } = &event
                    && label == "main"
                {
                    #[cfg(feature = "functional-test-observer")]
                    let defer_main_window_close =
                        app_handle.try_state::<FunctionalObserverState>().is_some();
                    #[cfg(not(feature = "functional-test-observer"))]
                    let defer_main_window_close = false;
                    if defer_main_window_close {
                        _api.prevent_close();
                    }
                    (true, defer_main_window_close)
                } else {
                    (false, false)
                };
            if !main_window_close_requested && !matches!(event, RunEvent::ExitRequested { .. }) {
                return;
            }
            if let Some(state) = app_handle.try_state::<BackendState>()
                && let Ok(mut shutdown) = state.shutdown.lock()
                && let Some(sender) = shutdown.take()
            {
                let _ = sender.send(());
            }
            #[cfg(feature = "functional-test-observer")]
            let mut final_observation_delivery = None;
            #[cfg(feature = "functional-test-observer")]
            if let Some(state) = app_handle.try_state::<FunctionalObserverState>() {
                let _publication = state
                    .publication
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                state.closing.store(true, Ordering::Release);
                let mut snapshot = state.observer.snapshot();
                snapshot.revision = snapshot.revision.saturating_add(1);
                snapshot.runtime.transport_state = "closed".to_owned();
                snapshot.runtime.worker_count = 0;
                snapshot.runtime.listener_count = 0;
                snapshot.runtime.child_process_count = 0;
                if state.observer.publish(snapshot.clone()).is_ok() {
                    if defer_main_window_close {
                        final_observation_delivery =
                            Some((state.observer.clone(), snapshot.revision));
                    } else {
                        let _ = state
                            .observer
                            .wait_until_delivered(snapshot.revision, Duration::from_millis(250));
                    }
                }
            }
            if defer_main_window_close {
                #[cfg(feature = "functional-test-observer")]
                if let Some((observer, revision)) = final_observation_delivery {
                    exit_after_final_observation_delivery(app_handle.clone(), observer, revision);
                    return;
                }
                destroy_main_window_or_exit(app_handle.clone());
            }
        });
}

fn destroy_main_window_or_exit(app_handle: tauri::AppHandle) {
    let destroyed = app_handle
        .get_webview_window("main")
        .is_some_and(|window| window.destroy().is_ok());
    if !destroyed {
        app_handle.exit(0);
    }
}

#[cfg(feature = "functional-test-observer")]
fn exit_after_final_observation_delivery(
    app_handle: tauri::AppHandle,
    observer: ObserverState,
    revision: u64,
) {
    let fallback_handle = app_handle.clone();
    let spawn_result = thread::Builder::new()
        .name("rssh-tauri-observer-exit".to_owned())
        .spawn(move || {
            let _ = observer.wait_until_delivered(revision, Duration::from_secs(2));
            thread::sleep(Duration::from_secs(1));
            destroy_main_window_or_exit(app_handle);
        });
    if spawn_result.is_err() {
        destroy_main_window_or_exit(fallback_handle);
    }
}

#[cfg(feature = "functional-test-observer")]
fn functional_observer_from_environment()
-> Result<Option<FunctionalObserverState>, Box<dyn std::error::Error>> {
    let endpoint = env::var_os("RSSH_FUNCTIONAL_OBSERVER_ENDPOINT");
    let token = env::var("RSSH_FUNCTIONAL_OBSERVER_TOKEN").ok();
    let (endpoint, token) = match (endpoint, token) {
        (None, None) => return Ok(None),
        (Some(endpoint), Some(token)) => (endpoint, token),
        (Some(_), None) | (None, Some(_)) => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "functional observer endpoint and token must be configured together",
            )
            .into());
        }
    };
    let token = ObserverToken::from_child_process(&token)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?;
    let observer = ObserverState::new(ObserverSnapshotV1 {
        schema: 1,
        revision: 0,
        config_generation: 0,
        config_diagnostic_present: false,
        terminal: TerminalObservationV1 {
            text: String::new(),
            cursor_row: 0,
            cursor_column: 0,
            modes: BTreeMap::new(),
        },
        window: WindowObservationV1 {
            width: 0,
            height: 0,
            active_tab_id: None,
            active_pane_id: None,
            overlay: None,
            panes: Vec::new(),
        },
        runtime: RuntimeObservationV1 {
            transport_state: "starting".to_owned(),
            effects: Vec::new(),
            render_digest: None,
            worker_count: 1,
            listener_count: 1,
            child_process_count: 0,
        },
    })?;
    let mut server = ObserverServer::bind(Path::new(&endpoint), token, observer.clone())?;
    thread::Builder::new()
        .name("rssh-tauri-functional-observer".to_owned())
        .spawn(move || {
            if let Err(error) = server.serve_one() {
                eprintln!("Tauri functional observer error: {error}");
            }
        })?;
    Ok(Some(FunctionalObserverState {
        observer,
        publication: Mutex::new(()),
        closing: AtomicBool::new(false),
    }))
}

fn web_root(_app: &tauri::AppHandle) -> Result<PathBuf, Box<dyn std::error::Error>> {
    #[cfg(debug_assertions)]
    {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../web/dist");
        if !root.join("index.html").is_file() {
            return Err(format!(
                "web assets are missing at {}; run `cd web && npm run build` first",
                root.display()
            )
            .into());
        }
        Ok(root)
    }

    #[cfg(not(debug_assertions))]
    {
        Ok(_app.path().resolve("web/dist", BaseDirectory::Resource)?)
    }
}
