# Maturity ledger

One row per mechanism of spec §1.4. The Executor edits only **Proposed**; Kennedy edits **Accepted**; the spec's own §1.4 table changes only at a version bump. Maturity: **S** specified, **I** implemented (a named, versioned, retrievable artifact), **V** validated (implemented, and its stated conformance or adversarial tests passed, with an evidence record). A proposal names the artifact, version, evidence, checking scope, and environment; it is not a permanent badge.

The **Accepted** column starts with the values printed in spec v0.10 §1.4.

| §1.4 mechanism | Spec § | Artifact path | Version | Proposed (Executor) | Evidence | Checking scope | Environment | Accepted (Kennedy, date, ref) |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Structural laws as review criteria | §4 | – | – | S | – | – | – | S (spec v0.10) |
| Crate-graph checker, `cargo xtask architecture` | §6.13 | `xtask/src/architecture.rs` (stub) | xtask 0.1.0 | S: the stub reports `not_run`; the checker is CHG-003 | – | – | – | S* (spec v0.10) |
| `cargo-generate` baseline template | §6.16.7 | – | – | S: deferred (plan §10) | – | – | – | S* (spec v0.10) |
| Module-graph check inside a crate | §6.13 | – | – | S: CHG-007 | – | – | – | S (spec v0.10) |
| Ambient-effect deny list; `no_std` core option | §6.8 | `xtask/templates/core-clippy.toml` (111 entries), `xtask/src/clippy_template.rs` (rule `effect.core_clippy_template`), `xtask/tests/clippy_corpus.rs` | xtask 0.1.0 (CHG-001, CHG-001.1, CHG-001.2) | **I** for the deny list: every one of the template's 111 entries fails the build where a fixture crate uses it, the configuration discovery rule is recorded (ADR-0002), and so is what switches the list off. Not V yet: no product crate uses the template, the rule is not enforced in L0 until CHG-003, and the attribute escape of ADR-0002 rule 5 is open (V proposal planned at CHG-006). `no_std` option: S (experiment E1) | `evidence/w1-clippy/20260919T224449Z-bf5aed920d40/` (23 experiment records at the head this row describes; two superseded sets sit beside it, 21 against the 80-entry template and 7 from the scripted first run); L0: `evidence/CHG-001/20260919T224504Z-bae4b34894a2.json`; CI: `evidence/ci/35474230710.json` | generated corpus fixtures only, crate level: direct uses in fixture crates on x86_64 Linux; not transitive calls, not macro-generated calls, not paths missing from the list, not a crate that allows the lint, and not a build whose `RUSTFLAGS` or `.cargo/config.toml` lowers the lint level (ADR-0002 rules 5 and 6); the rule is tested on temporary directories and is not yet run by any L0 command | Clippy 0.1.98; local WSL2 and GitHub `ubuntu24` (the corpus runs in `L0.nextest` in both) | S (spec v0.10) |
| Fast-lane CI configuration | §12.1 | `.rha/policy.toml` `[lanes.L0]`, `xtask/src/lanes.rs`, `.github/workflows/ci.yml` | xtask 0.1.0; tag `v0.10-m0` | **I**: the eight §12.1 commands run through `cargo xtask ci` locally and in CI; `L0.architecture` is `not_run` until CHG-003, so no V is proposed | local: `evidence/CHG-000/20260919T205530Z-c48f1bd1f44d.json`; CI: `evidence/ci/35468943628.json` (run 35468943628) | L0 lane on this repository | local WSL2 workstation; GitHub `ubuntu-24.04` | S (spec v0.10) |
| Statistical comparison protocol | §10.9 | – | – | S: CHG-016 | – | – | – | S (spec v0.10) |
| Held-out acceptance checks | §9.14 | – | – | S: DP-1.1, DP-1.4 | – | – | – | S (spec v0.10) |
| Review-efficacy audit with seeded changes | §11.4 | – | – | S: deferred (plan §10) | – | – | – | S (spec v0.10) |
| Machine policy, evidence envelope, protected verifier | §11.7 | `.rha/policy.toml`, `xtask/src/evidence/` | xtask 0.1.0 | S: a machine policy and unauthenticated evidence records exist; the protected verifier, schema, and envelope do not (CHG-019, CHG-020) | – | – | – | S (spec v0.10) |
| Repository semantic model and five related contribution views | §§3.9, 8.4, 11.8 | – | – | S: CHG-021 | – | – | – | S (spec v0.10) |
| Human/Agent/Team mechanisms and actor-role records | §11.6 | `.rha/tasks/` | – | S: task records exist (CHG-000); nothing validates them | – | – | – | S (spec v0.10) |
| in-toto-based RHA verification predicate | §11.7.11 | – | – | S: CHG-019 | – | – | – | S (spec v0.10) |
| Impact/topology/context pilots | §§17.1–17.2 | – | – | S: deferred (plan §10) | – | – | – | S (spec v0.10) |
