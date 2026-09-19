//! The Clippy corpus of CHG-001 (ADR-0002), rerun on every L0 run so a
//! toolchain change re-checks the configuration rules the deny list relies on.
//!
//! No fixture file is a copy. Each test generates its fixture workspace under
//! `target/clippy-corpus/` from the files that own each fact: the root
//! `Cargo.toml` (`[workspace.package]` and the lint tables), the root
//! `clippy.toml`, and the core template. Only the crate sources are committed,
//! in `tests/corpus/clippy/`. A fixture is emptied and regenerated before its
//! first command, so no cached build stands in for a fresh one.
//!
//! With `RHA_CLIPPY_RECORD=<dir>` set, every experiment also writes a record,
//! `<dir>/<id>.txt`: the generated files with their digests, the command, the
//! Clippy version, the revision, the exit status, stderr, and stdout. A
//! relative `<dir>` is taken from the workspace root.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use toml::{Table, Value};
use xtask::util::{UtcTime, command, command_stdout, sha256_hex};

const TEMPLATE: &str = "xtask/templates/core-clippy.toml";
const SOURCES: &str = "xtask/tests/corpus/clippy";
const RECORD_VAR: &str = "RHA_CLIPPY_RECORD";

/// Variables that would change what Clippy reads or how it reports; every
/// experiment runs without them unless it sets one on purpose.
const CLEARED: [&str; 4] = [
    "CLIPPY_CONF_DIR",
    "RUSTFLAGS",
    "CARGO_ENCODED_RUSTFLAGS",
    "CARGO_BUILD_RUSTFLAGS",
];

const UNWRAP_FINDING: &str = "used `unwrap()` on a `Result` value";

fn root() -> PathBuf {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest_dir.parent().unwrap_or(manifest_dir).to_path_buf()
}

fn read(path: &Path) -> Vec<u8> {
    std::fs::read(path).unwrap_or_else(|e| panic!("reading {}: {e}", path.display()))
}

fn parse(bytes: &[u8], what: &str) -> Table {
    let text = std::str::from_utf8(bytes).unwrap_or_else(|e| panic!("{what} is not UTF-8: {e}"));
    toml::from_str(text).unwrap_or_else(|e| panic!("parsing {what}: {e}"))
}

fn toml_file(rel: &str) -> Table {
    parse(&read(&root().join(rel)), rel)
}

/// How a generated crate's Clippy configuration is made.
#[derive(Debug, Clone, Copy)]
enum Config {
    /// No file in the crate: the fixture root's `clippy.toml` is the nearest.
    Inherit,
    /// `clippy.toml` is the template, byte for byte.
    Template,
    /// `clippy.toml` is the template without the keys the root file sets.
    TemplateWithoutRootKeys,
    /// `clippy.toml` is the template, and `.clippy.toml` is the root file.
    TemplateAndDotfile,
}

/// A generated crate: `source` is a file in `tests/corpus/clippy/`, used as
/// `src/lib.rs`.
#[derive(Debug, Clone, Copy)]
struct Crate {
    name: &'static str,
    role: &'static str,
    source: &'static str,
    config: Config,
}

const fn core(name: &'static str, source: &'static str, config: Config) -> Crate {
    Crate {
        name,
        role: "core",
        source,
        config,
    }
}

const fn adapter(name: &'static str, source: &'static str) -> Crate {
    Crate {
        name,
        role: "adapter",
        source,
        config: Config::Inherit,
    }
}

/// A generated fixture workspace and the digest of every file written.
struct Fixture {
    dir: PathBuf,
    files: Vec<(String, String)>,
}

impl Fixture {
    /// Writes `target/clippy-corpus/<name>` from the owning files;
    /// `lint_overrides` are added to the copied `[workspace.lints.clippy]`.
    fn generate(name: &str, lint_overrides: &[(&str, &str)], crates: &[Crate]) -> Self {
        let dir = root().join("target/clippy-corpus").join(name);
        if dir.exists() {
            std::fs::remove_dir_all(&dir)
                .unwrap_or_else(|e| panic!("emptying {}: {e}", dir.display()));
        }
        let mut fixture = Self {
            dir,
            files: Vec::new(),
        };
        let root_clippy = read(&root().join("clippy.toml"));
        let template = read(&root().join(TEMPLATE));
        fixture.write("Cargo.toml", &workspace_manifest(lint_overrides));
        fixture.write("clippy.toml", &root_clippy);
        for krate in crates {
            let base = format!("crates/{}", krate.name);
            let manifest = format!(
                "[package]\nname = \"{}\"\nversion = \"0.0.0\"\nedition.workspace = true\n\
                 rust-version.workspace = true\npublish = false\n\n\
                 [package.metadata.rha]\nrole = \"{}\"\n\n[lints]\nworkspace = true\n",
                krate.name, krate.role
            );
            fixture.write(&format!("{base}/Cargo.toml"), manifest.as_bytes());
            let source = read(&root().join(SOURCES).join(krate.source));
            fixture.write(&format!("{base}/src/lib.rs"), &source);
            let local = format!("{base}/clippy.toml");
            match krate.config {
                Config::Inherit => {}
                Config::Template => fixture.write(&local, &template),
                Config::TemplateWithoutRootKeys => {
                    let root_keys = parse(&root_clippy, "clippy.toml");
                    let mut reduced = parse(&template, TEMPLATE);
                    reduced.retain(|key, _| !root_keys.contains_key(key));
                    let text = toml::to_string(&reduced)
                        .unwrap_or_else(|e| panic!("serializing the reduced template: {e}"));
                    fixture.write(&local, text.as_bytes());
                }
                Config::TemplateAndDotfile => {
                    fixture.write(&local, &template);
                    fixture.write(&format!("{base}/.clippy.toml"), &root_clippy);
                }
            }
        }
        fixture
    }

    fn write(&mut self, rel: &str, bytes: &[u8]) {
        let path = self.dir.join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .unwrap_or_else(|e| panic!("creating {}: {e}", parent.display()));
        }
        std::fs::write(&path, bytes).unwrap_or_else(|e| panic!("writing {}: {e}", path.display()));
        self.files.push((rel.to_owned(), sha256_hex(bytes)));
    }

    /// Runs `cargo clippy ARGS` in the fixture with `env` set on top of a
    /// cleared environment, and records the run when recording is on.
    fn clippy(&self, id: &str, purpose: &str, args: &[&str], env: &[(&str, &str)]) -> Run {
        let mut cmd = command("cargo");
        cmd.arg("clippy")
            .args(args)
            .current_dir(&self.dir)
            .env("CARGO_TARGET_DIR", self.dir.join("target"))
            .env("CARGO_TERM_COLOR", "never");
        for var in CLEARED {
            cmd.env_remove(var);
        }
        cmd.envs(env.iter().copied());
        let output = cmd
            .output()
            .unwrap_or_else(|e| panic!("running cargo clippy in {}: {e}", self.dir.display()));
        let run = Run {
            id: id.to_owned(),
            status: output.status.code().unwrap_or(-1),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        };
        if let Some(dir) = std::env::var_os(RECORD_VAR) {
            self.record(&root().join(dir), purpose, args, env, &run);
        }
        run
    }

    fn record(&self, dir: &Path, purpose: &str, args: &[&str], env: &[(&str, &str)], run: &Run) {
        let git = |argv: &[&str]| command_stdout(&root(), argv).unwrap_or_default();
        let revision = git(&["git", "rev-parse", "HEAD"]);
        let dirty = if git(&["git", "status", "--porcelain"]).is_empty() {
            "no"
        } else {
            "yes"
        };
        let clippy = command_stdout(&self.dir, &["cargo", "clippy", "--version"])
            .unwrap_or_else(|e| format!("unknown ({e})"));
        let fixture = self.dir.strip_prefix(root()).unwrap_or(&self.dir).display();
        let mut files = String::new();
        for (path, digest) in &self.files {
            let _ = writeln!(files, "  {digest}  {path}");
        }
        let mut set = String::new();
        for (key, value) in env {
            let _ = write!(set, "; set {key}={value}");
        }
        let text = format!(
            "experiment:   {id}\npurpose:      {purpose}\n\
             fixture:      {fixture} (generated by xtask/tests/clippy_corpus.rs; emptied first)\n\
             generated:\n{files}\
             environment:  unset {cleared}; CARGO_TARGET_DIR={fixture}/target{set}\n\
             command:      cargo clippy {args}\nclippy:       {clippy}\n\
             revision:     {revision} (working tree dirty: {dirty})\n\
             recorded_at:  {at}\nexit_status:  {status}\n\
             --- stderr ---\n{stderr}--- stdout ---\n{stdout}",
            id = run.id,
            cleared = CLEARED.join(", "),
            args = args.join(" "),
            at = UtcTime::now().rfc3339(),
            status = run.status,
            stderr = run.stderr,
            stdout = run.stdout,
        );
        std::fs::create_dir_all(dir).unwrap_or_else(|e| panic!("creating {}: {e}", dir.display()));
        let path = dir.join(format!("{}.txt", run.id));
        std::fs::write(&path, text).unwrap_or_else(|e| panic!("writing {}: {e}", path.display()));
    }
}

/// One Clippy run, with chainable assertions that print stderr on failure.
struct Run {
    id: String,
    status: i32,
    stderr: String,
    stdout: String,
}

impl Run {
    #[track_caller]
    fn exits_zero(&self) -> &Self {
        assert_eq!(self.status, 0, "experiment {}:\n{}", self.id, self.stderr);
        self
    }

    #[track_caller]
    fn exits_nonzero(&self) -> &Self {
        assert_ne!(self.status, 0, "experiment {}:\n{}", self.id, self.stderr);
        self
    }

    #[track_caller]
    fn reports(&self, needle: &str) -> &Self {
        assert!(
            self.stderr.contains(needle),
            "experiment {}: expected {needle:?} in:\n{}",
            self.id,
            self.stderr
        );
        self
    }

    #[track_caller]
    fn silent(&self, needle: &str) -> &Self {
        assert!(
            !self.stderr.contains(needle),
            "experiment {}: unexpected {needle:?} in:\n{}",
            self.id,
            self.stderr
        );
        self
    }
}

/// The root manifest's `[workspace.package]` and `[workspace.lints]`, with
/// `lint_overrides` added to the Clippy table, as a fixture workspace root.
fn workspace_manifest(lint_overrides: &[(&str, &str)]) -> Vec<u8> {
    let manifest = toml_file("Cargo.toml");
    let workspace = &manifest["workspace"];
    let mut lints = workspace["lints"].clone();
    for (lint, level) in lint_overrides {
        lints["clippy"]
            .as_table_mut()
            .unwrap_or_else(|| panic!("root Cargo.toml has no [workspace.lints.clippy]"))
            .insert((*lint).to_owned(), (*level).into());
    }
    let mut table = Table::new();
    table.insert("members".into(), Value::Array(vec!["crates/*".into()]));
    table.insert("resolver".into(), "3".into());
    table.insert("package".into(), workspace["package"].clone());
    table.insert("lints".into(), lints);
    let doc = Table::from_iter([("workspace".to_owned(), Value::Table(table))]);
    toml::to_string(&doc)
        .unwrap_or_else(|e| panic!("serializing a fixture manifest: {e}"))
        .into_bytes()
}

/// Each path of the template's three lists, with its list name.
fn template_entries() -> Vec<(String, String)> {
    let template = toml_file(TEMPLATE);
    let mut entries = Vec::new();
    for list in [
        "disallowed-methods",
        "disallowed-types",
        "disallowed-macros",
    ] {
        for item in template[list].as_array().into_iter().flatten() {
            let path = item["path"].as_str().unwrap_or_default();
            entries.push((list.to_owned(), path.to_owned()));
        }
    }
    entries
}

#[test]
fn template_repeats_every_root_setting() {
    let template = toml_file(TEMPLATE);
    for (key, value) in &toml_file("clippy.toml") {
        assert_eq!(
            template.get(key),
            Some(value),
            "template must repeat root setting {key}"
        );
    }
}

#[test]
fn discovery_is_per_crate_and_the_nearest_file_is_not_merged() {
    assert_eq!(
        toml_file("clippy.toml").get("allow-unwrap-in-tests"),
        Some(&Value::Boolean(true)),
        "experiments 3 and 3c need the root allowance"
    );
    let fixture = Fixture::generate(
        "discovery",
        &[],
        &[
            core(
                "core-deny-only",
                "test_unwrap.rs",
                Config::TemplateWithoutRootKeys,
            ),
            adapter("adapter-x", "adapter_clock.rs"),
        ],
    );

    fixture
        .clippy(
            "2",
            "an adapter with no clippy.toml of its own makes the seeded call",
            &["-p", "adapter-x"],
            &[],
        )
        .exits_zero()
        .silent("disallowed");

    fixture
        .clippy(
            "3",
            "the core file lacks allow-unwrap-in-tests, which the root file sets; test unwrap()",
            &["-p", "core-deny-only", "--all-targets"],
            &[],
        )
        .exits_zero()
        .reports(UNWRAP_FINDING);

    fixture
        .clippy(
            "3c",
            "control for 3: the same test unwrap() in an adapter that uses the root file",
            &["-p", "adapter-x", "--all-targets"],
            &[],
        )
        .exits_zero()
        .silent(UNWRAP_FINDING);
}

#[test]
fn the_template_denies_every_entry_and_keeps_test_allowances() {
    let fixture = Fixture::generate(
        "template",
        &[],
        &[
            core("core-seeded", "seeded_clock.rs", Config::Template),
            core("core-clean", "test_unwrap.rs", Config::Template),
            core("core-every", "every_entry.rs", Config::Template),
        ],
    );

    fixture
        .clippy(
            "1",
            "a core crate with the template calls std::time::SystemTime::now()",
            &["-p", "core-seeded"],
            &[],
        )
        .exits_nonzero()
        .reports("clippy::disallowed_methods")
        .reports("`std::time::SystemTime::now`");

    fixture
        .clippy(
            "3b",
            "the template repeats the root settings; nothing on the deny list; test unwrap()",
            &["-p", "core-clean", "--all-targets"],
            &[],
        )
        .exits_zero()
        .silent(UNWRAP_FINDING);

    let run = fixture.clippy(
        "4",
        "one use of every template entry in a crate with the template",
        &["-p", "core-every"],
        &[],
    );
    run.exits_nonzero();
    for (list, path) in template_entries() {
        let fired = run
            .stderr
            .lines()
            .any(|l| l.contains("use of a disallowed") && l.contains(&format!("`{path}`")));
        assert!(fired, "{list} entry {path} did not fire:\n{}", run.stderr);
    }
}

#[test]
fn a_dotfile_beside_the_template_silently_replaces_it() {
    let fixture = Fixture::generate(
        "dotfile",
        &[],
        &[core(
            "core-pair",
            "seeded_clock.rs",
            Config::TemplateAndDotfile,
        )],
    );

    fixture
        .clippy(
            "5",
            "clippy.toml is the template, .clippy.toml the root file; the seeded call",
            &["-p", "core-pair"],
            &[],
        )
        .exits_zero()
        .silent("disallowed")
        .reports("using config file")
        .reports("clippy.toml` will be ignored");
}

/// The three lints the template configures, set to `forbid` in the lint table
/// of the second fixture below.
const DENY_LINTS: [(&str, &str); 3] = [
    ("disallowed_methods", "forbid"),
    ("disallowed_types", "forbid"),
    ("disallowed_macros", "forbid"),
];

const INCOMPATIBLE: &str = "incompatible with previous forbid";
const SEEDED_FINDING: &str = "use of a disallowed method `std::time::SystemTime::now`";

/// Experiment 6: a lint attribute in the crate switches the deny list off,
/// and the deny level does not stop it. `--cap-lints` is the boundary of what
/// any lint level can promise.
#[test]
fn a_lint_attribute_switches_the_deny_list_off() {
    let fixture = Fixture::generate(
        "escape",
        &[],
        &[
            core("core-allow-item", "allow_attribute.rs", Config::Template),
            core("core-expect-item", "expect_attribute.rs", Config::Template),
            core("core-allow-crate", "allow_crate.rs", Config::Template),
            core("core-forbid-crate", "forbid_crate.rs", Config::Template),
            core("core-seeded", "seeded_clock.rs", Config::Template),
        ],
    );

    fixture
        .clippy(
            "6a",
            "the seeded call under an item-level #[allow]",
            &["-p", "core-allow-item"],
            &[],
        )
        .exits_zero()
        .silent("disallowed");

    fixture
        .clippy(
            "6b",
            "the seeded call under an item-level #[expect]",
            &["-p", "core-expect-item"],
            &[],
        )
        .exits_zero()
        .silent("disallowed");

    fixture
        .clippy(
            "6c",
            "the seeded call under a crate-level #![allow]",
            &["-p", "core-allow-crate"],
            &[],
        )
        .exits_zero()
        .silent("disallowed");

    fixture
        .clippy(
            "6d",
            "a crate-level #![forbid] above an item-level #[allow]",
            &["-p", "core-forbid-crate"],
            &[],
        )
        .exits_nonzero()
        .reports(INCOMPATIBLE);

    fixture
        .clippy(
            "6e",
            "the seeded call with the lint allowed through RUSTFLAGS",
            &["-p", "core-seeded"],
            &[("RUSTFLAGS", "-Aclippy::disallowed_methods")],
        )
        .exits_zero()
        .silent("disallowed");
}

/// Experiment 6, continued: the same crates under a lint table that forbids
/// the three lints the template configures.
#[test]
fn forbid_in_the_lint_table_closes_the_attribute_escape() {
    let fixture = Fixture::generate(
        "escape-forbid",
        &DENY_LINTS,
        &[
            core("core-allow-item", "allow_attribute.rs", Config::Template),
            core("core-expect-item", "expect_attribute.rs", Config::Template),
            core("core-allow-crate", "allow_crate.rs", Config::Template),
            core("core-seeded", "seeded_clock.rs", Config::Template),
            core("core-clean", "test_unwrap.rs", Config::Template),
            adapter("adapter-x", "adapter_clock.rs"),
        ],
    );

    for (id, krate) in [
        ("6f", "core-allow-item"),
        ("6g", "core-expect-item"),
        ("6h", "core-allow-crate"),
    ] {
        fixture
            .clippy(
                id,
                "the attribute escape under a forbidding lint table",
                &["-p", krate],
                &[],
            )
            .exits_nonzero()
            .reports(INCOMPATIBLE);
    }

    fixture
        .clippy(
            "6i",
            "the seeded call with no attribute, under forbid",
            &["-p", "core-seeded"],
            &[],
        )
        .exits_nonzero()
        .reports(SEEDED_FINDING);

    fixture
        .clippy(
            "6j",
            "a core crate with nothing on the deny list, under forbid",
            &["-p", "core-clean", "--all-targets"],
            &[],
        )
        .exits_zero();

    fixture
        .clippy(
            "6k",
            "an adapter making the seeded call, under forbid",
            &["-p", "adapter-x", "--all-targets"],
            &[],
        )
        .exits_zero();

    fixture
        .clippy(
            "6l",
            "forbid against an allow in RUSTFLAGS",
            &["-p", "core-seeded"],
            &[("RUSTFLAGS", "-Aclippy::disallowed_methods")],
        )
        .exits_nonzero()
        .reports(SEEDED_FINDING);

    fixture
        .clippy(
            "6m",
            "forbid against --cap-lints in RUSTFLAGS",
            &["-p", "core-seeded"],
            &[("RUSTFLAGS", "--cap-lints=warn")],
        )
        .exits_zero()
        .reports(&format!("warning: {SEEDED_FINDING}"));
}

/// Experiment 6n: `.cargo/config.toml` is read from the directory of the
/// build upward, and it is not a protected surface in this repository.
#[test]
fn a_cargo_config_can_lower_every_lint() {
    let mut fixture = Fixture::generate(
        "cargo-config",
        &DENY_LINTS,
        &[core("core-seeded", "seeded_clock.rs", Config::Template)],
    );
    fixture.write(
        ".cargo/config.toml",
        b"[build]\nrustflags = [\"--cap-lints=warn\"]\n",
    );

    fixture
        .clippy(
            "6n",
            "the seeded call with --cap-lints=warn in the fixture's .cargo/config.toml",
            &["-p", "core-seeded"],
            &[],
        )
        .exits_zero()
        .reports(&format!("warning: {SEEDED_FINDING}"));
}
