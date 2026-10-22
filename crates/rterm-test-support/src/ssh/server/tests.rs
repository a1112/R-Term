use std::{
    net::{Ipv4Addr, TcpListener, TcpStream},
    process::Command,
    sync::Arc,
    time::{Duration, Instant},
};

use russh::{ChannelMsg, client, keys::PrivateKeyWithHashAlg};
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};

use super::{CommandResponse, HermeticSshServer, OpenSshTool, SshEvent, SshFixtureError};
use crate::ssh::{IdentityFixture, LoopbackEchoServer};

const DEADLINE: Duration = Duration::from_secs(3);

struct ExpectedHostKey(russh::keys::ssh_key::PublicKey);

impl client::Handler for ExpectedHostKey {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        server_public_key: &russh::keys::ssh_key::PublicKey,
    ) -> Result<bool, Self::Error> {
        Ok(server_public_key == &self.0)
    }
}

#[test]
fn start_is_ready_and_stop_releases_the_port_within_deadline() {
    let server = HermeticSshServer::start(DEADLINE).expect("start SSH fixture");
    let address = server.address();
    assert!(address.ip().is_loopback());
    TcpStream::connect_timeout(&address, Duration::from_secs(1)).expect("server ready");

    let started = Instant::now();
    server.stop(DEADLINE).expect("stop SSH fixture");
    assert!(started.elapsed() < DEADLINE);
    TcpListener::bind(address).expect("SSH port released");
}

#[test]
fn drop_is_bounded_even_with_an_incomplete_handshake() {
    let server = HermeticSshServer::start(DEADLINE).expect("start SSH fixture");
    let address = server.address();
    let connection = TcpStream::connect(server.address()).expect("open incomplete handshake");
    let started = Instant::now();
    drop(server);
    assert!(started.elapsed() < DEADLINE);
    drop(connection);
    TcpListener::bind(address).expect("Drop released the SSH listener port");
}

#[test]
fn stop_disconnects_an_authenticated_session_before_returning() {
    let server = HermeticSshServer::start(DEADLINE).expect("start SSH fixture");
    let address = server.address();
    let host_key = server.host_key().clone();
    let identity = Arc::clone(server.agent().private_key());
    let (ready_tx, ready_rx) = std::sync::mpsc::sync_channel(1);
    let (closed_tx, closed_rx) = std::sync::mpsc::sync_channel(1);
    let client = std::thread::spawn(move || {
        runtime().block_on(async move {
            let mut client = client::connect(
                Arc::new(client::Config::default()),
                address,
                ExpectedHostKey(host_key),
            )
            .await
            .unwrap();
            assert!(
                client
                    .authenticate_publickey(
                        "fixture-user",
                        PrivateKeyWithHashAlg::new(identity, None),
                    )
                    .await
                    .unwrap()
                    .success()
            );
            ready_tx.send(()).unwrap();
            let disconnected = tokio::time::timeout(Duration::from_secs(2), async {
                while !client.is_closed() {
                    tokio::time::sleep(Duration::from_millis(5)).await;
                }
            })
            .await
            .is_ok();
            closed_tx.send(disconnected).unwrap();
        });
    });
    ready_rx
        .recv_timeout(DEADLINE)
        .expect("client authenticated");
    server.stop(DEADLINE).expect("stop SSH fixture");
    assert!(
        closed_rx
            .recv_timeout(DEADLINE)
            .expect("client observation")
    );
    client.join().unwrap();
}

#[test]
fn stop_drains_an_active_direct_tcpip_child_before_returning() {
    let server = HermeticSshServer::start(DEADLINE).expect("start SSH fixture");
    let probe = server.task_probe();
    let target_listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    let target_address = target_listener.local_addr().unwrap();
    let address = server.address();
    let host_key = server.host_key().clone();
    let identity = Arc::clone(server.agent().private_key());
    let (ready_tx, ready_rx) = std::sync::mpsc::sync_channel(1);
    let client = std::thread::spawn(move || {
        runtime().block_on(async move {
            let mut client = client::connect(
                Arc::new(client::Config::default()),
                address,
                ExpectedHostKey(host_key),
            )
            .await
            .unwrap();
            assert!(
                client
                    .authenticate_publickey(
                        "fixture-user",
                        PrivateKeyWithHashAlg::new(identity, None),
                    )
                    .await
                    .unwrap()
                    .success()
            );
            let channel = client
                .channel_open_direct_tcpip(
                    "127.0.0.1",
                    u32::from(target_address.port()),
                    "127.0.0.1",
                    44000,
                )
                .await
                .unwrap();
            ready_tx.send(()).unwrap();
            while !client.is_closed() {
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
            drop(channel);
        });
    });
    let (mut target_connection, _) = target_listener.accept().unwrap();
    target_connection.set_read_timeout(Some(DEADLINE)).unwrap();
    ready_rx.recv_timeout(DEADLINE).unwrap();
    let started = Instant::now();
    while probe.active() == 0 && started.elapsed() < DEADLINE {
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(probe.active(), 1);
    server.stop(DEADLINE).unwrap();
    assert_eq!(probe.active(), 0);
    let mut byte = [0_u8; 1];
    assert_eq!(
        std::io::Read::read(&mut target_connection, &mut byte).unwrap(),
        0
    );
    client.join().unwrap();
}

#[test]
fn shutdown_deadline_hands_a_never_finishing_child_to_the_process_reaper() {
    let server = HermeticSshServer::builder()
        .test_never_finish_child(Duration::from_millis(300))
        .start(DEADLINE)
        .expect("start SSH fixture with teardown seam");
    let probe = server.task_probe();
    let address = server.address();
    let host_key = server.host_key().clone();
    let identity = Arc::clone(server.agent().private_key());
    let (ready_tx, ready_rx) = std::sync::mpsc::sync_channel(1);
    let (closed_tx, closed_rx) = std::sync::mpsc::sync_channel(1);
    let client = std::thread::spawn(move || {
        runtime().block_on(async move {
            let mut client = client::connect(
                Arc::new(client::Config::default()),
                address,
                ExpectedHostKey(host_key),
            )
            .await
            .unwrap();
            assert!(
                client
                    .authenticate_publickey(
                        "fixture-user",
                        PrivateKeyWithHashAlg::new(identity, None),
                    )
                    .await
                    .unwrap()
                    .success()
            );
            let channel = client.channel_open_session().await.unwrap();
            ready_tx.send(()).unwrap();
            while !client.is_closed() {
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
            drop(channel);
            let _ = closed_tx.send(());
        });
    });
    ready_rx.recv_timeout(DEADLINE).unwrap();
    let active_deadline = Instant::now() + DEADLINE;
    while probe.active() == 0 && Instant::now() < active_deadline {
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(probe.active(), 1);

    let shutdown_deadline = Duration::from_millis(50);
    let started = Instant::now();
    assert!(matches!(
        server.stop(shutdown_deadline),
        Err(SshFixtureError::TimedOut { .. })
    ));
    assert!(started.elapsed() < Duration::from_millis(200));
    TcpListener::bind(address).expect("listener released at the absolute shutdown deadline");

    let reaped_deadline = Instant::now() + Duration::from_secs(2);
    while probe.active() != 0 && Instant::now() < reaped_deadline {
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(probe.active(), 0);
    closed_rx
        .recv_timeout(DEADLINE)
        .expect("client disconnected by bounded shutdown");
    client.join().unwrap();
}

#[test]
fn ssh_and_agent_teardown_share_one_public_shutdown_budget() {
    let server = HermeticSshServer::builder()
        .test_agent_teardown_delay(Duration::from_millis(300))
        .start(DEADLINE)
        .expect("start SSH fixture with slow agent teardown");
    let address = server.address();
    let public_budget = Duration::from_millis(50);
    let started = Instant::now();
    assert!(matches!(
        server.stop(public_budget),
        Err(SshFixtureError::TimedOut {
            operation: "agent teardown",
            ..
        })
    ));
    assert!(started.elapsed() < Duration::from_millis(150));
    TcpListener::bind(address).expect("SSH listener released within shared budget");
}

#[test]
fn ssh_worker_panic_before_ready_is_not_misclassified_as_timeout() {
    assert!(matches!(
        HermeticSshServer::builder()
            .test_worker_panic_before_ready()
            .start(DEADLINE),
        Err(SshFixtureError::WorkerPanicked)
    ));
}

#[test]
fn ssh_worker_panic_after_ready_is_not_misclassified_as_timeout() {
    let server = HermeticSshServer::builder()
        .test_worker_panic_after_ready()
        .start(DEADLINE)
        .expect("start SSH worker panic seam");
    assert!(matches!(
        server.stop(DEADLINE),
        Err(SshFixtureError::WorkerPanicked)
    ));
}

#[test]
fn host_keys_and_known_hosts_are_generated_per_fixture() {
    let first = HermeticSshServer::start(DEADLINE).expect("start first fixture");
    let second = HermeticSshServer::start(DEADLINE).expect("start second fixture");
    assert_ne!(first.host_key(), second.host_key());
    assert_ne!(first.known_hosts_path(), second.known_hosts_path());
    let known_hosts = std::fs::read_to_string(first.known_hosts_path()).unwrap();
    assert!(known_hosts.starts_with(&format!("[127.0.0.1]:{} ", first.address().port())));
    assert!(known_hosts.contains(&first.host_key().to_openssh().unwrap()));
    first.stop(DEADLINE).unwrap();
    second.stop(DEADLINE).unwrap();
}

#[test]
fn configures_openssh_with_only_fixture_host_and_identity_state() {
    let server = HermeticSshServer::start(DEADLINE).expect("start SSH fixture");
    let mut command = Command::new("ssh");
    server.configure_ssh_command(&mut command, "fixture-user");
    let args = command
        .get_args()
        .map(|value| value.to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    let port = server.address().port().to_string();
    assert!(
        args.windows(2)
            .any(|pair| pair[0] == "-p" && pair[1] == port)
    );
    assert!(args.iter().any(|arg| arg == "StrictHostKeyChecking=yes"));
    assert!(args.iter().any(|arg| {
        arg == &format!("UserKnownHostsFile={}", server.known_hosts_path().display())
    }));
    assert!(args.iter().any(|arg| arg == "GlobalKnownHostsFile=none"));
    assert_eq!(
        args.last().map(String::as_str),
        Some("fixture-user@127.0.0.1")
    );
    server.stop(DEADLINE).unwrap();
}

#[test]
fn openssh_configuration_ignores_ambient_home_and_config() {
    let server = HermeticSshServer::start(DEADLINE).expect("start SSH fixture");
    let ambient = tempfile::tempdir().unwrap();
    let ambient_ssh = ambient.path().join(".ssh");
    std::fs::create_dir(&ambient_ssh).unwrap();
    std::fs::write(
        ambient_ssh.join("config"),
        "Host *\n  ProxyCommand ambient-malicious-proxy\n",
    )
    .unwrap();
    let mut command = Command::new("ssh");
    command.env("HOME", ambient.path());
    command.env("USERPROFILE", ambient.path());
    server.configure_openssh_command(&mut command, OpenSshTool::Ssh);

    let args = command
        .get_args()
        .map(|value| value.to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    assert!(args.windows(2).any(|pair| {
        pair[0] == "-F" && pair[1] == server.isolated_ssh_config_path().to_string_lossy()
    }));
    assert_eq!(
        std::fs::read(server.isolated_ssh_config_path()).unwrap(),
        b""
    );
    let environment = command
        .get_envs()
        .map(|(key, value)| {
            (
                key.to_string_lossy().into_owned(),
                value.map(std::ffi::OsStr::to_os_string),
            )
        })
        .collect::<std::collections::HashMap<_, _>>();
    let isolated_home = server.temp_home().path().as_os_str().to_os_string();
    assert_eq!(
        environment.get("HOME").and_then(Option::as_ref),
        Some(&isolated_home)
    );
    assert_eq!(
        environment.get("USERPROFILE").and_then(Option::as_ref),
        Some(&isolated_home)
    );
    assert!(args.iter().any(|arg| arg == "IdentityAgent=none"));
    assert!(args.iter().any(|arg| arg == "IdentitiesOnly=yes"));
    server.stop(DEADLINE).unwrap();
}

#[test]
fn all_openssh_tools_default_to_noninteractive_publickey_only_authentication() {
    let server = HermeticSshServer::start(DEADLINE).expect("start SSH fixture");
    for (program, tool) in [
        ("ssh", OpenSshTool::Ssh),
        ("sftp", OpenSshTool::Sftp),
        ("scp", OpenSshTool::Scp),
    ] {
        let mut command = Command::new(program);
        for variable in ["SSH_ASKPASS", "SSH_ASKPASS_REQUIRE", "DISPLAY"] {
            command.env(variable, "ambient-interactive-helper");
        }
        server.configure_openssh_command(&mut command, tool);
        let args = command
            .get_args()
            .map(|value| value.to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        for expected in [
            "BatchMode=yes",
            "PreferredAuthentications=publickey",
            "PasswordAuthentication=no",
            "KbdInteractiveAuthentication=no",
            "NumberOfPasswordPrompts=0",
        ] {
            assert!(args.iter().any(|argument| argument == expected));
        }
        let environment = command
            .get_envs()
            .map(|(key, value)| (key.to_string_lossy().into_owned(), value.is_none()))
            .collect::<std::collections::HashMap<_, _>>();
        for variable in ["SSH_ASKPASS", "SSH_ASKPASS_REQUIRE", "DISPLAY"] {
            assert_eq!(environment.get(variable), Some(&true));
        }
    }
    server.stop(DEADLINE).unwrap();
}

#[test]
fn builder_accepts_password_and_additional_encrypted_ed25519_identity() {
    let identity = IdentityFixture::runtime_ed25519_encrypted("fixture-ed25519-passphrase")
        .expect("generate Ed25519 fixture identity");
    let private_key = Arc::new(
        russh::keys::load_secret_key(identity.identity_path(), identity.passphrase())
            .expect("decrypt Ed25519 fixture identity"),
    );
    let server = HermeticSshServer::builder()
        .password("password-user", "fixture-password")
        .authorize_public_key(identity.public_key().clone())
        .start(DEADLINE)
        .expect("start configured SSH fixture");
    let address = server.address();
    let host_key = server.host_key().clone();
    runtime().block_on(async move {
        let mut password_client = client::connect(
            Arc::new(client::Config::default()),
            address,
            ExpectedHostKey(host_key.clone()),
        )
        .await
        .unwrap();
        assert!(
            password_client
                .authenticate_password("password-user", "fixture-password")
                .await
                .unwrap()
                .success()
        );
        password_client
            .disconnect(russh::Disconnect::ByApplication, "password test", "")
            .await
            .unwrap();

        let mut key_client = client::connect(
            Arc::new(client::Config::default()),
            address,
            ExpectedHostKey(host_key),
        )
        .await
        .unwrap();
        assert!(
            key_client
                .authenticate_publickey(
                    "ed25519-user",
                    PrivateKeyWithHashAlg::new(private_key, None),
                )
                .await
                .unwrap()
                .success()
        );
    });
    server.stop(DEADLINE).unwrap();
}

#[test]
fn configured_commands_emit_stdout_stderr_status_and_signal() {
    let server = HermeticSshServer::builder()
        .command(
            "fixture-status",
            CommandResponse::status(b"standard-output", b"standard-error", 42),
        )
        .command(
            "fixture-signal",
            CommandResponse::signal(
                b"before-signal",
                b"signal-error",
                russh::Sig::TERM,
                false,
                "terminated by fixture",
            ),
        )
        .start(DEADLINE)
        .expect("start configured SSH fixture");
    let address = server.address();
    let host_key = server.host_key().clone();
    let identity = Arc::clone(server.agent().private_key());
    runtime().block_on(async move {
        let mut client = client::connect(
            Arc::new(client::Config::default()),
            address,
            ExpectedHostKey(host_key),
        )
        .await
        .unwrap();
        assert!(
            client
                .authenticate_publickey("fixture-user", PrivateKeyWithHashAlg::new(identity, None),)
                .await
                .unwrap()
                .success()
        );
        let mut status_channel = client.channel_open_session().await.unwrap();
        status_channel.exec(true, "fixture-status").await.unwrap();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let mut status = None;
        while let Some(message) = status_channel.wait().await {
            match message {
                ChannelMsg::Data { data } => stdout.extend_from_slice(&data),
                ChannelMsg::ExtendedData { data, ext: 1 } => stderr.extend_from_slice(&data),
                ChannelMsg::ExitStatus { exit_status } => status = Some(exit_status),
                ChannelMsg::Close => break,
                _ => {}
            }
        }
        assert_eq!(stdout, b"standard-output");
        assert_eq!(stderr, b"standard-error");
        assert_eq!(status, Some(42));

        let mut signal_channel = client.channel_open_session().await.unwrap();
        signal_channel.exec(true, "fixture-signal").await.unwrap();
        let mut signal = None;
        while let Some(message) = signal_channel.wait().await {
            match message {
                ChannelMsg::ExitSignal {
                    signal_name,
                    core_dumped,
                    error_message,
                    ..
                } => signal = Some((signal_name, core_dumped, error_message)),
                ChannelMsg::Close => break,
                _ => {}
            }
        }
        assert!(matches!(
            signal,
            Some((russh::Sig::TERM, false, ref message))
                if message == "terminated by fixture"
        ));
    });
    server.stop(DEADLINE).unwrap();
}

#[test]
fn real_ssh_exec_authenticates_only_the_injected_identity_and_records_events() {
    let server = HermeticSshServer::start(DEADLINE).expect("start SSH fixture");
    let address = server.address();
    let host_key = server.host_key().clone();
    let identity = Arc::clone(server.agent().private_key());
    let untrusted_identity = Arc::clone(
        crate::ssh::AgentFixture::new()
            .expect("generate untrusted identity")
            .private_key(),
    );

    let (output, status) = runtime().block_on(async move {
        let mut rejected_client = client::connect(
            Arc::new(client::Config::default()),
            address,
            ExpectedHostKey(host_key.clone()),
        )
        .await
        .expect("connect untrusted SSH client");
        assert!(
            !rejected_client
                .authenticate_publickey(
                    "fixture-user",
                    PrivateKeyWithHashAlg::new(untrusted_identity, None),
                )
                .await
                .expect("reject untrusted identity")
                .success()
        );
        rejected_client
            .disconnect(russh::Disconnect::ByApplication, "negative test", "")
            .await
            .expect("disconnect untrusted client");
        let mut client = client::connect(
            Arc::new(client::Config::default()),
            address,
            ExpectedHostKey(host_key),
        )
        .await
        .expect("connect real SSH client");
        assert!(
            client
                .authenticate_publickey("fixture-user", PrivateKeyWithHashAlg::new(identity, None),)
                .await
                .expect("authenticate")
                .success()
        );
        let mut channel = client.channel_open_session().await.expect("open session");
        channel
            .exec(true, b"rterm-test-marker lifecycle".as_slice())
            .await
            .expect("execute white-listed command");
        let mut output = Vec::new();
        let mut status = None;
        while let Some(message) = channel.wait().await {
            match message {
                ChannelMsg::Data { data } => output.extend_from_slice(&data),
                ChannelMsg::ExitStatus { exit_status } => status = Some(exit_status),
                ChannelMsg::Close => break,
                _ => {}
            }
        }
        client
            .disconnect(russh::Disconnect::ByApplication, "test complete", "")
            .await
            .expect("disconnect client");
        (output, status)
    });
    assert_eq!(output, b"lifecycle");
    assert_eq!(status, Some(0));
    assert!(server.events().iter().any(|event| matches!(
        event,
        SshEvent::Exec { command, accepted: true } if command == "rterm-test-marker lifecycle"
    )));
    server.stop(DEADLINE).unwrap();
}

#[test]
fn real_ssh_authenticates_through_the_in_memory_agent_protocol() {
    let server = HermeticSshServer::start(DEADLINE).expect("start SSH fixture");
    let address = server.address();
    let host_key = server.host_key().clone();
    runtime().block_on(async {
        let mut agent = server.agent().connect().expect("connect fixture agent");
        let identities = agent
            .request_identities()
            .await
            .expect("request identities");
        assert_eq!(identities.len(), 1);
        let mut client = client::connect(
            Arc::new(client::Config::default()),
            address,
            ExpectedHostKey(host_key),
        )
        .await
        .expect("connect SSH client");
        let authenticated = client
            .authenticate_publickey_with(
                "fixture-user",
                identities[0].public_key().into_owned(),
                None,
                &mut agent,
            )
            .await
            .expect("authenticate through fixture agent");
        assert!(authenticated.success());
        client
            .disconnect(russh::Disconnect::ByApplication, "agent test", "")
            .await
            .unwrap();
    });
    server.stop(DEADLINE).unwrap();
}

#[test]
fn pty_shell_resize_and_rejected_commands_are_observable() {
    let server = HermeticSshServer::start(DEADLINE).expect("start SSH fixture");
    let address = server.address();
    let host_key = server.host_key().clone();
    let identity = Arc::clone(server.agent().private_key());
    runtime().block_on(async move {
        let mut client = client::connect(
            Arc::new(client::Config::default()),
            address,
            ExpectedHostKey(host_key),
        )
        .await
        .unwrap();
        assert!(
            client
                .authenticate_publickey("fixture-user", PrivateKeyWithHashAlg::new(identity, None),)
                .await
                .unwrap()
                .success()
        );
        let mut channel = client.channel_open_session().await.unwrap();
        channel
            .request_pty(true, "xterm-256color", 80, 24, 0, 0, &[])
            .await
            .unwrap();
        channel.window_change(132, 43, 0, 0).await.unwrap();
        channel
            .exec(true, b"not-on-the-whitelist".as_slice())
            .await
            .unwrap();
        let mut status = None;
        while let Some(message) = channel.wait().await {
            match message {
                ChannelMsg::ExitStatus { exit_status } => status = Some(exit_status),
                ChannelMsg::Close => break,
                _ => {}
            }
        }
        assert_eq!(status, Some(126));
    });
    let events = server.events();
    assert!(events.iter().any(|event| matches!(
        event,
        SshEvent::Pty {
            columns: 80,
            rows: 24,
            ..
        }
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        SshEvent::Resize {
            columns: 132,
            rows: 43,
            ..
        }
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        SshEvent::Exec {
            accepted: false,
            ..
        }
    )));
    server.stop(DEADLINE).unwrap();
}

#[test]
fn real_shell_request_accepts_input_emits_output_and_closes() {
    let server = HermeticSshServer::start(DEADLINE).expect("start SSH fixture");
    let address = server.address();
    let host_key = server.host_key().clone();
    let identity = Arc::clone(server.agent().private_key());
    let (output, status) = runtime().block_on(async move {
        let mut client = client::connect(
            Arc::new(client::Config::default()),
            address,
            ExpectedHostKey(host_key),
        )
        .await
        .unwrap();
        assert!(
            client
                .authenticate_publickey("fixture-user", PrivateKeyWithHashAlg::new(identity, None),)
                .await
                .unwrap()
                .success()
        );
        let mut channel = client.channel_open_session().await.unwrap();
        channel
            .request_pty(true, "xterm", 80, 24, 0, 0, &[])
            .await
            .unwrap();
        channel.request_shell(true).await.unwrap();
        channel.window_change(100, 30, 0, 0).await.unwrap();
        channel
            .data_bytes(b"echo shell-protocol\n".to_vec())
            .await
            .unwrap();
        let mut output = Vec::new();
        let mut status = None;
        while let Some(message) = tokio::time::timeout(DEADLINE, channel.wait())
            .await
            .expect("shell response deadline")
        {
            match message {
                ChannelMsg::Data { data } => output.extend(data),
                ChannelMsg::ExitStatus { exit_status } => status = Some(exit_status),
                ChannelMsg::Close => break,
                _ => {}
            }
        }
        (output, status)
    });
    assert_eq!(output, b"shell-protocol\n");
    assert_eq!(status, Some(0));
    let events = server.events();
    assert!(events.iter().any(|event| matches!(event, SshEvent::Shell)));
    assert!(events.iter().any(|event| matches!(
        event,
        SshEvent::Resize {
            columns: 100,
            rows: 30,
            ..
        }
    )));
    server.stop(DEADLINE).unwrap();
}

#[test]
fn real_agent_forward_request_is_rejected() {
    let server = HermeticSshServer::start(DEADLINE).expect("start SSH fixture");
    let address = server.address();
    let host_key = server.host_key().clone();
    let identity = Arc::clone(server.agent().private_key());
    runtime().block_on(async move {
        let mut client = client::connect(
            Arc::new(client::Config::default()),
            address,
            ExpectedHostKey(host_key),
        )
        .await
        .unwrap();
        assert!(
            client
                .authenticate_publickey("fixture-user", PrivateKeyWithHashAlg::new(identity, None),)
                .await
                .unwrap()
                .success()
        );
        let mut channel = client.channel_open_session().await.unwrap();
        channel.agent_forward(true).await.unwrap();
        loop {
            let message = tokio::time::timeout(DEADLINE, channel.wait())
                .await
                .expect("agent-forward rejection deadline")
                .expect("agent-forward channel closed without rejection");
            match message {
                ChannelMsg::Failure => break,
                ChannelMsg::Success => panic!("agent forwarding unexpectedly accepted"),
                _ => {}
            }
        }
    });
    assert!(
        server
            .events()
            .iter()
            .any(|event| matches!(event, SshEvent::AgentForward { accepted: false }))
    );
    server.stop(DEADLINE).unwrap();
}

#[test]
fn direct_tcpip_rejects_non_loopback_targets() {
    let server = HermeticSshServer::start(DEADLINE).expect("start SSH fixture");
    let address = server.address();
    let host_key = server.host_key().clone();
    let identity = Arc::clone(server.agent().private_key());
    runtime().block_on(async move {
        let mut client = client::connect(
            Arc::new(client::Config::default()),
            address,
            ExpectedHostKey(host_key),
        )
        .await
        .unwrap();
        assert!(
            client
                .authenticate_publickey("fixture-user", PrivateKeyWithHashAlg::new(identity, None),)
                .await
                .unwrap()
                .success()
        );
        let rejected = client
            .channel_open_direct_tcpip("192.0.2.1", 22, "127.0.0.1", 40000)
            .await;
        assert!(rejected.is_err());
    });
    assert!(server.events().iter().any(|event| matches!(
        event,
        SshEvent::DirectTcpip { target, accepted: false, .. } if target == "192.0.2.1"
    )));
    server.stop(DEADLINE).unwrap();
}

#[test]
fn direct_tcpip_bridges_a_real_loopback_tcp_target() {
    let target = LoopbackEchoServer::start(DEADLINE).expect("start forwarding target");
    let server = HermeticSshServer::start(DEADLINE).expect("start SSH fixture");
    let address = server.address();
    let host_key = server.host_key().clone();
    let identity = Arc::clone(server.agent().private_key());
    let target_address = target.address();
    runtime().block_on(async move {
        let mut client = client::connect(
            Arc::new(client::Config::default()),
            address,
            ExpectedHostKey(host_key),
        )
        .await
        .unwrap();
        assert!(
            client
                .authenticate_publickey("fixture-user", PrivateKeyWithHashAlg::new(identity, None),)
                .await
                .unwrap()
                .success()
        );
        let channel = client
            .channel_open_direct_tcpip(
                "127.0.0.1",
                u32::from(target_address.port()),
                "127.0.0.1",
                40000,
            )
            .await
            .expect("open loopback direct-tcpip");
        let mut stream = channel.into_stream();
        stream.write_all(b"forwarded").await.unwrap();
        let mut echoed = [0_u8; 9];
        stream.read_exact(&mut echoed).await.unwrap();
        assert_eq!(&echoed, b"forwarded");
    });
    assert!(server.events().iter().any(|event| matches!(
        event,
        SshEvent::DirectTcpip { target, accepted: true, .. } if target == "127.0.0.1"
    )));
    server.stop(DEADLINE).unwrap();
    target.stop(DEADLINE).unwrap();
}

struct ForwardingClient {
    host_key: russh::keys::ssh_key::PublicKey,
}

impl client::Handler for ForwardingClient {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        server_public_key: &russh::keys::ssh_key::PublicKey,
    ) -> Result<bool, Self::Error> {
        Ok(server_public_key == &self.host_key)
    }

    async fn server_channel_open_forwarded_tcpip(
        &mut self,
        mut channel: russh::Channel<client::Msg>,
        _connected_address: &str,
        _connected_port: u32,
        _originator_address: &str,
        _originator_port: u32,
        _session: &mut client::Session,
    ) -> Result<(), Self::Error> {
        tokio::spawn(async move {
            while let Some(message) = channel.wait().await {
                match message {
                    ChannelMsg::Data { data } => {
                        let _ = channel.data_bytes(data).await;
                    }
                    ChannelMsg::Eof | ChannelMsg::Close => break,
                    _ => {}
                }
            }
        });
        Ok(())
    }
}

#[test]
fn remote_forward_binds_only_loopback_and_can_be_cancelled() {
    let server = HermeticSshServer::start(DEADLINE).expect("start SSH fixture");
    let address = server.address();
    let host_key = server.host_key().clone();
    let identity = Arc::clone(server.agent().private_key());
    runtime().block_on(async move {
        let mut client = client::connect(
            Arc::new(client::Config::default()),
            address,
            ForwardingClient { host_key },
        )
        .await
        .unwrap();
        assert!(
            client
                .authenticate_publickey("fixture-user", PrivateKeyWithHashAlg::new(identity, None),)
                .await
                .unwrap()
                .success()
        );
        let port = client.tcpip_forward("127.0.0.1", 0).await.unwrap();
        assert_ne!(port, 0);
        let mut forwarded =
            tokio::net::TcpStream::connect(("127.0.0.1", u16::try_from(port).unwrap()))
                .await
                .expect("connect remote-forward listener");
        forwarded.write_all(b"remote").await.unwrap();
        let mut echoed = [0_u8; 6];
        tokio::time::timeout(Duration::from_secs(1), forwarded.read_exact(&mut echoed))
            .await
            .expect("forwarded echo deadline")
            .unwrap();
        assert_eq!(&echoed, b"remote");
        drop(forwarded);
        client
            .cancel_tcpip_forward("127.0.0.1", port)
            .await
            .unwrap();
        assert!(TcpListener::bind(("127.0.0.1", u16::try_from(port).unwrap())).is_ok());
    });
    let events = server.events();
    assert!(
        events
            .iter()
            .any(|event| matches!(event, SshEvent::RemoteForward { accepted: true, .. }))
    );
    assert!(events.iter().any(|event| matches!(
        event,
        SshEvent::RemoteForwardCancelled { accepted: true, .. }
    )));
    server.stop(DEADLINE).unwrap();
}

#[test]
fn remote_forward_rejects_non_loopback_request() {
    let server = HermeticSshServer::start(DEADLINE).expect("start SSH fixture");
    let address = server.address();
    let host_key = server.host_key().clone();
    let identity = Arc::clone(server.agent().private_key());
    runtime().block_on(async move {
        let mut client = client::connect(
            Arc::new(client::Config::default()),
            address,
            ForwardingClient { host_key },
        )
        .await
        .unwrap();
        assert!(
            client
                .authenticate_publickey("fixture-user", PrivateKeyWithHashAlg::new(identity, None),)
                .await
                .unwrap()
                .success()
        );
        assert!(matches!(
            client.tcpip_forward("0.0.0.0", 0).await,
            Err(russh::Error::RequestDenied)
        ));
    });
    assert!(server.events().iter().any(|event| matches!(
        event,
        SshEvent::RemoteForward { address, accepted: false, .. } if address == "0.0.0.0"
    )));
    server.stop(DEADLINE).unwrap();
}

fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
}
