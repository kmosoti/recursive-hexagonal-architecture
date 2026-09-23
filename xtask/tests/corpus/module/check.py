#!/usr/bin/env python3
"""Check only generator artifacts: compiler legality, replay and self-checks."""
import argparse
import hashlib
import json
import os
import shutil
import subprocess
import sys
import tomllib
from collections import Counter
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
from generate import BASE, dump, materialize, reference


def rule_expectations(case, extracted):
    """Finite flat-component diagnostics under the pre-implementation contract.

    Raw extraction remains the primary oracle. SCC membership controls only
    diagnostic grouping; it never removes an observed edge or a D4 fact.
    """
    rules = tomllib.loads(case['files']['rha-modules.toml'])
    components = {key: tuple(value.split('::')) for key, value in rules['components'].items()}
    def owner(parts):
        options = [key for key, value in components.items() if parts[:len(value)] == value]
        return max(options, key=lambda k: len(components[k])) if options else None
    violations = set()
    provenance = {}
    edges = set()
    def violation(rule, source, target, extraction):
        key = (rule, source, target)
        violations.add(key)
        provenance.setdefault(key, set()).add(extraction)
    for edge in extracted['edges']:
        if edge['test_only']:
            continue
        source_parts, target_parts = tuple(edge['source'].split('::')), tuple(edge['target'].split('::'))
        source, target = owner(source_parts), owner(target_parts)
        if source is None or source == target:
            continue
        if target is None:
            violation('modules.child_to_parent_private', source, edge['target'], edge['extraction'])
        else:
            edges.add((source, target, edge['extraction']))
            if target not in rules['allow'].get(source, []):
                violation('modules.undeclared_dependency', source, target, edge['extraction'])
            if len(target_parts) > len(components[target]) + 1:
                violation('modules.foreign_internal', source, edge['target'], edge['extraction'])
    # Reachability closure over tiny finite graphs; no generator's intended edges.
    reach = {(a, b) for a, b, _ in edges}
    while True:
        more = reach | {(a, d) for a, b in reach for c, d in reach if b == c}
        if more == reach:
            break
        reach = more
    cyclic = {tuple(sorted(y for y in components if (x, y) in reach and (y, x) in reach))
              for x in components if (x, x) in reach}
    membership = {member: members for members in cyclic for member in members}
    adjacency = {node: sorted({b for a, b, _ in edges if a == node}) for node in components}
    undeclared = {members: [] for members in cyclic}
    findings = []
    for rule, source, target in sorted(violations):
        extraction = 'heuristic' if 'heuristic' in provenance[rule, source, target] else 'syntax'
        fact = {'from': source, 'to': target, 'extraction': extraction}
        if (rule == 'modules.undeclared_dependency' and source in membership
                and membership[source] == membership.get(target)):
            undeclared[membership[source]].append(fact)
        else:
            finding = {'rule': rule, **fact}
            if rule == 'modules.child_to_parent_private':
                finding['depth'] = len(components[source])
            findings.append(finding)
    for members in sorted(cyclic):
        start = members[0]
        # FIFO simple-path search: shortest cycle through the least start first;
        # sorted adjacency makes ties lexicographic over the entire closed path.
        queue = [(start,)]
        path = None
        while queue and path is None:
            prefix = queue.pop(0)
            for target in adjacency[prefix[-1]]:
                if target == start:
                    path = (*prefix, start)
                    break
                if target in members and target not in prefix:
                    queue.append((*prefix, target))
        if path is None:
            raise AssertionError('cyclic SCC has no return path')
        heuristic = any((source, target, 'heuristic') in edges
                        for source, target in zip(path, path[1:]))
        findings.append({'rule': 'modules.cycle', 'members': list(members), 'path': list(path),
                         'extraction': 'heuristic' if heuristic else 'syntax',
                         'undeclared_edges': undeclared[members],
                         'subsumed_rules': ['modules.undeclared_dependency'] if undeclared[members] else []})
    return {'component_edges': [dict(source=a, target=b, extraction=c) for a, b, c in sorted(edges)],
            'violations': findings}


def run_compiler(command, cwd):
    env = dict(os.environ, CARGO_INCREMENTAL='0', RUSTUP_AUTO_INSTALL='0')
    result = subprocess.run(command, cwd=cwd, text=True, capture_output=True, env=env)
    if result.returncode:
        raise RuntimeError(json.dumps({'argv': command, 'cwd': str(cwd), 'exit_status': result.returncode,
                                       'stdout': result.stdout, 'stderr': result.stderr}, indent=2))
    return result


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--jobs', type=int, default=4)
    args = parser.parse_args()
    subprocess.run([sys.executable, '-B', str(BASE / 'test_reference.py')], check=True)
    work = BASE / '.check-work'
    work.mkdir(exist_ok=True)
    cases = sorted((BASE / 'random').glob('R[0-9][0-9][0-9][0-9].json'))
    def random_case(path):
        case = json.loads(path.read_text())
        directory = work / case['id']
        materialize(case['files'], directory)
        expected = json.loads(path.with_suffix('.expected.json').read_text())
        observed = reference(directory, case['crate_name'], case['edition'])
        if observed != expected:
            raise RuntimeError('Reference replay mismatch: ' + case['id'])
        for test in (False, True):
            command = ['rustc', '--edition=' + case['edition'], '--crate-name', case['crate_name'],
                       '--crate-type=lib', '--emit=metadata', 'src/lib.rs', '-o', 'legality.rmeta']
            if test:
                command += ['--cfg', 'test']
            run_compiler(command, directory)
        shutil.rmtree(directory)
        return case['id'], rule_expectations(case, observed)
    with ThreadPoolExecutor(max_workers=args.jobs) as pool:
        result = dict(pool.map(random_case, cases))
    print(f'{len(result)} random cases: reference replay and both rustc modes passed', flush=True)
    manifest = tomllib.loads((BASE.parents[2] / 'xtask/tests/corpus/manifest.toml').read_text())
    frozen = {case['id']: case for case in manifest['case'] if case['level'] == 'module'}
    headline = []
    for directory in sorted((BASE / 'headline').iterdir()):
        contract = json.loads((directory / 'contract.json').read_text())
        if contract != frozen[directory.name]:
            raise RuntimeError('Frozen-contract projection changed: ' + directory.name)
        package = tomllib.loads((directory / 'Cargo.toml').read_text())['package']
        if reference(directory, package['name'], package['edition']) != json.loads((directory / 'reference.json').read_text()):
            raise RuntimeError('Headline extraction replay mismatch: ' + directory.name)
        run_compiler(['cargo', 'check', '--offline', '--all-targets', '--target-dir', str(work / 'cargo-target')], directory)
        headline.append({'id': directory.name, 'cargo_check_offline_all_targets': 'passed', 'exit_status': 0})
    shutil.rmtree(work)
    dump(BASE / 'random-rule-expectations.json', result)
    rule_counts = Counter(f['rule'] for r in result.values() for f in r['violations'])
    subsumed_d4 = sum(len(f.get('undeclared_edges', [])) for r in result.values() for f in r['violations'])
    no_violations = sum(not r['violations'] for r in result.values())
    summary = {
        'scope': 'Artifact legality and reference consistency only; no production grading.',
        'random_cases': len(cases), 'rustc_checks': len(cases) * 2,
        'rustc_modes': ['production', '--cfg test'], 'reference_replays': len(cases) + len(headline),
        'reference_hand_tests': 14, 'headline_cargo_checks': headline,
        'random_rule_summary': {'legal_cases': no_violations, 'violating_cases': len(cases) - no_violations,
                                'finding_counts': dict(sorted(rule_counts.items())),
                                'top_level_findings': sum(rule_counts.values()),
                                'subsumed_undeclared_edges': subsumed_d4,
                                'total_undeclared_edge_facts': rule_counts['modules.undeclared_dependency'] + subsumed_d4},
        'tools': {tool: subprocess.run([tool, '--version'], check=True, text=True, capture_output=True).stdout.strip()
                  for tool in ('python3', 'rustc', 'cargo')},
        'compile_commands': [
            'rustc --edition=2021 --crate-name <crate> --crate-type=lib --emit=metadata src/lib.rs -o legality.rmeta',
            'rustc --edition=2021 --crate-name <crate> --crate-type=lib --emit=metadata src/lib.rs -o legality.rmeta --cfg test',
            'cargo check --offline --all-targets --target-dir <staging>/.check-work/cargo-target'],
    }
    dump(BASE / 'checks.json', summary)
    print(json.dumps(summary, sort_keys=True))


if __name__ == '__main__':
    main()
