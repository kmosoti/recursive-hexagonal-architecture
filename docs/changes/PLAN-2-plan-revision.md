# PLAN-2: the implementation plan, revision 2

Task record: [`.rha/tasks/PLAN-2-plan-revision.toml`](../../.rha/tasks/PLAN-2-plan-revision.toml). Base: `bb1ee4e`, the merge that accepted CHG-004.6.

## Intent and scope

Kennedy asked that the lessons of W0 to W4 be taken into a redeveloped plan, with larger work items that make use of a capable model's generation. This change revises [the plan](../plan/IMPLEMENTATION-PLAN.md) and nothing else, apart from the CHG-004.6 acceptance record, which is its first commit.

## Deltas

- **Front matter and §1.** Revision 2's provenance. A packet kickoff replaces the per-item kickoff. A status table covers W0 to W4.
- **§2, rewritten.** §2.1 lists twelve measured lessons, each tied to a rule. §2.2 gives the operating method M1 to M10: packets built in green stages; one writer per branch; a scope and layout guard; one L0 record per packet and no PR-head CI records; one local review with at most three rounds; registration before data; computed provenance; shell hygiene. §2.3 explains why items are larger now. §2.4 is the stochastic-generation protocol: generate wide and verify narrow; fix the grader before the generator runs; best-of-N with the losers kept; differential oracles; adversarial corpora at scale; bounds.
- **§8.1, new.** Six packets: P-A product and M1 close; P-B trust, the verifier moved from last to second; P-C module level; P-D growth under measurement; P-E efficiency; P-F model and close. Each has stages and gates. **§8.2** keeps revision 1's briefs verbatim as the acceptance reference.
- **§9.** The catalogue is updated to packet triggers, and DP-5.1 to 5.3 are new. **§11** is rewritten; revision 1's risks remain in force. **§12**: the W4 expectation now reflects what was observed.
- **Unchanged:** §3 to §7 and §10, and every CHG id.

## Evidence

The measurements behind §2.1 were taken on `main` at `bb1ee4e`: 132 non-merge commits, 46 of them records-only, in 8 pull requests; 60 CI runs. The L0 record for this change is under `evidence/PLAN-2/`.

### Review (local Codex, gpt-5.6-sol, medium, read-only), round 1 on `b85e5c0`, determinations

1. **P1, "the executor must use JJ, a separate workspace and the deep verify tier." Declined.** These rules come from the reviewer's own global instructions, not this repository. The repository's AGENTS.md and CONTRIBUTING.md say none of it, and Kennedy told the Codex executor to use Git here.
2. **P2, the record-only count is not reproducible. Confirmed.** The count depends on which paths count as records. §2.1 now states the definition: 46 of 132 counting evidence, generated indexes and change records, or 61 also counting task and acceptance records. It also gives the recount method.
3. **P1, §6 still ran held-out cases through the bare checker. Confirmed.** §6 now names `cargo xtask corpus held-out`, which verifies the commitment and prints only opaque output.
4. **P1, DP-1.2 had three triggers. Confirmed.** It is now one gate at the start of P-A stage 3, in the table, the stage and §9.
5. **P2, DP-2.1 was gated only in P-C although P-D's mutation stage needs it. Confirmed.** P-D stage 3 now waits for it if P-C has not recorded it.
6. **P2, graded corpora were generated after the code they grade. Confirmed; it contradicted §2.4 rule 2.** P-A registers the markdown corpus in stage 1, before any product code. P-B registers its lint and verifier corpora in stage 1, before any lint code. P-C generates its random graphs, and a separate session writes the reference extractor, before the real extractor exists.

## Acceptance concerns

1. **Merging this approves revision 2** (DP-5.1): the packets, the method, and the generation protocol.
2. **The verifier moves second.** Decline this at merge if you prefer the original order; §8.1's table is the only place it lives.
3. **Larger PRs mean more to read per merge.** The stages, the gates and one review index per packet are the mitigation. If a packet's diff is too large to review, split it at a stage boundary.
