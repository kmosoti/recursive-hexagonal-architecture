# Transclusion URI fixture package

This package is a pre-code, independent reference for the bounded lexical URI
rebasing rules in the frozen contract snapshots. It contains 25 hand-reviewed
controls and 128 deterministic cases generated from seed `90092028`. It does
not claim complete URI conformance.

`reference.py` is read-only by default. Its reference operates on canonical
source-derived page IDs, preserves special destinations byte-for-byte, applies
lexical POSIX dot-segment handling, and maps only valid link-only fragments.
Images never use the anchor map. The expected values in `CASES.json` are
checked against the same reference and the controls are explicitly asserted in
the generator source.

## Reproduce from a fresh staging copy

Use a fresh `target/m2` staging directory containing a copy of the committed
eventual package at `xtask/tests/corpus/transclusion/uris`:

```sh
mkdir -p target/m2
cp -R xtask/tests/corpus/transclusion/uris target/m2/transclusion-uris
cd target/m2/transclusion-uris
python3 -B reference.py
python3 -B reference.py --reproduce-to reproduced
python3 -B - <<'PY'
from pathlib import Path

root = Path('.')
copy = root / 'reproduced'
names = sorted(path.relative_to(root) for path in root.rglob('*')
               if path.is_file() and copy not in path.parents)
assert names == sorted(path.relative_to(copy) for path in copy.rglob('*') if path.is_file())
for name in names:
    assert (root / name).read_bytes() == (copy / name).read_bytes()
PY
rm -rf reproduced
```

The first command is a read-only self-test. The second command writes only the
owned `reproduced/` subdirectory of the copied package; the comparison checks
all nine files, not only `CASES.json`. Remove that directory after comparison.
The strict child must be fresh and direct under the package. Do not run
reproduction in the committed package.

The source snapshots are the exact frozen inputs used for this package.
Metadata points at their eventual committed paths and never requires equality
with a future live source file. `SHA256SUMS` covers every payload file except
itself and `registration.toml`, in sorted relative-path order.
