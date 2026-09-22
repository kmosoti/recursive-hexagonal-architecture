# RHA-Core self-assessment (spec §15), at the close of M1

A recorded pass through the §15 checklist, with gaps named, is what an RHA-Core conformance claim consists of (§1.4). This pass covers `rhawiki` and its tooling at the P-A packet (CHG-005, CHG-006). The answers are **yes**, **partial**, **no**, or **n/a**, each with where to look.

## Scope and structure

| Item | Answer | Where |
| --- | --- | --- |
| Target class, non-target parts named | yes | plan §3: CLI with a separable decision core; HTML templates (`adapter-html`) are the named non-target part |
| Each component states what it owns | yes | BDR-0001 to BDR-0004; crate docs |
| Outsiders use components without internals | yes | facades at crate roots; `site` re-exports what a renderer needs |
| Port owner and polarity named | yes | plan §3.1; `implements` metadata on each adapter |
| Composite children bound or exported; no adapter below the root | yes | `site`: `Assemble` bound to its sibling, `PageRenderer` exported; adapters only in `app-cli` |
| Sibling graph acyclic, no child depends on its parent | partial | crate level checked (`graph.cycle`); inside `site`, `rha-modules.toml` is declared but enforced by review until P-C (BDR-0003) |
| External effects explicit at ports | yes | `SourceRepository`, `OutputSink`, `Clock` |
| Ambient effects kept out of cores by a check | yes | the 111-entry deny list in every core's `clippy.toml`, the crate-root `forbid`, and `effect.core_clippy_template` in L0 |
| Controller updates from observations (Law 15) | partial | `build::step` diffs against the sink's listing; the served reconciler and its traces are W14 (P-E) |
| Ports semantic, not vendor-named | yes | no pulldown-cmark type in any public API (BDR-0002 a) |
| Forbidden patterns executable in CI, enforcement map honest | yes | `rha-crates.toml` forbidden edges; `docs/enforcement-map.md` generated from H4 evidence |
| Simple when one component suffices | yes | no framework; four core crates |
| Experiments isolated | yes | `experiments/` (E1 is in P-C) |

## Boundaries

| Item | Answer | Where |
| --- | --- | --- |
| Decision records with refutation criteria written before the boundary | yes | BDR-0001 to 0004, accepted at DP-1.3 before any product crate existed |
| Co-change statistics with §7.3 filters | no | no history yet; the first measurement is P-D's per-stage change spread |

## Functional assurance

| Item | Answer | Where |
| --- | --- | --- |
| Important invariants have property tests | yes | plan §3.2 table: `library`, `document`, `graph`, `site`, `adapter-html` tests |
| Adapters share a contract suite | yes | `library::contract`, `site::contract`, run by every implementation and by seeded violators |
| Boundary transformations property-tested | yes | parser totality over arbitrary strings; escaping round trip |
| Mutation analysis | no | P-D stage 3 (CHG-014) |
| Mutation read as discrimination evidence | n/a | – |
| Composition tests | yes | `crates/app-cli/tests/cli.rs`: the 60-site corpus through the binary; the spec build |
| Compact witnesses checkable independently | yes | §3.2 witnesses; the corpus grades them by construction |
| t-way coverage | no | not yet justified by a configuration space |
| Boundary values concentrated at limits | partial | the adversarial markdown corpus targets Unicode, case and anchor edges |
| Release-critical invariants tied to claim, evidence and owner | partial | the ledger and this table; no release record yet |
| Component assumptions recorded, undischarged ones listed | yes | `.rha/assumptions.toml`: seven current, two undischarged at the composition root |
| Held-out acceptance check for agent-authored consequential changes | partial | crate level: DP-1.1b, not yet run. Markdown: DP-1.4, fixtures pending; `corpus held-out --kind check` is ready |

## Efficiency

All five items are **no** at M1. P-E (DP-3.1, `bench`, `compare`, counts) is where they are answered.

## Rust profile

| Item | Answer | Where |
| --- | --- | --- |
| Invalid values prevented by types | yes | `RelPath`, `PageId` constructors |
| Deliberate facades | yes | crate roots |
| Ports owned by their core, free of vendor types | yes | as above |
| Async confined | yes (n/a) | fully synchronous |
| `unsafe` forbidden in core and app crates | yes | workspace lint `unsafe_code = "deny"` |
| Compile-fail tests for costly guarantees | partial | the deny-list corpus (CHG-001) is compile-fail over fixtures; product crates have none yet |
| Crate graph enforces direction, including "only roots depend on adapters" | yes | `cargo xtask architecture`, H4 crate level 22/22 (V proposed, DP-1.6) |
| Composite declares children and allowed edges, checked | partial | declared in `crates/site/rha-modules.toml`; checked from P-C |
| Specialized tools routed by risk | partial | mutation and bench arrive with their packets |
| Assurance tools separate from runtime dependencies | yes | xtask is a `tool`; `dir.tool_depended_on` and machete |
| Every PR runs L0 | yes | `.github/workflows/ci.yml`, branch protection |
| Heavier tools activated by risk | partial | the L1 trigger mechanism is not implemented |
| Build flags measured before default | n/a | none adopted |

## Work architecture and coordination

| Item | Answer | Where |
| --- | --- | --- |
| Mode from the execution mechanism | yes | task records: `mode = "agent"` whatever the Git author |
| Actor, responsibility, authority recorded separately | yes | `[[actors]]`, `[roles]`, decisions with `decided_by` |
| Cross-owner handoffs with readiness conditions | yes | gates in plan §8.1; decisions quoted in task records |
| Team mode Integrator | n/a | Solo profile |
| Records no more elaborate than needed | partial | plan revision 2 cut recording churn (L1); the verifier (P-B) will lint records |

**Claim.** Structure and functional assurance meet RHA-Core at crate level, with the gaps above. Efficiency is not claimed at M1. Enforcement inside `site` is by review until P-C. Every record remains advisory until P-B.
