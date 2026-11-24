//! Snapshot and atomically replace one configuration file at a known revision.
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

pub(crate) struct ConfigFile {
    path: PathBuf,
    bytes: Option<Vec<u8>>,
}
impl ConfigFile {
    pub fn load(path: &Path) -> Result<Self, String> {
        let bytes = read_optional(path)?;
        // Replace the linked file, not the symlink itself.
        let path = if bytes.is_some() {
            fs::canonicalize(path).map_err(error)?
        } else {
            path.to_owned()
        };
        Ok(Self { path, bytes })
    }
    pub fn text(&self) -> Result<&str, String> {
        self.bytes
            .as_deref()
            .map(std::str::from_utf8)
            .transpose()
            .map_err(error)
            .map(|s| s.unwrap_or(""))
    }
    pub fn revision(&self) -> String {
        let mut h = Sha256::new();
        h.update(self.path.as_os_str().as_encoded_bytes());
        h.update([0, self.bytes.is_some() as u8]);
        if let Some(bytes) = &self.bytes {
            h.update(bytes);
        }
        format!("{:x}", h.finalize())
    }
    pub fn replace(&self, bytes: &[u8]) -> Result<(), String> {
        let parent = self
            .path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        fs::create_dir_all(parent).map_err(error)?;
        let mut tmp = tempfile::NamedTempFile::new_in(parent).map_err(error)?;
        if let Ok(metadata) = fs::metadata(&self.path) {
            tmp.as_file()
                .set_permissions(metadata.permissions())
                .map_err(error)?;
        }
        tmp.write_all(bytes).map_err(error)?;
        tmp.as_file().sync_all().map_err(error)?;
        if read_optional(&self.path)? != self.bytes {
            return Err(conflict());
        }
        tmp.persist(&self.path).map_err(error)?;
        Ok(())
    }
}
fn error(e: impl std::fmt::Display) -> String {
    e.to_string()
}
pub fn conflict() -> String {
    "The rules file changed on disk. Your draft is still here. Reload before saving again.".into()
}
fn read_optional(path: &Path) -> Result<Option<Vec<u8>>, String> {
    match fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(error(e)),
    }
}
