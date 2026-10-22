use std::{
    collections::HashMap,
    fmt, io,
    io::{Read as _, Seek as _, Write as _},
    path::{Component, Path, PathBuf},
};

use russh_sftp::protocol::{
    Attrs, Data, File as SftpFile, FileAttributes, Handle, Name, OpenFlags, Status, StatusCode,
    Version,
};

const MAX_IO_SIZE: usize = 256 * 1024;

/// An isolated filesystem root for SFTP and SCP interoperability tests.
pub struct SftpRoot {
    directory: tempfile::TempDir,
    canonical_root: PathBuf,
}

impl SftpRoot {
    /// Creates a fresh temporary root.
    ///
    /// # Errors
    ///
    /// Returns an error if the temporary directory cannot be created or canonicalized.
    pub fn new() -> io::Result<Self> {
        let directory = tempfile::Builder::new()
            .prefix("rterm-sftp-root-")
            .tempdir()?;
        let canonical_root = directory.path().canonicalize()?;
        Ok(Self {
            directory,
            canonical_root,
        })
    }

    /// Returns the temporary root path.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.canonical_root
    }

    /// Resolves an existing relative path while rejecting traversal and symlink escape.
    ///
    /// # Errors
    ///
    /// Returns an error for unsafe paths, symlink escape, missing paths, or filesystem
    /// resolution failures.
    pub fn resolve_existing(&self, relative: &Path) -> Result<PathBuf, SftpPathError> {
        validate_relative(relative)?;
        let resolved = self
            .directory
            .path()
            .join(relative)
            .canonicalize()
            .map_err(|source| SftpPathError::Io {
                path: relative.to_path_buf(),
                source,
            })?;
        self.ensure_contained(relative, resolved)
    }

    /// Resolves a new relative path, requiring its existing parent to remain contained.
    ///
    /// # Errors
    ///
    /// Returns an error for unsafe paths, symlink escape, missing parents, or filesystem
    /// resolution failures.
    ///
    /// This is a best-effort containment check. Stable Rust does not expose a portable
    /// no-follow create/open primitive, so callers must not concurrently mutate the
    /// fixture filesystem between validation and use.
    pub fn resolve_for_create(&self, relative: &Path) -> Result<PathBuf, SftpPathError> {
        validate_relative(relative)?;
        let joined = self.canonical_root.join(relative);
        match std::fs::symlink_metadata(&joined) {
            Ok(metadata) => {
                if metadata_is_redirect(&metadata) {
                    return Err(SftpPathError::OutsideRoot {
                        path: relative.to_path_buf(),
                    });
                }
                let resolved = joined.canonicalize().map_err(|source| SftpPathError::Io {
                    path: relative.to_path_buf(),
                    source,
                })?;
                return self.ensure_contained(relative, resolved);
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(source) => {
                return Err(SftpPathError::Io {
                    path: relative.to_path_buf(),
                    source,
                });
            }
        }
        let parent = joined.parent().ok_or_else(|| SftpPathError::UnsafePath {
            path: relative.to_path_buf(),
        })?;
        let canonical_parent = parent.canonicalize().map_err(|source| SftpPathError::Io {
            path: relative.to_path_buf(),
            source,
        })?;
        ensure_canonical_containment(&self.canonical_root, &canonical_parent).map_err(|_| {
            SftpPathError::OutsideRoot {
                path: relative.to_path_buf(),
            }
        })?;
        let file_name = joined
            .file_name()
            .ok_or_else(|| SftpPathError::UnsafePath {
                path: relative.to_path_buf(),
            })?;
        Ok(canonical_parent.join(file_name))
    }

    fn ensure_contained(
        &self,
        requested: &Path,
        resolved: PathBuf,
    ) -> Result<PathBuf, SftpPathError> {
        ensure_canonical_containment(&self.canonical_root, &resolved)
            .map(|()| resolved)
            .map_err(|_| SftpPathError::OutsideRoot {
                path: requested.to_path_buf(),
            })
    }
}

fn ensure_canonical_containment(root: &Path, candidate: &Path) -> Result<(), SftpPathError> {
    if candidate.starts_with(root) {
        Ok(())
    } else {
        Err(SftpPathError::OutsideRoot {
            path: candidate.to_path_buf(),
        })
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

fn validate_relative(path: &Path) -> Result<(), SftpPathError> {
    if path.as_os_str().is_empty()
        || path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(SftpPathError::UnsafePath {
            path: path.to_path_buf(),
        });
    }
    Ok(())
}

/// A rejected or unavailable path in an isolated transfer root.
#[derive(Debug)]
pub enum SftpPathError {
    /// The lexical path is absolute, empty, or contains parent traversal.
    UnsafePath { path: PathBuf },
    /// Canonical resolution crossed the temporary root through a symlink.
    OutsideRoot { path: PathBuf },
    /// A filesystem operation failed without exposing file contents.
    Io { path: PathBuf, source: io::Error },
}

impl fmt::Display for SftpPathError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsafePath { path } => {
                write!(formatter, "unsafe fixture path: {}", path.display())
            }
            Self::OutsideRoot { path } => {
                write!(
                    formatter,
                    "fixture path escapes temporary root: {}",
                    path.display()
                )
            }
            Self::Io { path, source } => {
                write!(
                    formatter,
                    "fixture path operation failed for {}: {source}",
                    path.display()
                )
            }
        }
    }
}

impl std::error::Error for SftpPathError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::UnsafePath { .. } | Self::OutsideRoot { .. } => None,
        }
    }
}

pub(crate) struct SandboxedSftpSession {
    canonical_root: PathBuf,
    handles: HashMap<String, SftpHandle>,
    next_handle: u64,
    initialized: bool,
}

enum SftpHandle {
    File(std::fs::File),
    Directory(Option<Vec<SftpFile>>),
}

impl SandboxedSftpSession {
    pub(crate) fn new(root: &Path) -> Self {
        Self {
            canonical_root: root.to_path_buf(),
            handles: HashMap::new(),
            next_handle: 0,
            initialized: false,
        }
    }

    fn insert_handle(&mut self, handle: SftpHandle) -> String {
        let token = format!("fixture-{}", self.next_handle);
        self.next_handle = self.next_handle.wrapping_add(1);
        self.handles.insert(token.clone(), handle);
        token
    }

    fn relative_path(requested: &str) -> Result<PathBuf, StatusCode> {
        if requested.contains('\0') {
            return Err(StatusCode::BadMessage);
        }
        if requested == "." || requested == "/" {
            return Ok(PathBuf::new());
        }
        let requested = requested.strip_prefix("./").unwrap_or(requested);
        // SFTP paths live in a virtual server namespace. OpenSSH canonicalizes
        // relative batch operands through REALPATH and then sends `/name` back
        // to the server, so one leading slash denotes this fixture root rather
        // than the host operating-system root.
        let requested = requested.strip_prefix('/').unwrap_or(requested);
        if requested.contains('\\')
            || requested
                .as_bytes()
                .get(1)
                .is_some_and(|separator| *separator == b':')
        {
            return Err(StatusCode::PermissionDenied);
        }
        let path = PathBuf::from(requested);
        if path.as_os_str().is_empty()
            || path.is_absolute()
            || path.components().any(|component| {
                matches!(
                    component,
                    Component::ParentDir | Component::RootDir | Component::Prefix(_)
                )
            })
        {
            return Err(StatusCode::PermissionDenied);
        }
        Ok(path)
    }

    fn resolve_existing(&self, requested: &str) -> Result<PathBuf, StatusCode> {
        let relative = Self::relative_path(requested)?;
        let joined = self.canonical_root.join(relative);
        let resolved = joined.canonicalize().map_err(map_io_status)?;
        self.ensure_contained(resolved)
    }

    fn resolve_for_create(&self, requested: &str) -> Result<PathBuf, StatusCode> {
        let relative = Self::relative_path(requested)?;
        if relative.as_os_str().is_empty() {
            return Err(StatusCode::PermissionDenied);
        }
        let joined = self.canonical_root.join(relative);
        match std::fs::symlink_metadata(&joined) {
            Ok(metadata) => {
                if metadata_is_redirect(&metadata) {
                    return Err(StatusCode::PermissionDenied);
                }
                let resolved = joined.canonicalize().map_err(map_io_status)?;
                return self.ensure_contained(resolved);
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(map_io_status(error)),
        }
        let parent = joined.parent().ok_or(StatusCode::PermissionDenied)?;
        let parent = parent.canonicalize().map_err(map_io_status)?;
        let parent = self.ensure_contained(parent)?;
        let file_name = joined.file_name().ok_or(StatusCode::PermissionDenied)?;
        Ok(parent.join(file_name))
    }

    fn ensure_contained(&self, resolved: PathBuf) -> Result<PathBuf, StatusCode> {
        if resolved.starts_with(&self.canonical_root) {
            Ok(resolved)
        } else {
            Err(StatusCode::PermissionDenied)
        }
    }
}

impl russh_sftp::server::Handler for SandboxedSftpSession {
    type Error = StatusCode;

    fn unimplemented(&self) -> Self::Error {
        StatusCode::OpUnsupported
    }

    async fn init(
        &mut self,
        _version: u32,
        _extensions: HashMap<String, String>,
    ) -> Result<Version, Self::Error> {
        if self.initialized {
            return Err(StatusCode::BadMessage);
        }
        self.initialized = true;
        Ok(Version::new())
    }

    async fn open(
        &mut self,
        id: u32,
        filename: String,
        pflags: OpenFlags,
        attrs: FileAttributes,
    ) -> Result<Handle, Self::Error> {
        let path = if pflags.contains(OpenFlags::CREATE) {
            self.resolve_for_create(&filename)?
        } else {
            self.resolve_existing(&filename)?
        };
        let options: std::fs::OpenOptions = pflags.into();
        let file = options.open(&path).map_err(map_io_status)?;
        apply_file_attributes(&file, &path, &attrs).map_err(map_io_status)?;
        let handle = self.insert_handle(SftpHandle::File(file));
        Ok(Handle { id, handle })
    }

    async fn close(&mut self, id: u32, handle: String) -> Result<Status, Self::Error> {
        if self.handles.remove(&handle).is_none() {
            return Err(StatusCode::Failure);
        }
        Ok(ok_status(id))
    }

    async fn read(
        &mut self,
        id: u32,
        handle: String,
        offset: u64,
        len: u32,
    ) -> Result<Data, Self::Error> {
        let Some(SftpHandle::File(file)) = self.handles.get_mut(&handle) else {
            return Err(StatusCode::Failure);
        };
        file.seek(io::SeekFrom::Start(offset))
            .map_err(map_io_status)?;
        let length = usize::try_from(len).unwrap_or(MAX_IO_SIZE).min(MAX_IO_SIZE);
        let mut data = vec![0_u8; length];
        let read = file.read(&mut data).map_err(map_io_status)?;
        if read == 0 {
            return Err(StatusCode::Eof);
        }
        data.truncate(read);
        Ok(Data { id, data })
    }

    async fn write(
        &mut self,
        id: u32,
        handle: String,
        offset: u64,
        data: Vec<u8>,
    ) -> Result<Status, Self::Error> {
        if data.len() > MAX_IO_SIZE {
            return Err(StatusCode::Failure);
        }
        let Some(SftpHandle::File(file)) = self.handles.get_mut(&handle) else {
            return Err(StatusCode::Failure);
        };
        file.seek(io::SeekFrom::Start(offset))
            .map_err(map_io_status)?;
        file.write_all(&data).map_err(map_io_status)?;
        file.flush().map_err(map_io_status)?;
        Ok(ok_status(id))
    }

    async fn lstat(&mut self, id: u32, path: String) -> Result<Attrs, Self::Error> {
        self.stat(id, path).await
    }

    async fn fstat(&mut self, id: u32, handle: String) -> Result<Attrs, Self::Error> {
        let Some(SftpHandle::File(file)) = self.handles.get(&handle) else {
            return Err(StatusCode::Failure);
        };
        let metadata = file.metadata().map_err(map_io_status)?;
        Ok(Attrs {
            id,
            attrs: FileAttributes::from(&metadata),
        })
    }

    async fn setstat(
        &mut self,
        id: u32,
        path: String,
        attrs: FileAttributes,
    ) -> Result<Status, Self::Error> {
        let path = self.resolve_existing(&path)?;
        if !path.is_dir()
            && let Some(size) = attrs.size
        {
            let file = std::fs::OpenOptions::new()
                .write(true)
                .open(&path)
                .map_err(map_io_status)?;
            file.set_len(size).map_err(map_io_status)?;
        }
        apply_path_permissions(&path, &attrs).map_err(map_io_status)?;
        Ok(ok_status(id))
    }

    async fn fsetstat(
        &mut self,
        id: u32,
        handle: String,
        attrs: FileAttributes,
    ) -> Result<Status, Self::Error> {
        let Some(SftpHandle::File(file)) = self.handles.get(&handle) else {
            return Err(StatusCode::Failure);
        };
        apply_open_file_attributes(file, &attrs).map_err(map_io_status)?;
        Ok(ok_status(id))
    }

    async fn opendir(&mut self, id: u32, path: String) -> Result<Handle, Self::Error> {
        let path = self.resolve_existing(&path)?;
        if !path.is_dir() {
            return Err(StatusCode::NoSuchFile);
        }
        let mut entries = Vec::new();
        for entry in std::fs::read_dir(path).map_err(map_io_status)? {
            let entry = entry.map_err(map_io_status)?;
            let canonical = entry.path().canonicalize().map_err(map_io_status)?;
            if !canonical.starts_with(&self.canonical_root) {
                continue;
            }
            let metadata = std::fs::symlink_metadata(entry.path()).map_err(map_io_status)?;
            if metadata.file_type().is_symlink() {
                continue;
            }
            entries.push(SftpFile::new(
                entry.file_name().to_string_lossy(),
                FileAttributes::from(&metadata),
            ));
        }
        entries.sort_by(|left, right| left.filename.cmp(&right.filename));
        let handle = self.insert_handle(SftpHandle::Directory(Some(entries)));
        Ok(Handle { id, handle })
    }

    async fn readdir(&mut self, id: u32, handle: String) -> Result<Name, Self::Error> {
        let Some(SftpHandle::Directory(entries)) = self.handles.get_mut(&handle) else {
            return Err(StatusCode::Failure);
        };
        let files = entries.take().ok_or(StatusCode::Eof)?;
        if files.is_empty() {
            return Err(StatusCode::Eof);
        }
        Ok(Name { id, files })
    }

    async fn remove(&mut self, id: u32, filename: String) -> Result<Status, Self::Error> {
        let path = self.resolve_existing(&filename)?;
        if path.is_dir() {
            return Err(StatusCode::PermissionDenied);
        }
        std::fs::remove_file(path).map_err(map_io_status)?;
        Ok(ok_status(id))
    }

    async fn mkdir(
        &mut self,
        id: u32,
        path: String,
        attrs: FileAttributes,
    ) -> Result<Status, Self::Error> {
        let path = self.resolve_for_create(&path)?;
        std::fs::create_dir(&path).map_err(map_io_status)?;
        apply_path_permissions(&path, &attrs).map_err(map_io_status)?;
        Ok(ok_status(id))
    }

    async fn rmdir(&mut self, id: u32, path: String) -> Result<Status, Self::Error> {
        let path = self.resolve_existing(&path)?;
        if path == self.canonical_root {
            return Err(StatusCode::PermissionDenied);
        }
        std::fs::remove_dir(path).map_err(map_io_status)?;
        Ok(ok_status(id))
    }

    async fn realpath(&mut self, id: u32, path: String) -> Result<Name, Self::Error> {
        let relative = Self::relative_path(&path)?;
        let resolved = self.resolve_existing(&path)?;
        if !resolved.starts_with(&self.canonical_root) {
            return Err(StatusCode::PermissionDenied);
        }
        let virtual_path = if relative.as_os_str().is_empty() {
            "/".to_owned()
        } else {
            format!("/{}", relative.to_string_lossy().replace('\\', "/"))
        };
        Ok(Name {
            id,
            files: vec![SftpFile::dummy(virtual_path)],
        })
    }

    async fn stat(&mut self, id: u32, path: String) -> Result<Attrs, Self::Error> {
        let path = self.resolve_existing(&path)?;
        let metadata = path.metadata().map_err(map_io_status)?;
        Ok(Attrs {
            id,
            attrs: FileAttributes::from(&metadata),
        })
    }

    async fn rename(
        &mut self,
        id: u32,
        oldpath: String,
        newpath: String,
    ) -> Result<Status, Self::Error> {
        let oldpath = self.resolve_existing(&oldpath)?;
        let newpath = self.resolve_for_create(&newpath)?;
        std::fs::rename(oldpath, newpath).map_err(map_io_status)?;
        Ok(ok_status(id))
    }

    async fn readlink(&mut self, _id: u32, _path: String) -> Result<Name, Self::Error> {
        Err(StatusCode::PermissionDenied)
    }

    async fn symlink(
        &mut self,
        _id: u32,
        _linkpath: String,
        _targetpath: String,
    ) -> Result<Status, Self::Error> {
        Err(StatusCode::PermissionDenied)
    }
}

fn ok_status(id: u32) -> Status {
    Status {
        id,
        status_code: StatusCode::Ok,
        error_message: "ok".to_owned(),
        language_tag: "en-US".to_owned(),
    }
}

fn map_io_status(error: io::Error) -> StatusCode {
    let kind = error.kind();
    drop(error);
    match kind {
        io::ErrorKind::NotFound => StatusCode::NoSuchFile,
        io::ErrorKind::PermissionDenied => StatusCode::PermissionDenied,
        io::ErrorKind::InvalidInput | io::ErrorKind::InvalidData => StatusCode::BadMessage,
        _ => StatusCode::Failure,
    }
}

fn apply_file_attributes(
    file: &std::fs::File,
    path: &Path,
    attrs: &FileAttributes,
) -> io::Result<()> {
    apply_open_file_attributes(file, attrs)?;
    apply_path_permissions(path, attrs)
}

fn apply_open_file_attributes(file: &std::fs::File, attrs: &FileAttributes) -> io::Result<()> {
    if let Some(size) = attrs.size {
        file.set_len(size)?;
    }
    Ok(())
}

#[cfg(unix)]
fn apply_path_permissions(path: &Path, attrs: &FileAttributes) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt as _;

    if let Some(mode) = attrs.permissions {
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode & 0o777))?;
    }
    Ok(())
}

#[cfg(not(unix))]
fn apply_path_permissions(path: &Path, attrs: &FileAttributes) -> io::Result<()> {
    if let Some(mode) = attrs.permissions {
        let mut permissions = path.metadata()?.permissions();
        permissions.set_readonly(mode & 0o222 == 0);
        std::fs::set_permissions(path, permissions)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{
        path::{Path, PathBuf},
        sync::Arc,
        time::Duration,
    };

    use russh::{client, keys::PrivateKeyWithHashAlg};
    use russh_sftp::client::SftpSession;
    use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};

    use super::{SandboxedSftpSession, SftpPathError, SftpRoot};
    use crate::ssh::{
        HermeticSshServer,
        redirect::{DanglingLeafRedirect, DirectoryRedirect},
    };

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
    fn resolves_only_paths_contained_by_the_temporary_root() {
        let root = SftpRoot::new().expect("create SFTP root");
        std::fs::create_dir(root.path().join("inside")).unwrap();
        std::fs::write(root.path().join("inside/file.txt"), b"safe").unwrap();

        let path = root.resolve_existing(Path::new("inside/file.txt")).unwrap();
        assert!(path.starts_with(root.path()));
        assert_eq!(std::fs::read(path).unwrap(), b"safe");
        assert!(matches!(
            root.resolve_existing(Path::new("../outside")),
            Err(SftpPathError::UnsafePath { .. })
        ));
        let absolute = if cfg!(windows) {
            Path::new(r"C:\Windows")
        } else {
            Path::new("/etc")
        };
        assert!(matches!(
            root.resolve_existing(absolute),
            Err(SftpPathError::UnsafePath { .. })
        ));
    }

    #[test]
    fn protocol_virtual_absolute_path_matrix_preserves_root_and_rejects_escape_forms() {
        assert_eq!(
            SandboxedSftpSession::relative_path("/name").unwrap(),
            PathBuf::from("name")
        );
        for rejected in [
            "/../outside",
            "//server/share",
            "/C:/outside",
            r"/C:\outside",
            r"\outside",
        ] {
            assert!(
                SandboxedSftpSession::relative_path(rejected).is_err(),
                "unexpectedly accepted {rejected:?}"
            );
        }
    }

    #[test]
    fn resolve_for_create_rejects_parent_traversal() {
        let root = SftpRoot::new().expect("create SFTP root");
        std::fs::create_dir(root.path().join("uploads")).unwrap();
        assert!(
            root.resolve_for_create(Path::new("uploads/new.txt"))
                .unwrap()
                .starts_with(root.path())
        );
        assert!(matches!(
            root.resolve_for_create(Path::new("uploads/../../escape.txt")),
            Err(SftpPathError::UnsafePath { .. })
        ));
    }

    #[test]
    fn canonical_containment_seam_rejects_escape_on_every_platform() {
        let root = Path::new("fixture-root");
        let outside = Path::new("outside-target");
        assert!(super::ensure_canonical_containment(root, outside).is_err());
    }

    #[test]
    fn directory_redirect_escape_is_rejected_on_every_platform() {
        let root = SftpRoot::new().expect("create SFTP root");
        let outside = tempfile::tempdir().unwrap();
        std::fs::write(outside.path().join("secret.txt"), b"outside").unwrap();
        let redirect = DirectoryRedirect::create(outside.path(), &root.path().join("escape"))
            .expect("create directory redirect without elevated privileges");
        assert!(redirect.path().exists());
        assert!(matches!(
            root.resolve_existing(Path::new("escape/secret.txt")),
            Err(SftpPathError::OutsideRoot { .. })
        ));
        assert!(matches!(
            root.resolve_for_create(Path::new("escape/new.txt")),
            Err(SftpPathError::OutsideRoot { .. })
        ));
    }

    #[test]
    fn real_sftp_subsystem_supports_files_directories_and_metadata() {
        let server = HermeticSshServer::start(DEADLINE).expect("start SSH fixture");
        runtime().block_on(async {
            let (ssh, sftp) = connect_sftp(&server).await;
            assert_eq!(sftp.canonicalize(".").await.unwrap(), "/");
            sftp.create_dir("uploads").await.unwrap();
            let mut directory_metadata = sftp.metadata("uploads").await.unwrap();
            directory_metadata.permissions = Some(0o755);
            sftp.set_metadata("uploads", directory_metadata)
                .await
                .unwrap();
            let mut file = sftp.create("uploads/payload.bin").await.unwrap();
            file.write_all(b"hermetic-sftp").await.unwrap();
            file.shutdown().await.unwrap();
            let metadata = sftp.metadata("uploads/payload.bin").await.unwrap();
            assert_eq!(metadata.len(), 13);
            let mut file = sftp.open("uploads/payload.bin").await.unwrap();
            let mut payload = Vec::new();
            file.read_to_end(&mut payload).await.unwrap();
            file.shutdown().await.unwrap();
            assert_eq!(payload, b"hermetic-sftp");
            let entries = sftp.read_dir("uploads").await.unwrap();
            assert!(
                entries
                    .into_iter()
                    .any(|entry| entry.file_name() == "payload.bin")
            );
            sftp.rename("uploads/payload.bin", "uploads/renamed.bin")
                .await
                .unwrap();
            sftp.remove_file("uploads/renamed.bin").await.unwrap();
            sftp.remove_dir("uploads").await.unwrap();
            sftp.close().await.unwrap();
            ssh.disconnect(russh::Disconnect::ByApplication, "SFTP test", "")
                .await
                .unwrap();
        });
        server.stop(DEADLINE).unwrap();
    }

    #[test]
    fn real_sftp_subsystem_rejects_traversal_absolute_and_directory_redirect_escape() {
        let server = HermeticSshServer::start(DEADLINE).expect("start SSH fixture");
        let outside = tempfile::tempdir().unwrap();
        std::fs::write(
            server.sftp().path().join("virtual-root.txt"),
            b"virtual-root",
        )
        .unwrap();
        std::fs::write(outside.path().join("secret.txt"), b"outside").unwrap();
        let redirect =
            DirectoryRedirect::create(outside.path(), &server.sftp().path().join("escape"))
                .expect("create directory redirect without elevated privileges");
        runtime().block_on(async {
            let (ssh, sftp) = connect_sftp(&server).await;
            let mut virtual_root = sftp.open("/virtual-root.txt").await.unwrap();
            let mut virtual_contents = Vec::new();
            virtual_root
                .read_to_end(&mut virtual_contents)
                .await
                .unwrap();
            assert_eq!(virtual_contents, b"virtual-root");
            assert!(sftp.create("../outside.txt").await.is_err());
            for rejected in [
                "/../outside.txt",
                "//server/share",
                "/C:/outside.txt",
                r"/C:\outside.txt",
                r"\outside.txt",
            ] {
                assert!(
                    sftp.create(rejected).await.is_err(),
                    "accepted {rejected:?}"
                );
            }
            let absolute = if cfg!(windows) {
                r"C:\Windows\system.ini"
            } else {
                "/etc/passwd"
            };
            assert!(sftp.open(absolute).await.is_err());
            assert!(sftp.open("escape/secret.txt").await.is_err());
            assert!(sftp.open("/escape/secret.txt").await.is_err());
            assert!(sftp.create("escape/new.txt").await.is_err());
            sftp.close().await.unwrap();
            ssh.disconnect(russh::Disconnect::ByApplication, "SFTP security test", "")
                .await
                .unwrap();
        });
        assert!(!outside.path().join("outside.txt").exists());
        assert!(!outside.path().join("new.txt").exists());
        drop(redirect);
        server.stop(DEADLINE).unwrap();
    }

    #[test]
    fn real_sftp_create_rejects_a_dangling_leaf_redirect() {
        let server = HermeticSshServer::start(DEADLINE).expect("start SSH fixture");
        let outside = tempfile::tempdir().unwrap();
        let redirect = DanglingLeafRedirect::create(
            outside.path(),
            &server.sftp().path().join("dangling-leaf"),
        )
        .expect("create dangling leaf redirect");
        assert!(!redirect.target().exists());
        assert!(
            server
                .sftp()
                .resolve_for_create(Path::new("dangling-leaf"))
                .is_err()
        );

        runtime().block_on(async {
            let (ssh, sftp) = connect_sftp(&server).await;
            assert!(sftp.create("dangling-leaf").await.is_err());
            sftp.close().await.unwrap();
            ssh.disconnect(
                russh::Disconnect::ByApplication,
                "SFTP leaf security test",
                "",
            )
            .await
            .unwrap();
        });
        assert!(!redirect.target().exists());
        drop(redirect);
        server.stop(DEADLINE).unwrap();
    }

    async fn connect_sftp(
        server: &HermeticSshServer,
    ) -> (client::Handle<ExpectedHostKey>, SftpSession) {
        let mut ssh = client::connect(
            Arc::new(client::Config::default()),
            server.address(),
            ExpectedHostKey(server.host_key().clone()),
        )
        .await
        .expect("connect SFTP SSH client");
        assert!(
            ssh.authenticate_publickey(
                "fixture-user",
                PrivateKeyWithHashAlg::new(Arc::clone(server.agent().private_key()), None),
            )
            .await
            .expect("authenticate SFTP SSH client")
            .success()
        );
        let channel = ssh.channel_open_session().await.expect("open SFTP channel");
        channel
            .request_subsystem(true, "sftp")
            .await
            .expect("request SFTP subsystem");
        let sftp = SftpSession::new(channel.into_stream())
            .await
            .expect("initialize SFTP protocol");
        (ssh, sftp)
    }

    fn runtime() -> tokio::runtime::Runtime {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
    }
}
