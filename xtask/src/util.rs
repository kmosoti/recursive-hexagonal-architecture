//! Digests, UTC timestamps, version-token parsing, and command helpers.

use std::fmt::Write as _;
use std::path::Path;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use sha2::{Digest as _, Sha256};

use crate::error::{Context as _, Error, Result};

/// Lowercase hex encoding.
#[must_use]
pub fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        let _ = write!(out, "{b:02x}");
    }
    out
}

/// SHA-256 of `bytes` as 64 lowercase hex digits.
#[must_use]
pub fn sha256_hex(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}

/// SHA-256 of a file's bytes as 64 lowercase hex digits.
///
/// # Errors
/// Fails if the file cannot be read.
pub fn sha256_file(path: &Path) -> Result<String> {
    let bytes = std::fs::read(path).context(|| format!("reading {}", path.display()))?;
    Ok(sha256_hex(&bytes))
}

/// A UTC instant with second precision plus milliseconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UtcTime {
    unix_secs: i64,
    millis: u32,
}

impl UtcTime {
    /// The current time from the system clock.
    #[must_use]
    pub fn now() -> Self {
        let since = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        Self {
            unix_secs: i64::try_from(since.as_secs()).unwrap_or(i64::MAX),
            millis: since.subsec_millis(),
        }
    }

    #[must_use]
    pub fn from_unix(unix_secs: i64, millis: u32) -> Self {
        Self { unix_secs, millis }
    }

    fn parts(self) -> (i64, i64, i64, i64, i64, i64) {
        let days = self.unix_secs.div_euclid(86_400);
        let secs = self.unix_secs.rem_euclid(86_400);
        let (y, m, d) = civil_from_days(days);
        (y, m, d, secs / 3600, (secs % 3600) / 60, secs % 60)
    }

    /// RFC 3339, for example `2026-09-19T20:15:00.123Z`.
    #[must_use]
    pub fn rfc3339(self) -> String {
        let (y, m, d, hh, mm, ss) = self.parts();
        format!(
            "{y:04}-{m:02}-{d:02}T{hh:02}:{mm:02}:{ss:02}.{:03}Z",
            self.millis
        )
    }

    /// Filename-safe compact form, for example `20260919T201500Z`.
    #[must_use]
    pub fn compact(self) -> String {
        let (y, m, d, hh, mm, ss) = self.parts();
        format!("{y:04}{m:02}{d:02}T{hh:02}{mm:02}{ss:02}Z")
    }
}

/// Proleptic Gregorian date from days since 1970-01-01 (H. Hinnant's algorithm).
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    (y, m, d)
}

/// The first `MAJOR.MINOR.PATCH[-pre]` token in `text`.
///
/// ```
/// assert_eq!(xtask::util::first_version("cargo-nextest 0.9.104 (abc 2026-09-01)"), Some("0.9.104"));
/// assert_eq!(xtask::util::first_version("rustfmt 1.9.0-stable (48a229ceae)"), Some("1.9.0-stable"));
/// assert_eq!(xtask::util::first_version("no version here"), None);
/// ```
#[must_use]
pub fn first_version(text: &str) -> Option<&str> {
    text.split(|c: char| c.is_whitespace() || c == '(' || c == ')' || c == ',')
        .map(|token| token.trim_start_matches('v'))
        .find(|token| is_version(token))
}

fn is_version(token: &str) -> bool {
    let core = token.split('-').next().unwrap_or("");
    let parts: Vec<&str> = core.split('.').collect();
    parts.len() == 3
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
}

/// A `Command` for `program` without the variables `cargo run` sets for
/// xtask itself. Leaked into a child, `CARGO_PKG_NAME` makes cargo-machete
/// believe it was not started by cargo and read its own name as a path.
#[must_use]
pub fn command(program: &str) -> Command {
    const PER_PACKAGE: [&str; 6] = [
        "CARGO_MANIFEST_DIR",
        "CARGO_MANIFEST_PATH",
        "CARGO_CRATE_NAME",
        "CARGO_BIN_NAME",
        "CARGO_PRIMARY_PACKAGE",
        "CARGO_RUSTC_CURRENT_DIR",
    ];
    let mut cmd = Command::new(program);
    for (key, _) in std::env::vars_os() {
        let key = key.to_string_lossy();
        if key.starts_with("CARGO_PKG_") || PER_PACKAGE.contains(&key.as_ref()) {
            cmd.env_remove(key.as_ref());
        }
    }
    cmd
}

/// Runs a command in `root` and returns trimmed stdout if it exits zero.
///
/// # Errors
/// Fails if the command cannot be spawned or exits nonzero.
pub fn command_stdout(root: &Path, argv: &[&str]) -> Result<String> {
    let (program, rest) = argv
        .split_first()
        .ok_or_else(|| Error::new("empty command"))?;
    let output = command(program)
        .args(rest)
        .current_dir(root)
        .output()
        .context(|| format!("running `{}`", argv.join(" ")))?;
    if !output.status.success() {
        return Err(Error::new(format!(
            "`{}` exited with {}: {}",
            argv.join(" "),
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn civil_dates_match_known_days() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(11_017), (2000, 3, 1));
        assert_eq!(civil_from_days(18_993), (2022, 1, 1));
        assert_eq!(civil_from_days(-1), (1969, 12, 31));
    }

    #[test]
    fn timestamps_format_as_rfc3339_and_compact() {
        let t = UtcTime::from_unix(18_993 * 86_400 + 3_723, 45);
        assert_eq!(t.rfc3339(), "2022-01-01T01:02:03.045Z");
        assert_eq!(t.compact(), "20220101T010203Z");
    }

    #[test]
    fn sha256_of_empty_input_is_the_standard_value() {
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn version_tokens_need_three_numeric_parts() {
        assert_eq!(
            first_version("clippy 0.1.98 (48a229ceae 2026-09-01)"),
            Some("0.1.98")
        );
        assert_eq!(first_version("typos-cli 1.40.0"), Some("1.40.0"));
        assert_eq!(first_version("cargo-deny 0.20.2"), Some("0.20.2"));
        assert_eq!(first_version("1.98"), None);
        assert_eq!(first_version("date 2026-09-01"), None);
    }
}
