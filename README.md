# Recursive Hexagonal Architecture

**An architecture specification that refuses to take its own word for anything.**

Most architecture documents tell you how to arrange your code and then leave. This one arranges the code, then builds the machinery that catches it lying: a crate-graph checker, an ambient-effect deny list, seeded violations that the checks must find, and an evidence record for every claim, with a five-state vocabulary in which `not_run` is a first-class answer and never quietly becomes `passed`.

The specification is v0.10, written by Kennedy Mosoti. This repository is where it stops being prose.

## What is actually here today

A maturity ledger, [docs/maturity.md](docs/maturity.md), is the only place that says what works. Every row names an artifact, the evidence for it, the exact scope that evidence covers, and the environment it ran in. Nothing graduates on enthusiasm.

| Mechanism | Where it stands |
| --- | --- |
| **Ambient-effect deny list** | 111 paths a pure core crate may not touch: clocks, the filesystem, sockets, processes, the environment, blocking, randomness, every way of printing. A fixture crate uses each one and the build fails 122 times. |
| **Fast lane** | Eight commands, one entry point, `cargo xtask ci`, which writes a JSON record of what ran rather than a green tick. |
| **Crate-graph checker** | Implemented at crate level; H4 corpus validation is pending (CHG-004), with scope and limits in the [maturity ledger](docs/maturity.md). Module checks (CHG-007) and transitive analysis remain unavailable. |
| **Everything else** | Specified. The [plan](docs/plan/IMPLEMENTATION-PLAN.md) says in what order, and the ledger will say when. |

## The parts worth reading

**[The specification](docs/spec/rha-spec-v0.10.md)** grades its own evidence. Claims it cannot support are labelled as judgment, and a whole appendix explains which famous results it is *not* invoking, including one sampling scheme an earlier version got wrong and withdrew.

**[The decision records](docs/adr/)** are where measurement beats intuition. [ADR-0002](docs/adr/ADR-0002-clippy-config-discovery.md) asks what a lint configuration actually does, and answers with twenty-six recorded experiments. Among them: a dotfile that replaces your deny list in silence, an `#[allow]` that switches it off, a `forbid` that refuses the `#[allow]` but breaks any crate whose macros expand to a group allow, and a single line in `.cargo/config.toml` that disables the lot.

**[The evidence](evidence/)** is the raw material: commands, exit statuses, captured output, file digests. Unglamorous, and the point of the whole exercise.

**[The change records](docs/changes/)** carry an "Acceptance concerns" section that lists what is still wrong with the change being proposed. It is the most useful section in the repository.

## Working in it

```
cargo xtask ci                      # the eight commands of the fast lane
cargo xtask ci --print              # what those eight commands are
cargo xtask ci --record evidence/CHG-0nn   # keep the record with the change
```

[CONTRIBUTING.md](CONTRIBUTING.md) has the workflow, `.rha/policy.toml` owns the check list, and [AGENTS.md](AGENTS.md) is the entry point for an agent. Human or agent, the contract is the same: report what ran, not what you expected to pass.

## Where it is going

`rhawiki`, a Rust wiki that renders these documents, is the pilot subject. It is being built under the specification's own rules so the mechanisms are tested on something real rather than on a diagram. Start at [docs/index.md](docs/index.md).
