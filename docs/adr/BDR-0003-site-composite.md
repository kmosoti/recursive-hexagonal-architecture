# BDR-0003: `site` is one composite crate with `assembly` and `build` as child modules

> **Status: proposed for DP-1.3.** Drafted by the Executor in P-A stage 1. Kennedy accepts or amends the refutation criterion before any product crate exists (§7.8 step 10). The candidate scoring is in [`evidence/CHG-005/bdr-candidates/`](../../evidence/CHG-005/bdr-candidates/): three stub decompositions were run through `cargo xtask architecture`. A is this plan's §3, B has fewer crates, C has more. All three pass the checker with zero errors. The choice therefore rests on the §7.2 evidence below, not on a rule violation.

```text
Boundary: site  (composite; children assembly and build as modules; Assemble as a trait)
Level:    crate for site; module for assembly and build

Hypothesis:
  Page assembly (TOC, breadcrumbs, backlinks) and build planning (the pure step
  transition) are distinct decisions with one consumer, the site glue; module privacy
  suffices between them, and the crate boundary buys isolation from adapters.

Evidence (data source):
  - build must stay a pure (state, observation) -> (state', commands) transition;
    assembly is a pure projection                                    (domain argument)
  - C's crates each have exactly one consumer                  (code structure: stub C)

Counter-evidence:
  - §7.5: a component whose correctness depends on enforced direction has earned a
    crate unless the module-graph check is in place and validated. Until P-C, the
    direction build -> assembly, never the reverse, is enforced by review only.

Expected benefit:
  fewer crates and edges than C; one composite with a rha-modules.toml policy

Expected cost:
  unenforced direction between children until P-C validates the module check

Refutation criterion (proposed; Kennedy decides at DP-1.3):
  metric:     (a) the P-C module corpus result for the module cells (law3-d1, law6-b3,
              d2); (b) share of stages that change both assembly and build
  source:     (a) evidence/h4-module (P-C); (b) git, stage ranges (P-D)
  window:     (a) P-C acceptance; (b) the five P-D feature stages
  threshold:  (a) any module cell not validated; (b) > 60 %
  action:     (a) promote assembly and build to crates (candidate C);
              (b) merge the two modules
```

## Candidates considered (plan §2.4 rule 3)

| Candidate | Crates (cores) | Normal edges | Checker | Why not chosen |
| --- | --- | --- | --- | --- |
| **A**, plan §3 (chosen) | 8 (4) | 18 | 0 errors, 1 warning (`adapter.foreign_core`: `adapter-html → document`) | – |
| B, `library` and `document` merged into `content` | 7 (3) | 14 | 0 errors, 1 warning | `adapter-fs` must depend on `content` to implement `SourceRepository`, which puts the markdown parser in the dependency closure of an adapter that never parses (§7.2, "dependencies worth keeping out of the parent"). Page identity and syntax also have different invariants: unique ids versus unique slugs. |
| C, `assembly` and `build` as crates | 10 (6) | 28 | 0 errors, 2 warnings | Each of the two crates has exactly one consumer, `site`, which is §7.2's "one consumer and no enforcement benefit" cost, and it adds 10 edges. It does buy enforced direction between them, which A lacks until P-C validates the module check. That is BDR-0003's refutation condition. |
