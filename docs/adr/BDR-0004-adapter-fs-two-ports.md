# BDR-0004: `adapter-fs` implements two ports

> **Status: accepted at DP-1.3** by Kennedy on 2026-09-22, criteria as written. Originally proposed: Drafted by the Executor in P-A stage 1. Kennedy accepts or amends the refutation criterion before any product crate exists (§7.8 step 10). The candidate scoring is in [`evidence/CHG-005/bdr-candidates/`](../../evidence/CHG-005/bdr-candidates/): three stub decompositions were run through `cargo xtask architecture`. A is this plan's §3, B has fewer crates, C has more. All three pass the checker with zero errors. The choice therefore rests on the §7.2 evidence below, not on a rule violation.

```text
Boundary: adapter-fs  (FsSources: library::SourceRepository; FsSink: site::OutputSink)
Level:    crate

Hypothesis:
  Reading sources and writing output are one filesystem concern: shared path
  normalization, the same error vocabulary, and the same platform quirks.

Evidence (data source):
  - Both ports need /-separated relative paths and NFC handling     (domain argument)
  - Contract suites exist per port (library::contract, site::contract), so each
    implementation is checked on its own                              (code structure)

Counter-evidence:
  - The two halves fail independently: a read-only source tree with a writable
    output directory.
  - adapter.foreign_core does not fire, because it implements both owners' ports,
    but the adapter depends on two cores.

Expected benefit:
  one adapter to wire in app-cli; shared path code

Expected cost:
  a change to one port's implementation rebuilds and retests the other

Refutation criterion (proposed; Kennedy decides at DP-1.3):
  metric:     share of adapter-fs changes that touch only one of src/source.rs and
              src/sink.rs and nothing shared
  source:     git history, qualifying = touches crates/adapter-fs/src
  window:     the next 20 qualifying changes, or 6 months, whichever is later
  threshold:  > 80 %, together with no change to shared path code
  action:     split into adapter-fs-source and adapter-fs-sink
```

## Candidates considered (plan §2.4 rule 3)

| Candidate | Crates (cores) | Normal edges | Checker | Why not chosen |
| --- | --- | --- | --- | --- |
| **A**, plan §3 (chosen) | 8 (4) | 18 | 0 errors, 1 warning (`adapter.foreign_core`: `adapter-html → document`) | – |
| B, `library` and `document` merged into `content` | 7 (3) | 14 | 0 errors, 1 warning | `adapter-fs` must depend on `content` to implement `SourceRepository`, which puts the markdown parser in the dependency closure of an adapter that never parses (§7.2, "dependencies worth keeping out of the parent"). Page identity and syntax also have different invariants: unique ids versus unique slugs. |
| C, `assembly` and `build` as crates | 10 (6) | 28 | 0 errors, 2 warnings | Each of the two crates has exactly one consumer, `site`, which is §7.2's "one consumer and no enforcement benefit" cost, and it adds 10 edges. It does buy enforced direction between them, which A lacks until P-C validates the module check. That is BDR-0003's refutation condition. |
