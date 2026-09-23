#!/usr/bin/env python3
"""Generate source only. Independent expected extraction uses a subprocess."""
import argparse
import json
import shutil
import subprocess
import sys
from pathlib import Path

BASE = Path(__file__).resolve().parent
SEED = 20260922


class Rng:
    """SplitMix64: reproducible integers without Python random-version coupling."""
    def __init__(self, seed):
        self.state = seed

    def n(self, upper):
        mask = (1 << 64) - 1
        self.state = (self.state + 0x9E3779B97F4A7C15) & mask
        z = self.state
        z = ((z ^ (z >> 30)) * 0xBF58476D1CE4E5B9) & mask
        z = ((z ^ (z >> 27)) * 0x94D049BB133111EB) & mask
        return (z ^ (z >> 31)) % upper


def dump(path, obj):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(obj, sort_keys=True, indent=2) + '\n')


def materialize(files, directory):
    for name, source in sorted(files.items()):
        path = directory / name
        if not path.resolve().is_relative_to(directory.resolve()):
            raise ValueError('unsafe source-map path')
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(source)


def reference(directory, crate_name, edition='2021'):
    request = {'root': str((directory / 'src/lib.rs').resolve()),
               'crate_name': crate_name, 'edition': edition}
    result = subprocess.run([sys.executable, '-B', str(BASE / 'reference.py')],
                            input=json.dumps(request), text=True, capture_output=True, check=True)
    return json.loads(result.stdout)


def witness_source(target, index, syntax):
    """Only source construction; no edge oracle or expected-edge annotations."""
    if syntax == 0:
        return f'pub fn f{index}() {{ let _ = crate::{target}::score(); }}'
    if syntax == 1:
        return f'pub fn f{index}(_: crate::{target}::Token) {{}}'
    if syntax == 2:
        return f'pub struct Impl{index}; impl crate::{target}::Port for Impl{index} {{}}'
    if syntax == 3:
        return f'pub fn f{index}() {{ use crate::{target}::Token as Alias; let _ = Alias; }}'
    if syntax == 4:
        return f'pub fn f{index}() {{ use crate::{target} as dep; let _ = dep::score(); }}'
    if syntax == 5:
        return f'pub fn f{index}() {{ use crate::{target}::*; let _: Token = Token; let _ = score(); }}'
    if syntax == 6:
        return f'pub fn f{index}() {{ use crate::{{{target}::{{self as dep, Token as Alias}}}}; let _: Alias = dep::Token; let _ = dep::score(); }}'
    if syntax == 7:
        return f'pub mod edge_{index} {{ pub fn f(_: super::super::{target}::Token) {{}} }}'
    if syntax == 8:
        return f'pub fn f{index}() {{ use super::{target}; use {target}::Token as Alias; let _ = Alias; }}'
    if syntax == 9:
        return f'pub fn f{index}() {{ assert_eq!(crate::{target}::score(), 1); }}'
    return f'pub use crate::{target}::Token as Reexport{index};'


def rules_text(crate, names, allowed):
    text = '[components]\n'
    text += ''.join(f'{name} = "{crate}::{name}"\n' for name in names)
    text += '\n[allow]\n'
    text += ''.join(f'{name} = {json.dumps(allowed[name])}\n' for name in names)
    return text + '\n[deny]\ncycles = true\nchild_to_parent_private = true\nforeign_internal = true\n'


def make_case(rng, index):
    count = 3 + rng.n(3)
    names = [f'c{i}' for i in range(count)]
    crate = f'random_{index:04d}'
    # Rules are fixed independently of sampled actual dependencies.
    allowed = {name: names[:i] for i, name in enumerate(names)}
    actual = {(1, 0)}
    for source in range(1, count):
        for target in range(source):
            if rng.n(3) != 0:
                actual.add((source, target))
    regime = index % 4
    if regime == 1:
        actual.add((0, 1))  # Cycle plus undeclared direction.
    elif regime == 2:
        # Reverse direction without return path; remove every edge out of last.
        actual = {(s, t) for s, t in actual if s != count - 1}
        actual.add((0, count - 1))
    bodies = {name: [
        'pub struct Token; pub trait Port {} pub fn score() -> u8 { 1 }',
        'pub mod inner { pub struct Local; pub fn back(_: super::Token) {} pub fn own(_: self::Local) {} }',
        'pub fn local(_: self::inner::Local) {}',
    ] for name in names}
    syntax_counts = {str(i): 0 for i in range(11)}
    for ordinal, (source, target) in enumerate(sorted(actual)):
        syntax = (index + ordinal) % 11
        syntax_counts[str(syntax)] += 1
        bodies[names[source]].append(witness_source(names[target], ordinal, syntax))
    if regime == 3:
        if index % 8 == 3:
            bodies['c1'].append('pub fn forbidden(_: crate::c0::inner::Local) {}')
        else:
            bodies['c0'].append('pub fn upward() { crate::root_helper(); }')
    test_target = names[1 + rng.n(count - 1)]
    bodies['c0'].append(f'#[cfg(test)] mod tests {{ use crate::{test_target}::Token as TestToken; fn probe(_: TestToken) {{}} }}')
    bodies[names[-1]].append('#[cfg(test)] pub fn item_test() { let _ = crate::c0::score(); }')
    files = {}
    root = ['#![allow(dead_code, unused_imports)]',
            '// Decoy: crate::ghost::Missing and /* comments */ are not paths.',
            'pub fn root_helper() { let _ = "crate::ghost::Missing"; }']
    layouts = []
    for i, name in enumerate(names):
        layout = i if i < 3 else rng.n(4)
        layouts.append(layout)
        if layout == 0:
            root.append(f'pub mod {name} {{\n' + '\n'.join(bodies[name]) + '\n}')
        else:
            if layout == 1:
                filename = f'src/{name}.rs'
                root.append(f'pub mod {name};')
            elif layout == 2:
                filename = f'src/mapped_{name}.rs'
                root.append(f'#[path = "mapped_{name}.rs"] pub mod {name};')
            else:
                filename = f'src/{name}/mod.rs'
                root.append(f'pub mod {name};')
            if i == 1:
                bodies[name].append('pub mod leaf;')
                files[f'src/{name}/leaf.rs'] = 'pub fn back(_: super::Token) {}\n'
            files[filename] = '\n'.join(bodies[name]) + '\n'
    root.append('pub fn glue(_: crate::c0::Token, _: crate::c1::Token) {}')
    files['src/lib.rs'] = '\n'.join(root) + '\n'
    files['Cargo.toml'] = f'[package]\nname = "{crate}"\nversion = "0.0.0"\nedition = "2021"\n[workspace]\n[package.metadata.rha]\nrole = "core"\n'
    files['rha-modules.toml'] = rules_text(crate, names, allowed)
    return {'id': f'R{index:04d}', 'crate_name': crate, 'edition': '2021', 'files': files,
            'generation': {'sample_index': index, 'regime': ['legal', 'cycle', 'undeclared', 'boundary'][regime],
                           'syntax_templates': syntax_counts, 'module_layouts': layouts}}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--output', type=Path, default=BASE)
    parser.add_argument('--count', type=int, default=256)
    args = parser.parse_args()
    out = args.output.resolve()
    if not out.is_relative_to(BASE) or args.count < 256:
        parser.error('output must stay under module-generation; count must be >= 256')
    rng = Rng(SEED)
    random_dir = out / 'random'
    random_dir.mkdir(parents=True, exist_ok=True)
    work = out / '.materialized'
    work.mkdir(parents=True, exist_ok=True)
    for index in range(args.count):
        case = make_case(rng, index)
        directory = work / case['id']
        materialize(case['files'], directory)
        expected = reference(directory, case['crate_name'], case['edition'])
        if expected['limitations']:
            raise RuntimeError(f'{case["id"]}: unexpected limitations {expected["limitations"]}')
        dump(random_dir / (case['id'] + '.json'), case)
        dump(random_dir / (case['id'] + '.expected.json'), expected)
        shutil.rmtree(directory)
    work.rmdir()
    print(json.dumps({'seed': SEED, 'cases': args.count}, sort_keys=True))


if __name__ == '__main__':
    main()
