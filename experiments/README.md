# Experiments

The research membrane of spec §5.5. An unproven idea lives here as a hypothesis, an experiment, and its evidence until a recorded decision moves it into production code or drops it. Nothing outside `experiments/` depends on anything inside it, and nothing here is a workspace member.

Each experiment is a directory `experiments/<hypothesis>/` holding `hypothesis.md` (the claim, what would refute it, and how it is run), its run scripts, and `results/`. The first planned experiment is `no-std-graph` (IMPLEMENTATION-PLAN.md, E1).
