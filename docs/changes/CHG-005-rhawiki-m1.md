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

## Acceptance concerns

1. **An earlier unapproved protected edit, found by the new guard.** CHG-004.6's commit `ad13efa` added the held-out paragraph to CONTRIBUTING.md. That was a protected edit whose approval was recorded only as owned scope, not quoted from Kennedy. The CHG-004.6 acceptance record cannot be edited, so it is disclosed here. Kennedy may approve the paragraph retroactively or have it removed.
2. **The CONTRIBUTING line for `cargo xtask scope`** waits for approval (DP-5.2).
