# BDR-0006: `adapter-json` is a presentation renderer with an immutable search index

> **Status: decided** by `agent:executor` under `human:kennedy`'s delegation
> on 2026-09-23. This accepts the refutation criterion; it is not a PR or
> artifact acceptance. No `adapter-json` crate exists yet.

```text
Boundary: adapter-json  (JsonRenderer: site::PageRenderer)
Level:    crate

Decision:
  adapter-json owns presentation JSON and the immutable search-index asset.
  It depends normally only on site and serde_json 1.0.151. app-cli constructs
  it from the same corpus used by site::build_all.
```

The governing task decision is `json-adapter-boundary` in
[CHG-008](../../.rha/tasks/CHG-008-growth.toml). Its two instructions are
quoted in full:

> “Determine a large slice to hand off to gpt-6-astra ultra. Make sure it has
> a goal to implement its work and fix any and all issue in accordance with
> rha design. (Let it flag and improve the architecture and tooling if it can
> justify it no need for my approval). I approve all PRs as long as both an
> opus 5.5 and gpt-6 agent has reviewed and approved the code.”

> “Use recommended. Shift using codex to validate that the deterministic schema
> is used according to rha guidelines rather than regenerating code as thats
> the contract that should determine the qiality of the code. Reviee the PR
> codex reviews. Accept the recommend and most correct implementations going
> forward instead of escalating to me.”

## Decision

`JsonRenderer` implements `site::PageRenderer`. Its constructor is:

```text
JsonRenderer::new(&[PageModel])
  -> Result<Self, JsonRendererError>
```

The only constructor error is:

```text
JsonRendererError::DuplicatePageId { id: String }
```

The constructor rejects duplicate page IDs, copies the corpus-derived data
needed by the index, orders search entries by page ID, and precomputes
`assets/search-index.json`. It does not mutate the caller's page-model slice.
Page and node vectors retain their input order; the search-index `entries`
array is the specified ID-sorted exception.

Each page renders to `<id>.json`. `assets()` returns a stable clone of the
precomputed `assets/search-index.json` output on every call. Page IDs use the
same source-derived relative-path domain as the HTML renderer. A renderer
must be constructed again whenever the corpus changes and its input corpus
must be the corpus passed to the corresponding `build_all` call.

The adapter has no clock, filesystem, parser, graph, or core-production
responsibility. It uses normal dependencies only:

```text
site
serde_json = "1.0.151"
```

There is no core/API/allowlist change in this boundary decision. Adapters are
constructed only in `app-cli`.

The fixed JSON contract is
[JSON Renderer Contract v1](../architecture/json-renderer-contract.md).
`build --format html|json` selects the renderer, with HTML as the default.
`check --format json` keeps its existing v1 output contract unchanged.

## Rationale

| Candidate | Purity | Core isolation | Lifecycle | Simplicity | Total |
| --- | ---: | ---: | ---: | ---: | ---: |
| **Immutable JSON renderer and precomputed index (chosen)** | 5 | 5 | 5 | 3 | **18/20** |
| Batch port returning pages and an index | 5 | 1 | 5 | 5 | 16/20 |
| Stateful renderer accumulating an index during `render` | 1 | 5 | 1 | 1 | 8/20 |

The chosen design gives a third renderer without changing core production
code. The cost is index memory and a second pure `site::analyse` plus
`DefaultAssembler` pass in `app-cli` before the existing `build_all` call:
duplicate orchestration of `O(n+e)` work. The second pass is accepted as the
small-scope lifecycle cost of keeping the renderer immutable and compatible
with the existing `PageRenderer` port.

The existing `Inv_K` check rejects a page/asset path collision before any
write is applied. In particular, a page whose source-derived output collides
with `assets/search-index.json` is a failed build with no writes, not a
renderer-specific overwrite rule.

## H3 and assurance scope

W8 requires the owner contract's three seeded violators to remain observable:

1. a nondeterministic renderer;
2. a panicking renderer; and
3. a renderer that drops the TOC.

The sampled metamorphic TOC guard in `site::contract::page_renderer` is not a
semantic proof. `adapter-json` therefore requires an exact golden assertion
for its TOC projection, independent of that sampled guard.

Pre-existing assurance repairs are measured separately from the adapter
change. The change record must retain raw, uncombined outputs for:

1. the assurance-repair range;
2. the adapter-stage range; and
3. the whole P-C-to-H3 core range:

```text
git diff --numstat <before>..<after> -- \
  crates/library crates/document crates/graph crates/site
```

No counts, hashes, or results are asserted by this proposal.

## Refutation criterion

The criterion accepted under the delegation is:

```text
metric:
  (a) W8 requires any core production lines solely to add adapter-json in
      crates/library, crates/document, crates/graph, or crates/site; or
  (b) more than 50% of the next 10 changes touching adapter-json also modify
      PageRenderer or PageModel

source:
  raw stage numstat and repository history, with qualifying adapter changes
  identified from their touched adapter and port/model files

window:
  W8 plus the next 10 qualifying adapter changes, or three months,
  whichever is later

action:
  propose a batch/finalize PageRenderer port, and record the zero-core-change
  half of H3 as refuted for this adapter class
```

The criterion is a boundary-design measurement, not a requirement that a PR
or implementation artifact already exist.

