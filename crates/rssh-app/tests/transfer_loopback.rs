use std::{path::Path, process::Command, time::Duration};

use rssh_test_support::{
    ChildGuard, ChildOutput, OpenSshClientTool, TempHome, probe_openssh_tools_from_environment,
    ssh::{HermeticSshServer, OpenSshTool},
};
use sha2::{Digest as _, Sha256};

const DEADLINE: Duration = Duration::from_secs(5);
const PROCESS_DEADLINE: Duration = Duration::from_secs(15);

fn transfer_tools_available() -> bool {
    probe_openssh_tools_from_environment(&[OpenSshClientTool::Sftp, OpenSshClientTool::Scp])
        .expect("required OpenSSH transfer tool probe")
}

fn run(command: Command) -> ChildOutput {
    ChildGuard::spawn(command, PROCESS_DEADLINE)
        .expect("spawn deadline-bound transfer process")
        .wait()
        .expect("wait for deadline-bound transfer process")
}

#[cfg(windows)]
fn prepare_identity_for_openssh(server: &HermeticSshServer) {
    let principal = std::env::var("USERNAME").expect("Windows username");
    let mut command = Command::new("icacls.exe");
    command
        .arg(server.agent().identity_path())
        .arg("/inheritance:r")
        .arg("/grant:r")
        .arg(format!("{principal}:(R)"));
    let output = run(command);
    assert!(output.status.success());
}

#[cfg(not(windows))]
fn prepare_identity_for_openssh(_server: &HermeticSshServer) {}

fn system_transfer_command(server: &HermeticSshServer, tool: OpenSshTool) -> Command {
    let program = match tool {
        OpenSshTool::Sftp => "sftp",
        OpenSshTool::Scp => "scp",
        OpenSshTool::Ssh => panic!("transfer command requires sftp or scp"),
    };
    let mut command = Command::new(program);
    server.configure_openssh_command(&mut command, tool);
    command
}

fn portable_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn sha256(bytes: &[u8]) -> Vec<u8> {
    Sha256::digest(bytes).to_vec()
}

#[test]
fn system_sftp_upload_and_download_preserve_sha256_content() {
    if !transfer_tools_available() {
        return;
    }
    let server = HermeticSshServer::start(DEADLINE).expect("start SSH fixture");
    prepare_identity_for_openssh(&server);
    let client = TempHome::new().expect("create isolated transfer home");
    let source = client.path().join("sftp-source.bin");
    let downloaded = client.path().join("sftp-downloaded.bin");
    let payload = b"sftp-content\0with-binary\xff";
    std::fs::write(&source, payload).expect("write SFTP source");
    let batch = client.path().join("sftp.batch");
    std::fs::write(
        &batch,
        format!(
            "put {} sftp-file.bin\nget sftp-file.bin {}\n",
            portable_path(&source),
            portable_path(&downloaded)
        ),
    )
    .expect("write SFTP batch");

    let mut command = system_transfer_command(&server, OpenSshTool::Sftp);
    command.arg("-b").arg(&batch).arg("fixture-user@127.0.0.1");
    let output = run(command);
    assert!(
        output.status.success(),
        "SFTP failed: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );
    let remote =
        std::fs::read(server.sftp().path().join("sftp-file.bin")).expect("read SFTP remote file");
    let downloaded = std::fs::read(downloaded).expect("read SFTP download");
    assert_eq!(sha256(&remote), sha256(payload));
    assert_eq!(sha256(&downloaded), sha256(payload));
    server.stop(DEADLINE).expect("stop SSH fixture");
}

#[test]
fn system_scp_upload_download_and_recursive_transfer_preserve_sha256_content() {
    if !transfer_tools_available() {
        return;
    }
    let server = HermeticSshServer::start(DEADLINE).expect("start SSH fixture");
    prepare_identity_for_openssh(&server);
    let client = TempHome::new().expect("create isolated transfer home");
    let payload = b"scp-single-content\0\xfe";
    let source = client.path().join("scp-source.bin");
    std::fs::write(&source, payload).expect("write SCP source");

    let mut upload = system_transfer_command(&server, OpenSshTool::Scp);
    upload
        .arg("-O")
        .arg(&source)
        .arg("fixture-user@127.0.0.1:scp-file.bin");
    let output = run(upload);
    assert!(
        output.status.success(),
        "SCP upload: {:?}; server events: {:?}",
        String::from_utf8_lossy(&output.stderr),
        server.events()
    );

    let downloaded = client.path().join("scp-downloaded.bin");
    let mut download = system_transfer_command(&server, OpenSshTool::Scp);
    download
        .arg("-O")
        .arg("fixture-user@127.0.0.1:scp-file.bin")
        .arg(&downloaded);
    let output = run(download);
    assert!(
        output.status.success(),
        "SCP download: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        sha256(&std::fs::read(&downloaded).expect("read SCP download")),
        sha256(payload)
    );

    let tree = client.path().join("tree");
    std::fs::create_dir_all(tree.join("nested")).expect("create recursive source");
    std::fs::write(tree.join("root.txt"), b"recursive-root").expect("write root file");
    std::fs::write(tree.join("nested/data.bin"), b"recursive-nested\0\xfd")
        .expect("write nested file");
    std::fs::create_dir(server.sftp().path().join("recursive")).expect("create remote directory");
    let mut recursive_upload = system_transfer_command(&server, OpenSshTool::Scp);
    recursive_upload
        .args(["-O", "-r"])
        .arg(&tree)
        .arg("fixture-user@127.0.0.1:recursive");
    let output = run(recursive_upload);
    assert!(
        output.status.success(),
        "recursive upload: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );

    let recursive_download = client.path().join("recursive-download");
    std::fs::create_dir(&recursive_download).expect("create recursive download directory");
    let mut download_tree = system_transfer_command(&server, OpenSshTool::Scp);
    download_tree
        .args(["-O", "-r"])
        .arg("fixture-user@127.0.0.1:recursive/tree")
        .arg(&recursive_download);
    let output = run(download_tree);
    assert!(
        output.status.success(),
        "recursive download: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );
    for relative in ["root.txt", "nested/data.bin"] {
        let original = std::fs::read(tree.join(relative)).expect("read recursive source");
        let copied = std::fs::read(recursive_download.join("tree").join(relative))
            .expect("read recursive download");
        assert_eq!(sha256(&copied), sha256(&original));
    }

    server.stop(DEADLINE).expect("stop SSH fixture");
}

#[test]
fn rssh_app_sftp_and_scp_entrypoints_use_only_isolated_fixture_state() {
    if !transfer_tools_available() {
        return;
    }
    let server = HermeticSshServer::start(DEADLINE).expect("start SSH fixture");
    prepare_identity_for_openssh(&server);
    let client = TempHome::new().expect("create isolated transfer home");
    let source = client.path().join("app-transfer.bin");
    let payload = b"rssh-app-transfer-content";
    std::fs::write(&source, payload).expect("write app transfer source");

    app_scp_roundtrip(&server, &client, &source, payload);
    app_sftp_roundtrip(&server, &client, payload);
    app_recursive_scp_roundtrip(&server, &client);

    for remote in ["app-scp.bin", "app-sftp.bin"] {
        let copied = std::fs::read(server.sftp().path().join(remote)).expect("read app transfer");
        assert_eq!(sha256(&copied), sha256(payload));
    }
    server.stop(DEADLINE).expect("stop SSH fixture");
}

fn app_scp_roundtrip(server: &HermeticSshServer, client: &TempHome, source: &Path, payload: &[u8]) {
    let mut scp = app_command(server);
    scp.arg("scp")
        .args(common_app_args(server))
        .arg("-O")
        .arg(source)
        .arg("fixture-user@127.0.0.1:app-scp.bin");
    let output = run(scp);
    assert!(
        output.status.success(),
        "app SCP: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );
    let scp_download = client.path().join("app-scp-downloaded.bin");
    let mut scp_get = app_command(server);
    scp_get
        .arg("scp")
        .args(common_app_args(server))
        .arg("-O")
        .arg("fixture-user@127.0.0.1:app-scp.bin")
        .arg(&scp_download);
    let output = run(scp_get);
    assert!(
        output.status.success(),
        "app SCP download: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        sha256(&std::fs::read(&scp_download).expect("read app SCP download")),
        sha256(payload)
    );
}

fn app_sftp_roundtrip(server: &HermeticSshServer, client: &TempHome, payload: &[u8]) {
    let sftp_source = client.path().join("app-sftp-source.bin");
    std::fs::write(&sftp_source, payload).expect("write app SFTP source");
    let batch = client.path().join("app-sftp.batch");
    std::fs::write(
        &batch,
        format!(
            "put {} app-sftp.bin\nget app-sftp.bin {}\n",
            portable_path(&sftp_source),
            portable_path(&client.path().join("app-sftp-downloaded.bin"))
        ),
    )
    .expect("write app SFTP batch");
    let mut sftp = app_command(server);
    sftp.arg("sftp")
        .args(common_app_args(server))
        .arg("-b")
        .arg(&batch)
        .arg("fixture-user@127.0.0.1");
    let output = run(sftp);
    assert!(
        output.status.success(),
        "app SFTP: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        sha256(
            &std::fs::read(client.path().join("app-sftp-downloaded.bin"))
                .expect("read app SFTP download")
        ),
        sha256(payload)
    );
}

fn app_recursive_scp_roundtrip(server: &HermeticSshServer, client: &TempHome) {
    let tree = client.path().join("app-tree");
    std::fs::create_dir_all(tree.join("nested")).expect("create app recursive source");
    std::fs::write(tree.join("root.txt"), b"app-recursive-root").expect("write app root");
    std::fs::write(tree.join("nested/data.bin"), b"app-recursive-nested\0\xfc")
        .expect("write app nested");
    std::fs::create_dir(server.sftp().path().join("app-recursive"))
        .expect("create app remote recursive root");
    let mut recursive_put = app_command(server);
    recursive_put
        .arg("scp")
        .args(common_app_args(server))
        .args(["-O", "-r"])
        .arg(&tree)
        .arg("fixture-user@127.0.0.1:app-recursive");
    let output = run(recursive_put);
    assert!(
        output.status.success(),
        "app recursive SCP upload: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );
    let recursive_download = client.path().join("app-recursive-download");
    std::fs::create_dir(&recursive_download).expect("create app recursive download");
    let mut recursive_get = app_command(server);
    recursive_get
        .arg("scp")
        .args(common_app_args(server))
        .args(["-O", "-r"])
        .arg("fixture-user@127.0.0.1:app-recursive/app-tree")
        .arg(&recursive_download);
    let output = run(recursive_get);
    assert!(
        output.status.success(),
        "app recursive SCP download: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );
    for relative in ["root.txt", "nested/data.bin"] {
        let original = std::fs::read(tree.join(relative)).expect("read app recursive source");
        let copied = std::fs::read(recursive_download.join("app-tree").join(relative))
            .expect("read app recursive download");
        assert_eq!(sha256(&copied), sha256(&original));
    }
}

fn app_command(server: &HermeticSshServer) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_rterm"));
    server.temp_home().apply_to(&mut command);
    command
        .env_remove("SSH_AUTH_SOCK")
        .env_remove("SSH_ASKPASS")
        .env_remove("SSH_ASKPASS_REQUIRE")
        .env_remove("DISPLAY");
    command
}

fn common_app_args(server: &HermeticSshServer) -> Vec<String> {
    vec![
        "-F".to_owned(),
        server.isolated_ssh_config_path().display().to_string(),
        "-P".to_owned(),
        server.address().port().to_string(),
        "-i".to_owned(),
        server.agent().identity_path().display().to_string(),
        "-o".to_owned(),
        "StrictHostKeyChecking=yes".to_owned(),
        "-o".to_owned(),
        format!("UserKnownHostsFile={}", server.known_hosts_path().display()),
        "-o".to_owned(),
        "GlobalKnownHostsFile=none".to_owned(),
        "-o".to_owned(),
        "IdentityAgent=none".to_owned(),
        "-o".to_owned(),
        "IdentitiesOnly=yes".to_owned(),
        "-o".to_owned(),
        "BatchMode=yes".to_owned(),
    ]
}
