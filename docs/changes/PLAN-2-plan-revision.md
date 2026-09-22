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

## Acceptance concerns

1. **Merging this approves revision 2** (DP-5.1): the packets, the method, and the generation protocol.
2. **The verifier moves second.** Decline this at merge if you prefer the original order; §8.1's table is the only place it lives.
3. **Larger PRs mean more to read per merge.** The stages, the gates and one review index per packet are the mitigation. If a packet's diff is too large to review, split it at a stage boundary.
