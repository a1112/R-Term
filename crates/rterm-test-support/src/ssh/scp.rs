use std::{
    collections::VecDeque,
    io,
    path::{Component, Path, PathBuf},
    time::{Duration, UNIX_EPOCH},
};

use futures::future::BoxFuture;
use russh::{Channel, ChannelMsg, server::Msg};

const SCP_IO_DEADLINE: Duration = Duration::from_secs(2);
const MAX_CONTROL_LINE: usize = 8 * 1024;
const MAX_FILE_SIZE: u64 = 16 * 1024 * 1024;
const MAX_RECURSION_DEPTH: usize = 32;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ScpDirection {
    Sink,
    Source,
}

#[derive(Clone, Debug)]
pub(super) struct ScpRequest {
    pub direction: ScpDirection,
    pub recursive: bool,
    pub preserve_times: bool,
    pub target_is_directory: bool,
    pub target: PathBuf,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct ScpProtocolError;

pub(super) fn parse_scp_request(command: &str) -> Option<Result<ScpRequest, ScpProtocolError>> {
    let words = match parse_shell_words(command) {
        Ok(words) => words,
        Err(error) => {
            return command
                .trim_start()
                .starts_with("scp")
                .then_some(Err(error));
        }
    };
    if words.first().map(String::as_str) != Some("scp") {
        return None;
    }
    let mut direction = None;
    let mut recursive = false;
    let mut preserve_times = false;
    let mut target_is_directory = false;
    let mut target = None;
    for word in &words[1..] {
        if let Some(flags) = word.strip_prefix('-')
            && !flags.is_empty()
        {
            for flag in flags.chars() {
                match flag {
                    't' => {
                        if set_direction(&mut direction, ScpDirection::Sink).is_err() {
                            return Some(Err(ScpProtocolError));
                        }
                    }
                    'f' => {
                        if set_direction(&mut direction, ScpDirection::Source).is_err() {
                            return Some(Err(ScpProtocolError));
                        }
                    }
                    'r' => recursive = true,
                    'p' => preserve_times = true,
                    'd' => target_is_directory = true,
                    _ => return Some(Err(ScpProtocolError)),
                }
            }
        } else if target.replace(PathBuf::from(word)).is_some() {
            return Some(Err(ScpProtocolError));
        }
    }
    let Some(direction) = direction else {
        return Some(Err(ScpProtocolError));
    };
    let Some(target) = target else {
        return Some(Err(ScpProtocolError));
    };
    Some(Ok(ScpRequest {
        direction,
        recursive,
        preserve_times,
        target_is_directory,
        target,
    }))
}

fn set_direction(
    direction: &mut Option<ScpDirection>,
    value: ScpDirection,
) -> Result<(), ScpProtocolError> {
    if direction.replace(value).is_some() {
        return Err(ScpProtocolError);
    }
    Ok(())
}

fn parse_shell_words(command: &str) -> Result<Vec<String>, ScpProtocolError> {
    if command
        .chars()
        .any(|character| character.is_control() || matches!(character, '$' | '`' | '\0'))
    {
        return Err(ScpProtocolError);
    }
    let mut words = Vec::new();
    let mut word = String::new();
    let mut quote = None;
    let mut escaped = false;
    for character in command.chars() {
        if escaped {
            word.push(character);
            escaped = false;
        } else if character == '\\' && quote != Some('\'') {
            escaped = true;
        } else if matches!(character, '\'' | '"') {
            if quote == Some(character) {
                quote = None;
            } else if quote.is_none() {
                quote = Some(character);
            } else {
                word.push(character);
            }
        } else if character.is_ascii_whitespace() && quote.is_none() {
            if !word.is_empty() {
                words.push(std::mem::take(&mut word));
            }
        } else {
            word.push(character);
        }
    }
    if escaped || quote.is_some() {
        return Err(ScpProtocolError);
    }
    if !word.is_empty() {
        words.push(word);
    }
    Ok(words)
}

pub(super) async fn serve_scp(
    channel: Channel<Msg>,
    root: PathBuf,
    request: Result<ScpRequest, ScpProtocolError>,
) {
    let mut wire = ScpWire {
        channel,
        buffered: VecDeque::new(),
        end_of_input: false,
        incomplete_control: false,
    };
    let result = match request {
        Ok(request) => match request.direction {
            ScpDirection::Sink => receive_sink(&mut wire, &root, &request).await,
            ScpDirection::Source => send_source(&mut wire, &root, &request).await,
        },
        Err(error) => Err(error),
    };
    if result.is_err() {
        let _ = wire.send_fatal().await;
    }
    let _ = wire.channel.exit_status(u32::from(result.is_err())).await;
    let _ = wire.channel.eof().await;
    let _ = wire.channel.close().await;
}

struct ScpWire {
    channel: Channel<Msg>,
    buffered: VecDeque<u8>,
    end_of_input: bool,
    incomplete_control: bool,
}

impl ScpWire {
    async fn send(&self, bytes: impl AsRef<[u8]>) -> Result<(), ScpProtocolError> {
        self.channel
            .data_bytes(bytes.as_ref().to_vec())
            .await
            .map_err(|_| ScpProtocolError)
    }

    async fn send_ack(&self) -> Result<(), ScpProtocolError> {
        self.send([0]).await
    }

    async fn send_fatal(&self) -> Result<(), ScpProtocolError> {
        self.send(b"\x02SCP request rejected by hermetic fixture\n")
            .await
    }

    async fn read_byte(&mut self) -> Result<u8, ScpProtocolError> {
        loop {
            if let Some(byte) = self.buffered.pop_front() {
                return Ok(byte);
            }
            let message = tokio::time::timeout(SCP_IO_DEADLINE, self.channel.wait())
                .await
                .map_err(|_| ScpProtocolError)?
                .ok_or(ScpProtocolError)?;
            match message {
                ChannelMsg::Data { data } => self.buffered.extend(data),
                ChannelMsg::Eof | ChannelMsg::Close => {
                    self.end_of_input = true;
                    return Err(ScpProtocolError);
                }
                _ => {}
            }
        }
    }

    async fn read_line(&mut self) -> Result<Vec<u8>, ScpProtocolError> {
        let mut line = Vec::new();
        while line.len() < MAX_CONTROL_LINE {
            let byte = match self.read_byte().await {
                Ok(byte) => byte,
                Err(error) => {
                    self.incomplete_control = !line.is_empty();
                    return Err(error);
                }
            };
            line.push(byte);
            if byte == b'\n' {
                self.incomplete_control = false;
                return Ok(line);
            }
        }
        Err(ScpProtocolError)
    }

    async fn read_exact(&mut self, length: usize) -> Result<Vec<u8>, ScpProtocolError> {
        let mut data = Vec::with_capacity(length);
        while data.len() < length {
            data.push(self.read_byte().await?);
        }
        Ok(data)
    }

    async fn expect_ack(&mut self) -> Result<(), ScpProtocolError> {
        if self.read_byte().await? == 0 {
            Ok(())
        } else {
            Err(ScpProtocolError)
        }
    }
}

async fn receive_sink(
    wire: &mut ScpWire,
    root: &Path,
    request: &ScpRequest,
) -> Result<(), ScpProtocolError> {
    let target = resolve_sink_target(root, request)?;
    let mut directories = vec![target.directory];
    let mut forced_file = target.forced_file;
    wire.send_ack().await?;
    loop {
        let line = match wire.read_line().await {
            Ok(line) => line,
            Err(_)
                if directories.len() == 1
                    && wire.end_of_input
                    && !wire.incomplete_control
                    && forced_file.is_none() =>
            {
                return Ok(());
            }
            Err(error) => return Err(error),
        };
        match line.first().copied() {
            Some(b'T') => {
                parse_times(&line)?;
                wire.send_ack().await?;
            }
            Some(b'D') => {
                if forced_file.is_some()
                    || !request.recursive
                    || directories.len() >= MAX_RECURSION_DEPTH
                {
                    return Err(ScpProtocolError);
                }
                let (mode, size, name) = parse_control(&line, b'D')?;
                if size != 0 {
                    return Err(ScpProtocolError);
                }
                let relative = child_relative(root, directories.last().unwrap(), &name)?;
                let path = resolve_for_create(root, &relative)?;
                std::fs::create_dir(&path).map_err(|_| ScpProtocolError)?;
                apply_permissions(&path, mode)?;
                directories.push(path.canonicalize().map_err(|_| ScpProtocolError)?);
                wire.send_ack().await?;
            }
            Some(b'E') if line == b"E\n" => {
                if directories.len() == 1 {
                    return Err(ScpProtocolError);
                }
                directories.pop();
                wire.send_ack().await?;
            }
            Some(b'C') => {
                let (mode, size, name) = parse_control(&line, b'C')?;
                if size > MAX_FILE_SIZE {
                    return Err(ScpProtocolError);
                }
                let name = portable_control_name(&name)?;
                let path = if let Some(path) = forced_file.take() {
                    path
                } else {
                    let relative = child_relative(root, directories.last().unwrap(), name)?;
                    resolve_for_create(root, &relative)?
                };
                wire.send_ack().await?;
                let data = wire
                    .read_exact(usize::try_from(size).map_err(|_| ScpProtocolError)?)
                    .await?;
                wire.expect_ack().await?;
                std::fs::write(&path, data).map_err(|_| ScpProtocolError)?;
                apply_permissions(&path, mode)?;
                wire.send_ack().await?;
            }
            _ => return Err(ScpProtocolError),
        }
    }
}

struct SinkTarget {
    directory: PathBuf,
    forced_file: Option<PathBuf>,
}

fn resolve_sink_target(root: &Path, request: &ScpRequest) -> Result<SinkTarget, ScpProtocolError> {
    validate_relative(&request.target)?;
    let joined = root.join(&request.target);
    match std::fs::symlink_metadata(&joined) {
        Ok(metadata) => {
            if metadata_is_redirect(&metadata) {
                return Err(ScpProtocolError);
            }
            let canonical = joined.canonicalize().map_err(|_| ScpProtocolError)?;
            if !canonical.starts_with(root) {
                return Err(ScpProtocolError);
            }
            if canonical.is_dir() {
                Ok(SinkTarget {
                    directory: canonical,
                    forced_file: None,
                })
            } else if canonical.is_file() && !request.target_is_directory {
                Ok(SinkTarget {
                    directory: canonical.parent().ok_or(ScpProtocolError)?.to_path_buf(),
                    forced_file: Some(canonical),
                })
            } else {
                Err(ScpProtocolError)
            }
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            let created = resolve_for_create(root, &request.target)?;
            if request.target_is_directory {
                std::fs::create_dir(&created).map_err(|_| ScpProtocolError)?;
                Ok(SinkTarget {
                    directory: created.canonicalize().map_err(|_| ScpProtocolError)?,
                    forced_file: None,
                })
            } else {
                Ok(SinkTarget {
                    directory: created.parent().ok_or(ScpProtocolError)?.to_path_buf(),
                    forced_file: Some(created),
                })
            }
        }
        Err(_) => Err(ScpProtocolError),
    }
}

async fn send_source(
    wire: &mut ScpWire,
    root: &Path,
    request: &ScpRequest,
) -> Result<(), ScpProtocolError> {
    validate_relative(&request.target)?;
    let path = root
        .join(&request.target)
        .canonicalize()
        .map_err(|_| ScpProtocolError)?;
    if !path.starts_with(root)
        || std::fs::symlink_metadata(&path)
            .map_err(|_| ScpProtocolError)?
            .file_type()
            .is_symlink()
    {
        return Err(ScpProtocolError);
    }
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or(ScpProtocolError)?
        .to_owned();
    validate_name(&name)?;
    wire.expect_ack().await?;
    send_entry(wire, root, &path, &name, request, 0).await
}

fn send_entry<'a>(
    wire: &'a mut ScpWire,
    root: &'a Path,
    path: &'a Path,
    name: &'a str,
    request: &'a ScpRequest,
    depth: usize,
) -> BoxFuture<'a, Result<(), ScpProtocolError>> {
    Box::pin(async move {
        if depth >= MAX_RECURSION_DEPTH || !path.starts_with(root) {
            return Err(ScpProtocolError);
        }
        let metadata = std::fs::symlink_metadata(path).map_err(|_| ScpProtocolError)?;
        if metadata.file_type().is_symlink() {
            return Err(ScpProtocolError);
        }
        if request.preserve_times {
            send_times(wire, &metadata).await?;
        }
        if metadata.is_dir() {
            if !request.recursive {
                return Err(ScpProtocolError);
            }
            wire.send(format!("D{:04o} 0 {name}\n", permissions_mode(&metadata)))
                .await?;
            wire.expect_ack().await?;
            let mut entries = std::fs::read_dir(path)
                .map_err(|_| ScpProtocolError)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| ScpProtocolError)?;
            entries.sort_by_key(std::fs::DirEntry::file_name);
            for entry in entries {
                let child_name = entry
                    .file_name()
                    .into_string()
                    .map_err(|_| ScpProtocolError)?;
                validate_name(&child_name)?;
                let child = entry.path().canonicalize().map_err(|_| ScpProtocolError)?;
                if !child.starts_with(root) {
                    return Err(ScpProtocolError);
                }
                send_entry(wire, root, &child, &child_name, request, depth + 1).await?;
            }
            wire.send(b"E\n").await?;
            wire.expect_ack().await
        } else if metadata.is_file() {
            if metadata.len() > MAX_FILE_SIZE {
                return Err(ScpProtocolError);
            }
            wire.send(format!(
                "C{:04o} {} {name}\n",
                permissions_mode(&metadata),
                metadata.len()
            ))
            .await?;
            wire.expect_ack().await?;
            let data = std::fs::read(path).map_err(|_| ScpProtocolError)?;
            wire.send(data).await?;
            wire.send_ack().await?;
            wire.expect_ack().await
        } else {
            Err(ScpProtocolError)
        }
    })
}

async fn send_times(
    wire: &mut ScpWire,
    metadata: &std::fs::Metadata,
) -> Result<(), ScpProtocolError> {
    let modified = metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |duration| duration.as_secs());
    let accessed = metadata
        .accessed()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map_or(modified, |duration| duration.as_secs());
    wire.send(format!("T{modified} 0 {accessed} 0\n")).await?;
    wire.expect_ack().await
}

fn parse_times(line: &[u8]) -> Result<(), ScpProtocolError> {
    let text = std::str::from_utf8(line).map_err(|_| ScpProtocolError)?;
    let fields = text[1..text.len().saturating_sub(1)]
        .split_ascii_whitespace()
        .collect::<Vec<_>>();
    if fields.len() != 4 || fields.iter().any(|field| field.parse::<u64>().is_err()) {
        return Err(ScpProtocolError);
    }
    Ok(())
}

fn parse_control(line: &[u8], kind: u8) -> Result<(u32, u64, String), ScpProtocolError> {
    if line.first().copied() != Some(kind) || line.last().copied() != Some(b'\n') {
        return Err(ScpProtocolError);
    }
    let text = std::str::from_utf8(&line[1..line.len() - 1]).map_err(|_| ScpProtocolError)?;
    let mut fields = text.splitn(3, ' ');
    let mode = u32::from_str_radix(fields.next().ok_or(ScpProtocolError)?, 8)
        .map_err(|_| ScpProtocolError)?;
    let size = fields
        .next()
        .ok_or(ScpProtocolError)?
        .parse::<u64>()
        .map_err(|_| ScpProtocolError)?;
    let name = fields.next().ok_or(ScpProtocolError)?.to_owned();
    Ok((mode & 0o777, size, name))
}

fn validate_relative(path: &Path) -> Result<(), ScpProtocolError> {
    if path.as_os_str().is_empty()
        || path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(ScpProtocolError);
    }
    Ok(())
}

fn validate_name(name: &str) -> Result<(), ScpProtocolError> {
    if name.is_empty()
        || matches!(name, "." | "..")
        || name.contains(['/', '\\'])
        || name.chars().any(char::is_control)
    {
        return Err(ScpProtocolError);
    }
    Ok(())
}

fn child_relative(root: &Path, directory: &Path, name: &str) -> Result<PathBuf, ScpProtocolError> {
    let name = portable_control_name(name)?;
    let relative = directory.strip_prefix(root).map_err(|_| ScpProtocolError)?;
    Ok(relative.join(name))
}

fn portable_control_name(name: &str) -> Result<&str, ScpProtocolError> {
    if validate_name(name).is_ok() {
        return Ok(name);
    }
    let bytes = name.as_bytes();
    if bytes.len() < 4
        || !bytes[0].is_ascii_alphabetic()
        || bytes[1] != b':'
        || !matches!(bytes[2], b'\\' | b'/')
    {
        return Err(ScpProtocolError);
    }
    let leaf = name.rsplit(['\\', '/']).next().ok_or(ScpProtocolError)?;
    validate_name(leaf)?;
    Ok(leaf)
}

fn resolve_for_create(root: &Path, relative: &Path) -> Result<PathBuf, ScpProtocolError> {
    // This check is intentionally best-effort: stable Rust has no portable no-follow
    // create/open API, so fixture roots must not be concurrently mutated between path
    // validation and the subsequent filesystem operation.
    validate_relative(relative)?;
    let joined = root.join(relative);
    let parent = joined.parent().ok_or(ScpProtocolError)?;
    let canonical_parent = parent.canonicalize().map_err(|_| ScpProtocolError)?;
    if !canonical_parent.starts_with(root) {
        return Err(ScpProtocolError);
    }
    let name = joined.file_name().ok_or(ScpProtocolError)?;
    let candidate = canonical_parent.join(name);
    match std::fs::symlink_metadata(&candidate) {
        Ok(metadata) => {
            if metadata_is_redirect(&metadata) {
                return Err(ScpProtocolError);
            }
            let canonical = candidate.canonicalize().map_err(|_| ScpProtocolError)?;
            if !canonical.starts_with(root) {
                return Err(ScpProtocolError);
            }
            Ok(canonical)
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(candidate),
        Err(_) => Err(ScpProtocolError),
    }
}

fn metadata_is_redirect(metadata: &std::fs::Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt as _;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
        metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
    }
    #[cfg(not(windows))]
    {
        false
    }
}

#[cfg(unix)]
fn permissions_mode(metadata: &std::fs::Metadata) -> u32 {
    use std::os::unix::fs::PermissionsExt as _;
    metadata.permissions().mode() & 0o777
}

#[cfg(not(unix))]
fn permissions_mode(_metadata: &std::fs::Metadata) -> u32 {
    0o644
}

#[cfg(unix)]
fn apply_permissions(path: &Path, mode: u32) -> Result<(), ScpProtocolError> {
    use std::os::unix::fs::PermissionsExt as _;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode & 0o777))
        .map_err(|_| ScpProtocolError)
}

#[cfg(not(unix))]
fn apply_permissions(path: &Path, _mode: u32) -> Result<(), ScpProtocolError> {
    std::fs::metadata(path)
        .map(|_| ())
        .map_err(|_| ScpProtocolError)
}

#[cfg(test)]
mod tests {
    use std::{collections::VecDeque, path::Path, sync::Arc, time::Duration};

    use russh::{
        Channel, ChannelMsg, client,
        keys::{PrivateKeyWithHashAlg, ssh_key},
    };

    use crate::ssh::{
        HermeticSshServer,
        redirect::{DanglingLeafRedirect, DirectoryRedirect},
    };

    const DEADLINE: Duration = Duration::from_secs(3);

    struct ExpectedHostKey(ssh_key::PublicKey);

    impl client::Handler for ExpectedHostKey {
        type Error = russh::Error;

        async fn check_server_key(
            &mut self,
            server_public_key: &ssh_key::PublicKey,
        ) -> Result<bool, Self::Error> {
            Ok(server_public_key == &self.0)
        }
    }

    struct ScpWire {
        channel: Channel<client::Msg>,
        buffered: VecDeque<u8>,
    }

    impl ScpWire {
        async fn read_byte(&mut self) -> u8 {
            loop {
                if let Some(byte) = self.buffered.pop_front() {
                    return byte;
                }
                let message = tokio::time::timeout(DEADLINE, self.channel.wait())
                    .await
                    .expect("SCP protocol deadline")
                    .expect("SCP channel closed before protocol byte");
                match message {
                    ChannelMsg::Data { data } => self.buffered.extend(data),
                    ChannelMsg::Success => {}
                    other => panic!("expected SCP protocol data, received {other:?}"),
                }
            }
        }

        async fn read_line(&mut self) -> Vec<u8> {
            let mut line = Vec::new();
            loop {
                let byte = self.read_byte().await;
                line.push(byte);
                if byte == b'\n' {
                    return line;
                }
            }
        }

        async fn read_exact(&mut self, length: usize) -> Vec<u8> {
            let mut bytes = Vec::with_capacity(length);
            while bytes.len() < length {
                bytes.push(self.read_byte().await);
            }
            bytes
        }

        async fn send(&self, bytes: impl AsRef<[u8]>) {
            self.channel
                .data_bytes(bytes.as_ref().to_vec())
                .await
                .expect("send SCP protocol data");
        }

        async fn expect_ack(&mut self) {
            assert_eq!(self.read_byte().await, 0, "expected SCP ACK");
        }

        async fn finish(mut self) -> u32 {
            let mut status = None;
            while let Some(message) = tokio::time::timeout(DEADLINE, self.channel.wait())
                .await
                .expect("SCP completion deadline")
            {
                match message {
                    ChannelMsg::ExitStatus { exit_status } => status = Some(exit_status),
                    ChannelMsg::Close => break,
                    ChannelMsg::Data { data } => self.buffered.extend(data),
                    _ => {}
                }
            }
            status.expect("SCP exit status")
        }
    }

    #[test]
    fn real_scp_sink_accepts_recursive_c_d_e_t_and_ack_protocol() {
        let server = HermeticSshServer::start(DEADLINE).expect("start SSH fixture");
        std::fs::create_dir(server.sftp().path().join("uploads")).unwrap();
        runtime().block_on(async {
            let mut wire = connect_scp(&server, "scp -r -p -t uploads").await;
            wire.expect_ack().await;
            wire.send(b"T1700000000 0 1700000001 0\n").await;
            wire.expect_ack().await;
            wire.send(b"D0750 0 nested\n").await;
            wire.expect_ack().await;
            wire.send(b"T1700000002 0 1700000003 0\n").await;
            wire.expect_ack().await;
            wire.send(b"C0640 11 payload.txt\n").await;
            wire.expect_ack().await;
            wire.send(b"hello-world\0").await;
            wire.expect_ack().await;
            wire.send(b"E\n").await;
            wire.expect_ack().await;
            wire.channel.eof().await.unwrap();
            assert_eq!(wire.finish().await, 0);
        });
        assert_eq!(
            std::fs::read(server.sftp().path().join("uploads/nested/payload.txt")).unwrap(),
            b"hello-world"
        );
        server.stop(DEADLINE).unwrap();
    }

    #[test]
    fn real_scp_source_emits_recursive_d_c_e_and_ack_protocol() {
        let server = HermeticSshServer::start(DEADLINE).expect("start SSH fixture");
        std::fs::create_dir(server.sftp().path().join("tree")).unwrap();
        std::fs::create_dir(server.sftp().path().join("tree/sub")).unwrap();
        std::fs::write(
            server.sftp().path().join("tree/sub/data.bin"),
            b"source-data",
        )
        .unwrap();
        runtime().block_on(async {
            let mut wire = connect_scp(&server, "scp -r -f tree").await;
            wire.send([0]).await;
            let directory = wire.read_line().await;
            assert!(directory.starts_with(b"D"));
            assert!(directory.ends_with(b" tree\n"));
            wire.send([0]).await;
            let nested = wire.read_line().await;
            assert!(nested.starts_with(b"D"));
            assert!(nested.ends_with(b" sub\n"));
            wire.send([0]).await;
            let file = wire.read_line().await;
            assert!(file.starts_with(b"C"));
            assert!(file.ends_with(b" data.bin\n"));
            wire.send([0]).await;
            assert_eq!(wire.read_exact(11).await, b"source-data");
            assert_eq!(wire.read_byte().await, 0);
            wire.send([0]).await;
            assert_eq!(wire.read_line().await, b"E\n");
            wire.send([0]).await;
            assert_eq!(wire.read_line().await, b"E\n");
            wire.send([0]).await;
            assert_eq!(wire.finish().await, 0);
        });
        server.stop(DEADLINE).unwrap();
    }

    #[test]
    fn real_scp_rejects_target_and_control_record_escape() {
        let server = HermeticSshServer::start(DEADLINE).expect("start SSH fixture");
        let outside = tempfile::tempdir().unwrap();
        let outside_file = outside.path().join("outside.txt");
        std::fs::write(&outside_file, b"unchanged").unwrap();

        let target_status = runtime().block_on(async {
            let mut wire = connect_scp(&server, "scp -t ../outside.txt").await;
            assert_eq!(wire.read_byte().await, 2);
            let _ = wire.read_line().await;
            wire.finish().await
        });
        assert_ne!(target_status, 0);

        std::fs::create_dir(server.sftp().path().join("uploads")).unwrap();
        let control_status = runtime().block_on(async {
            let mut wire = connect_scp(&server, "scp -t uploads").await;
            wire.expect_ack().await;
            wire.send(b"C0644 1 ../escape.txt\n").await;
            assert_eq!(wire.read_byte().await, 2);
            let _ = wire.read_line().await;
            wire.finish().await
        });
        assert_ne!(control_status, 0);
        let forced_control_status = runtime().block_on(async {
            let mut wire = connect_scp(&server, "scp -t forced.bin").await;
            wire.expect_ack().await;
            wire.send(b"C0644 1 ../escape.txt\n").await;
            assert_eq!(wire.read_byte().await, 2);
            let _ = wire.read_line().await;
            wire.finish().await
        });
        assert_ne!(forced_control_status, 0);
        assert_eq!(std::fs::read(outside_file).unwrap(), b"unchanged");
        assert!(!server.sftp().path().join("escape.txt").exists());
        server.stop(DEADLINE).unwrap();
    }

    #[test]
    fn real_scp_source_and_sink_reject_directory_redirect_escape() {
        let server = HermeticSshServer::start(DEADLINE).expect("start SSH fixture");
        let outside = tempfile::tempdir().expect("create outside directory");
        let secret = outside.path().join("secret.txt");
        std::fs::write(&secret, b"outside-secret").unwrap();
        let redirect =
            DirectoryRedirect::create(outside.path(), &server.sftp().path().join("escape"))
                .expect("create directory redirect without elevated privileges");

        let source_status = runtime().block_on(async {
            let mut wire = connect_scp(&server, "scp -f escape/secret.txt").await;
            assert_eq!(wire.read_byte().await, 2);
            let _ = wire.read_line().await;
            wire.finish().await
        });
        assert_ne!(source_status, 0);

        let sink_status = runtime().block_on(async {
            let mut wire = connect_scp(&server, "scp -t escape/new.txt").await;
            assert_eq!(wire.read_byte().await, 2);
            let _ = wire.read_line().await;
            wire.finish().await
        });
        assert_ne!(sink_status, 0);
        assert_eq!(std::fs::read(secret).unwrap(), b"outside-secret");
        assert!(!outside.path().join("new.txt").exists());

        drop(redirect);
        server.stop(DEADLINE).unwrap();
    }

    #[test]
    fn real_scp_sink_rejects_a_dangling_leaf_redirect() {
        let server = HermeticSshServer::start(DEADLINE).expect("start SSH fixture");
        let outside = tempfile::tempdir().expect("create outside directory");
        let redirect = DanglingLeafRedirect::create(
            outside.path(),
            &server.sftp().path().join("dangling-leaf"),
        )
        .expect("create dangling leaf redirect");
        assert!(!redirect.target().exists());
        assert!(
            super::resolve_for_create(server.sftp().path(), Path::new("dangling-leaf")).is_err()
        );

        let status = runtime().block_on(async {
            let mut wire = connect_scp(&server, "scp -t dangling-leaf").await;
            assert_eq!(wire.read_byte().await, 2);
            let _ = wire.read_line().await;
            wire.finish().await
        });
        assert_ne!(status, 0);
        assert!(!redirect.target().exists());
        drop(redirect);
        server.stop(DEADLINE).unwrap();
    }

    #[test]
    fn real_scp_sink_rejects_incomplete_control_record_at_eof() {
        let server = HermeticSshServer::start(DEADLINE).expect("start SSH fixture");
        std::fs::create_dir(server.sftp().path().join("uploads")).unwrap();
        let status = runtime().block_on(async {
            let mut wire = connect_scp(&server, "scp -t uploads").await;
            wire.expect_ack().await;
            wire.send(b"C0644").await;
            wire.channel.eof().await.unwrap();
            assert_eq!(wire.read_byte().await, 2);
            let _ = wire.read_line().await;
            wire.finish().await
        });
        assert_ne!(status, 0);
        server.stop(DEADLINE).unwrap();
    }

    #[test]
    fn real_scp_sink_honors_a_nonexistent_file_destination() {
        let server = HermeticSshServer::start(DEADLINE).expect("start SSH fixture");
        runtime().block_on(async {
            let mut wire = connect_scp(&server, "scp -t renamed.bin").await;
            wire.expect_ack().await;
            wire.send(b"C0644 7 original.bin\n").await;
            wire.expect_ack().await;
            wire.send(b"renamed\0").await;
            wire.expect_ack().await;
            wire.channel.eof().await.unwrap();
            assert_eq!(wire.finish().await, 0);
        });
        assert_eq!(
            std::fs::read(server.sftp().path().join("renamed.bin")).unwrap(),
            b"renamed"
        );
        assert!(!server.sftp().path().join("original.bin").exists());
        server.stop(DEADLINE).unwrap();
    }

    #[test]
    fn real_scp_sink_ignores_the_source_name_for_a_forced_file_destination() {
        let server = HermeticSshServer::start(DEADLINE).expect("start SSH fixture");
        runtime().block_on(async {
            let mut wire = connect_scp(&server, "scp -t renamed.bin").await;
            wire.expect_ack().await;
            wire.send(b"C0644 7 C:\\Users\\runner\\source.bin\n").await;
            wire.expect_ack().await;
            wire.send(b"renamed\0").await;
            wire.expect_ack().await;
            wire.channel.eof().await.unwrap();
            assert_eq!(wire.finish().await, 0);
        });
        assert_eq!(
            std::fs::read(server.sftp().path().join("renamed.bin")).unwrap(),
            b"renamed"
        );
        server.stop(DEADLINE).unwrap();
    }

    #[test]
    fn real_scp_sink_uses_the_leaf_of_git_for_windows_recursive_source_names() {
        let server = HermeticSshServer::start(DEADLINE).expect("start SSH fixture");
        std::fs::create_dir(server.sftp().path().join("uploads")).unwrap();
        runtime().block_on(async {
            let mut wire = connect_scp(&server, "scp -r -t uploads").await;
            wire.expect_ack().await;
            wire.send(b"D0755 0 C:\\Users\\runner\\tree\n").await;
            wire.expect_ack().await;
            wire.send(b"C0644 7 C:\\Users\\runner\\tree\\data.bin\n")
                .await;
            wire.expect_ack().await;
            wire.send(b"payload\0").await;
            wire.expect_ack().await;
            wire.send(b"E\n").await;
            wire.expect_ack().await;
            wire.channel.eof().await.unwrap();
            assert_eq!(wire.finish().await, 0);
        });
        assert_eq!(
            std::fs::read(server.sftp().path().join("uploads/tree/data.bin")).unwrap(),
            b"payload"
        );
        server.stop(DEADLINE).unwrap();
    }

    async fn connect_scp(server: &HermeticSshServer, command: &str) -> ScpWire {
        let mut client = client::connect(
            Arc::new(client::Config::default()),
            server.address(),
            ExpectedHostKey(server.host_key().clone()),
        )
        .await
        .expect("connect SCP SSH client");
        assert!(
            client
                .authenticate_publickey(
                    "fixture-user",
                    PrivateKeyWithHashAlg::new(Arc::clone(server.agent().private_key()), None),
                )
                .await
                .expect("authenticate SCP SSH client")
                .success()
        );
        let channel = client
            .channel_open_session()
            .await
            .expect("open SCP channel");
        channel
            .exec(true, command)
            .await
            .expect("request SCP command");
        ScpWire {
            channel,
            buffered: VecDeque::new(),
        }
    }

    fn runtime() -> tokio::runtime::Runtime {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
    }
}
