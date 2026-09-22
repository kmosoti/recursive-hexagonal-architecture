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
    #[must_use]
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }
}

fn sink_error(path: &RelPath, e: &std::io::Error) -> SinkError {
    SinkError {
        path: Some(path.clone()),
        message: e.to_string(),
    }
}

impl OutputSink for FsSink {
    fn list(&self) -> Result<Vec<(RelPath, Digest)>, SinkError> {
        let paths = walk(&self.root).map_err(|message| SinkError {
            path: None,
            message,
        })?;
        paths
            .into_iter()
            .filter(|p| {
                !p.as_str()
                    .rsplit('/')
                    .next()
                    .is_some_and(|n| n.starts_with(".rhawiki-tmp-"))
            })
            .map(|p| {
                let bytes = fs::read(self.root.join(p.as_str())).map_err(|e| sink_error(&p, &e))?;
                Ok((p, Digest::of(&bytes)))
            })
            .collect()
    }

    fn write(&mut self, path: &RelPath, bytes: &[u8]) -> Result<(), SinkError> {
        let full = self.root.join(path.as_str());
        let dir = full.parent().unwrap_or(&self.root).to_path_buf();
        fs::create_dir_all(&dir).map_err(|e| sink_error(path, &e))?;
        let name = full.file_name().and_then(|n| n.to_str()).unwrap_or("out");
        let tmp = dir.join(format!(".rhawiki-tmp-{name}"));
        fs::write(&tmp, bytes).map_err(|e| sink_error(path, &e))?;
        fs::rename(&tmp, &full).map_err(|e| {
            let _ = fs::remove_file(&tmp);
            sink_error(path, &e)
        })
    }

    fn delete(&mut self, path: &RelPath) -> Result<(), SinkError> {
        fs::remove_file(self.root.join(path.as_str())).map_err(|e| sink_error(path, &e))
    }
}
