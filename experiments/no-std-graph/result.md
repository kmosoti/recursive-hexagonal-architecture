# E1 result: direct std coupling in graph

The three hypotheses in [hypothesis.md](hypothesis.md) are supported in this local run. The experiment package was declared after [BDR-0005](../../docs/adr/BDR-0005-no-std-graph-experiment.md), and the hypothesis and manifest were committed before execution. The source baseline is c8c6c5bbcad94de280580ecc3c77e3e590cb36b5; [sources.sha256](sources.sha256) contains the computed source and test identities.

| Observation | Command outcome | Evidence |
| --- | --- | --- |
| Adding only the no_std attribute | cargo check exited 101; 15 diagnostics include the unresolved std import and missing String/Vec prelude types | [naive check](../../evidence/CHG-007/E1/e1-naive-no-std.txt) |
| Importing collections, String and Vec from alloc | cargo check --lib --locked exited 0 | [alloc check](../../evidence/CHG-007/E1/e1-alloc-check.txt) |
| Original graph oracle and properties | cargo test --locked exited 0; one oracle test and three property test functions passed; zero unit tests and zero doctests were selected | [tests](../../evidence/CHG-007/E1/e1-tests.txt) |
| Intentional direct std reference | the negative-std-probe feature check exited 101, rejecting both std references | [negative compiler probe](../../evidence/CHG-007/E1/e1-negative-std-probe.txt) |
| Snapshot identity | approved edit reversal reproduces the original source hash; copied tests match; all three baseline Git objects checked | [source verification](../../evidence/CHG-007/E1/source-verification.txt) |
| Deliberate snapshot drift | the verifier exited 1 on an appended comment; the drift was restored | [negative identity control](../../evidence/CHG-007/E1/source-verification-negative.txt) |

All Cargo commands used offline resolution and the experiment lockfile after its initial offline generation. The isolated lockfile resolved 54 packages compatible with Rust 1.98.1. The package is unpublished and outside the product workspace; its build directory is locally ignored.

The friction was confined to explicit allocation imports and loss of the std prelude. The resolver body and test bytes are unchanged. This caught direct std coupling in collection/prelude names; it did not uncover an ambient-I/O defect in the existing resolver.

For spec §6.8, optional no_std supplies a direct-name availability check and makes allocation requirements visible. The real document/library dependencies retain std, and the tests use a host harness. The result makes no claim of a std-free dependency closure, embedded portability, heap freedom, panic freedom, bounded resource use, or production conversion. The unique-PageId input domain of the copied permutation property remains as stated in the hypothesis.

Run python3 experiments/no-std-graph/verify_snapshot.py to verify the frozen identities and allowed source edits. On a shallow checkout it reports any unavailable Git object explicitly while still comparing the snapshot and test bytes to their recorded hashes. Run the positive and negative Cargo commands in [README.md](README.md) separately; the negative feature is intentionally uncompilable.
