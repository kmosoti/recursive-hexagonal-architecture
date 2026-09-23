#!/usr/bin/env python3
"""Author staged fixtures; copy frozen contract facts exactly, never grade them."""
import argparse
import json
import tomllib
from pathlib import Path
from generate import BASE, dump, materialize, reference

REPO = BASE.parents[2]


def create(case):
    id_ = case['id']
    crate = 'x' if id_ == 'M05' else 'fixture_' + id_.lower().replace('-', '_')
    edition = '2015' if id_ == 'M21' else '2021'
    constraints = 'pub struct Rule;'
    ordering = 'pub struct Wave; pub trait Trait {} pub fn score() -> u8 { 1 }'
    root = []
    files = {}
    extra_components = {}
    allow = {'constraints': [], 'ordering': ['constraints']}
    cycles = {'M01', 'M12', 'M13', 'M14', 'M15', 'M16', 'M17', 'M18', 'M20', 'M21'}
    if id_ in cycles:
        ordering += '\npub fn reverse(_: crate::constraints::Rule) {}'
        if id_ != 'M01':
            allow['constraints'] = ['ordering']
    if id_ == 'M01':
        constraints += '\nuse crate::ordering::Wave;'
    elif id_ == 'M02':
        root.append('pub mod model { pub struct Model; }')
        extra_components['model'] = f'{crate}::model'
        allow['model'] = []
        constraints += '\npub fn f(_: crate::model::Model) {}'
    elif id_ == 'M03':
        root.append('fn helper() {}')
        ordering += '\npub fn f() { super::helper(); }'
    elif id_ == 'M04':
        root.append('pub fn plan() {}')
        ordering += '\npub fn f() { crate::plan(); }'
    elif id_ == 'M05':
        ordering += '\npub mod scoring { use crate::constraints; }'
        extra_components['scoring'] = 'x::ordering::scoring'
        allow['scoring'] = ['constraints']
    elif id_ == 'M06':
        constraints += '\npub mod rules { pub struct Rule; }'
        ordering += '\nuse crate::constraints::rules::Rule;'
    elif id_ == 'M07':
        ordering += '\nuse crate::constraints::*; pub fn f(_: Rule) {}'
    elif id_ == 'M08':
        ordering += '\npub mod inner { pub struct Local; pub fn f(_: self::Local, _: super::Wave) {} }\npub fn f(_: self::inner::Local) {}'
    elif id_ == 'M09':
        root.append('extern crate std as external;')
        ordering += '\npub fn f() { std::hint::black_box(1); core::hint::black_box(2); external::hint::black_box(3); }'
    elif id_ == 'M10':
        constraints += '\npub mod rules { pub struct Rule; }'
        root.append('#[path = "elsewhere.rs"] mod part;')
        files['src/elsewhere.rs'] = 'use crate::constraints::rules;\n'
        extra_components['part'] = f'{crate}::part'
        allow['part'] = ['constraints']
    elif id_ == 'M11':
        constraints += '\n#[cfg(test)] mod tests { use crate::ordering; }'
    elif id_ == 'M12':
        constraints += '\npub mod extra { use crate::ordering::Wave; }'
    elif id_ == 'M13':
        constraints += '\npub struct X; impl crate::ordering::Trait for X {}'
    elif id_ == 'M14':
        ordering = ordering.replace('score()', 'score(_: u8)')
        constraints += '\npub fn f(x: u8) { crate::ordering::score(x); }'
    elif id_ == 'M15':
        constraints += '\npub fn f(w: crate::ordering::Wave) { let _ = w; }'
    elif id_ == 'M16':
        constraints += '\npub fn f() { use crate::ordering::Wave; let _ = Wave; }'
    elif id_ == 'M17':
        constraints += '\npub mod extra { use crate::{ordering::Wave, constraints::Rule}; }'
    elif id_ == 'M18':
        constraints += '\npub use crate::ordering::Wave;'
    elif id_ == 'M19':
        root.append('pub use ordering::score;')
        constraints += '\npub fn f() { crate::score(); }'
    elif id_ == 'M20':
        constraints += '\npub fn f() { assert_eq!(crate::ordering::score(), 1); }'
    elif id_ == 'M21':
        constraints += '\nuse ordering::Wave;'
    elif id_ == 'L-M02':
        root.append('pub fn glue(_: crate::constraints::Rule, _: crate::ordering::Wave) {}')
    elif id_ == 'EM-M01':
        root.append('macro_rules! hidden { () => { crate::ordering::score() }; }')
        constraints += '\npub fn f() { let _ = hidden!(); }'
        ordering += '\npub fn reverse(_: crate::constraints::Rule) {}'
    elif id_ == 'EM-M02':
        files['src/constraints.rs'] = constraints + '\ninclude!("../fragments/uses_ordering.in");\n'
        files['fragments/uses_ordering.in'] = 'pub fn f() { let _ = crate::ordering::score(); }\n'
        ordering += '\npub fn reverse(_: crate::constraints::Rule) {}'
    elif id_ == 'EM-M03':
        root.append('pub mod model { pub struct Value; }')
        extra_components['model'] = f'{crate}::model'
        allow['constraints'] = ['ordering']
        allow['ordering'] = ['model']
        allow['model'] = []
        ordering += '\npub trait Hidden { fn touch(&self); }\nimpl Hidden for crate::model::Value { fn touch(&self) {} }\npub fn make() -> crate::model::Value { crate::model::Value }'
        constraints += '\nuse crate::ordering::*; pub fn f() { let value = make(); value.touch(); }'
    else:
        raise ValueError(id_)
    root.insert(0, '#![allow(dead_code, unused_imports)]')
    root.append('pub mod constraints;' if id_ == 'EM-M02' else 'pub mod constraints {\n' + constraints + '\n}')
    root.append('pub mod ordering {\n' + ordering + '\n}')
    files['src/lib.rs'] = '\n'.join(root) + '\n'
    files['Cargo.toml'] = f'[package]\nname = "{crate}"\nversion = "0.0.0"\nedition = "{edition}"\n[workspace]\n[package.metadata.rha]\nrole = "core"\n'
    components = {'constraints': f'{crate}::constraints', 'ordering': f'{crate}::ordering', **extra_components}
    rules = '[components]\n' + ''.join(f'{k} = {json.dumps(v)}\n' for k, v in sorted(components.items()))
    rules += '\n[allow]\n' + ''.join(f'{k} = {json.dumps(v)}\n' for k, v in sorted(allow.items()))
    rules += '\n[deny]\ncycles = true\nchild_to_parent_private = true\nforeign_internal = true\n'
    files['rha-modules.toml'] = rules
    return files, crate, edition


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--output', type=Path, default=BASE)
    args = parser.parse_args()
    out = args.output.resolve()
    if not out.is_relative_to(BASE):
        parser.error('output must stay under module-generation')
    manifest = tomllib.loads((REPO / 'xtask/tests/corpus/manifest.toml').read_text())
    count = 0
    for case in manifest['case']:
        if case['level'] != 'module' or case['id'] in ('L-M01', 'X-M01'):
            continue
        files, crate, edition = create(case)
        directory = out / 'headline' / case['id']
        materialize(files, directory)
        dump(directory / 'contract.json', case)
        dump(directory / 'reference.json', reference(directory, crate, edition))
        count += 1
    dump(out / 'headline-contract.json', {
        'grading': manifest['grading'],
        'cases': [case for case in manifest['case'] if case['level'] == 'module'],
        'notice': 'Read-only projection; frozen manifest remains the headline grading authority. No production implementation was run.'})
    print(json.dumps({'authored_headline_fixtures': count}, sort_keys=True))


if __name__ == '__main__':
    main()
