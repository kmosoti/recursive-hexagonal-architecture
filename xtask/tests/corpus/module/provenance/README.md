# Pre-registration proposal correction

First proposal content SHA256:

```text
5cd3ff86e179d428c4800bb42a6585b44670248ad7bf5864bb18e41762cfbd29
```

This is the SHA256 of the original `SHA256SUMS` bytes, now archived unchanged
as `first-proposal.SHA256SUMS`. `first-proposal-registration.toml` is the original
registration, and `first-proposal-rule-expectations.json` is its historical
optional projection. These archives preserve the proposal; they are not
current expectations. All 679 inventoried files were hash-checked before the
correction began.

The first proposal used **flat raw predicate findings**: a cyclic SCC and each
undeclared edge inside it appeared as separate top-level findings. Its notes
consequently described M01 as an extra-finding conflict and M05 depth as an
unresolved counting convention. A read-only pre-registration contract review
identified the mismatch with the committed, pre-implementation diagnostic
contract at `docs/architecture/module-check-contract.md`.

The correction retains every undeclared intra-SCC edge as a structured
`undeclared_edges` subfact and lists `modules.undeclared_dependency` in
`subsumed_rules` on the owning cycle. Other undeclared edges remain independent
findings. It adds canonical closed cycle paths and witness-specific heuristic
flags. M05 depth is canonical component-path segment count including crate
root; `x::ordering::scoring` has depth three. M21's edition-2015 disclosure
remains unchanged.

There was no committed or production-graded generator artifact and no real
extractor implementation when this correction was requested. This is a
pre-registration correction of optional diagnostic representation, **not a
retrospective regrade**. The parent is the sole branch writer and still owns
committing the corrected artifacts before real extractor implementation.

The raw oracle remains primary: `reference.py`, all random source maps and
primary extraction expectations, the headline sources and their reference
outputs, and the frozen manifest are byte-for-byte unchanged. The audit
compares current protected artifacts to the archived first inventory and
compares original D4 facts to the union of corrected independent and subsumed
facts. No D4 fact is lost, duplicated, or made conformant by grouping.

Exact input hashes (computed from file bytes; also bound by registration):

| Input | SHA256 |
| --- | --- |
| Original prompt, `target/m2/module-generator-prompt.md` | `6c9af342e6dedf1383c5ee0aabb9f3b3217a771a1476370d16511e42c2eef112` |
| Correction prompt, `target/m2/module-generator-correction.md` | `98bfc6941a6c973ee9e98941914f3ee93f0bde103b03a1e2a4cff2f3f568c23a` |
| Diagnostic contract, `docs/architecture/module-check-contract.md` | `3f0b6d3c4c647d9210ea5e86517b15ffb2e7126eb08add2b33bbf7ef7751133e` |
