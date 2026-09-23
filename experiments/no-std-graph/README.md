# E1: `no_std` graph

This is an isolated Cargo package, not a member of the repository workspace.
It copies the production graph resolver and its original oracle/property
tests, while depending directly on `document` and `library` for production
types.

The graph crate uses `alloc`, but the direct production dependencies and their
full dependency closure still use `std`. This experiment therefore does not
establish embedded support, heap freedom, panic freedom, or end-to-end
`no_std`.

From the repository root, run:

```sh
cargo check --manifest-path experiments/no-std-graph/Cargo.toml --lib --locked
cargo test --manifest-path experiments/no-std-graph/Cargo.toml --locked
cargo check --manifest-path experiments/no-std-graph/Cargo.toml --lib --locked --features negative-std-probe
```

The first two commands are the positive checks. The feature-probe command is
expected to fail because it references `std::collections::BTreeMap`.

The observed results and scope are in [result.md](result.md). Verify the frozen snapshot without requiring historical Git objects:

    python3 experiments/no-std-graph/verify_snapshot.py

A missing historical object is reported explicitly; snapshot/test hash checks still run. The recorded baseline applies to this experiment, even as product code evolves in later packets.
