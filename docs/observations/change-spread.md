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
| Transclusion (CHG-009) | document + graph + site::assembly | document, graph, site::assembly, site::build, site facade, app-cli | Renderer setup tests migrate Assemble calls; owner-local oracle tests, dev-only serde_json manifests/live test allow-list, records and generated docs. Renderer production remains unchanged. | This BDR/contract/prediction commit, before corpus and code | pending |
