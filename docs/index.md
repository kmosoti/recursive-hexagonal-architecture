# RHA documentation

The landing page of the docs that `rhawiki` renders (CHG-005). Links use wiki syntax, `[[page-id]]`, where a page id is the path under `docs/` without `.md`.

## Specification and plan

- [[spec/rha-spec-v0.10]]: the Recursive Hexagonal Architecture working specification, v0.10.
- [[plan/IMPLEMENTATION-PLAN]]: the authorized work order for implementing and validating its mechanisms.

## Status

- [[maturity]]: the maturity ledger. The Executor proposes; Kennedy accepts.
- [[threat-model]]: deviations from spec §11.0, the `ECC-Solo` profile, and why every evidence record is advisory.
- [[toolchain]]: tools and required versions (generated).

## Records

- [[tasks/index]]: task records (generated).
- [[evidence/index]]: evidence records (generated).
- [[changes/CHG-000-bootstrap]]: the bootstrap change record.
- [[changes/CHG-001-ambient-effect-deny-list]]: the ambient-effect deny list and Clippy discovery change record.

## Decisions and architecture

- [[adr/ADR-0001-bootstrap-decisions]]: repository bootstrap decisions.
- [[adr/ADR-0002-clippy-config-discovery]]: how the pinned Clippy finds its configuration, and the core-crate template.
- [[architecture/components]]: components and their ports.
