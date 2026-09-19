# Components

## At this revision

The workspace has one member, `xtask`, with role `tool`: repository automation that runs the policy's lanes, writes evidence records, and generates docs. It is not a component of the product, and nothing may depend on it (rule `dir.tool_depended_on`, CHG-003).

There are no core, adapter, or app crates yet.

## Planned

The `rhawiki` components, their ports, and the crate dependency direction are designed in [the plan, §3.1](../plan/IMPLEMENTATION-PLAN.md). They are created in CHG-005, after Kennedy writes boundary decision records BDR-0001 to BDR-0004 with refutation criteria (spec §7.8 step 10, DP-1.3). From CHG-005 on, this page owns the component table and the plan's copy becomes history.
