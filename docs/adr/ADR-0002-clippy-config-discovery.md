# ADR-0002: Clippy configuration discovery and the core-crate template

- **Status:** proposed in CHG-001, revised in CHG-001.1 and CHG-001.2; accepted when Kennedy accepts CHG-001.
- **Date:** 2026-09-19
- **Environment:** `clippy 0.1.98 (48a229ceae 2026-09-01)`, rustc 1.98.1, Linux (WSL2). Every experiment runs with `CLIPPY_CONF_DIR`, `RUSTFLAGS`, `CARGO_ENCODED_RUSTFLAGS`, and `CARGO_BUILD_RUSTFLAGS` unset, unless it sets one on purpose. The same runs happen on GitHub `ubuntu24` in `L0.nextest`.

## Context

Spec §6.8 gives core crates a `clippy.toml` deny list of ambient-effect paths and says: "Verify how the pinned Clippy locates and merges configuration files." The Clippy book says the search starts at `CLIPPY_CONF_DIR`, else `CARGO_MANIFEST_DIR`, else the current directory, and walks up until one file is found; it does not document merging. The plan expected "nearest file wins, no merge" and named a fallback, `CLIPPY_CONF_DIR` set per crate by `xtask ci`, in case the observation disagreed.

## Observations

Each experiment runs in a fixture workspace that [`xtask/tests/clippy_corpus.rs`](../../xtask/tests/clippy_corpus.rs) generates under `target/clippy-corpus/` from the three files that own the facts involved: the root `Cargo.toml` (`[workspace.package]` and the lint tables), the root `clippy.toml`, and [the template](../../xtask/templates/core-clippy.toml). Only the crate sources are committed, in [`xtask/tests/corpus/clippy/`](../../xtask/tests/corpus/clippy/). The fixture is emptied first, so no cached build stands in for a fresh one. The same file runs the experiments in every `L0.nextest` run and, with `RHA_CLIPPY_RECORD=<dir>`, writes one record per experiment: the generated files with their digests, the literal command, the exit status, and the captured output. The records of these experiments are in [`evidence/w1-clippy/20260919T221927Z-edd4bac756f9/`](../../evidence/w1-clippy/20260919T221927Z-edd4bac756f9/). Seven loose files beside that directory are the first run, at `28ae543`, written by the shell script CHG-001.1 replaced; they name the committed fixtures of the time, which no longer exist.

**Discovery and merging.** Crates marked *template* hold a byte-equal copy of the template; *reduced* holds the template without the settings the root file sets.

| # | Crate and command | Observed |
| --- | --- | --- |
| 1 | core, template, calls `std::time::SystemTime::now()`: `cargo clippy -p core-seeded` | Exit 101. ``error: use of a disallowed method `std::time::SystemTime::now` `` and `` = note: `-D clippy::disallowed-methods` implied by `-D clippy::all` ``. |
| 2 | adapter, no local file, same call: `cargo clippy -p adapter-x` | Exit 0, no finding. |
| 3 | core, reduced, a test calls `unwrap()`: `cargo clippy -p core-deny-only --all-targets` | Exit 0 with ``warning: used `unwrap()` on a `Result` value``: **`unwrap_used` fired**, although the fixture root sets `allow-unwrap-in-tests = true`. |
| 3c | control for 3: the same test `unwrap()` in the adapter, whose nearest file is the fixture root | Exit 0, no `unwrap_used`: the root allowance works where the root file is the nearest. |
| 3b | core, template, a test calls `unwrap()`: `cargo clippy -p core-clean --all-targets` | Exit 0, no `unwrap_used`: the template's repeat of the root settings restores the allowance. |
| 2-control | core, template, the same call as 2: `cargo clippy -p core-seeded` | Exit 101 with the disallowed-method error: the deny list does fire in this fixture, which is what makes 2 and 3c meaningful. |
| 4 | core, template, one use of every template entry: `cargo clippy -p core-every` | Exit 101; every entry reported as `use of a disallowed method/type/macro`, 122 findings for 111 entries, because a use that names a type and calls one of its methods fires both. No unresolved-path warning. |
| 5 | core, `clippy.toml` the template and `.clippy.toml` the root file, calls the clock | Exit 0 and no finding, with ``warning: using config file `….clippy.toml`, `…clippy.toml` will be ignored``: the dotfile **silently replaces the deny list**. |

**Lint attributes and flags (experiment 6).** Every crate holds the template. 6a–6e use the repository's lint table; 6f–6n use one where `disallowed_methods`, `disallowed_types`, and `disallowed_macros` are `forbid`.

| # | Crate or flag | Observed |
| --- | --- | --- |
| 6-control | no attribute and no flag: `cargo clippy -p core-seeded` | Exit 101 with the disallowed-method error, so the silent runs below mean something. |
| 6a | `#[allow(clippy::disallowed_methods)]` on the item | Exit 0, no finding: the deny list is off for that item. |
| 6b | `#[expect(clippy::disallowed_methods)]` on the item | Exit 0, no finding. |
| 6c | `#![allow(clippy::disallowed_methods)]` at the crate root | Exit 0, no finding. |
| 6d | `#![forbid(...)]` at the crate root above an item `#[allow]` | Exit 101, `error[E0453]: allow(clippy::disallowed_methods) incompatible with previous forbid`. |
| 6e | no attribute, `RUSTFLAGS=-Aclippy::disallowed_methods` | Exit 0, no finding. |
| 6f, 6g, 6h | the three attribute forms of 6a, 6b, 6c, under the forbidding lint table | Exit 101, `E0453` in each. |
| 6i | no attribute, under forbid | Exit 101, the ordinary disallowed-method error. |
| 6j | core crate with nothing on the deny list, under forbid | Exit 0: no false alarm. |
| 6k | adapter making the same call, under forbid | Exit 0: adapters are unaffected, because their configuration lists no paths. |
| 6l | no attribute, under forbid, `RUSTFLAGS=-Aclippy::disallowed_methods` | Exit 101, the error stands: `forbid` is not lowered by an allow flag. |
| 6m | no attribute, under forbid, `RUSTFLAGS=--cap-lints=warn` | Exit 0, the finding appears as a warning. |
| 6n | the same `--cap-lints=warn` in the fixture's `.cargo/config.toml` | Exit 0, the finding appears as a warning. |
| 7a | a crate whose macro expands to `#[allow(clippy::style)]`, under the forbidding lint table | Exit 101, `E0453`: a group allow a macro wrote is refused exactly as one written by hand. |
| 7b | the same crate under the deny level | Exit 0. |
| 7c | a core crate with `#![forbid(...)]` at its own root and nothing on the deny list | Exit 0: the per-crate route costs clean code nothing. |

## Observed rule

This record owns these observations (§11.7.1). The specification states the consequence and points here; what a finding would change in its text is queued in [`docs/spec/proposals/v0.11.md`](../spec/proposals/v0.11.md) for the version bump.

1. **Per-crate discovery.** Clippy configures each crate from the first configuration file found in the crate's directory or its ancestors (1, 2, 3c). A core crate's local file does not apply to a sibling adapter.
2. **No merge.** The file found replaces every file further up; a crate with its own file loses the root file's settings (3; control 3c; 3b).
3. **Dotfile precedence.** In one directory, `.clippy.toml` wins over `clippy.toml`. The warning does not change the exit status, so a deny list can be replaced silently (5).
4. **Deny level.** Under the workspace lint table (`clippy::all` at deny), every template entry resolves and fails the build (1, 4).
5. **Attributes outrank the configuration file.** The file decides *which paths* are reported; the lint level decides *whether* a report is an error, and `#[allow]`, `#[expect]`, and `#![allow]` in the crate set that level locally (6a, 6b, 6c). A `forbid` above them, from the crate root or the lint table, makes each of those attributes `E0453` (6d, 6f, 6g, 6h) without changing anything else (6i, 6j, 6k).
6. **A level is not a ceiling.** `--cap-lints=warn`, from `RUSTFLAGS` or from `.cargo/config.toml`, lowers a forbidden finding to a warning (6m, 6n). An `-A` flag does not (6l).

8. **A forbidding lint table refuses macro-generated group allows.** Where any macro in the crate expands to `#[allow(clippy::style)]`, forbidding a lint of that group is `E0453` (7a), and at the deny level the same crate is silent (7b). This is not hypothetical: setting the three lints to `forbid` in this repository's own `Cargo.toml` stopped `xtask` compiling with 15 such errors, all of them from `clap`'s `derive(Parser)` and `derive(Subcommand)`. A `serde` derive does not do this, so a core crate can forbid the lints at its own root (7c).
7. **A type entry fires where the type is named.** It catches a type in a signature, a type annotation, or a qualified path such as `File::open`, but not a value a dependency hands back, and not a bare unit-struct value: `let a = std::alloc::System;` is silent where `let a: std::alloc::System = …;` fires. The effectful method of each such type is therefore listed beside it.

Rules 1 and 2 are what the plan pre-registered as its expectation, and both hold, so the `CLIPPY_CONF_DIR` fallback is not used. Rule 3 answers the plan's listed Unknown, and its answer is the outcome the plan told the Executor to avoid rather than one it expected. Rules 4 to 7 were not pre-registered.

**Not observed.** Whether the upward search stops at the workspace root or continues to the filesystem root: in every fixture a file exists at the workspace root. `CLIPPY_CONF_DIR` itself. Whether the attribute results carry to `disallowed_types` and `disallowed_macros`: 6a to 6n name `clippy::disallowed_methods` only, although all three lints were forbidden in the second fixture. Other Clippy versions: the corpus reruns every experiment whenever the toolchain changes.

## Decision

- Every crate with role `core` has a `clippy.toml` byte-equal to [`xtask/templates/core-clippy.toml`](../../xtask/templates/core-clippy.toml). Adapters, apps, and tools have none and use the root file. The template holds the §6.8 entries, the plan's W1 additions, the entries added in CHG-001.2, and, because of rule 2, every setting of the root `clippy.toml`.
- Rule `effect.core_clippy_template` reports, per core crate, a missing `clippy.toml`, one that differs from the template, and a `.clippy.toml` beside it (rule 3). Its witness is the path and the sha256 digests. It is implemented in [`xtask/src/clippy_template.rs`](../../xtask/src/clippy_template.rs) and runs in L0 once `cargo xtask architecture` exists (CHG-003).
- Fixtures are generated, not committed (CHG-001.1). A committed fixture is a copy of the root lint table, the root `clippy.toml`, or the template, and copies need drift tests to stay true. The one copy left is the template's repeat of the root settings, which rule 2 forces; `template_repeats_every_root_setting` guards it.
- The deny list is extended in CHG-001.2 from 15 entries to 111, and experiment 4 keeps every entry honest: a path that stops resolving stops firing, and the test fails. The entries come from the ten paths this record named in CHG-001, a sweep of the Rust 1.98.1 standard-library source, and two adversarial reviews of that sweep, all kept to the effects §6.8 names.

| Category | Entries | Examples |
| --- | --- | --- |
| Clocks | 4 | `SystemTime::now`, `Instant::elapsed` |
| Environment, arguments, working directory | 16 | `env::var_os`, `env::args`, `env::current_dir`, `path::absolute`, `IsTerminal::is_terminal` |
| Storage | 36 | 19 `std::fs` functions, `OpenOptions::open`, `DirBuilder::create`, the 10 `Path` methods that touch the filesystem, and the types `File`, `OpenOptions`, `DirBuilder`, `ReadDir`, `DirEntry` |
| Network | 4 | the types `TcpStream`, `TcpListener`, `UdpSocket`, and `ToSocketAddrs::to_socket_addrs` |
| Process, process-global state, telemetry | 18 | the types `Command`, `Child` and `alloc::System`, the effectful methods of the first two, `process::exit`, `abort`, `id`, `panic::set_hook`, `take_hook`, `Backtrace::capture`, `force_capture`, `alloc::handle_alloc_error` |
| Concurrency and scheduling | 20 | `thread::spawn`, `scope`, `sleep`, `park`, `current`, the `Builder` type, and the blocking parts of `std::sync`: `mpsc::channel`, `sync_channel`, the `Receiver` type with `recv` and `recv_timeout`, `Condvar` with `wait` and `wait_timeout`, `Barrier`, `Once::wait` |
| Randomness | 1 | `hash::RandomState` |
| Input and output | 12 | `io::stdin`, `stdout`, `stderr`, `pipe`, the `Stdin`, `Stdout` and `Stderr` types, and the five print macros |

  A type entry replaces the narrower constructor entry it subsumes, as `std::process::Command` replaced `Command::new`; by rule 7 the effectful methods of those types are listed as well, so a value obtained from elsewhere is still caught.

  **Candidates this change did not take**, each verified to resolve and fire, each a judgment about cost in ordinary code rather than about whether it is an effect: `Mutex::lock`, `RwLock::read` and `write`; the atomics; `Once::call_once`, `OnceLock::get_or_init` and `set`, `LazyLock`; `Condvar::wait_while` and `wait_timeout_while`, `Once::wait_force`, `mpsc::Sender::send`, `SyncSender::send`, `Receiver::iter`; `panic::panic_any`; the `Backtrace` type, whose `Display` impl formats the captured stack; and the build-time filesystem reads `include_str!` and `include_bytes!`. Extending the template is a protected change for Kennedy.
- The attribute escape of rule 5 is closed one crate at a time, not workspace-wide. Every core crate carries `#![forbid(clippy::disallowed_methods, clippy::disallowed_types, clippy::disallowed_macros)]` at its crate root, which refuses an item-level allow (6d) and costs clean code nothing (7c). The root `Cargo.toml` keeps the deny level, because rule 8 makes a forbidding table incompatible with `clap`; it carries a comment saying so.
- `effect.core_clippy_template` therefore reports a fourth problem, `unforbidden`, naming the lints a crate root leaves out. The check is a text scan of the crate root for `#![forbid(...)]`; it does not understand a group forbid such as `forbid(clippy::all)`, and it would be fooled by the text inside a comment.
- `.cargo/config.toml` is a protected surface from CHG-001.3, because `--cap-lints=warn` there disables every lint at once (6n).

## Consequences

- A change to the root `clippy.toml` needs the same change in the template and in every core crate's copy. The root file and the template are protected surfaces; the crates' copies are not, but the rule reports any copy that differs. This costs more edits than `CLIPPY_CONF_DIR`, but it keeps §12.1's commands and environment unchanged.
- A `.clippy.toml` at the repository root would shadow the root `clippy.toml` for every crate without its own file. The rule checks core crate directories only, so this remains a review item.
- **The deny list sees direct uses only** (§6.8). It does not see a call through a helper in another crate, a call the compiler generates from a macro, or an effect reached through a trait object. Experiment 4 proves each entry fires on a direct use, and nothing more.
- **What the list leaves out, on purpose.** `HashMap::new` and `HashMap::with_capacity` seed themselves from `RandomState`, which the list catches only when a crate names it; iteration order stays a source of nondeterminism that a deny list cannot reach. Platform extension traits, such as `std::os::unix::process::CommandExt::exec` and `std::os::unix::process::parent_id`, are effectful but would not resolve on another target, so listing them would trade a real check for a portability warning. Unstable APIs, such as `std::random`, are left out until they are stable. Each is a candidate for a later change, not an oversight.
- **A core crate cannot allow these lints anywhere, including its own tests.** That is the point of the crate-root `forbid`, and it is also its cost: a core-crate test that wants a real clock has to take one through a port, like the code it tests.
- **Four ways to switch it off** are now recorded. An attribute in the crate (rule 5), unless the lint table forbids the lint. An allow flag in `RUSTFLAGS`, which works at the repository's current deny level (6e) and stops working under `forbid` (6l). `--cap-lints=warn` from `RUSTFLAGS` or `.cargo/config.toml` (rule 6), which no lint level survives; `.cargo/config.toml` is not on the protected list in `.rha/policy.toml`. And a crate whose `clippy.toml` is missing, edited, or shadowed, which is what `effect.core_clippy_template` reports.
- Until CHG-003 wires the rule in and CHG-005 creates core crates, the template protects no product code: the deny list is demonstrated on fixtures only.
