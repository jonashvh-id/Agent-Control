use crate::error::{Error, Result};
#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
use std::{
    fs::{self, OpenOptions},
    io::{Read, Write},
    path::Path,
};
pub fn private_open(path: &Path, limit: u64) -> Result<fs::File> {
    #[cfg(not(unix))]
    return Err(Error::new("private_files_require_unix", 503));
    if fs::symlink_metadata(path)?.file_type().is_symlink() {
        return Err(Error::new("private_file_required", 503));
    }
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    options.custom_flags(libc_no_follow());
    let file = options.open(path)?;
    let meta = file.metadata()?;
    if !meta.is_file() || meta.len() > limit {
        return Err(Error::new("private_file_required", 503));
    }
    #[cfg(unix)]
    if meta.mode() & 0o077 != 0 || meta.uid() != rustix::process::geteuid().as_raw() {
        return Err(Error::new("private_file_required", 503));
    }
    Ok(file)
}
pub fn private_read(path: &Path, limit: u64) -> Result<Vec<u8>> {
    let file = private_open(path, limit)?;
    let mut data = Vec::new();
    file.take(limit + 1).read_to_end(&mut data)?;
    if data.len() as u64 > limit {
        return Err(Error::new("private_file_required", 503));
    }
    Ok(data)
}
// O_NOFOLLOW values come from libc through a small platform dependency, not unsafe calls.
#[cfg(unix)]
fn libc_no_follow() -> i32 {
    libc::O_NOFOLLOW
}
pub fn create_private(path: &Path, contents: &[u8]) -> Result<()> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    options.mode(0o600);
    let mut file = options.open(path)?;
    file.write_all(contents)?;
    file.sync_all()?;
    Ok(())
}
pub fn private_directory(path: &Path) -> Result<()> {
    if path.exists() && fs::symlink_metadata(path)?.file_type().is_symlink() {
        return Err(Error::new("invalid_runtime_directory", 503));
    }
    fs::create_dir_all(path)?;
    #[cfg(unix)]
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    Ok(())
}
