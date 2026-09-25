# Transclusion section-selection corpus

This package is a frozen, independent reference for section selection over a
normalized `document::Node` tree. It is not a Markdown parser and does not
load the repository implementation. `CASES.json` contains 16 explicit
boundary/shell controls and 64 deterministic structural cases generated from
seed `90092027`.

The selector matches a final slug exactly and case-sensitively. A null anchor
selects the complete body. An anchored selection ends at the next heading of
less than or equal level in the same block sequence; nested quote and list
sequences are searched recursively, preserving only the ancestor shells and
adjusting ordered-list starts. The input tree is never mutated. Descriptor
outputs are copied complete `Transclusion` objects in tree order.

## Read-only self-test

From this directory, run:

```sh
PYTHONDONTWRITEBYTECODE=1 python3 reference.py
```

The default command reads the committed `CASES.json`, checks every expected
tree and descriptor list, checks contiguous preorder IDs and input immutability,
rejects transclusions under paragraph/heading inline containers, and runs a
negative injected-illegal-tree check. It does not write files.

## Reproduction workflow

Use a fresh staging copy of the committed package. The copied script rebuilds
all nine package files from frozen package inputs, then remove only the
reproduction directory owned by that copy:

```sh
mkdir -p target/m2
cp -a xtask/tests/corpus/transclusion/sections target/m2/transclusion-sections
PYTHONDONTWRITEBYTECODE=1 python3 target/m2/transclusion-sections/reference.py \
  --reproduce-to reproduced
cmp target/m2/transclusion-sections/CASES.json \
  target/m2/transclusion-sections/reproduced/CASES.json
diff -ru --no-dereference \
  xtask/tests/corpus/transclusion/sections \
  target/m2/transclusion-sections/reproduced
find target/m2/transclusion-sections/reproduced -type f -delete
rmdir target/m2/transclusion-sections/reproduced
```

The copied script rejects reproduction paths outside its own package directory.
Reproduction writes `CASES.json`, `README.md`, `reference.py`,
`selftestreport.txt`, `registration.toml`, `SHA256SUMS`, and all three
`source-snapshots/*` files. It never requires equality with live contract, ADR,
or prompt files; the committed source snapshots are the authoritative inputs
for the recorded digests.
