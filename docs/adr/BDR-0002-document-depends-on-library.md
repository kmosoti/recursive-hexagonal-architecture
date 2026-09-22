# BDR-0002: `document` depends on `library`

> **Status: accepted at DP-1.3** by Kennedy on 2026-09-22, criteria as written. Originally proposed: Drafted by the Executor in P-A stage 1. Kennedy accepts or amends the refutation criterion before any product crate exists (§7.8 step 10). The candidate scoring is in [`evidence/CHG-005/bdr-candidates/`](../../evidence/CHG-005/bdr-candidates/): three stub decompositions were run through `cargo xtask architecture`. A is this plan's §3, B has fewer crates, C has more. All three pass the checker with zero errors. The choice therefore rests on the §7.2 evidence below, not on a rule violation.

```text
Boundary: document  (markdown to typed document), with the declared edge document -> library
Level:    crate

Hypothesis:
  Parsing is a pure transformation of a library Source into document's own node tree;
  no vendor (pulldown-cmark) type crosses its public API, so the parser choice stays
  document's decision.

Evidence (data source):
  - pulldown-cmark is needed by document only; the forbidden rule
    site -> pulldown-cmark (plan §5) enforces it                (code structure, checker)
  - Slug and heading invariants have an owner with witnesses (plan §3.2)
                                                                    (domain argument)

Counter-evidence:
  - document needs PageId and Source from library: the edge is real, declared (D4),
    and acyclic, but it couples the two.

Expected benefit:
  parser isolation; a replaceable parser behind document's own types

Expected cost:
  one mapping from pulldown events to document::Node

Refutation criterion (proposed; Kennedy decides at DP-1.3):
  metric:     (a) number of pulldown-cmark types in document's public API;
              (b) share of document changes that require a library API change
  source:     (a) cargo public-api or rustdoc JSON of document; (b) git, stage ranges
  window:     (a) every P-D stage; (b) the five P-D feature stages
  threshold:  (a) > 0; (b) > 50 %
  action:     (a) defect: wrap the type; (b) merge-back proposal (with BDR-0001)
```

## Candidates considered (plan §2.4 rule 3)

| Candidate | Crates (cores) | Normal edges | Checker | Why not chosen |
| --- | --- | --- | --- | --- |
| **A**, plan §3 (chosen) | 8 (4) | 18 | 0 errors, 1 warning (`adapter.foreign_core`: `adapter-html → document`) | – |
| B, `library` and `document` merged into `content` | 7 (3) | 14 | 0 errors, 1 warning | `adapter-fs` must depend on `content` to implement `SourceRepository`, which puts the markdown parser in the dependency closure of an adapter that never parses (§7.2, "dependencies worth keeping out of the parent"). Page identity and syntax also have different invariants: unique ids versus unique slugs. |
| C, `assembly` and `build` as crates | 10 (6) | 28 | 0 errors, 2 warnings | Each of the two crates has exactly one consumer, `site`, which is §7.2's "one consumer and no enforcement benefit" cost, and it adds 10 edges. It does buy enforced direction between them, which A lacks until P-C validates the module check. That is BDR-0003's refutation condition. |
