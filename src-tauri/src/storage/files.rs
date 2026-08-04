use std::ffi::OsStr;
use std::io;
use std::path::{Component, Path, PathBuf};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StorageKey(String);

impl StorageKey {
    pub fn parse(value: impl Into<String>) -> io::Result<Self> {
        let value = value.into();
        let path = Path::new(&value);
        let valid = !value.is_empty()
            && !path.is_absolute()
            && path.components().all(|component| match component {
                Component::Normal(part) => valid_part(part),
                _ => false,
            });
        valid
            .then_some(Self(value))
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "invalid storage key"))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

fn valid_part(value: &OsStr) -> bool {
    let Some(value) = value.to_str() else {
        return false;
    };
    !value.is_empty()
        && value != "."
        && value != ".."
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

#[derive(Clone, Debug)]
pub struct AppDataFiles {
    root: PathBuf,
}

impl AppDataFiles {
    pub fn new(root: impl AsRef<Path>) -> io::Result<Self> {
        std::fs::create_dir_all(root.as_ref())?;
        Ok(Self {
            root: std::fs::canonicalize(root)?,
        })
    }

    pub fn write(&self, key: &StorageKey, bytes: &[u8]) -> io::Result<()> {
        let path = self.resolve_for_write(key)?;
        std::fs::write(path, bytes)
    }

    pub fn read(&self, key: &StorageKey) -> io::Result<Vec<u8>> {
        std::fs::read(self.resolve_existing(key)?)
    }

    pub fn delete(&self, key: &StorageKey) -> io::Result<()> {
        let path = self.resolve_existing(key)?;
        match std::fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error),
        }
    }

    fn resolve_for_write(&self, key: &StorageKey) -> io::Result<PathBuf> {
        let path = self.root.join(key.as_str());
        let parent = path
            .parent()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "missing parent"))?;
        self.reject_existing_symlinks(parent)?;
        std::fs::create_dir_all(parent)?;
        let parent = std::fs::canonicalize(parent)?;
        if !parent.starts_with(&self.root) {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "storage path escapes app-data root",
            ));
        }
        let path = parent.join(
            path.file_name()
                .expect("validated storage key has a filename"),
        );
        if path
            .symlink_metadata()
            .is_ok_and(|metadata| metadata.file_type().is_symlink())
        {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "storage path is a symlink",
            ));
        }
        Ok(path)
    }

    fn resolve_existing(&self, key: &StorageKey) -> io::Result<PathBuf> {
        let path = self.root.join(key.as_str());
        self.reject_existing_symlinks(&path)?;
        let path = std::fs::canonicalize(path)?;
        path.starts_with(&self.root).then_some(path).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::PermissionDenied,
                "storage path escapes app-data root",
            )
        })
    }

    fn reject_existing_symlinks(&self, path: &Path) -> io::Result<()> {
        let relative = path.strip_prefix(&self.root).map_err(|_| {
            io::Error::new(
                io::ErrorKind::PermissionDenied,
                "storage path escapes app-data root",
            )
        })?;
        let mut current = self.root.clone();
        for component in relative.components() {
            current.push(component);
            match std::fs::symlink_metadata(&current) {
                Ok(metadata) if metadata.file_type().is_symlink() => {
                    return Err(io::Error::new(
                        io::ErrorKind::PermissionDenied,
                        "storage path contains a symlink",
                    ));
                }
                Ok(_) => {}
                Err(error) if error.kind() == io::ErrorKind::NotFound => break,
                Err(error) => return Err(error),
            }
        }
        Ok(())
    }
}
