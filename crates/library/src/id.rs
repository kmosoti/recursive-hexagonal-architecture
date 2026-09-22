use std::fmt;

use unicode_normalization::UnicodeNormalization as _;

/// A relative, `/`-separated path with no empty, `.` or `..` segment.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RelPath(String);

/// Why a string is not a [`RelPath`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RelPathError {
    Empty,
    Absolute,
    Backslash,
    BadSegment,
}

impl RelPath {
    /// Validates a relative path.
    ///
    /// # Errors
    /// Empty, absolute, backslash-separated, or containing an empty, `.` or
    /// `..` segment.
    pub fn new(path: &str) -> Result<Self, RelPathError> {
        if path.is_empty() {
            return Err(RelPathError::Empty);
        }
        if path.starts_with('/') {
            return Err(RelPathError::Absolute);
        }
        if path.contains('\\') {
            return Err(RelPathError::Backslash);
        }
        if path
            .split('/')
            .any(|s| s.is_empty() || s == "." || s == "..")
        {
            return Err(RelPathError::BadSegment);
        }
        Ok(Self(path.to_owned()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Whether this is a markdown page, by its `.md` suffix. Case-sensitive on
    /// purpose: the page contract says `.md`, and `A.MD` is not a page.
    #[must_use]
    #[allow(clippy::case_sensitive_file_extension_comparisons)]
    pub fn is_page(&self) -> bool {
        self.0.ends_with(".md") && self.0.len() > 3 && !self.0.ends_with("/.md")
    }
}

impl fmt::Display for RelPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A page's identity: the relative path without `.md`, NFC-normalized, case
/// preserved.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PageId(String);

impl PageId {
    /// The id of the page at `path`.
    #[must_use]
    pub fn from_path(path: &RelPath) -> Self {
        let raw = path.as_str();
        let stem = raw.strip_suffix(".md").unwrap_or(raw);
        Self(stem.nfc().collect())
    }

    /// An id from its text, NFC-normalized.
    #[must_use]
    pub fn new(id: &str) -> Self {
        Self(id.nfc().collect())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The last `/` segment.
    #[must_use]
    pub fn basename(&self) -> &str {
        self.0.rsplit('/').next().unwrap_or(&self.0)
    }
}

impl fmt::Display for PageId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_nfc_and_keep_case() {
        let nfd = RelPath::new("Cafe\u{301}/Page.md").expect("valid");
        assert_eq!(PageId::from_path(&nfd).as_str(), "Caf\u{e9}/Page");
        assert_eq!(PageId::from_path(&nfd).basename(), "Page");
    }

    #[test]
    fn bad_paths_are_rejected() {
        for bad in ["", "/a.md", "a\\b.md", "a//b.md", "./a.md", "a/../b.md"] {
            assert!(RelPath::new(bad).is_err(), "{bad}");
        }
        assert!(RelPath::new("a/b.md").expect("valid").is_page());
        assert!(!RelPath::new("a/b.txt").expect("valid").is_page());
    }
}
