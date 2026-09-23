# CHG-008 (P-D): growth under measurement

Task: [CHG-008](../../.rha/tasks/CHG-008-growth.toml). Covers CHG-008 through CHG-014; plan P-D/W8/W9/W10. Agent mode, ECC-Solo. Kennedy is Planner, Integrator and Acceptor; this Executor owns implementation and evidence, with separate GPT-6 and supervising Opus reviews.

## Intent and scope

Add adapter-json, measure H3, deliver the five W9 features as separately predicted and gated contributions, then diagnose mutation survivors. P-D starts from the unmerged P-C head and will open a stacked PR; no P-C acceptance is fabricated and no merge is performed.

## Deltas

Stage 0 is the declared assurance prerequisite investigation. Each later feature gets a committed expected touched set and contract/corpus before implementation. Protected deltas and alternatives will be recorded under the quoted delegation.

## Evidence

The assurance prerequisite gate passes 280 tests with current generated docs and zero scope findings. P-C evidence remains in its own record. Later feature stages, raw change-spread measurements, mutation questions and final lane/review will be linked here as they run.

## Decisions

Task decisions delegation-m2, assurance-prerequisites and protected-growth-scope state initial authority. Status lives in the decision ledger; DP-2.1 was decided in P-C, and DP-2.2 will adjudicate each observed survivor.

## Acceptance concerns

Held-out material remains not_run and Kennedy-owned. Both PRs remain unmerged. Authentic/protected producer limits remain DP-4.1. No inherited corpus grade will be silently changed.

### Stage 0: assurance prerequisites, before H3

P-C's forward CI repair was merged into this local stacked branch at 2fa29ae93ea71020113301f8e7deee3abf09a935. The only merge conflict was its generated task index, resolved by cargo xtask docs. Both remote PRs remain subject to review; no PR was merged.

**Repair attempt 1.** Hypothesis: the owner contract can pass a TOC-dropping renderer, lets asset panics escape, misses page/asset collisions and discards a post-delete listing error; build_all also subtracts asset writes from its source-page unchanged count. The [control source](../../evidence/CHG-008/stage-0/pd-assurance-controls.rs) was compiled against the unchanged built crates. [Observed result](../../evidence/CHG-008/stage-0/pd-assurance-negative.txt): three positive controls pass and five discriminating tests fail. In the concrete two-page asset-only update, unchanged is 1 instead of 2. Decision confirmed-assurance-repairs fixes those consequences before adding the JSON adapter. Implementation and repaired results follow in the next stage commit.

**Graph API determination.** [A separate executable probe](../../evidence/CHG-008/stage-0/pd-graph-precondition.txt) observes permutation-sensitive resolution for duplicate PageIds and confirms Corpus rejects the same duplicate sources. The product input domain is protected. The repair documents the uniqueness precondition on graph::resolve; it does not invent resolution semantics for invalid duplicate-ID slices.

The stage prediction was committed first in 25c75c6. Before production edits, it is refined to include site::build port documentation in addition to site::contract, site::testing, site glue and graph API documentation. The original prediction remains visible. Both the full P-C-to-H3 core diff and the isolated adapter-stage diff will be retained.

**Repair result and stage gate:** the owner contract now distinguishes the dropped TOC, contains asset panics, reports asset nondeterminism and duplicate paths, and propagates post-delete listing errors. The fake preserves the observed TOC. Asset-only writes no longer reduce unchanged source-page counts. The graph API states its existing uniqueness precondition. [Targeted positive/negative controls pass](../../evidence/CHG-008/stage-0/pd-stage0-controls.txt); [all 280 nextest tests pass](../../evidence/CHG-008/stage-0/pd-stage0-gate.txt), none skipped; [Clippy](../../evidence/CHG-008/stage-0/pd-stage0-clippy.txt), [docs](../../evidence/CHG-008/stage-0/pd-stage0-docs-check.txt) and [scope](../../evidence/CHG-008/stage-0/pd-stage0-scope.txt) pass. TOC sensitivity is a sampled discrimination check; each adapter still needs an exact semantic assertion. No old test or corpus grade was weakened.

Stage 0 implementation range: 25c75c6..e687b1ab4171935173d90dbf5fcbe568a9118e61. [Raw core numstat](../../evidence/CHG-008/stage-0/core-numstat.txt) records the exact changed files; build and graph changes in this prerequisite stage are documentation only. P-C PR18 now has green CI run 35824096298 on head 8e3037b5b4a342a3e2bec36af874246838c610b7 and independent GPT-6 approval of both implementation and repair.

### Stage 1 preparation: adapter-json and H3

[BDR-0006](../adr/BDR-0006-adapter-json-immutable-index.md) accepts the boundary's refutation criterion under Kennedy's delegation before a new crate exists. Three alternatives are scored; the immutable constructor/index wins 18/20. [The JSON contract](../architecture/json-renderer-contract.md) fixes exact page/node/search semantics and lifecycle behavior before the separate corpus generator and implementation. Object member order is not graded; values and deterministic output bytes are. The search index is fresh configuration for one corpus, avoiding hidden render-call state. Its cost is one additional pure analysis/assembly pass at the composition root. Decision json-adapter-boundary authorizes manifests and later assumption-ledger support; no core allow-list or port change is planned.

The adapter-stage predicted product set is adapter-json and app-cli, including their tests. The empty-core target applies to the full isolated stage diff; the earlier core repairs remain visible in the complete P-C-to-H3 diff. No JSON implementation or corpus evaluation has occurred at this preparation commit.

### JSON corpus registered before implementation

The separate generator, forbidden access to renderer implementations/tests, produced [79 registered cases](../../xtask/tests/corpus/json-renderer/registration.toml): 70 projection cases including 64 seeded random cases, two duplicate-ID cases, and the reconciliation, collision, owner-contract and four CLI cases. Seed 8072026 and every prompt digest are recorded in task provenance. [Root's inventory/source audit](../../evidence/CHG-008/stage-1/pd-json-registration-audit.txt) verifies all ten payloads, source snapshots and prompt digests. [Twelve package files reproduce byte-identically](../../evidence/CHG-008/stage-1/pd-json-reproduction.txt); [corrupting a copied case stream is refused](../../evidence/CHG-008/stage-1/pd-json-integrity-negative.txt). No target encoder or adapter has run at this registration commit.

**Pre-registration determinations.** A read-only audit identified live source citations that would become stale during W9, ambiguous code-block search-text wording and imprecise asset cardinality. The contract now names the intended text and exactly one asset; exact contract/Node/PageModel snapshots are frozen in the package. Future source changes do not regrade those snapshots. A [language probe](../../evidence/CHG-008/stage-1/pd-json-whitespace-reference-probe.txt) also showed Python split and Rust split_whitespace differ on four control characters. The independent reference now uses the explicit Unicode White_Space property with [hand-derived controls](../../evidence/CHG-008/stage-1/pd-json-reference-selftests.txt). Its original 79 case bytes did not change. No registered grade was changed and no implementation result informed the corpus.

**Generator repair attempt 2.** A whitespace follow-up [stopped on an invented filename](../../evidence/CHG-008/stage-1/pd-json-whitespace-correction-result.md), adding an extra generator segment to the actual supplied prompt path. The file existed at the supplied path. A bounded follow-up corrected that lookup, completed the normalization repair and fixed the documented staging reproduction command. Package metadata contains three instruction snapshots; the fourth finishing instruction is also archived and hashed in task provenance. This process failure is retained rather than presented as a completed repair.

The corpus fixes decoded JSON members/types/values and array order; repeated bytes and newline formatting are additional obligations. Every registered lifecycle/CLI case must run as written, and old corpus grades remain unchanged. Later feature extensions require separate registered expectations and must preserve these old cases.
