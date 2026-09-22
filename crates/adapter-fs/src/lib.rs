//! Filesystem adapter (BDR-0004): `FsSources` reads pages, `FsSink` writes
//! output. Symbolic links are never followed; paths are `/`-separated and
//! relative to the root.

use std::fs;
use std::path::{Path, PathBuf};

use library::{Digest, RelPath, RepositoryError, SourceRepository};
use site::{OutputSink, SinkError};

/// Every regular file under `root`, relative and `/`-separated, sorted. Links
/// and non-UTF-8 names are reported, not followed or mangled.
fn walk(root: &Path) -> Result<Vec<RelPath>, String> {
    fn visit(root: &Path, dir: &Path, out: &mut Vec<RelPath>) -> Result<(), String> {
        let entries = fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        for entry in entries {
            let entry = entry.map_err(|e| e.to_string())?;
            let kind = entry.file_type().map_err(|e| e.to_string())?;
            let path = entry.path();
            if kind.is_symlink() {
                continue;
            }
            if kind.is_dir() {
                visit(root, &path, out)?;
            } else if kind.is_file() {
                let rel = path.strip_prefix(root).map_err(|e| e.to_string())?;
                let text = rel
                    .to_str()
                    .ok_or_else(|| format!("{}: name is not UTF-8", rel.display()))?;
                let slashed = text.replace(std::path::MAIN_SEPARATOR, "/");
                out.push(RelPath::new(&slashed).map_err(|e| format!("{slashed}: {e:?}"))?);
            }
        }
        Ok(())
    }
    let mut out = Vec::new();
    if root.is_dir() {
        visit(root, root, &mut out)?;
    }
    out.sort();
    Ok(out)
}

/// Pages under a directory.
#[derive(Debug, Clone)]
pub struct FsSources {
    root: PathBuf,
}

impl FsSources {
    #[must_use]
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }
}

impl SourceRepository for FsSources {
    fn list(&self) -> Result<Vec<RelPath>, RepositoryError> {
        if !self.root.is_dir() {
            return Err(RepositoryError::Io {
                path: None,
                message: format!("{} is not a directory", self.root.display()),
            });
        }
        let all = walk(&self.root).map_err(|message| RepositoryError::Io {
            path: None,
            message,
        })?;
        Ok(all.into_iter().filter(RelPath::is_page).collect())
    }

    fn read(&self, path: &RelPath) -> Result<String, RepositoryError> {
        let full = self.root.join(path.as_str());
        match fs::symlink_metadata(&full) {
            Ok(m) if m.is_file() => {}
            _ => return Err(RepositoryError::NotFound(path.clone())),
        }
        let bytes = fs::read(&full).map_err(|e| RepositoryError::Io {
            path: Some(path.clone()),
            message: e.to_string(),
        })?;
        String::from_utf8(bytes).map_err(|_| RepositoryError::NotUtf8(path.clone()))
    }
}

/// Output under a directory; each write goes to a temporary file in the same
/// directory and is renamed into place, so a write is all or nothing.
#[derive(Debug, Clone)]
pub struct FsSink {
    root: PathBuf,
}

impl FsSink {
    /// The root is kept as its components, so a trailing `/` or `.` cannot
    /// make `symlink_metadata` resolve through a link at the last component
    /// (validation round, PR 16: `--out out-link/`).
    #[must_use]
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into().components().collect(),
        }
    }
}

fn sink_error(path: &RelPath, e: &std::io::Error) -> SinkError {
    SinkError {
        path: Some(path.clone()),
        message: e.to_string(),
    }
}

/// Whether `name` is one of this sink's temporary files. Renderer outputs
/// never end in `.tmp` (pages are `.html`), so a page cannot be mistaken for
/// one (review finding 2).
fn is_temporary(name: &str) -> bool {
    name.starts_with(".rhawiki-tmp-") && name.ends_with(".tmp")
}

impl FsSink {
    /// Refuses a root that is itself a symbolic link. `list` would inventory
    /// its target and the build would delete what it does not produce there
    /// (review round 4, PR 16 thread on the output root).
    fn checked_root(&self, path: Option<&RelPath>) -> Result<(), SinkError> {
        if fs::symlink_metadata(&self.root).is_ok_and(|m| m.file_type().is_symlink()) {
            return Err(SinkError {
                path: path.cloned(),
                message: format!(
                    "output root {} is a symbolic link; refusing to use it",
                    self.root.display()
                ),
            });
        }
        Ok(())
    }

    /// Refuses a path any of whose existing components under the root is a
    /// symbolic link, and creates missing directories one level at a time, so
    /// nothing is written outside the root (review finding 1).
    fn prepare(&self, path: &RelPath) -> Result<PathBuf, SinkError> {
        let refuse = |at: &Path| SinkError {
            path: Some(path.clone()),
            message: format!(
                "{} is a symbolic link; refusing to write through it",
                at.display()
            ),
        };
        // The root is the caller's choice and may not exist yet; everything
        // below it is checked one level at a time.
        self.checked_root(Some(path))?;
        fs::create_dir_all(&self.root).map_err(|e| sink_error(path, &e))?;
        let mut dir = self.root.clone();
        let segments: Vec<&str> = path.as_str().split('/').collect();
        let (file, parents) = segments.split_last().unwrap_or((&"", &[]));
        for segment in parents {
            dir.push(segment);
            match fs::symlink_metadata(&dir) {
                Ok(m) if m.file_type().is_symlink() => return Err(refuse(&dir)),
                Ok(m) if m.is_dir() => {}
                Ok(_) => {
                    return Err(SinkError {
                        path: Some(path.clone()),
                        message: format!("{} is not a directory", dir.display()),
                    });
                }
                Err(_) => fs::create_dir(&dir).map_err(|e| sink_error(path, &e))?,
            }
        }
        let full = dir.join(file);
        if fs::symlink_metadata(&full).is_ok_and(|m| m.file_type().is_symlink()) {
            return Err(refuse(&full));
        }
        Ok(full)
    }
}

impl OutputSink for FsSink {
    fn list(&self) -> Result<Vec<(RelPath, Digest)>, SinkError> {
        self.checked_root(None)?;
        let paths = walk(&self.root).map_err(|message| SinkError {
            path: None,
            message,
        })?;
        paths
            .into_iter()
            .filter(|p| !p.as_str().rsplit('/').next().is_some_and(is_temporary))
            .map(|p| {
                let bytes = fs::read(self.root.join(p.as_str())).map_err(|e| sink_error(&p, &e))?;
                Ok((p, Digest::of(&bytes)))
            })
            .collect()
    }

    fn write(&mut self, path: &RelPath, bytes: &[u8]) -> Result<(), SinkError> {
        use std::io::Write as _;
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let full = self.prepare(path)?;
        let dir = full.parent().unwrap_or(&self.root).to_path_buf();
        let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let tmp = dir.join(format!(".rhawiki-tmp-{}-{n}.tmp", std::process::id()));
        // create_new: an existing file or link at this name is an error, never
        // truncated through (review finding 2).
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp)
            .map_err(|e| sink_error(path, &e))?;
        let written = file.write_all(bytes).and_then(|()| file.sync_all());
        drop(file);
        if let Err(e) = written {
            let _ = fs::remove_file(&tmp);
            return Err(sink_error(path, &e));
        }
        fs::rename(&tmp, &full).map_err(|e| {
            let _ = fs::remove_file(&tmp);
            sink_error(path, &e)
        })
    }

    fn delete(&mut self, path: &RelPath) -> Result<(), SinkError> {
        self.checked_root(Some(path))?;
        let mut dir = self.root.clone();
        let segments: Vec<&str> = path.as_str().split('/').collect();
        for segment in &segments[..segments.len().saturating_sub(1)] {
            dir.push(segment);
            if fs::symlink_metadata(&dir).is_ok_and(|m| m.file_type().is_symlink()) {
                return Err(SinkError {
                    path: Some(path.clone()),
                    message: format!(
                        "{} is a symbolic link; refusing to delete through it",
                        dir.display()
                    ),
                });
            }
        }
        fs::remove_file(self.root.join(path.as_str())).map_err(|e| sink_error(path, &e))
    }
}
