use std::{
    io,
    path::{Path, PathBuf},
};

pub(crate) struct DirectoryRedirect {
    link: PathBuf,
}

pub(crate) struct DanglingLeafRedirect {
    link: PathBuf,
    target: PathBuf,
}

impl DanglingLeafRedirect {
    pub(crate) fn create(outside: &Path, link: &Path) -> io::Result<Self> {
        let target = outside.join("dangling-target");
        create_dangling_leaf_redirect(&target, link)?;
        Ok(Self {
            link: link.to_path_buf(),
            target,
        })
    }

    pub(crate) fn target(&self) -> &Path {
        &self.target
    }
}

impl Drop for DanglingLeafRedirect {
    fn drop(&mut self) {
        let _ = remove_directory_redirect(&self.link);
        let _ = std::fs::remove_file(&self.target);
        let _ = std::fs::remove_dir(&self.target);
    }
}

impl DirectoryRedirect {
    pub(crate) fn create(target: &Path, link: &Path) -> io::Result<Self> {
        match std::fs::symlink_metadata(link) {
            Ok(_) => {
                return Err(io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    format!("redirect path already exists: {}", link.display()),
                ));
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
        create_directory_redirect(target, link)?;
        Ok(Self {
            link: link.to_path_buf(),
        })
    }

    pub(crate) fn path(&self) -> &Path {
        &self.link
    }

    fn remove(&self) -> io::Result<()> {
        remove_directory_redirect(&self.link)
    }
}

impl Drop for DirectoryRedirect {
    fn drop(&mut self) {
        let _ = self.remove();
    }
}

#[cfg(unix)]
fn create_directory_redirect(target: &Path, link: &Path) -> io::Result<()> {
    std::os::unix::fs::symlink(target, link)
}

#[cfg(unix)]
fn remove_directory_redirect(link: &Path) -> io::Result<()> {
    std::fs::remove_file(link)
}

#[cfg(unix)]
fn create_dangling_leaf_redirect(target: &Path, link: &Path) -> io::Result<()> {
    std::os::unix::fs::symlink(target, link)
}

#[cfg(windows)]
fn create_dangling_leaf_redirect(target: &Path, link: &Path) -> io::Result<()> {
    junction::create(target, link)
}

#[cfg(windows)]
fn create_directory_redirect(target: &Path, link: &Path) -> io::Result<()> {
    junction::create(target, link)
}

#[cfg(windows)]
fn remove_directory_redirect(link: &Path) -> io::Result<()> {
    junction::delete(link)?;
    match std::fs::remove_dir(link) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}
