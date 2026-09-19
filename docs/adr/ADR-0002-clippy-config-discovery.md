# ADR-0002: Clippy configuration discovery and the core-crate template

- **Status:** proposed in CHG-001; accepted when Kennedy accepts CHG-001.
- **Date:** 2026-09-19
- **Environment:** `clippy 0.1.98 (48a229ceae 2026-09-01)`, rustc 1.98.1, `CLIPPY_CONF_DIR` unset, Linux (WSL2).

## Context

Spec §6.8 gives core crates a `clippy.toml` deny list of ambient-effect paths and says: "Verify how the pinned Clippy locates and merges configuration files." The Clippy book says the search starts at `CLIPPY_CONF_DIR`, else `CARGO_MANIFEST_DIR`, else the current directory, and walks up until one file is found; it does not document merging. The plan expected "nearest file wins, no merge" and named a fallback, `CLIPPY_CONF_DIR` set per crate by `xtask ci`, in case the observation disagreed.

## Observations

Each experiment ran from a clean target directory in a fixture workspace excluded from the root workspace. The records, with literal commands, exit statuses, and captured stderr, are in [`evidence/w1-clippy/`](../../evidence/w1-clippy/). [`xtask/tests/corpus/clippy/run-experiments.sh`](../../xtask/tests/corpus/clippy/run-experiments.sh) reproduces them, and [`xtask/tests/clippy_corpus.rs`](../../xtask/tests/clippy_corpus.rs) reruns experiments 1 to 4 on every L0 run.

| # | Fixture and command | Observed |
| --- | --- | --- |
| 1 | `core-seeded`: `cargo clippy -p core-a` (template; calls `std::time::SystemTime::now()`) | Exit 101. ``error: use of a disallowed method `std::time::SystemTime::now` `` and `` = note: `-D clippy::disallowed-methods` implied by `-D clippy::all` ``. |
| 2 | `discovery`: `cargo clippy -p adapter-x` (no local file; same call) | Exit 0, no finding. |
| 3 | `discovery`: `cargo clippy -p core-a --all-targets` (local file holds only the deny list; the fixture root holds `allow-unwrap-in-tests = true`; a test calls `unwrap()`) | Exit 0 with ``warning: used `unwrap()` on a `Result` value``: **`unwrap_used` fired**. |
| 3c | `discovery`: `cargo clippy -p adapter-x --all-targets` (control: same test `unwrap()`, nearest file is the fixture root) | Exit 0, no `unwrap_used`: the root allowance works where the root file is the nearest. |
| 3b | `core-seeded`: `cargo clippy -p core-b --all-targets` (template, which repeats the allowances; a test calls `unwrap()`) | Exit 0, no `unwrap_used`. |
| 4 | `core-seeded`: `cargo clippy -p core-every` (one use of each of the 15 template entries) | Exit 101. Every entry reported as `use of a disallowed method/type/macro` (18 errors: `dbg!` expands to `eprintln!`, and `TcpStream` is named twice). No unresolved-path warning. |
| 5 | scratch crate with both `clippy.toml` and `.clippy.toml`: `cargo clippy` | Exit 0 with ``warning: using config file `…/.clippy.toml`, `…/clippy.toml` will be ignored``. |

## Observed rule

1. **Per-crate discovery.** Clippy configures each crate from the first configuration file found in the crate's directory or its ancestors (experiments 1, 2, 3c). A core crate's local file does not apply to a sibling adapter.
2. **No merge.** The file found replaces every file further up; a crate with its own file loses the root file's settings (experiment 3; control 3c).
3. **Dotfile precedence.** In one directory, `.clippy.toml` wins over `clippy.toml`, with a warning that does not change the exit status (experiment 5).
4. **Deny level.** Under the workspace lint table (`clippy::all` at deny), every template entry resolves and fails the build (experiments 1, 4).

This matches the plan's expectation, so the `CLIPPY_CONF_DIR` fallback is not used.

**Not observed.** Whether the upward search stops at the workspace root or continues to the filesystem root: in both fixtures a file existed at the workspace root. `CLIPPY_CONF_DIR` was not exercised. Other Clippy versions: the L0 corpus test re-checks rules 1, 2, and 4 whenever the toolchain changes.

## Decision

- Every crate with role `core` has a `clippy.toml` byte-equal to [`xtask/templates/core-clippy.toml`](../../xtask/templates/core-clippy.toml). Adapters, apps, and tools have none and use the root file. The template holds the §6.8 entries, the plan's W1 additions (`std::fs::read`, `std::fs::read_to_string`, `std::fs::write`, `std::env::vars`, `std::io::stdin`, and the macros `std::println`, `std::eprintln`, `std::dbg`), and, because of rule 2, every setting of the root `clippy.toml`.
- Rule `effect.core_clippy_template` reports, per core crate, a missing `clippy.toml`, one that differs from the template, and a `.clippy.toml` beside it (rule 3). Its witness is the path and the sha256 digests. It is implemented in `xtask/src/clippy_template.rs` and runs in L0 once `cargo xtask architecture` exists (CHG-003).
- `xtask/tests/clippy_corpus.rs` fails if the template stops repeating a root setting, or if a fixture's lint table or root `clippy.toml` drifts from the repository's.

## Consequences

- A change to the root `clippy.toml` needs the same change in the template and in every core crate's copy. The root file and the template are protected surfaces; the crates' copies are not, but the rule reports any copy that differs. The corpus test and the rule fail until all three agree. This costs more edits than `CLIPPY_CONF_DIR`, but it keeps §12.1's commands and environment unchanged.
- A `.clippy.toml` at the repository root would shadow the root `clippy.toml` for every crate without its own file. The rule checks core crate directories only, so this remains a review item.
- The deny list is incomplete by nature and sees only direct calls (§6.8). Known effectful paths that it does not list include `std::time::SystemTime::elapsed`, `std::time::Instant::elapsed`, `std::fs::OpenOptions`, `std::fs::read_dir`, `std::fs::remove_file`, `std::net::UdpSocket`, `std::env::args`, `std::io::stdout`, `std::process::exit`, and `std::thread::sleep`. Extending the template is a protected change for Kennedy to decide.
- Until CHG-003 wires the rule in and CHG-005 creates core crates, the template protects no product code: the deny list is demonstrated on fixtures only.
