# RHA v0.10 implementation program: authorized work order

> **Provenance.** Planned 2026-09-19 with Claude Fable 5.1 as a planning tool under Kennedy Mosoti's direction; approved by Kennedy the same day. This file is the Planner artifact for the program (spec §11.7.1: "Task brief or issue" owns authorized intent, acceptance examples, non-goals, and constraints; it grants no authority beyond what the executing session's permissions and `.rha/policy.toml` allow). W0 moves it to `docs/plan/` and it is rendered by the wiki like every other record. Revised the same day at Kennedy's direction (CHG-001.1): the Executor is model-agnostic. Its actor id is `agent:executor`, and each task record's provenance names the model and harness that acted.

## How to start the executing session

Open a new coding-agent session in this directory, with any agent and model, then paste:

```text
You are the Executor for the RHA v0.10 implementation program, in `agent` contribution mode under the ECC-Solo authority profile. Kennedy Mosoti is Planner, Integrator, and Acceptor.

1. Read IMPLEMENTATION-PLAN.md in full, then rha-spec-v0.10.md in full (it fits in context). Do not scan other directories under ~/projects; this workspace is self-contained.
2. Follow the plan's operating method (section 2) exactly: one work item per PR; a task record, change record, and evidence record per item; the five outcome states; nothing fabricated; a check that did not run is `not_run`. Never edit the spec, its maturity table, or a protected surface without my explicit approval in this conversation.
3. Start with Phase 0 (W0). Before writing any file, ask me decision points DP-0.1 through DP-0.6 in one question, presenting the plan's defaults as the recommended options.
4. In W0 keep the first commit spec-only, then move IMPLEMENTATION-PLAN.md to docs/plan/ in the same PR.
5. At the end of every work item stop and report: what ran, what passed, what is not_run, and what you need from me. Do not start the next item until I confirm the PR is merged.
```

---

# Plan: implement and validate RHA v0.10 mechanisms in this workspace

Handoff document. Written for a fresh agent session, of any model, that executes it as the **Executor** in the spec's `agent` contribution mode. Kennedy Mosoti is Planner (through this plan), Integrator, and Acceptor under the `ECC-Solo` authority profile (§11.6, §11.7.6).

## 1. Context

`rha-spec-v0.10.md` (the only file here, 3455 lines) specifies Recursive Hexagonal Architecture. Its maturity table (§1.4, lines 151–191) lists 14 mechanisms, all at **S** (Specified). Nothing is implemented, there is no git repository, so no "named, versioned, retrievable artifact" exists, which is the spec's definition of maturity **I**. Maturity **V** additionally needs a passed conformance or adversarial test with an evidence record.

This plan sets the workspace up as the spec intends (canonical Rust workspace §6.3/§6.16.7, baseline tooling §6.16, contribution artifacts §11.7.3–11.7.5/§11.7.11, lanes §12.1), implements the mechanisms so they reach I, and builds the harnesses that validate the claims that can be validated without a second team or a second system (H3, H4, H5 at predicate level, the §10.9 protocol), so they reach V with evidence. The work itself runs under the spec's contribution protocol: every work item is a contribution with intent, scope, delta, and evidence; nothing is fabricated; checks not run are `not_run`.

Decisions Kennedy made on 2026-09-19:

| Decision | Choice |
| --- | --- |
| Executor | A fresh agent session reading this file, of any model (revised from "Opus 5" in CHG-001.1); Kennedy accepts each milestone |
| Hosting | GitHub remote with Actions from Phase 0 |
| Pilot subject | A Rust-native research repository/wiki that renders markdown with custom features; working name `rhawiki` (rename at DP-0.2); it dogfoods this repo's own docs, including the spec |
| Scope | Full program, Phases 0–4 as milestones |

## 2. Operating method for the executing session

1. **Load context.** Read this plan fully, then the entire spec (≈65K tokens; it fits). After Phase 0 also read `AGENTS.md` and `CONTRIBUTING.md`. Do not scan other directories under `~/projects`; this workspace is self-contained.
2. **Authority order** (§11.7.2): infrastructure permissions cap execution; `.rha/policy.toml` defines acceptance; this plan is the authorized task list; guides help. The Executor MUST NOT: edit the spec's §1.4 table or any spec text (Kennedy promotes rows at a version bump from `docs/maturity.md`); read or write held-out material (§9.14); record `passed` for a check that did not run; remove or weaken a test without the §9.14 justification; edit `.rha/policy.toml`, `rha-crates.toml` allow-lists, `.rha/acceptances/`, or `.rha/exceptions.log` after W0 without an explicit Kennedy approval (protected surfaces, §11.7.6); expand a work item beyond its Owned scope without asking; invent versions, SHAs, or command output.
3. **Work-item loop** (§11.7.7). For each `W-nn` (change id `CHG-0nn`): write `.rha/tasks/CHG-0nn.toml` (mode `agent`, executor `agent:executor`, `accountable_to = "human:kennedy"`, `authority_profile = "ECC-Solo"`, provenance per §11.6.5: model id, harness/version, instruction sources with digests, task scope, permissions, budgets; unknown stays `"unknown"`) → branch `chg/0nn-<slug>` → one coherent patch → `cargo xtask ci` plus the item's own checks → `evidence/CHG-0nn/<utc-ts>-<shortsha>.json` (shape in §7) → `docs/changes/CHG-0nn.md` in the §11.7.5 template → `gh pr create` with the PR template → stop and report. Kennedy reviews, merges, and writes `.rha/acceptances/CHG-0nn.toml`. "Done" is a proposal, not an acceptance event (§11.6.3).
4. **Iteration must buy information** (§11.7.10). Each repair attempt records hypothesis, discriminating check, change, result in the change record. After `repair_loop.max_attempts` (policy, default 3) unexplained attempts on one item, stop and escalate.
5. **Milestones.** End of each phase: tag `v0.10-m<phase>`, update `docs/maturity.md` "Proposed" column for every mechanism the phase touched, regenerate `cargo xtask docs`, and hand Kennedy a milestone report: what reached I or V, what is `not_run`, what was refuted or downgraded.
6. **Decision points** (§9) are where the session pauses and asks Kennedy. It never guesses at them.
7. **Commits.** One PR per work item. Git author is Kennedy Mosoti (authorship per §11.6.1; mode is recorded in the task record, not the author string). Commit trailer `RHA-Task: CHG-0nn` plus the attribution trailer the executing session's system reminder specifies.

**Facts already established (do not re-derive; re-verify only where marked):**

| Fact | Observed 2026-09-19 |
| --- | --- |
| Toolchain | rustc/cargo 1.98.0 stable installed; clippy 0.1.98; nightly present. **1.98.1 exists** (`static.rust-lang.org/dist/channel-rust-1.98.1.toml` → 200; `rustup check` shows the update), so the spec's pin needs only an install, no deviation record |
| Installed tools | cargo-deny 0.20.x, cargo-fuzz, cargo-miri, `just`, `jj` |
| Missing tools | cargo-nextest, cargo-mutants, cargo-modules, cargo-machete, typos-cli, cargo-generate |
| GitHub | `gh` authenticated as `kmosoti`, scopes include `repo`, `workflow` |
| Git identity | no `user.name`/`user.email` configured in any scope (DP-0.1) |
| Registry cache (offline-capable versions; re-verify with `cargo search` before pinning) | pulldown-cmark 0.13.4, syn 2.0.x, walkdir 2.5.0, notify 8.x, serde 1.0.x, serde_json 1.0.x, sha2 0.10.x, proptest 1.11.x, petgraph 0.8.x, jsonschema 0.49.x, toml 1.1.x, thiserror 2.0.x, regex 1.x, similar 3.x, unicode-normalization 0.1.x, indexmap 2.x |
| pulldown-cmark 0.13 | `Options::ENABLE_WIKILINKS` (`LinkType::WikiLink{has_pothole}`), `ENABLE_GFM` (alerts/callouts), `ENABLE_TABLES`, `ENABLE_HEADING_ATTRIBUTES` |
| Clippy config discovery (Clippy book) | searched from `CLIPPY_CONF_DIR`, else `CARGO_MANIFEST_DIR`, else cwd, walking up until **one** file is found; merging is undocumented → W1 verifies "nearest file wins, no merge" |
| Spec content | 182 headings outside code fences, no duplicate slugs; 270 `§` references; 168 distinct `[Rn]` citations; 2 mermaid fences |

## 3. Pilot subject: `rhawiki`

**Target-class fit (§1.1 checklist ★).** A CLI (later a daemon: `serve`) that integrates filesystem, later git and HTTP, with a separable decision core: markdown-to-typed-document parsing, cross-page link resolution, per-page assembly, and build planning as a pure `(state, observation) -> (state', commands)` transition (the conditional-fit form of §1.1 and Law 15). The presentational HTML templates are the named non-target part and live only in `adapter-html`. Threat to validity to record with any result: this system sits at the favourable end of the target class (§17).

### 3.1 Components (§14.1 format; ports are traits, static dispatch, synchronous, §6.7)

| Component | Kind | Provides | Requires | Owns ports |
| --- | --- | --- | --- | --- |
| `library` | primitive core | `load(&impl SourceRepository) -> Result<Corpus, LibraryError>`; `PageId`, `RelPath`, `Source{id, path, text, digest}`, `Corpus`, `Digest` | `SourceRepository` | `SourceRepository` (required) |
| `document` | primitive core | `parse(&Source) -> Document` (total; witnesses in `diagnostics`); own `Node` tree (no vendor types cross ports); `Heading{level, text, slug, span}`; `Link` | – | – |
| `graph` | primitive core | `resolve(&[Document]) -> SiteGraph` (resolved links, backlinks, witnesses) | – | – |
| `site` | **composite** core | `Site::build_all(&Corpus, &impl Clock, &mut impl OutputSink) -> Result<BuildReport, SiteError>` | `PageRenderer` (exported from `build`), `OutputSink`, `Clock` | `OutputSink`, `Clock` (glue, `site::port`) |
| `site::assembly` | primitive child | `Assemble`: `(Document, SiteGraph, AssembleContext) -> PageModel` (TOC, breadcrumbs, backlinks, resolved links) | – | `Assemble` (provided; defined here like `Feasibility` in §14.1) |
| `site::build` | primitive child | `step(&SiteState, &Observation, &BuildInputs, &impl Assemble, &impl PageRenderer) -> (SiteState, Vec<Command>)`; `Command::Write{path, bytes, page} | Delete{path}` | `Assemble` (bound to sibling), `PageRenderer` (exported; `pub use build::PageRenderer` at crate root like `Solver`) | `PageRenderer` (required) |
| `adapter-fs` | adapter | `FsSources: SourceRepository` (walkdir, `.md` only, `/`-separated relative paths); `FsSink: OutputSink` (temp-file + rename, `list()` with sha256) | | |
| `adapter-html` | adapter (non-target part) | `HtmlRenderer: PageRenderer`, `assets()` (`assets/style.css`) | | |
| `adapter-sys` | adapter | `SystemClock: Clock` (in scope only with the "built at" footer, DP-1.2) | | |
| `adapter-json` (Phase 2, H3 probe) | adapter | `JsonRenderer: PageRenderer` (page JSON + search index) | | |
| `adapter-git`, `adapter-http` (later) | adapters | `SourceRepository` at a revision; driving adapter `serve`/watch | | |
| `app-cli` | composition root | binary `rhawiki`: `build --root --out`, `check --root [--format json]`, later `serve` | | |

`PageRenderer` is a pure mechanism-variation port (deterministic, total: `PageModel -> Rendered{path, bytes}`); that is what keeps `step` pure while rendering happens inside it. Effects (`OutputSink`, `Clock`) are reached only by the glue.

**Crate dependency direction** (declared in Cargo.toml, checked by W3): `adapter-fs → library, site`; `adapter-html → site, document`; `adapter-sys → site`; `site → library, document, graph`; `graph → document, library`; `document → library` (`Source`, `PageId` are library types; D4 declared, chain acyclic); `app-cli → all`; nothing but `app-cli` and test harnesses → `adapter-*` (D5).

**Module rules** `crates/site/rha-modules.toml` (§6.13 shape):

```toml
[components]
assembly = "site::assembly"
build    = "site::build"
[allow]
build    = ["assembly"]
assembly = []
[deny]
cycles = true
child_to_parent_private = true
foreign_internal = true
```

**Fakes and contract suites.** In-process fakes live with the port owner, always compiled (pure, no deps): `library::testing::MemorySources`, `site::testing::{RecordingSink, FixedClock, StubRenderer, FakeAssembler}`. Contract suites (§9.8) are `pub fn`s in the owner returning witness lists: `library::contract::source_repository`, `site::contract::{page_renderer, output_sink}`; each adapter's `tests/contract.rs` calls them (adapter → owner is the existing D1 edge). Seeded violating implementations live in the owner's `tests/`, never as crates.

### 3.2 Invariants and witnesses (§9.3, Law 11)

| Owner | Invariant | Witness | Independent check (no owner code) |
| --- | --- | --- | --- |
| `library` | page ids unique in a `Corpus` | `DuplicatePageId{id, paths:[a,b]}` | normalize both paths with the 5-line normalizer; both equal `id` |
| `library` | complete-or-fail load | `LibraryError::Repository{path, error}`, no `Corpus` on any failure | a `Corpus` exists only if every listed path was read |
| `document` | heading slugs unique per document | `DuplicateSlug{slug, first:(line,text), second:(line,text)}` | `slugify(first.text) == slugify(second.text) == slug` |
| `document` | parse is total; unsupported syntax is visible | `Diagnostic::Unsupported{kind, span}` | the span exists in the source |
| `graph` | every wikilink resolves or is witnessed | `BrokenLink{from, link_text, target, candidates, span}`, `AmbiguousLink{…}`, `MissingAnchor{from, target, heading}` | `target ∉ ids`; every candidate ∈ ids |
| `graph` | backlinks symmetric | `AsymmetricBacklink{a, b}` | `a ∈ backlinks[b] ⟺ (a→b) ∈ links` |
| `graph` (Phase 2) | no transclusion cycle | `TransclusionCycle{path:[a,…,a]}` | each consecutive pair is a transclusion edge |
| `site::assembly` | TOC in document order; every anchor exists | `TocAnchorMissing{entry, page}` | anchor ∈ slugs; positions increasing |
| `site` (`Inv_K`) | every command names a page (Write) or output (Delete) present in the observation | `{command, observation_digest}` | set membership |
| `adapter-html` | text nodes escaped | property over rendered bytes | decode-and-compare |

### 3.3 Assumption ledger seed (`.rha/assumptions.toml`; §8.2 fields, §8.3 states)

| id | Consumer / port | Assumption | Discharged by (support kind) | State |
| --- | --- | --- | --- | --- |
| `library.source_repository.complete_or_fail` | `library` / `SourceRepository` | `list()` returns every `.md` under the root exactly once with stable relative paths; `read()` returns whole text or fails | `library::contract::source_repository` per adapter (sampled) | current |
| `document.source_complete` | `document` on G(library) | `Source.text` is the whole file | library contract + construction (sampled) | current |
| `graph.slugs_unique_or_witnessed` | `graph` on G(document) | slugs unique per document or a witness exists | document property test (sampled) | current |
| `site.build.assemble_deterministic` | `site::build` / `Assemble` | same inputs → identical `PageModel` | assembly property test (sampled) | current |
| `site.build.renderer_deterministic_total` | `site::build` / `PageRenderer` | never panics, deterministic | `site::contract::page_renderer` per adapter (sampled) | current |
| `site.glue.sink_atomic_listable` | `site` / `OutputSink` | `write` all-or-nothing; `list()` reflects completed writes with digests | `site::contract::output_sink` per adapter (sampled) | current |
| `site.glue.clock_display_only` | `site` / `Clock` | `now()` affects displayed text only | asserted | current |
| `root.output_dir_not_concurrently_modified` | `app-cli` | nothing else writes the output dir during a build | – | **undischarged** (release record) |
| `root.fs_listing_fresh` | `app-cli` | listing reflects all completed writes | – | **undischarged** until Law 15 traces (W14) |

### 3.4 `docs/` content the wiki renders (dogfooding)

```
docs/index.md                         landing page (wikilinks to everything below)
docs/spec/rha-spec-v0.10.md           the spec (git mv in W0, DP-0.3; content unchanged, so line refs stay valid)
docs/plan/IMPLEMENTATION-PLAN.md      this file (moved in W0)
docs/adr/BDR-0001..0004, ADR-0001..   boundary decision records (§7.9 five-field criterion) and ADRs
docs/changes/CHG-0nn-<slug>.md        change records (§11.7.5 template)
docs/tasks/                           generated projection of .rha/tasks
docs/evidence/index.md                generated index of evidence/**/*.json
docs/maturity.md                      maturity ledger (Executor proposes, Kennedy accepts)
docs/enforcement-map.md               §4.1 cells with H4 status (generated from evidence/h4-*/latest)
docs/threat-model.md                  §11.0 deviations; ECC-Solo declaration; cooling-off; exception log path
docs/conformance/self-assessment.md   §15 checklist pass with gaps (what an RHA-Core claim consists of)
docs/architecture/{components.md, crate-graph.md (generated), ledger.md (generated)}
docs/observations/change-spread.md    components-touched per scripted feature change (descriptive, not H1)
docs/toolchain.md                     generated from rha-baseline.json
```

Generated files carry `<!-- generated by cargo xtask docs from <source> at <rev>; do not edit -->`; `cargo xtask docs --check` fails when stale (an L1 check triggered by `.rha/**`, `evidence/**`, `crates/*/Cargo.toml`; it cannot join L0 because §12.1 owns L0 membership).

### 3.5 Custom-feature roadmap

Phase 1 (W5): page ids, heading slugs/anchors, per-page TOC, `[[wikilinks]]` + backlinks, broken-link witness, HTML output of `docs/` with a "built at" footer. Phase 2 (W9), each as its own contribution with components-touched recorded: `§n.n` references auto-linked to spec headings, `[Rn]` citations linked to reference entries, callouts `> [!note]`, transclusion `![[Page#Section]]` with cycle witness, tags/front matter index. Phase 2 (W8): JSON renderer (H3). Phase 3 (W14): `serve --watch` reconciler (Law 15, §9.5 traces). Boundary decision records are written by Kennedy **before** any new crate is created (§7.8 step 10).

## 4. Repository layout

### 4.1 After Phase 0 (W0)

```
.cargo/config.toml                 alias xtask = "run -p xtask --"
.config/nextest.toml               [profile.ci] fail-fast=false, retries=1, flaky-result="fail", junit (§6.16.3)
.github/workflows/ci.yml           PR/push: toolchain from rust-toolchain.toml; pinned install-action (SHA via gh api) for nextest/machete/typos/deny; `cargo xtask ci --label ci --record target/rha`; upload evidence artifact
.github/workflows/assurance.yml    weekly + workflow_dispatch: L2 items; not-yet-implemented jobs record not_run
.github/dependabot.yml             cargo + github-actions, weekly
.github/pull_request_template.md   §11.7.5 headings verbatim
.gitignore                         target/; proptest-regressions are committed, not ignored
.rha/policy.toml                   machine policy (single owner of check ids/params): [lanes.L0] with source="§12.1" and the 8 checks (id, argv, kind, validity criterion); triggers; protected surfaces; non_waivable (all L0); exception authority; cooling_off_hours; repair_loop.max_attempts; proptest budgets (L0/L1); trusted_producers; required_inputs
.rha/assumptions.toml              ledger (schema_version + empty list until W5)
.rha/architecture.toml             declared semantic ids (components, ports, contracts)
.rha/tasks/CHG-000-bootstrap.toml  task record (§11.7.11 shape, TOML)
.rha/acceptances/                  Kennedy's acceptance records
.rha/exceptions.log                append-only (empty)
.rha/schemas/                      empty until W15
AGENTS.md                          literal §11.7.3 shape with rhawiki's two constraints
CONTRIBUTING.md                    six §11.7.4 sections; names `cargo xtask ci`; links §12.1 for membership; tool status table; §9.14 weakening rule
Cargo.toml                         [workspace] members=["crates/*","xtask","tools/*"], exclude=["xtask/tests/corpus"], resolver="3"; [workspace.package] edition 2024, rust-version 1.98.1; [workspace.lints] verbatim §6.4
Cargo.lock                         committed; CI uses --locked
rust-toolchain.toml                verbatim §6.4
clippy.toml                        verbatim §6.16.2 (test allowances)
deny.toml                          advisories deny; licences allow-list; bans multiple-versions=warn, wildcards=deny; sources crates.io only
rha-baseline.json                  tool inventory: name, required version, installed version, install command; consumed by `xtask ci` for tool_missing/version_mismatch
rha-crates.toml                    checker rules (schema §5); Phase 0: classification + empty allow-lists
_typos.toml                        excludes evidence/**, xtask/tests/corpus/**
README.md                          one paragraph + pointers
docs/                              §3.4 skeleton (index, spec moved, plan moved, ADR-0001, CHG-000, maturity.md, threat-model.md, architecture/components.md)
evidence/CHG-000/<ts>-<sha>.json   first local record; evidence/ci/<run-id>.json after the first CI run
experiments/README.md              research membrane statement (§5.5)
xtask/{Cargo.toml (role=tool), src/{main.rs, lib.rs, cli.rs, policy.rs, lanes.rs, evidence/*.rs, tools.rs}, tests/policy_drift.rs}
crates/.gitkeep
```

### 4.2 Added by Phase 1

```
crates/library/{Cargo.toml (metadata.rha.role="core"), clippy.toml (deny list), src/{lib.rs, page_id.rs, source.rs, corpus.rs, digest.rs, port.rs, contract.rs, testing.rs}, tests/{properties.rs, contract_seeded.rs}}
crates/document/{…, src/{lib.rs, parse.rs, node.rs, heading.rs (slugify), link.rs, diagnostic.rs}, tests/{properties.rs, spec_renders.rs}, proptest-regressions/}
crates/graph/{…, src/{lib.rs, resolve.rs, backlinks.rs, witness.rs}, tests/{properties.rs, witness_independent.rs}}
crates/site/{Cargo.toml (metadata.rha.composite="rha-modules.toml"), clippy.toml, rha-modules.toml, AGENTS.md (scoped guide, §11.7.3), src/{lib.rs (glue), port.rs, assembly/{mod.rs, toc.rs, model.rs}, build/{mod.rs, state.rs, command.rs, renderer.rs}, contract/{mod.rs, page_renderer.rs, output_sink.rs}, testing.rs}, tests/{inv_k.rs, build_properties.rs}}
crates/adapter-fs/{Cargo.toml (metadata.rha.implements=["library::SourceRepository","site::OutputSink"]), src/{lib.rs, source.rs, sink.rs}, tests/contract.rs}
crates/adapter-html/{…, src/{lib.rs, escape.rs, page.rs, assets.rs}, tests/{contract.rs, escaping.rs}}
crates/adapter-sys/{Cargo.toml, src/lib.rs}
crates/app-cli/{Cargo.toml ([[bin]] name="rhawiki"), src/{main.rs, cli.rs, wiring.rs, check.rs}, tests/{cli_build.rs, cli_check.rs}}
xtask/src/{metadata.rs, graph/{mod.rs, model.rs, classify.rs, rules.rs, check.rs, report.rs}, corpus.rs, docs.rs}
xtask/tests/{corpus_crate.rs, evidence_shape.rs}
xtask/tests/corpus/manifest.toml   pre-registered cases (W2, before checker code)
xtask/tests/corpus/crate/{violations,legitimate,expected_miss}/<case-id>/{Cargo.toml, rha-crates.toml, <crate>/Cargo.toml, <crate>/src/lib.rs}
xtask/tests/corpus/clippy/{core-seeded/, discovery/}
xtask/templates/core-clippy.toml   the one deny-list template every core crate's clippy.toml must equal
evidence/h4-crate/, evidence/w1-clippy/, evidence/ci/
docs/adr/BDR-0001..0004, docs/adr/ADR-0002-clippy-config-discovery.md, docs/changes/CHG-001..006, docs/enforcement-map.md, docs/conformance/self-assessment.md
```

## 5. `xtask` design

One crate, lib + bin (integration tests and later `rha-verifier` call the checker in-process). Role `tool`; nothing may depend on it (`dir.tool_depended_on`).

| Module | Responsibility |
| --- | --- |
| `cli` | clap: `architecture [--manifest-path P] [--rules P] [--format text|json|md] [--transitive]`; `ci [--label local|ci] [--record DIR] [--only ID] [--print]`; `l1 [--changed-from REV]`; `corpus run [--level crate|module] [--evidence DIR]`; `evidence subject`; `docs [--check]`; Phase 3 `bench`, `compare`; Phase 4 `rha lint|derive|explain|ledger` |
| `policy` | load `.rha/policy.toml`, typed model, sha256 of canonical bytes, strictness preorders per check kind |
| `metadata` | `cargo metadata --format-version 1 --no-deps --offline [--manifest-path]`; map `dependencies[]` (`name` is the package name even when renamed; `kind` null/dev/build; `optional`; `target`; `path`) |
| `graph::model` | `CrateGraph{crates, edges, mode: NoDeps|Resolved}`; `CrateNode{name, manifest_path, role: Core|Adapter|App|Tool|Harness, role_source: Metadata|Prefix|List, implements: Vec<PortRef>, has_build_script}`; `Edge{from, to: Member|External{name}, kind: Normal|Dev|Build, optional, target_cfg, rename}` |
| `graph::classify` | total: explicit `[package.metadata.rha] role` wins; else prefix (`adapter-`, `app-`); else `[classification].tools/harness` lists; else `class.unclassified` (error); prefix/metadata conflict → `class.prefix_role_conflict` |
| `graph::rules` | `rha-crates.toml` schema below; glob matching on package names |
| `graph::check` | evaluate rules → `Vec<Finding>`; own DFS cycle check |
| `graph::report` | JSON (always written to `target/rha/architecture.json`) + text + markdown; witness = concrete edge + manifest path + declaration site |
| `modules::*` (W7) | syn-based extraction (`full`, `visit`, `extra-traits`), path resolution, collapse to declared components, `rha-modules.toml` rules; token-scan heuristic for paths inside macro invocations flagged `extraction: heuristic` |
| `lanes` | `ci`: iterate `policy.lanes.L0.checks`, spawn each with the policy-owned env (`PROPTEST_CASES`), classify outcome, write evidence; `l1`: evaluate `policy.triggers` against `git diff --name-only <base>` |
| `evidence` | record builder (§7), subject identity (`git rev-parse HEAD`, `git write-tree`, dirty diff sha256, untracked list), tool identity vs `rha-baseline.json`, artifact digests |
| `corpus` | H4 harness: read `manifest.toml`, run checker per case, compare expected/observed, compute rates, write evidence |
| `docs` | generated projections (§3.4) |
| `stats` (W12) | t-interval, percentile bootstrap, Holm, run-count planner, decision rule |

**`rha-crates.toml` schema (root):**

```toml
schema_version = 1
[classification]
adapter_prefix = "adapter-"
app_prefix     = "app-"
tools          = ["xtask"]        # Phase 4 adds "rha-verifier"
harness        = []
# every other member MUST declare [package.metadata.rha] role = "core" | "adapter" | "app" | "tool" | "harness"
[core]
allow = ["pulldown-cmark", "sha2", "thiserror", "serde", "unicode-normalization"]   # purity judged manually (§6.8)
dev_allow = ["proptest", "serde_json", "similar"]
allow_build_scripts = false
[adapters]
require_port_owner_dependency = true   # each owner in metadata `implements` must be a normal dependency
foreign_core_dependency = "warn"
[[forbidden]]
from = "library";   to = "notify";         reason = "change observation crosses SourceRepository"
[[forbidden]]
from = "site";      to = "pulldown-cmark"; reason = "parsing is document's decision"
[[forbidden]]
from = "role:core"; to = "tokio";          reason = "no runtime in cores"
[transitive]
enabled = false                        # L2 mode: full resolution; the §4.1 transitive hole
```

**Rule ids (stable; used in reports and the corpus manifest):** `class.unclassified`, `class.prefix_role_conflict`, `dir.core_to_adapter` (D1/Law 3), `dir.core_to_app`, `dir.non_root_to_adapter` (D5; dev-kind edges excluded and listed as `harness_edges`), `dir.tool_depended_on`, `effect.core_disallowed_dependency` (Law 5), `effect.core_disallowed_dev_dependency`, `effect.core_build_script`, `effect.core_clippy_template` (W1), `adapter.missing_port_owner`, `adapter.port_owner_wrong_kind`, `adapter.foreign_core` (warn), `meta.unknown_port_owner`, `forbidden.edge`, `graph.cycle`, `transitive.core_disallowed_dependency` (L2), `modules.undeclared_dependency`, `modules.cycle`, `modules.child_to_parent_private`, `modules.foreign_internal`, `modules.unresolved_path` (limitation), `modules.not_implemented` (Phase 1 placeholder → `not_run`).

**Report** `--format json`: `{schema_version, tool{name, version, git_rev}, subject{workspace_root, manifest_path, metadata_mode, rules_path, rules_digest}, classification[], edges_examined{normal, dev, build}, findings[{rule, severity, from, to, kind, manifest_path, witness{edge, declared_in, optional, target}, message}], module_checks[{crate, rules_path, outcome, reason}], limitations[], summary{errors, warnings, outcome}}`. Text: `error[dir.core_to_adapter]: site -> adapter-html (normal) declared in crates/site/Cargo.toml [dependencies]: core crates must not depend on adapter crates (D1, §6.13)`.

**Exit codes:** 0 no error findings; 1 error findings; 2 usage/config error; 3 environment failure (`cargo metadata` failed) — never a pass; evidence entry `failed`, `error_class: tool_error`.

**`xtask ci` without a second command list:** `.rha/policy.toml [lanes.L0]` carries `source = "docs/spec/rha-spec-v0.10.md §12.1"` and the eight checks with validity criteria (for example `L0.nextest` requires `selected_tests > 0` parsed from the junit file). `xtask/tests/policy_drift.rs` extracts the fenced block that follows `L0 commands (RHA-Rust)` in the spec, strips `#` comments, and asserts equality with the policy argv. `ci.yml` and CONTRIBUTING invoke only `cargo xtask ci`. Outcome mapping: tool absent → `not_run` (`reason: tool_missing`); nonzero → `failed` with `error_class`; zero + validity → `passed`. Overall outcome is the worst check; a `not_run` required check blocks eligibility (§11.4).

## 6. Pre-registered H4 corpus (`xtask/tests/corpus/manifest.toml`, committed in W2 before checker code)

Each case: `id`, `level`, `seeded`, `rule`, `cell` (§4.1 row), `expected ∈ {detect, no_alarm, expected_miss}`. Mini-workspaces are synthetic (`core-a`, `core-b`, `adapter-x`, `adapter-y`, `app-main`, `tool-t`, `harness-h`); each crate's `src/lib.rs` contains `#[allow(unused_imports)] use <dep> as _;` per declared dependency so `cargo machete` raises no false alarm; corpus workspaces have their own `[workspace]` and are excluded from the root.

**Crate level** (`cargo metadata --no-deps --offline`; no compilation):

| id | Seeded | Rule | Expected |
| --- | --- | --- | --- |
| C01 | `core-a` `[dependencies] adapter-x` | dir.core_to_adapter | detect |
| C02 | `core-a` → `app-main` | dir.core_to_app | detect |
| C03 | `core-a` → external `tokio` not in allow-list | effect.core_disallowed_dependency | detect |
| C04 | `core-a` has `build.rs`, `allow_build_scripts=false` | effect.core_build_script | detect |
| C05 | `core-a` ↔ `core-b` path cycle | graph.cycle | detect (xtask DFS; a cargo error becomes this finding) |
| C06 | `adapter-x` → `adapter-y` | dir.non_root_to_adapter | detect |
| C07 | `tool-t` (not in harness list) → `adapter-x` | dir.non_root_to_adapter | detect |
| C08 | `core-a` → `aws-sdk-s3`, forbidden `from="core-a" to="aws-sdk-*"` | forbidden.edge | detect |
| C09 | `core-b` → `tokio` with `from="role:core"` rule (tokio allow-listed to isolate the rule) | forbidden.edge | detect |
| C10 | member `mystery`, no prefix, no metadata | class.unclassified | detect |
| C11 | `adapter-x` with `metadata.rha.role="core"` | class.prefix_role_conflict | detect |
| C12 | `core-a` → `tool-t` | dir.tool_depended_on | detect |
| C13 | `adapter-x` `implements=["core-a::Port"]`, depends only on `core-b` | adapter.missing_port_owner | detect |
| C14 | `adapter-x` depends on owner only as dev-dependency | adapter.port_owner_wrong_kind | detect |
| C15 | `core-a` `[target.'cfg(unix)'.dependencies] adapter-x` | dir.core_to_adapter | detect |
| C16 | `core-a` `adapter-x = { optional = true }` | dir.core_to_adapter (§6.12) | detect |
| C17 | `core-a` renamed `ax = { package = "adapter-x" }` | dir.core_to_adapter | detect (use package `name`, not `rename`) |
| C18 | `core-a` `adapter-x.workspace = true` | dir.core_to_adapter | detect |
| C19 | `core-a` → out-of-workspace path dep not allow-listed | effect.core_disallowed_dependency | detect |
| C20 | `adapter-x` `implements=["ghost::Port"]` | meta.unknown_port_owner | detect |
| C21 | `core-a` dev-dep `tokio` not in `dev_allow` | effect.core_disallowed_dev_dependency | detect |
| L01 | `adapter-x` → owner `core-a` | – | no_alarm |
| L02 | `core-b` → `core-a` (declared, acyclic) | – | no_alarm |
| L03 | `core-a` → `sha2` (allow-listed) | – | no_alarm |
| L04 | `core-a` dev-dep `proptest` (dev_allow) | – | no_alarm |
| L05 | `app-main` → cores + adapters | – | no_alarm |
| L06 | `harness-h` (listed) → `adapter-x` | – | no_alarm |
| L07 | `core-a` `[dev-dependencies] adapter-x` | – (listed under `harness_edges`) | no_alarm |
| L08 | `adapter-x` → `tokio` | – | no_alarm |
| L09 | adapter implements two ports, depends on both owners | – | no_alarm |
| L10 | `xtask` (tools list) with heavy deps | – | no_alarm |
| L11 | explicit `role="core"` on `planner-adapter-utils` (metadata wins) | – | no_alarm |
| EM-C01 | `core-a` → allow-listed `pure-looking` which depends on `tokio` | transitive.core_disallowed_dependency | expected_miss in L0 mode (§4.1 transitive hole); detect in `--transitive` |
| EM-C02 | `core-a` calls `std::fs::read` (no dependency) | – | expected_miss at crate level (covered by W1 clippy level) |
| R01 | `core-b` uses `core_a::private_mod::Item` (`cargo check --offline`) | rustc E0603 | detect by compiler (Law 2/D3 crate cell needs no xtask rule) |

**Module level** (W7; single crates, no external deps; `cargo check --offline` confirms each fixture is legal Rust). Components `constraints`, `ordering`, glue root:

| id | Seeded | Rule | Expected |
| --- | --- | --- | --- |
| M01 | `constraints` uses `crate::ordering::Wave`; allow says only `ordering=[constraints]` | modules.cycle | detect, witness = cycle path |
| M02 | third component `model`; `constraints → model` not allowed | modules.undeclared_dependency | detect |
| M03 | `ordering` calls `super::helper()` (private root fn) | modules.child_to_parent_private | detect |
| M04 | `ordering` calls `crate::plan()` (pub glue fn) | modules.child_to_parent_private ("never upward") | detect |
| M05 | nested component `scoring = "x::ordering::scoring"` references `crate::constraints` | modules.child_to_parent_private (depth 3) | detect |
| M06 | `ordering` uses `crate::constraints::rules::Rule` (below facade) | modules.foreign_internal | detect |
| M07 | `use crate::constraints::*` at facade | – | no_alarm |
| M08 | intra-component `self::`/`super::` inside `ordering::inner` | – | no_alarm |
| M09 | `std::`, `core::`, external paths | – | no_alarm |
| M10 | `#[path = "elsewhere.rs"] mod part;` referencing `crate::constraints::rules` | modules.foreign_internal | detect (re-register as expected_miss with reason if `#[path]` unsupported) |
| M11 | `#[cfg(test)] mod tests` referencing `crate::ordering` | – (test edges excluded by default, listed) | no_alarm |
| M12 | inline `mod extra {}` in `constraints` referencing `crate::ordering` | modules.cycle | detect |
| M13 | `impl crate::ordering::Trait for X` in `constraints` | modules.cycle | detect |
| M14 | expression path `crate::ordering::score(x)` in a fn body | modules.cycle | detect |
| M15 | type position `fn f(w: crate::ordering::Wave)` | modules.cycle | detect |
| M16 | `use` inside a fn body | modules.cycle | detect |
| M17 | nested `use crate::{ordering::Wave, constraints::Rule}` | modules.cycle | detect |
| M18 | `pub use crate::ordering::Wave;` inside `constraints` | modules.cycle | detect |
| M19 | root `pub use ordering::score;`, `constraints` calls `crate::score()` | modules.child_to_parent_private (upward, not the hidden edge) | detect (documented) |
| M20 | path inside macro args `assert_eq!(crate::ordering::score(), 1)` | modules.cycle (`extraction: heuristic`) | detect |
| M21 | `use ordering::Wave;` (uniform path to sibling) | modules.cycle | detect |
| L-M01 | product crate `site` with its real rules | – | no_alarm |
| L-M02 | glue root references both children | – | no_alarm |
| EM-M01 | root `macro_rules!` expanding to `crate::ordering::score()`, invoked in `constraints` | – | expected_miss (§4.1 "macro-generated paths") |
| EM-M02 | `include!("../fragments/uses_ordering.in")` | – | expected_miss |
| EM-M03 | trait method via glob-imported trait; type from `model` | – | expected_miss for the `model` edge only |
| X-M01 | `cargo modules dependencies --acyclic` on M01 and L-M01 | reference tool (uncollapsed, stricter) | recorded; `not_run` until installed (DP-2.1) |

Harness: `cargo xtask corpus run --level crate|module` computes `detection = detected/|violations|`, `false_alarm = alarms/|legitimate|`, lists expected-miss cases with hole citations, writes `evidence/h4-<level>/<ts>-<sha>.json`; any miss or unexplained alarm fails the harness and downgrades the cell in `docs/enforcement-map.md`. Kennedy's held-out cases (outside the repo) run via `cargo xtask architecture --manifest-path <held-out>/Cargo.toml --rules <held-out>/rha-crates.toml --format json` at acceptance.

## 7. Evidence record and maturity ledger shapes

`evidence/<CHG or campaign>/<utc-ts>-<shortsha>.json` (also `target/rha/evidence.json`, §12.3). Groups follow §11.3; five outcome states per §11.4; `evidence_class` is `local` or `ci`, never `protected` (the PR workflow executes candidate `xtask`, so acceptance logic is not isolated; stated in `docs/threat-model.md`).

```json
{"schema_version": 1, "record_kind": "evidence", "evidence_class": "local",
 "producer": {"principal": "agent:executor", "accountable_to": "human:kennedy", "tool": "xtask ci 0.1.0", "tool_git_rev": "<sha>"},
 "change_claim": {"task": "CHG-003", "intent": "…", "non_goals": [], "affected_components": [], "contract_changes": "none", "architecture_delta": "none", "performance_impact": "none claimed", "unresolved": []},
 "artifact_identity": {"revision": "<40 hex>", "tree": "<40 hex>", "branch": "chg/003-…", "dirty": false, "tracked_diff_sha256": null, "untracked_inputs": [], "baseline_revision": null, "digest_algorithm": "sha256"},
 "verification_identity": {"policy_path": ".rha/policy.toml", "policy_digest": "sha256:<64 hex>", "policy_revision": "<base sha>", "toolchain": {"rustc": "1.98.1 (…)", "cargo": "1.98.1", "clippy": "0.1.98"}, "target": "x86_64-unknown-linux-gnu", "features": "--all-features", "profile": "dev", "lockfile_sha256": "<64 hex>", "instruction_sources": [{"path": "AGENTS.md", "sha256": "…"}]},
 "observed_checks": [
   {"id": "L0.fmt", "kind": "format", "argv": ["cargo","fmt","--all","--","--check"], "params": {}, "outcome": "passed", "exit_status": 0, "error_class": null, "started_at": "2026-…Z", "duration_ms": 812, "selection_counts": null, "artifacts": [], "limits": []},
   {"id": "L0.nextest", "kind": "test", "argv": ["cargo","nextest","run","--workspace","--all-features","--profile","ci"], "params": {"proptest_cases": 256}, "outcome": "not_run", "exit_status": null, "reason": "tool_missing: cargo-nextest (rha-baseline.json)", "required": true},
   {"id": "L0.architecture", "kind": "architecture", "argv": ["cargo","xtask","architecture"], "outcome": "passed", "exit_status": 0, "selection_counts": {"crates": 8, "edges": 20, "findings": 0}, "artifacts": [{"path": "target/rha/architecture.json", "sha256": "…"}], "limits": ["no-deps mode; module checks not_run (W7)"]}],
 "evidence_inputs": {"fixtures": [{"path": "xtask/tests/corpus/manifest.toml", "sha256": "…"}], "seeds": {"proptest": "persisted in crates/*/proptest-regressions"}, "environment": {"os": "Linux 6.18.33.2-microsoft-standard-WSL2", "runner": "local"}},
 "performance": null,
 "agent_context": {"model_id": "<as exposed or unknown>", "harness": "<version or unknown>", "task_scope": "CHG-003", "permissions": "described in .rha/tasks/CHG-003.toml; enforced externally", "budgets": "unknown"},
 "disposition": {"eligibility": "blocked", "blocking": ["L0.nextest not_run"], "acceptance": "pending", "exception": null, "acceptor": null}}
```

Validation rules (enforced by W15's schema and semantic checks; observed by hand before that): unique check ids; every `policy.lanes.L0` id present; `passed ⇒ exit_status == 0 ∧ kind validity`; `failed ⇒ error_class`; `not_run ⇒ reason ∧ exit_status == null`; `not_applicable ⇒ policy_rule_id` + rationale (root policy only); `inconclusive` only for `kind: comparison`; 64-hex digests; RFC 3339 timestamps.

`docs/maturity.md` row: `| §1.4 mechanism | Spec § | Artifact path | Version (crate version + tag) | Proposed maturity (Executor) | Evidence record(s) | Checking scope | Environment | Accepted maturity (Kennedy, date, acceptance ref) |`. The Executor edits only "Proposed"; Kennedy edits "Accepted"; the spec's §1.4 changes only at a version bump.

## 8. Work items

Common brief fields (§11.7.4): **Mode** agent (one Executor). **Roles** Planner Kennedy (this plan), Executor the agent session (`agent:executor`, any model), Integrator Kennedy (merges), Verifier `xtask ci` local + GitHub Actions (class `ci`, advisory, not protected), Acceptor Kennedy. **Authority** the Executor may create/edit files in the repo, run cargo/git/gh, push `chg/*` branches, open PRs; it may not merge, tag, or touch protected surfaces (§2 item 2); escalation = stop and list the unknown under "Acceptance concerns" in the PR. Each item ships its task record, change record, evidence, and `cargo xtask docs` regeneration.

### Phase 0 (milestone M0, tag `v0.10-m0`)

**W0 (CHG-000) Bootstrap.**
- Intent: a repository that runs the §12.1 lane through one entry point and records honest evidence, with the spec's contribution artifacts installed and CI running on GitHub.
- Scope: everything in §4.1; `xtask` subcommands `ci`, `evidence`, `docs` only, plus an `architecture` stub that reports `not_run: not_implemented`. Non-goals: product crates, checker rules, corpus.
- Contracts: `Cargo.toml` lints verbatim §6.4 (no `-D warnings`); `rust-toolchain.toml` verbatim §6.4; nextest ci profile §6.16.3; `clippy.toml` §6.16.2; `AGENTS.md` = literal §11.7.3 shape with the constraints "Core crates perform no I/O and read no clock; effects cross ports" and "Adapters are constructed only in app-cli"; CONTRIBUTING sections §11.7.4; PR template §11.7.5; task record §11.7.11 (TOML); `docs/threat-model.md` declares ECC-Solo, cooling-off, exception log, and that CI evidence is class `ci`.
- Dependencies: DP-0.1 git identity, DP-0.2 repo name/visibility, DP-0.3 spec move, DP-0.4 record format, DP-0.5 policy parameters, DP-0.6 branch protection.
- Steps: `git init -b main`; set repo-local identity; first commit = spec only (the base every later record cites); `git mv rha-spec-v0.10.md docs/spec/` and `git mv IMPLEMENTATION-PLAN.md docs/plan/`; `rustup toolchain install 1.98.1`; `cargo install --locked cargo-nextest cargo-mutants cargo-modules cargo-machete typos-cli` (record versions in `rha-baseline.json`; cargo-modules/mutants may be deferred to DP-2.1); write the files; `gh repo create <name> --source=. --push`; branch protection via `gh api`; action SHAs via `gh api repos/<owner>/<repo>/commits/<tag>`.
- Acceptance: `cargo xtask ci --print` lists exactly the eight §12.1 commands in order; `cargo xtask ci --record evidence/CHG-000` produces a record where fmt/clippy/doctest/deny pass (deny may be `failed: network` on first fetch, recorded), `L0.architecture` is `not_run: not_implemented`, nextest/machete/typos `passed` if installed else `not_run: tool_missing`; `xtask/tests/policy_drift.rs` passes; CI on PR #1 uploads `evidence.json` with `evidence_class: "ci"`; branch protection active.
- Replan: 1.98.1 fails to install (pin 1.98.0, record in ADR-0001); `gh repo create` refused.
- Unknowns: whether `cargo deny --offline check` works after the first DB fetch; exact `install-action` SHA.

### Phase 1 (milestone M1) — execution order W1 → W2 → W3 → W4 → W5 → W6

**W1 (CHG-001) Ambient-effect deny list and Clippy discovery experiment.**
- Intent: a core-crate `clippy.toml` deny list that fails the build on a seeded `SystemTime::now()`, with the pinned Clippy's config discovery verified and recorded (§6.8 asks for exactly this).
- Scope: `xtask/tests/corpus/clippy/discovery/` (root `clippy.toml` with test allowances; `crates/core-a/clippy.toml` with the deny list; `crates/adapter-x/` with no local file) and `core-seeded/` (core-a calls `std::time::SystemTime::now()`); `xtask/templates/core-clippy.toml`; rule `effect.core_clippy_template` (every `role=core` crate's `clippy.toml` must equal the template). Non-goals: product crates.
- Contracts: deny list = §6.8 entries plus `std::fs::read`, `std::fs::read_to_string`, `std::fs::write`, `std::env::vars`, `std::io::stdin`, and `disallowed-macros = [std::println, std::eprintln, std::dbg]`; adapters set nothing.
- Acceptance: three recorded experiments with literal commands and captured stderr in `evidence/w1-clippy/`: (1) `cargo clippy -p core-a` in `core-seeded` exits nonzero citing `clippy::disallowed_methods`; (2) in `discovery`, `cargo clippy -p adapter-x` with the same call passes (per-crate discovery via `CARGO_MANIFEST_DIR` ancestors); (3) with `allow-unwrap-in-tests = true` only at root and a test `unwrap()` in `core-a`, the `unwrap_used` warning appears (no merge: nearest file wins entirely). Findings go to `docs/adr/ADR-0002-clippy-config-discovery.md`. If (2) or (3) contradicts the expectation, record the observed rule and switch to `CLIPPY_CONF_DIR` per crate set by `xtask ci` (environment, not command, so §12.1 is intact).
- Unknowns: whether Clippy 0.1.98 errors on `clippy.toml` + `.clippy.toml` in one directory (avoid the pair).

**W2 (CHG-002) Corpus pre-registration.**
- Intent: commit `xtask/tests/corpus/manifest.toml` with every case in §6 (ids, seeded description, rule, cell, expected) and the directory skeleton before any checker code exists (§17: thresholds and cases fixed before data; §9.14).
- Acceptance: Kennedy reviews the manifest (DP-1.1), confirms 2–3 held-out cases exist outside the repo, merges. The merge sha is cited as pre-registration identity in every H4 record.

**W3 (CHG-003) `cargo xtask architecture`, crate level.**
- Intent: the checker of §5 with the listed rule ids; `--manifest-path`/`--rules` for external workspaces.
- Scope: `xtask/src/{metadata, graph/*}`; `rha-crates.toml`; per-crate metadata convention. Non-goals: module graph (`module_checks` outcome `not_run: not_implemented`), transitive mode (flag accepted, `not_run` unless enabled).
- Contracts: exit codes and JSON of §5; total classification; dev-kind edges excluded from D5 but listed; renamed/optional/target/inherited deps resolved through `cargo metadata` fields, never by parsing Cargo.toml text.
- Dependencies: W2 merged (manifest is read-only input).
- Acceptance: unit tests per rule on synthetic `CrateGraph`s; `cargo xtask architecture` on the crate-less root exits 0 with `crates: 1 (xtask)`; policy drift test unchanged; CI green.
- Replan: `cargo metadata --no-deps` lacks a needed field → resolved mode with `--offline --locked`, mode recorded.

**W4 (CHG-004) H4 crate-level harness and evidence.**
- Intent: fixtures for every crate-level case in §6, `cargo xtask corpus run --level crate`, first `evidence/h4-crate/` record, `docs/enforcement-map.md` generation.
- Acceptance: 21/21 violations detected, 11/11 legitimate no-alarm, EM-C01/EM-C02 recorded as documented holes, R01 detected by `cargo check`; Kennedy's held-out cases behave as he expects; `docs/maturity.md` row "Crate-graph checker: proposed V (crate level)" citing the evidence path. Any miss: harness fails, cell downgraded, no promotion proposed.

**W5 (CHG-005) `rhawiki` minimal product.**
- Intent: `rhawiki build --root docs --out target/site` renders every page under `docs/` (including the spec) to HTML with stable heading anchors, a per-page TOC, resolved `[[wikilinks]]`, backlinks, and a "built at" footer; `rhawiki check --root docs [--format json]` prints witnesses (duplicate page ids, duplicate slugs, broken/ambiguous links, missing anchors) and exits 1 if any.
- Scope: crates `library`, `document`, `graph`, `site` (+ `rha-modules.toml`, scoped `AGENTS.md`), `adapter-fs`, `adapter-html`, `adapter-sys`, `app-cli`; `rha-crates.toml` allow-lists (approved with the PR); ledger entries §3.3; BDR-0001..0004 written by Kennedy first (DP-1.3); `docs/index.md`, `docs/architecture/components.md`. Non-goals: transclusion, `§`-refs, `[Rn]` citations, callout rendering (preserve as blockquote), mermaid execution (emit `<pre class="mermaid">`), front matter, incremental build, `serve`, search index.
- Contracts (self-contained; all sync, static dispatch):
  - `library`: `PageId` = NFC-normalized relative path without `.md`, `/`-separated, case preserved; `Digest` = sha256 of text bytes; `trait SourceRepository { fn list(&self) -> Result<Vec<RelPath>, RepositoryError>; fn read(&self, p: &RelPath) -> Result<String, RepositoryError>; }` with error vocabulary `NotFound | NotUtf8 | Io(String)`; `load()` complete-or-fail; `Corpus::new(Vec<Source>) -> Result<Corpus, DuplicatePageId>`.
  - `document`: `parse(&Source) -> Document{id, source_digest, title, headings, links, body: Vec<Node>, diagnostics}`; pulldown options `ENABLE_TABLES | ENABLE_WIKILINKS | ENABLE_GFM | ENABLE_HEADING_ATTRIBUTES`; slug = GitHub style (lowercase, whitespace→`-`, strip punctuation except `-`/`_`, NFC) with no silent dedupe (duplicate → `Diagnostic::DuplicateSlug`; the second heading gets `slug-2` deterministically so rendering proceeds).
  - `graph`: wikilink resolution = exact id, else unique case-insensitive basename, else `BrokenLink`/`AmbiguousLink`; `#heading` must match a slug (`MissingAnchor`).
  - `site`: `build_all` order: `sink.list()` → `Observation` → parse all → resolve → `step` → apply commands via `sink` → `BuildReport{written, deleted, witnesses}`; `Inv_K` checked in `step` (property test; witness returned in `SiteError::InvariantViolation`).
  - `adapter-html`: escaping in one function with property tests; output path `<page_id>.html`; `assets()` returns `assets/style.css`.
  - `check --format json`: `{"witnesses": [{"kind": "broken_link", …}], "counts": {…}}`; schema fixed in the change record so held-out expectations can be authored.
- Dependencies: W1 (template rule), W3 (checker runs from the first commit), DP-1.2, DP-1.3.
- Shared assumptions: pulldown-cmark 0.13 wikilink event shape; `walkdir` follows no symlinks by default.
- Acceptance: `cargo xtask ci` all `passed`; `cargo xtask architecture` 0 findings with classification `library|document|graph|site: core (metadata)`, `adapter-*: adapter (prefix)`, `app-cli: app (prefix)`, `xtask: tool (list)`; `rhawiki check --root docs` reports zero witnesses (or a list Kennedy accepts, DP-1.5); `rhawiki build` renders the spec with 182 headings, 2 mermaid fences preserved, tables rendered; contract suites pass for `FsSources`, `MemorySources`, `FsSink`, `RecordingSink`, `HtmlRenderer`, `StubRenderer`; seeded violators in `library/tests/contract_seeded.rs` (`SilentPartialSource` drops the last file; `TruncatingSource` returns half the text; `UnstablePathsSource` changes listing between calls; `DuplicatePathSource`) are each reported by the suite (list pre-registered in the change record); property tests: corpus ids unique or witness; slugs unique or witness; backlinks symmetric; permutation invariance of `resolve` (§9.4); `Inv_K`; `assemble` deterministic. Kennedy runs his held-out fixture set (DP-1.4) at acceptance. Components touched when `adapter-sys` was added is recorded in `docs/observations/change-spread.md` (first descriptive observation).
- Replan: pulldown wikilink events differ from assumption (post-process `Text` events; record); a needed core dependency is not allow-listed (escalate; never edit the allow-list silently).
- Unknowns: GFM alert parsing side effects on the spec's plain blockquotes; `ENABLE_HEADING_ATTRIBUTES` on headings containing `{`.

**W6 (CHG-006) Fast lane validated in CI; conformance self-assessment.**
- Intent: the §12.1 lane runs on GitHub Actions via `cargo xtask ci --label ci`; evidence artifact retained and a copy committed under `evidence/ci/`; `assurance.yml` scheduled skeleton records `not_run` jobs honestly; `docs/conformance/self-assessment.md` filled.
- Acceptance: one CI run whose `evidence.json` has all eight checks `passed` with `selection_counts.tests > 0`; `docs/maturity.md` rows "Fast-lane CI configuration: proposed V" (run id + record) and "Ambient-effect deny list: proposed V (crate level)"; the RHA-Rust claim statement per §1.4 ("fast lane runs; crate-graph checker at I/V") with scope and environment. Milestone report M1.

### Phase 2 (milestone M2)

**W7 (CHG-007) Module-graph check (§6.13) and module corpus.** `xtask/src/modules/*`: syn-based extraction of the module tree (`mod` items, `#[path]`, inline modules, `#[cfg(test)]` excluded by default) and every path in `use` trees, expressions, types, impls, and macro token streams (heuristic, flagged); resolve `crate::`, `self::`, `super::`, uniform sibling paths; collapse to `rha-modules.toml` components; rules `modules.cycle`, `modules.undeclared_dependency`, `modules.child_to_parent_private` (any reference to an ancestor module's items outside declared sibling components), `modules.foreign_internal` (path below another component's facade); unresolved paths reported as limitations. Fixtures for every M/L-M/EM-M case; `cargo xtask corpus run --level module`; `cargo modules dependencies --acyclic` recorded as X-M01 once installed (DP-2.1). Acceptance: all `detect` cases detected, no alarms on L-M01/L-M02 and on the real `site` crate, expected-miss cases recorded with §4.1 citations; `module_checks[site].outcome = passed` in the architecture report; maturity row "Module-graph check: proposed V" or the cell downgraded to "review" in `docs/enforcement-map.md`.

**W8 (CHG-008) `adapter-json` and the H3 measurement.** Add `adapter-json` implementing `PageRenderer` (page JSON + search index) and bind it in `app-cli` behind `--format json`. Record `git diff --numstat <before>..<after> -- crates/library crates/document crates/graph crates/site` (target: empty) and the seeded-violation detection list of `site::contract::page_renderer` (non-deterministic renderer, panicking renderer, renderer that drops the TOC). Ledger: `site.build.renderer_deterministic_total` discharged for a third adapter. Maturity/claims: H3 both halves observed at small scale, recorded in `docs/observations/` (not a controlled study).

**W9 (CHG-009…013) Custom features as scripted contributions.** One PR each, in this order, each recording components touched and the witness added: transclusion `![[Page#Section]]` with `TransclusionCycle` (document + graph + assembly expected); `§n.n` references auto-linked to spec headings (document only expected); `[Rn]` citations linked to reference entries (document only); callouts (document + adapter-html, presentational part named); tags/front matter index page (document + assembly). Each PR's change record states the expected touched set **before** implementation and the observed set after; `docs/observations/change-spread.md` accumulates the table.

**W10 (CHG-014) Mutation as diagnosis.** `.cargo/mutants.toml` with timeouts; L1 trigger: `cargo mutants --in-diff <(git diff <base>...HEAD) -p document -p graph -p site` when policy/invariant code changes; `assurance.yml` broad run weekly. Survivors are listed in the change record as questions ("which behavioural distinction can the suite not observe?"), never as a score; tests added only where a survivor names a real distinction. Kennedy adjudicates survivors kept as "not observable by design" (DP-2.2).

**E1 (experiment, no PR to product).** `experiments/no-std-graph/`: hypothesis.md, attempt `#![no_std]` + `alloc` for `graph`, record friction and what it caught; result feeds §6.8's "what practical signal does optional no_std add" question. Stays in `experiments/` (§5.5).

### Phase 3 (milestone M3)

**W11 (CHG-015) Bench harness and workloads.** `cargo xtask bench --arm <name> --workload docs|synth-1k|synth-5k --executions N --out FILE`: generates synthetic corpora with controlled link density (seeded RNG, seed recorded), runs `rhawiki build` in a fresh process per execution, records one summary per execution (wall-clock median of the build, peak RSS if available), toolchain, revision, hardware description, warm-up policy. Workload envelope written to `docs/architecture/efficiency.md` after DP-3.1.

**W12 (CHG-016) `cargo xtask compare` and its validation (§10.9).** Inputs two bench files; outputs effect estimate θ, two-sided 90% interval (t-interval when per-execution summaries are roughly normal, percentile bootstrap otherwise), decision `within | beyond | inconclusive` against δ, Holm correction for declared secondaries, and the run-count planner `n ≈ 12.4·(σ/δ)²`. Validation: (a) Monte Carlo test in `xtask/src/stats/tests.rs`: simulated A/A data at the σ/δ pairs of the §10.9.3 table yields `within` at ≈ 80% for the tabulated n and the interval covers zero ≈ 90%; (b) real A/A: two `bench` runs at the same revision → compare → record σ; (c) seeded performance mutation (replace the map lookup in `graph::resolve` with a linear scan, kept on a branch) → compare must not report `within` at the declared δ. Maturity row "Statistical comparison protocol: proposed V".

**W13 (CHG-017) Count-based checks (§10.3).** Counting global allocator in the bench harness (allocations per build at the declared workload); counting `SourceRepository` fake asserting O(1) `list()` calls and exactly one `read()` per page per build; budgets in `.rha/policy.toml` (DP-3.1) as L1 checks triggered by changes under `crates/site`, `crates/graph`. Row "count-based efficiency check: proposed I".

**W14 (CHG-018, optional) `serve --watch` as Law 15 reconciler.** `adapter-http` (driving adapter: HTTP server + `notify` watcher) turns requests and fs events into `Observation`s for `SiteState::step`; the core re-observes (`sink.list()`) before concluding the output matches. §9.5 model-based tests over traces: (a) observation after command, (b) stale observation ignored, (c) partial listing rejected, (d) convergence after a burst. Discharges `root.fs_listing_fresh` for the served mode.

### Phase 4 (milestone M4)

**W15 (CHG-019) Record schemas and `cargo xtask rha lint`.** JSON Schema 2020-12 files under `.rha/schemas/` for task record, policy, evidence record, and the in-toto Statement v1 envelope with an RHA predicate (§11.7.11); TOML records are converted to JSON values before validation; semantic checks from §7 (unique ids, required L0 ids, outcome/exit combinations, digest lengths, timestamps, subject match, selection counts, `not_applicable` provenance). Fixtures: legitimate and malformed (§17.2 stage 2), each with the expected rejection reason. Row "Record validation: proposed V".

**W16 (CHG-020) `tools/rha-verifier`: the §11.7.6 acceptance model.** Types `Check{id, kind, params}`, per-kind strictness preorders (δ smaller stricter; test selection superset stricter; n larger stricter; time limits unordered), refinement `⊑`, join `⊔` returning `Conflict` when params are incomparable, `Policy(base)` loaded from the base revision only, total `Surface(b, m)` from `git diff --name-only` with conservative default obligations, `T_root`/`T_loc` (local policy may only add), `R_eff`, `P_eff = P_env ∩ P_root ∩ P_loc`; predicates `Authentic` (ed25519 signature over the canonical record; trusted producer keys in policy), `Applicable`, `Complete`, `Passed` (kind validity), `Eligible`, `ValidException(now)` (issuer ≠ accountable change authority, or ECC-Solo cooling-off + append-only log), `MergeAllowed`. Property tests (proptest): Lemma 1 obligations only grow under any local policy; Lemma 2 permissions only narrow; Lemma 3 `R_eff` depends on the candidate only through `Surface`; Proposition 2 no silent pass. Conformance corpus: the 14 rows of §11.7.9 plus every §17.2 adversarial addition plus legitimate fixtures, each fixture with the expected decision; outcomes written to `evidence/h5/`. The isolation exercise stays `not_run` (no protected runner exists). Row "Machine policy, evidence envelope, protected verifier: proposed V (predicate level); isolation not_run".

**W17 (CHG-021) ChangeGraph derive/explain (§11.8.1), shadow mode.** `cargo xtask rha derive --base <b> --head <h>` computes the seed set (changed artifacts mapped to `.rha/architecture.toml` ids, unresolved mappings kept visible), the finite forward closure under a declared impact policy, and witness paths; `rha explain <id>` prints the path. Property tests: termination, monotonicity, witnessability. Output is advisory (proposes additional checks); L0 membership is untouched. Row "Repository model and views: proposed I".

**W18 (CHG-022) Assumption-ledger tool (§8.3).** `cargo xtask rha ledger`: each ledger entry records the digest of its supporting contract/evidence; when a digest changes, dependent entries flip to `needs_revalidation` (never to false) and propagate through declared claim dependencies; revalidation records the new evidence and returns entries to `current`. Test: change the `SourceRepository` contract text → `library.source_repository.complete_or_fail` and its dependents flip; rerun the contract suites → restored. Row "§8.3 invalidation: proposed I".

Milestone M4 report includes the proposed §1.4 table for v0.11 with every row's artifact, tag, evidence, scope, and environment. Kennedy performs the version bump.

## 9. Kennedy's decision points (in sequence)

| DP | Decision | When | Default if unanswered |
| --- | --- | --- | --- |
| 0.1 | Git identity for the repo | before first commit | Kennedy Mosoti / kennedy.rmosoti@gmail.com (repo-local) |
| 0.2 | GitHub repo name and visibility; product name | W0 | `recursive-hexagonal-architecture`, public; `rhawiki` |
| 0.3 | Move the spec to `docs/spec/` | W0 | move |
| 0.4 | `.rha/` record format | W0 | TOML (spec examples are YAML; `serde_yaml` is unmaintained and would fail `cargo deny`; §11.7.11 allows one equivalent store) |
| 0.5 | Policy parameters: `cooling_off_hours`, `repair_loop.max_attempts`, proptest cases L0/L1, protected surfaces, non-waivable set | W0 (policy is a protected surface) | 24 h; 3; 256/4096; as in §4.1; all L0 non-waivable |
| 0.6 | Branch protection | W0 | PR required, status check `ci` required, no force-push, admins included |
| 1.1 | Approve the corpus manifest; author 2–3 held-out corpus cases outside the repo | W2 | required |
| 1.2 | `Clock` port + "built at" footer in W5 scope | before W5 | yes |
| 1.3 | Refutation criterion (metric, source, window, threshold, action) for BDR-0001 (library owns page identity), BDR-0002 (document depends on library), BDR-0003 (site composite: assembly/build, `Assemble` as a trait), BDR-0004 (adapter-fs implements two ports) | before W5 crates exist (§7.8 step 10) | required |
| 1.4 | Private markdown held-out fixtures with expected `check --format json` output | before W5 acceptance | required |
| 1.5 | Accept or annotate the `rhawiki check` witness list on this repo's docs | W5 acceptance | zero witnesses expected |
| 1.6 | Accept maturity proposals (crate-graph checker, deny list, fast lane) | W4, W6 | – |
| 2.1 | Install cargo-modules and cargo-mutants; module-level held-out cases | W7/W10 | install |
| 2.2 | Adjudicate surviving mutants | W10 | – |
| 3.1 | Primary metric, δ, α, workload envelope, hardware description, secondaries, count budgets | before any measurement | median wall-clock of `rhawiki build` on synth-5k; δ = 5%; α = 0.05 |
| 4.1 | Trusted producer keys, exception parameters for `rha-verifier`; acceptance of corpus outcomes | W16 | – |
| any | Every exception: appended to `.rha/exceptions.log` after the cooling-off delay; never self-issued inside it | as needed | – |

## 10. Deferred (cannot be validated in this setting; say so, do not fake)

H1 (flat-baseline second build by another party), H2 (human localization time), H6 (context ablation with repeated agent runs; possible later, out of scope), H7 (long horizon), H8/H10 (teams), the §11.4 review-efficacy audit (needs reviewers other than the author), the in-toto authentication channel and CI isolation exercise (H5 second half; CI stays class `ci`, never `protected`), the `cargo-generate` template (build last, from the working baseline), a second language profile. Change-spread numbers from W9 are descriptive observations, not H1.

## 11. Risks and pitfalls

| Risk | Mitigation |
| --- | --- |
| Clippy config discovery differs from "nearest file wins, no merge" | W1 experiment first; fallback `CLIPPY_CONF_DIR` set per crate by `xtask ci`; never both `clippy.toml` and `.clippy.toml` in one directory |
| `disallowed_methods` not deny under `clippy::all` | it is in `style ⊂ all`; verified by the seeded case, not assumed |
| `cargo deny` needs the advisory DB (network) | first run online; `--offline` afterwards; a network failure is recorded `failed: network`, never hidden |
| nextest does not run doctests | `L0.doctest` is a separate §12.1 check |
| `cargo metadata` resolution needing network | L0 uses `--no-deps --offline`; transitive mode is L2 with `--locked --offline` |
| Corpus crates tripping `machete`/`typos`/workspace | `use dep as _;` in fixture libs; `_typos.toml` excludes; corpus workspaces excluded from the root workspace; crate-level cases never compile |
| Edition 2024 RPITIT / dyn compatibility | all ports static dispatch; no `impl Trait` in traits in Phase 1 |
| cargo-mutants runtime | `--in-diff`, per-package, timeouts; broad runs only in `assurance.yml` |
| xtask deps leaking into shipped crates | allow-list + `cargo machete` + `dir.tool_depended_on`; no `[workspace.dependencies]` entries for tool-only crates |
| `serde_yaml` advisory | TOML records (DP-0.4) |
| GitHub workflow executes candidate `xtask` | evidence class `ci`, never `protected`; stated in `docs/threat-model.md` |
| Fabricated versions/SHAs/output | everything pasted from `gh api`, `cargo search`, `--version` output; recorded in `rha-baseline.json`; PR review checks |
| pulldown-cmark option interactions on the spec | `document/tests/spec_renders.rs` asserts 182 headings, 2 mermaid fences, table count; failures are witnesses |
| Executor overfits the corpus or the visible tests | manifest pre-registered (W2); held-out cases outside the repo (DP-1.1, DP-1.4) |
| Spec line references drift | spec content frozen at v0.10; edits create v0.11 with re-derived references |

## 12. Verification (exact commands, expected observations)

**Phase 0**
1. `cargo --version` inside the repo → `cargo 1.98.1`.
2. `cargo xtask ci --print` → exactly the eight §12.1 commands, in order.
3. `cargo xtask ci --record evidence/CHG-000` → honest five-state outcomes (`jq '.observed_checks[]|[.id,.outcome]'`); after tool installs, all `passed` except `L0.architecture: not_run (not_implemented)`.
4. `cargo test -p xtask --test policy_drift` → passes; editing one argv in the policy makes it fail.
5. PR #1 → CI green; `gh run download -n rha-evidence` yields `evidence.json` with `evidence_class: "ci"`.
6. `gh api repos/<owner>/<repo>/branches/main/protection` → required status check `ci`.

**Phase 1**
1. W1: `grep -q disallowed_methods evidence/w1-clippy/*.txt`; ADR-0002 states the observed discovery rule.
2. W3: `cargo xtask architecture --format json | jq .summary` → `errors: 0`; on `xtask/tests/corpus/crate/violations/C01` → exit 1, one finding `dir.core_to_adapter`.
3. W4: `cargo xtask corpus run --level crate --evidence evidence/h4-crate` → `detection 21/21, false_alarm 0/11, expected_miss 2 (documented)`, exit 0; held-out run as Kennedy expects.
4. W5: `cargo run -p app-cli -- check --root docs --format json | jq .counts` → zeros; `cargo run -p app-cli -- build --root docs --out target/site` → `target/site/spec/rha-spec-v0.10.html` exists; `grep -c '<h[1-6] id=' …` → 182; `grep -c 'class="mermaid"' …` → 2; `cargo nextest run --workspace --all-features --profile ci` → all pass, selected > 0.
5. W6: `jq '[.observed_checks[].outcome]|unique' evidence/ci/<run>.json` → `["passed"]`; `docs/maturity.md` shows the three proposed rows.

**Phase 2**: `cargo xtask corpus run --level module` → all M-cases per §6, `module_checks[site].outcome = passed`; `git diff --numstat <before>..<after> -- crates/library crates/document crates/graph crates/site` → empty after `adapter-json`; `cargo mutants --in-diff … -p site` writes `mutants.out/`, survivors listed as questions; `docs/observations/change-spread.md` has one row per W9 feature with expected vs observed touched sets.

**Phase 3**: two `cargo xtask bench` runs at one revision → `cargo xtask compare --baseline a.json --candidate b.json --delta <δ> --alpha 0.05` prints effect, interval, decision; the Monte Carlo test asserts ≈ 80% `within` at the tabulated n; the seeded linear-scan branch never yields `within`.

**Phase 4**: `cargo xtask rha lint` rejects each malformed fixture with its expected reason and accepts legitimate ones; `cargo test -p rha-verifier` (Lemmas 1–3, Proposition 2, 14-row corpus + adversarial additions) passes; `evidence/h5/` records outcomes; `cargo xtask rha derive --base main --head HEAD` prints seeds, closure, witness paths, unresolved mappings; `cargo xtask rha ledger` flips and restores `needs_revalidation` in the documented test.
