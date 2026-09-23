#!/usr/bin/env python3
"""Regenerate source maps, expectations and headline source in a staging subdir."""
import hashlib
import json
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path
from generate import BASE, dump


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    comparisons = []
    with tempfile.TemporaryDirectory(dir=BASE, prefix='.reproduce-') as temporary:
        out = Path(temporary)
        for script in ('generate.py', 'headline.py'):
            subprocess.run([sys.executable, '-B', str(BASE / script), '--output', str(out)], check=True)
        for path in sorted(out.rglob('*')):
            if not path.is_file():
                continue
            name = path.relative_to(out).as_posix()
            expected = BASE / name
            if not expected.is_file() or path.read_bytes() != expected.read_bytes():
                raise RuntimeError('Reproduction mismatch: ' + name)
            comparisons.append(name)
        owned = sorted(p.relative_to(BASE).as_posix() for prefix in ('random', 'headline')
                       for p in (BASE / prefix).rglob('*') if p.is_file() and p.name != 'Cargo.lock')
        owned.append('headline-contract.json')
        if sorted(owned) != sorted(comparisons):
            raise RuntimeError('Unexpected or missing generated file')
    report = {'result': 'byte_identical', 'compared_files': len(comparisons),
              'exclusions': ['headline Cargo.lock files: produced by cargo check, not by source generator'],
              'generator_sha256': sha(BASE / 'generate.py'),
              'reference_sha256': sha(BASE / 'reference.py'),
              'headline_author_sha256': sha(BASE / 'headline.py')}
    dump(BASE / 'reproduction.json', report)
    print(json.dumps(report, sort_keys=True))


if __name__ == '__main__':
    main()
