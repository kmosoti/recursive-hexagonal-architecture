# ADR-0002: Clippy configuration discovery and the core-crate template

- **Status:** proposed in CHG-001, revised in CHG-001.1 and CHG-001.2; accepted when Kennedy accepts CHG-001.
- **Date:** 2026-09-19
- **Environment:** `clippy 0.1.98 (48a229ceae 2026-09-01)`, rustc 1.98.1, Linux (WSL2). Every experiment runs with `CLIPPY_CONF_DIR`, `RUSTFLAGS`, `CARGO_ENCODED_RUSTFLAGS`, and `CARGO_BUILD_RUSTFLAGS` unset, unless it sets one on purpose. The same runs happen on GitHub `ubuntu24` in `L0.nextest`.

## Context

Spec §6.8 gives core crates a `clippy.toml` deny list of ambient-effect paths and says: "Verify how the pinned Clippy locates and merges configuration files." The Clippy book says the search starts at `CLIPPY_CONF_DIR`, else `CARGO_MANIFEST_DIR`, else the current directory, and walks up until one file is found; it does not document merging. The plan expected "nearest file wins, no merge" and named a fallback, `CLIPPY_CONF_DIR` set per crate by `xtask ci`, in case the observation disagreed.

## Observations

Each experiment runs in a fixture workspace that [`xtask/tests/clippy_corpus.rs`](../../xtask/tests/clippy_corpus.rs) generates under `target/clippy-corpus/` from the three files that own the facts involved: the root `Cargo.toml` (`[workspace.package]` and the lint tables), the root `clippy.toml`, and [the template](../../xtask/templates/core-clippy.toml). Only the crate sources are committed, in [`xtask/tests/corpus/clippy/`](../../xtask/tests/corpus/clippy/). The fixture is emptied first, so no cached build stands in for a fresh one. The same file runs the experiments in every `L0.nextest` run and, with `RHA_CLIPPY_RECORD=<dir>`, writes one record per experiment: the generated files with their digests, the literal command, the exit status, and the captured output. The records are in [`evidence/w1-clippy/`](../../evidence/w1-clippy/).

**Discovery and merging.** Crates marked *template* hold a byte-equal copy of the template; *reduced* holds the template without the settings the root file sets.

| # | Crate and command | Observed |
| --- | --- | --- |
| 1 | core, template, calls `std::time::SystemTime::now()`: `cargo clippy -p core-seeded` | Exit 101. ``error: use of a disallowed method `std::time::SystemTime::now` `` and `` = note: `-D clippy::disallowed-methods` implied by `-D clippy::all` ``. |
| 2 | adapter, no local file, same call: `cargo clippy -p adapter-x` | Exit 0, no finding. |
| 3 | core, reduced, a test calls `unwrap()`: `cargo clippy -p core-deny-only --all-targets` | Exit 0 with ``warning: used `unwrap()` on a `Result` value``: **`unwrap_used` fired**, although the fixture root sets `allow-unwrap-in-tests = true`. |
| 3c | control for 3: the same test `unwrap()` in the adapter, whose nearest file is the fixture root | Exit 0, no `unwrap_used`: the root allowance works where the root file is the nearest. |
| 3b | core, template, a test calls `unwrap()`: `cargo clippy -p core-clean --all-targets` | Exit 0, no `unwrap_used`: the template's repeat of the root settings restores the allowance. |
| 4 | core, template, one use of every template entry: `cargo clippy -p core-every` | Exit 101; every entry reported as `use of a disallowed method/type/macro`. No unresolved-path warning. |
| 5 | core, `clippy.toml` the template and `.clippy.toml` the root file, calls the clock | Exit 0 and no finding, with ``warning: using config file `….clippy.toml`, `…clippy.toml` will be ignored``: the dotfile **silently replaces the deny list**. |

**Lint attributes and flags (experiment 6).** Every crate holds the template. 6a–6e use the repository's lint table; 6f–6n use one where `disallowed_methods`, `disallowed_types`, and `disallowed_macros` are `forbid`.

| # | Crate or flag | Observed |
| --- | --- | --- |
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

## Observed rule

1. **Per-crate discovery.** Clippy configures each crate from the first configuration file found in the crate's directory or its ancestors (1, 2, 3c). A core crate's local file does not apply to a sibling adapter.
2. **No merge.** The file found replaces every file further up; a crate with its own file loses the root file's settings (3; control 3c; 3b).
3. **Dotfile precedence.** In one directory, `.clippy.toml` wins over `clippy.toml`. The warning does not change the exit status, so a deny list can be replaced silently (5).
4. **Deny level.** Under the workspace lint table (`clippy::all` at deny), every template entry resolves and fails the build (1, 4).
5. **Attributes outrank the configuration file.** The file decides *which paths* are reported; the lint level decides *whether* a report is an error, and `#[allow]`, `#[expect]`, and `#![allow]` in the crate set that level locally (6a, 6b, 6c). A `forbid` above them, from the crate root or the lint table, makes each of those attributes `E0453` (6d, 6f, 6g, 6h) without changing anything else (6i, 6j, 6k).
6. **A level is not a ceiling.** `--cap-lints=warn`, from `RUSTFLAGS` or from `.cargo/config.toml`, lowers a forbidden finding to a warning (6m, 6n). An `-A` flag does not (6l).

Rules 1 to 4 match the plan's expectation, so the `CLIPPY_CONF_DIR` fallback is not used.

**Not observed.** Whether the upward search stops at the workspace root or continues to the filesystem root: in every fixture a file exists at the workspace root. `CLIPPY_CONF_DIR` itself. Whether the attribute results carry to `disallowed_types` and `disallowed_macros`: 6a to 6n name `clippy::disallowed_methods` only, although all three lints were forbidden in the second fixture. Other Clippy versions: the corpus reruns every experiment whenever the toolchain changes.

## Decision

- Every crate with role `core` has a `clippy.toml` byte-equal to [`xtask/templates/core-clippy.toml`](../../xtask/templates/core-clippy.toml). Adapters, apps, and tools have none and use the root file. The template holds the §6.8 entries, the plan's W1 additions, the entries added in CHG-001.2, and, because of rule 2, every setting of the root `clippy.toml`.
- Rule `effect.core_clippy_template` reports, per core crate, a missing `clippy.toml`, one that differs from the template, and a `.clippy.toml` beside it (rule 3). Its witness is the path and the sha256 digests. It is implemented in [`xtask/src/clippy_template.rs`](../../xtask/src/clippy_template.rs) and runs in L0 once `cargo xtask architecture` exists (CHG-003).
- Fixtures are generated, not committed (CHG-001.1). A committed fixture is a copy of the root lint table, the root `clippy.toml`, or the template, and copies need drift tests to stay true. The one copy left is the template's repeat of the root settings, which rule 2 forces; `template_repeats_every_root_setting` guards it.
- The deny list is extended in CHG-001.2 from 15 entries to 80, and experiment 4 keeps every entry honest: a path that stops resolving stops firing, and the test fails. The entries come from the ten paths this record named in CHG-001 plus a sweep of the Rust 1.98.1 standard-library source, kept to the effects §6.8 names.

| Category | Entries | Examples |
| --- | --- | --- |
| Clocks | 4 | `SystemTime::now`, `Instant::elapsed` |
| Environment, arguments, working directory | 16 | `env::var_os`, `env::args`, `env::current_dir`, `path::absolute`, `IsTerminal::is_terminal` |
| Storage | 33 | 19 `std::fs` functions, the 10 `Path` methods that touch the filesystem, and the types `File`, `OpenOptions`, `DirBuilder`, `ReadDir` |
| Network | 4 | the types `TcpStream`, `TcpListener`, `UdpSocket`, and `ToSocketAddrs::to_socket_addrs` |
| Process execution | 5 | the types `Command` and `Child`, `process::exit`, `abort`, `id` |
| Concurrency and scheduling | 8 | `thread::spawn`, the `Builder` type, `scope`, `sleep`, `park` |
| Randomness | 1 | `hash::RandomState` |
| Input and output | 9 | `io::stdin`, `stdout`, `stderr`, `pipe`, and the five print macros |

  A type entry fires on every mention, including associated functions, so `std::process::Command` replaced the narrower `Command::new`.
- The attribute escape of rule 5 is recorded, not closed. Closing it means `forbid` for the three lints in the root `Cargo.toml`, a protected surface, and that is Kennedy's decision (CHG-001 acceptance concerns).

## Consequences

- A change to the root `clippy.toml` needs the same change in the template and in every core crate's copy. The root file and the template are protected surfaces; the crates' copies are not, but the rule reports any copy that differs. This costs more edits than `CLIPPY_CONF_DIR`, but it keeps §12.1's commands and environment unchanged.
- A `.clippy.toml` at the repository root would shadow the root `clippy.toml` for every crate without its own file. The rule checks core crate directories only, so this remains a review item.
- **The deny list sees direct uses only** (§6.8). It does not see a call through a helper in another crate, a call the compiler generates from a macro, or an effect reached through a trait object. Experiment 4 proves each entry fires on a direct use, and nothing more.
- **What the list leaves out, on purpose.** `HashMap::new` and `HashMap::with_capacity` seed themselves from `RandomState`, which the list catches only when a crate names it; iteration order stays a source of nondeterminism that a deny list cannot reach. Platform extension traits, such as `std::os::unix::process::CommandExt::exec` and `std::os::unix::process::parent_id`, are effectful but would not resolve on another target, so listing them would trade a real check for a portability warning. Unstable APIs, such as `std::random`, are left out until they are stable. Each is a candidate for a later change, not an oversight.
- **Three ways to switch it off** are now recorded. An attribute in the crate (rule 5), unless the lint table forbids the lint. `--cap-lints=warn` from `RUSTFLAGS` or `.cargo/config.toml` (rule 6); `.cargo/config.toml` is not on the protected list in `.rha/policy.toml`. And a crate whose `clippy.toml` is missing, edited, or shadowed, which is what `effect.core_clippy_template` reports.
- Until CHG-003 wires the rule in and CHG-005 creates core crates, the template protects no product code: the deny list is demonstrated on fixtures only.
