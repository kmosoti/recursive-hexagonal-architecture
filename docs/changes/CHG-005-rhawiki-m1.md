# CHG-005 (P-A): the rhawiki product and the M1 close

Task record: [`.rha/tasks/CHG-005-rhawiki-m1.toml`](../../.rha/tasks/CHG-005-rhawiki-m1.toml). Plan: §8.1 P-A; §8.2 W5 and W6. Base: `72c5cfd`, the merge that approved plan revision 2.

## Intent and scope

Packet P-A of plan revision 2 covers CHG-005 (W5) and CHG-006 (W6). It builds the guards revision 2 names, then the product and the M1 close, in gated stages.

## Deltas

### Stage 0, guards

- `cargo xtask scope --task <id>` (plan §2.2 M4): every changed path, committed or not, must match the task's `scope_globs`, sit in the §4 layout, and, if protected, match `protected_scope`.
- The enforcement map claims no cell from an H4 record that graded an earlier manifest.
- Mechanical pedantic warnings in `xtask/src/corpus` are fixed; length and naming warnings stay advisory under §6.4.
- `corpus held-out --kind check` moves to stage 3, where `rhawiki check` exists (task record decision `held-out-check-kind-in-stage-3`).

**The guard's first run caught this packet.** Stage 0 had added a line about the new command to CONTRIBUTING.md, which `.rha/policy.toml` has protected since CHG-000, and no approval quoted it. The line is reverted. It is proposed for Kennedy's approval with DP-5.2.

### Stage 1, boundary decisions (§7.8 step 10) and corpus registration

- **Best-of-N (§2.4 rule 3).** Three decompositions were built as stub workspaces and run through `cargo xtask architecture`; the reports are in `evidence/CHG-005/bdr-candidates/`. A is the plan's §3, B merges `library` and `document`, C splits `assembly` and `build` into crates. All pass with zero errors, so the §7.2 evidence decides. A is chosen, and each BDR records B and C with the reasons they lost.
- **BDR-0001 to BDR-0004** in `docs/adr/` each carry a proposed refutation criterion (metric, source, window, threshold, action). **Gate DP-1.3:** Kennedy accepts or amends the four criteria before any product crate exists. BDR-0003 records the real trade-off. Until P-C validates the module check, the direction between `assembly` and `build` is enforced by review only. Candidate C is the fallback its criterion names.

- **Markdown corpus registered (§2.4 rules 2 and 5).** A separate Codex session (`gpt-6-astra`, extra-high) generated 60 sites, 300 pages and 99 planted witnesses by construction, with 20 clean sites. It ran before any product crate existed. `xtask/tests/corpus/markdown/registration.toml` records the generator, the prompt digest (the prompt itself is committed beside it), and the tree digest, which `xtask/tests/corpus_markdown.rs` pins. The implementing session has not edited a site. The witness keys it uses fix part of the `check --format json` schema in advance; stage 3 conforms to them.

## Acceptance concerns

1. **An earlier unapproved protected edit, found by the new guard.** CHG-004.6's commit `ad13efa` added the held-out paragraph to CONTRIBUTING.md. That was a protected edit whose approval was recorded only as owned scope, not quoted from Kennedy. The CHG-004.6 acceptance record cannot be edited, so it is disclosed here. Kennedy may approve the paragraph retroactively or have it removed.
2. **The CONTRIBUTING line for `cargo xtask scope`** waits for approval (DP-5.2).
