# Transclusion region graph corpus

This package is a frozen, independent reference for the graph-level
transclusion-region contract. Its graphs are algorithm-level fixtures; they
are not claims that every graph is realizable Markdown, and the package makes
no benchmark or product-performance claim.

`reference.py` performs read-only self-tests by default. It checks the frozen
cases, hand-derived controls, canonical witnesses, edge/path invariants,
permutation invariance, DAG results after removing every blocked occurrence,
payload hashes, and metadata. It never requires the live repository sources to
match the snapshots.

To reproduce in a fresh staging copy, keep the committed package untouched:

```sh
staging=$(mktemp -d target/m2/transclusion-regions-staging.XXXXXX)
mkdir -p "$staging/package"
cp -a xtask/tests/corpus/transclusion/regions/. "$staging/package/"
python3 -B "$staging/package/reference.py" --reproduce-to "$staging/package/reproduced"
diff -ru --exclude=reproduced "$staging/package" "$staging/package/reproduced"
rm -rf "$staging"
```

The copied reference writes only beneath its package directory and requires
the reproduction directory to be a fresh direct child. It recreates exactly
the same nine package files byte-for-byte, including the reference, README,
three snapshots, self-test report, registration, manifest, and cases. The
reproduction directory is owned by this check; remove it after comparison. Do
not run reproduction into the committed package itself. The authoritative inputs are the exact files under
`source-snapshots/`; their original repository paths in `registration.toml`
are documentary only.

The corpus uses seed `90092026`, six authored controls, and exactly 256 seeded
random normalized graphs. Region identity distinguishes `anchor = null` from
an anchored region; generated anchors are always nonempty. Case and
transclusion IDs are labels, so changing input array order does not change the
reference output.
