# ADR-0001: Repository bootstrap decisions

- **Status:** proposed in CHG-000; accepted when Kennedy accepts CHG-000.
- **Date:** 2026-09-19
- **Decided by:** `human:kennedy` (DP-0.1 to DP-0.6); the Executor's choices are marked as such below.

## Context

The v0.10 specification describes a Rust workspace, baseline tooling, machine policy, and contribution records, but it installs none of them (spec §1.4, §11.7.9). CHG-000 creates the repository in which the other work items run and are recorded.

## Decisions

| # | Decision | Source |
| --- | --- | --- |
| 1 | Git identity `Kennedy Mosoti <47609243+kmosoti@users.noreply.github.com>`, repo-local. The first choice, the personal address, was rejected by GitHub's email-privacy push protection (GH007). | DP-0.1 |
| 2 | Public repository `kmosoti/recursive-hexagonal-architecture`; product name `rhawiki`. | DP-0.2 |
| 3 | The spec lives at `docs/spec/rha-spec-v0.10.md`, byte-for-byte as committed in the base revision `8164186`. | DP-0.3 |
| 4 | `.rha/` records are TOML, not the spec's YAML examples: `serde_yaml` is unmaintained and would fail `cargo deny`, and §11.7.11 permits one equivalent record store. | DP-0.4 |
| 5 | Policy parameters: 24 h cooling-off, 3 repair attempts, proptest 256 cases in L0 and 4096 in L1, every L0 check non-waivable, protected surfaces as listed in `.rha/policy.toml`. | DP-0.5 |
| 6 | Branch protection on `main`: pull request required (0 approvals, because GitHub does not let an author approve their own pull request), required status `ci` with the branch up to date, applies to admins, no force-push or deletion. | DP-0.6 |
| 7 | Toolchain pinned to Rust 1.98.1, as §6.4 prints it. 1.98.1 installed, so the plan's fallback to 1.98.0 was not needed. | spec §6.4 |
| 8 | Workspace members are `crates/*` and `xtask`, exactly as §6.4 prints them. The plan's extra `tools/*` glob is left out until the first `tools/` crate exists (CHG-020): Cargo treats a glob that matches nothing as a literal path and fails. For the same reason, `crates/.gitkeep` must stay until the first crate exists. | Executor, observed |
| 9 | The exit status of `cargo xtask ci` reports failures (and, under `--label ci`, missing tools), not eligibility. Otherwise a required check that is `not_run` by design, `L0.architecture` until CHG-003, would make every merge impossible under branch protection. Eligibility is reported only in the record. | Executor; needs Kennedy's review |
| 10 | Generated docs carry a digest of their sources in the header, not a git revision. A revision in the header would change with every commit and make `cargo xtask docs --check` permanently stale. | Executor |

## Consequences

- Every evidence record is advisory (class `local` or `ci`) until CHG-020; see [the threat model](../threat-model.md).
- CHG-000 is not governed by any approved policy, because its base revision has none. Its acceptance is a controlled transition decided by Kennedy (§11.5).
- `L0.architecture` is required and non-waivable, but it cannot pass before CHG-003. CHG-001 and CHG-002 therefore cannot be `Eligible` under this policy either; see the acceptance concerns in [CHG-000](../changes/CHG-000-bootstrap.md).
