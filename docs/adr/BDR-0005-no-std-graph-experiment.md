# BDR-0005: `no_std` graph experiment uses product types

> **Status: E1 refutation criterion accepted** by agent:executor under human:kennedy's delegation. This records the experiment criterion, not P-C acceptance or an Accepted maturity change. See [task decision e1-boundary](../../.rha/tasks/CHG-007-module-level.toml) for the complete authority quotes.

```text
Boundary: experiments/no-std-graph  (standalone experiment package)
Level:    crate

Hypotheses:
  H1: the own resolver compiles under #![no_std] after import adaptation
  H2: existing graph property/oracle tests pass unchanged with real types
  H3: an intentional direct-std probe fails to compile

Refutation criterion (accepted for E1; task decision: e1-boundary):
  refuted if algorithm edits are needed, behavior controls differ, or the
  direct-std negative probe compiles
```

The experiment is a standalone package under `experiments/no-std-graph`, with
its own `[workspace]` root and `[lib] name = "graph"`. It copies the current
graph resolver and existing graph property/oracle tests byte-for-byte, adapting
only `no_std`/`alloc` imports. It uses actual product document/library types,
preserving real behavior rather than introducing local model projections.

There is no new product consumer, root workspace/member edit, or production
edit. This BDR precedes creation of the experiment `Cargo.toml` under the
new-crate rule. The future result path is
`experiments/no-std-graph/result.md`; it does not contain an existing pass.
Actual commands and outcomes will be recorded later. `no_std` permits
allocation, and actual dependencies may retain `std`; this makes no embedded,
heap-free, panic-free, specification, or method-change claim.

## Candidates considered

| Candidate | Semantic fidelity | Transitive isolation | Scope fit | Effort | Oracle reuse | Total |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| **Real product types snapshot (chosen)** | 5 | 2 | 5 | 4 | 5 | **21/25** |
| Dependency-free minimal model projection | 3 | 5 | 5 | 2 | 3 | 18/25 |
| Convert production closure to `no_std` | 5 | 5 | 1 | 1 | 5 | 17/25 |

Scores are qualitative judgments, five criteria at five points each. The
chosen snapshot scores highest overall because it preserves real behavior while
remaining experimental. Stronger isolation changes the model and weakens the
E1 signal; production conversion exceeds experiment scope. Responsibility
remains with the same graph resolver; the snapshot is frozen; existing acyclic
direct dependency direction and the oracle remain unchanged. No new logical
product boundary is introduced.
