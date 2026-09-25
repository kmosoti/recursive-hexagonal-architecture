# P-D change-spread observations

Each expected set is committed before its stage. Observed sets come from git diff --numstat over that stage's actual commit range. The original plan prediction remains visible alongside any justified pre-implementation refinement. Records/evidence/tooling are reported separately from product components.

| Stage | Original plan prediction | Predeclared product set | Start | End | Observed |
| --- | --- | --- | --- | --- | --- |
| Assurance prerequisites | Not a W9 feature; repairs before H3 | site::contract, site::testing, site glue, graph API documentation/tests, only if a concrete failure is reproduced | 25c75c6 | e687b1ab4171935173d90dbf5fcbe568a9118e61 | site::contract, site::testing, site glue, site::build port docs, graph API docs; [raw numstat](../../evidence/CHG-008/stage-0/core-numstat.txt) |

Pre-implementation addendum after probes, before the assurance repair: include site::build port documentation so the owner trait states the strengthened observable-TOC/assets contract. The original row remains the initial prediction; this addendum is committed before any production edit. Stage start remains the original prediction commit 25c75c6.

| Stage | Original plan prediction | Predeclared product set | Start | End | Observed |
| --- | --- | --- | --- | --- | --- |
| adapter-json / H3 | adapter-json + app-cli; core diff empty | adapter-json + app-cli; no core production or test edits | d2cc1cd793bbd820a130c9df1e120cc1c9f71e7f | bd708fd3277afcb80a3898b6a498ea3c010e0e4c | adapter-json + app-cli; [core diff empty](../../evidence/CHG-008/stage-1/h3-adapter-core-numstat.txt), [product numstat](../../evidence/CHG-008/stage-1/h3-product-numstat.txt) |

Adapter measurement includes all core paths (including tests). The complete P-C-to-H3 core diff will also be retained so the preceding assurance repairs remain visible. Protected metadata: adapter/app manifests, Cargo.lock and assumption-ledger support; no new core allow-list or module direction is planned.

[H3 observation](h3-json.md) records both measurements, the seeded detection list and the limits of the claim.

| Stage | Original plan prediction | Predeclared production set | Predeclared additional changes | Start | End / observed |
| --- | --- | --- | --- | --- | --- |
| Transclusion (CHG-009) | document + graph + site::assembly | document, graph, site::assembly, site::build, site facade, app-cli | Renderer setup tests migrate Assemble calls; owner-local oracle tests, dev-only serde_json manifests/live test allow-list, records and generated docs. Renderer production remains unchanged. | 863090b (the BDR, contract and prediction commit) | 79e89b3: production document (parse, node, section), graph (lib, transclusion), site::assembly, site::build, site facade (lib) and app-cli (main), exactly the predeclared production set; renderer production unchanged (adapter-html and adapter-json changed only in test setup that migrates `assemble` calls, as predicted). Original plan prediction (document + graph + site::assembly) exceeded by site::build, the site facade and app-cli, as refined before code. [Raw numstat](../../evidence/CHG-008/stage-2/transclusion-stage-numstat.txt) |

| Stage | Original plan prediction | Predeclared production set | Predeclared additional changes | Start | End / observed |
| --- | --- | --- | --- | --- | --- |
| §-references (CHG-010) | document only | document only: parse (recognition and resolution), lib (`Document::section_refs`, `SectionRef`, `Diagnostic::UnresolvedSectionRef`), possibly a new module inside document | The registered corpus and its grader (a separate session, before code); the one test in document that builds a `Document` literal gains the new field; records and generated docs. No change outside document: nothing else builds `Document` or matches `Diagnostic` exhaustively, and `rhawiki check` output is unchanged. | 0e69149 (the contract and prediction commit) | 3d3d836: production document only (lib, parse, new section_ref), exactly the plan's and the predeclared prediction; tests: the registered grader, the review regression tests and their golden data, the one `Document` literal. No other crate changed. [Raw numstat](../../evidence/CHG-008/stage-2/section-refs-stage-numstat.txt) |

| Stage | Original plan prediction | Predeclared production set | Predeclared additional changes | Start | End / observed |
| --- | --- | --- | --- | --- | --- |
| Citations (CHG-011) | document only | document (entries, citations, `Node::Anchor`, diagnostics), site::assembly (`PageNode::Anchor`, dropping anchors in transcluded content), adapter-html (render the anchor), adapter-json (render the anchor, add no search text) | The registered corpus and grader (a separate session, before code); tests that build `Document` or `PageNode` literals, or that match nodes exhaustively, gain the new field or variant; the JSON renderer contract is amended; records and generated docs. `rhawiki check` output is unchanged. The plan's "document only" is refined before code (decision citations-anchor-node): a new link target needs render support. | e2a2f95 (the contract and prediction commit) | 0339267: production document (lib, node, parse, section, section_ref, new citation), site::assembly, adapter-html and adapter-json, exactly the refined prediction; the plan's "document only" was refuted before code. Tests: the registered graders and golden trees, three rounds of review regression tests, the team's anchor, assembly, supplementary, edge-case and property tests, and grader-compat-anchor edits in three earlier test files. No other crate changed. [Raw numstat](../../evidence/CHG-008/stage-2/citations-stage-numstat.txt) |
