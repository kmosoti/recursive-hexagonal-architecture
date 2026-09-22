# BDR-0001: `library` owns page identity and the source repository port

> **Status: accepted at DP-1.3** by Kennedy on 2026-09-22, criteria as written. Originally proposed: Drafted by the Executor in P-A stage 1. Kennedy accepts or amends the refutation criterion before any product crate exists (§7.8 step 10). The candidate scoring is in [`evidence/CHG-005/bdr-candidates/`](../../evidence/CHG-005/bdr-candidates/): three stub decompositions were run through `cargo xtask architecture`. A is this plan's §3, B has fewer crates, C has more. All three pass the checker with zero errors. The choice therefore rests on the §7.2 evidence below, not on a rule violation.

```text
Boundary: library  (page identity, source loading)
Level:    crate

Hypothesis:
  Page identity (PageId, RelPath, Source, Digest, Corpus) and complete-or-fail loading
  change for different reasons than markdown syntax, and are consumed by more than
  the parser.

Evidence (data source):
  - Invariant sets differ: unique page ids (library) versus unique slugs and total
    parsing (document)                                              (domain argument)
  - adapter-fs implements SourceRepository and must not pull in the parser
                                                (code structure: candidate B's closure)
  - Three consumers: document, graph, site; plus adapter-fs as implementer
                                                        (code structure: stub graph A)

Counter-evidence:
  - Every parse consumes a Source, so document depends on library (BDR-0002).
  - Only one repository implementation exists until adapter-git (later).

Expected benefit:
  dependency isolation for adapters; an owner for the duplicate-id invariant

Expected cost:
  one crate and its public API; Source is passed by reference, so there is no conversion

Refutation criterion (proposed; Kennedy decides at DP-1.3):
  metric:     share of qualifying changes to library's public API that also change
              document's public API
  source:     git history of the P-D feature stages and later changes, by stage commit
              range (plan §8.1 P-D); qualifying = touches crates/library/src
  window:     the five P-D feature stages plus the next 20 qualifying changes,
              or 3 months, whichever is later
  threshold:  > 50 %
  action:     merge-back proposal (library + document); a review, not automatic (§7.3)
```

## Candidates considered (plan §2.4 rule 3)

| Candidate | Crates (cores) | Normal edges | Checker | Why not chosen |
| --- | --- | --- | --- | --- |
| **A**, plan §3 (chosen) | 8 (4) | 18 | 0 errors, 1 warning (`adapter.foreign_core`: `adapter-html → document`) | – |
| B, `library` and `document` merged into `content` | 7 (3) | 14 | 0 errors, 1 warning | `adapter-fs` must depend on `content` to implement `SourceRepository`, which puts the markdown parser in the dependency closure of an adapter that never parses (§7.2, "dependencies worth keeping out of the parent"). Page identity and syntax also have different invariants: unique ids versus unique slugs. |
| C, `assembly` and `build` as crates | 10 (6) | 28 | 0 errors, 2 warnings | Each of the two crates has exactly one consumer, `site`, which is §7.2's "one consumer and no enforcement benefit" cost, and it adds 10 edges. It does buy enforced direction between them, which A lacks until P-C validates the module check. That is BDR-0003's refutation condition. |
