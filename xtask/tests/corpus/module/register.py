#!/usr/bin/env python3
"""Bind final staged content; registration and digest list exclude themselves."""
import hashlib
import json
import tomllib
from collections import Counter
from pathlib import Path
from generate import BASE, SEED, dump

REPO = BASE.parents[2]


def digest_bytes(data):
    return hashlib.sha256(data).hexdigest()


def file_digest(path):
    return digest_bytes(path.read_bytes())


def main():
    cases = sorted((BASE / 'random').glob('R[0-9][0-9][0-9][0-9].json'))
    counters = Counter()
    layouts = Counter()
    syntax = Counter()
    topologies = set()
    rules = json.loads((BASE / 'random-rule-expectations.json').read_text())
    for path in cases:
        case = json.loads(path.read_text())
        result = json.loads(path.with_suffix('.expected.json').read_text())
        counters['random_cases'] += 1
        counters['random_source_map_files'] += len(case['files'])
        counters['random_rust_files'] += sum(name.endswith('.rs') for name in case['files'])
        counters['random_edges'] += len(result['edges'])
        counters['random_test_edges'] += sum(e['test_only'] for e in result['edges'])
        counters['random_heuristic_edges'] += sum(e['extraction'] == 'heuristic' for e in result['edges'])
        counters['random_limitations'] += len(result['limitations'])
        syntax.update({int(k): v for k, v in case['generation']['syntax_templates'].items()})
        layouts.update(case['generation']['module_layouts'])
        topologies.add(tuple(sorted({(e['source'], e['target']) for e in rules[case['id']]['component_edges']})))
    headline_dirs = sorted((BASE / 'headline').iterdir())
    counters['headline_fixtures'] = len(headline_dirs)
    counters['headline_rust_files'] = sum(1 for d in headline_dirs for _ in d.rglob('*.rs'))
    counters['headline_include_fragments'] = sum(1 for d in headline_dirs for _ in d.rglob('*.in'))
    counters['headline_edges'] = sum(len(json.loads((d / 'reference.json').read_text())['edges']) for d in headline_dirs)
    counters['headline_limitations'] = sum(len(json.loads((d / 'reference.json').read_text())['limitations']) for d in headline_dirs)
    counters['unique_flat_component_topologies'] = len(topologies)
    checks = json.loads((BASE / 'checks.json').read_text())
    audit = json.loads((BASE / 'correction-audit.json').read_text())
    counters['rule_exhaustive_graph_checks'] = audit['exhaustive_directed_graphs']
    counters['rule_targeted_checks'] = audit['targeted_checks']
    counters['rule_negative_mutation_checks'] = audit['negative_mutation_checks']
    counters['preserved_original_artifacts'] = audit['preserved_original_artifacts']
    rule_summary = checks['random_rule_summary']
    counters['optional_top_level_findings'] = rule_summary['top_level_findings']
    counters['optional_cycles'] = rule_summary['finding_counts'].get('modules.cycle', 0)
    counters['optional_top_level_undeclared_edges'] = rule_summary['finding_counts'].get('modules.undeclared_dependency', 0)
    counters['optional_subsumed_undeclared_edges'] = rule_summary['subsumed_undeclared_edges']
    counters['optional_total_undeclared_edge_facts'] = rule_summary['total_undeclared_edge_facts']
    coverage = {'counts': dict(sorted(counters.items())),
                'syntax_template_instances': {str(k): v for k, v in sorted(syntax.items())},
                'module_layout_instances': {str(k): v for k, v in sorted(layouts.items())},
                'rule_summary': checks['random_rule_summary']}
    dump(BASE / 'coverage.json', coverage)
    files = [p for p in sorted(BASE.rglob('*')) if p.is_file()
             and p.name not in ('registration.toml', 'SHA256SUMS')
             and not any(part.startswith('.') or part == '__pycache__' for part in p.relative_to(BASE).parts)]
    lines = [file_digest(p) + '  ' + p.relative_to(BASE).as_posix() + '\n' for p in files]
    checksum_bytes = ''.join(lines).encode()
    (BASE / 'SHA256SUMS').write_bytes(checksum_bytes)
    sections = {}
    for name, prefix in [('random', 'random/'), ('headline', 'headline/')]:
        data = ''.join(line for line, p in zip(lines, files) if p.relative_to(BASE).as_posix().startswith(prefix)).encode()
        sections[name] = digest_bytes(data)
    contract = REPO / 'xtask/tests/corpus/manifest.toml'
    prompt = REPO / 'target/m2/module-generator-prompt.md'
    correction_prompt = REPO / 'target/m2/module-generator-correction.md'
    diagnostics = REPO / 'docs/architecture/module-check-contract.md'
    first = tomllib.loads((BASE / 'provenance/first-proposal-registration.toml').read_text())
    if file_digest(BASE / 'provenance/first-proposal.SHA256SUMS') != first['content_sha256']:
        raise ValueError('first proposal inventory identity changed')
    for current, original in [(file_digest(BASE / 'reference.py'), first['reference_sha256']),
                              (sections['random'], first['random_content_sha256']),
                              (sections['headline'], first['headline_content_sha256']),
                              (file_digest(contract), first['headline_manifest_sha256']),
                              (file_digest(prompt), first['prompt_sha256'])]:
        if current != original:
            raise ValueError('raw oracle, headline corpus, manifest, or original prompt changed')
    registration = '\n'.join([
        'schema_version = 1', 'generation = "sampled"', 'role = "independent_generator"',
        f'seed = {SEED}', 'rng = "SplitMix64; constants and modulo sampling in generate.py"',
        'model = "gpt-6-astra"', 'effort = "xhigh"',
        'status = "staged; parent must commit before implementing or grading real extractor"',
        'production_implementation_read = false', 'production_implementation_run = false',
        f'prompt_sha256 = "{file_digest(prompt)}"',
        f'correction_prompt_sha256 = "{file_digest(correction_prompt)}"',
        f'module_check_contract_sha256 = "{file_digest(diagnostics)}"',
        f'first_proposal_content_sha256 = "{first["content_sha256"]}"',
        'correction_basis = "pre-registration diagnostic-contract review; first proposal used flat raw predicate findings; no production grading or retrospective regrade"',
        f'headline_manifest_sha256 = "{file_digest(contract)}"',
        f'plan_sha256 = "{file_digest(REPO / "docs/plan/IMPLEMENTATION-PLAN.md")}"',
        f'spec_sha256 = "{file_digest(REPO / "docs/spec/rha-spec-v0.10.md")}"',
        f'reference_sha256 = "{file_digest(BASE / "reference.py")}"',
        f'generator_sha256 = "{file_digest(BASE / "generate.py")}"',
        f'headline_author_sha256 = "{file_digest(BASE / "headline.py")}"',
        f'content_sha256 = "{digest_bytes(checksum_bytes)}"',
        f'random_content_sha256 = "{sections["random"]}"',
        f'headline_content_sha256 = "{sections["headline"]}"',
        f'content_files = {len(files)}', f'content_bytes = {sum(p.stat().st_size for p in files)}',
        'content_digest_protocol = "SHA256 of UTF-8 SHA256SUMS; sorted relative POSIX paths, lines <sha256>  <path> LF; excludes registration.toml and SHA256SUMS"',
        '', '[counts]',
        *[f'{key} = {value}' for key, value in sorted(counters.items())],
        f'rustc_legality_checks = {checks["rustc_checks"]}',
        f'cargo_headline_checks = {len(checks["headline_cargo_checks"])}',
        f'reference_hand_tests = {checks["reference_hand_tests"]}',
        f'legal_random_cases = {checks["random_rule_summary"]["legal_cases"]}',
        f'violating_random_cases = {checks["random_rule_summary"]["violating_cases"]}',
        '', '[scope]', 'headline_authority = "xtask/tests/corpus/manifest.toml, unchanged"',
        'contract_notes = "CONTRACT-NOTES.md"',
        'proposal_provenance = "provenance/README.md"',
        'excluded_cases = ["L-M01: live product sources not read", "X-M01: no external tool or live-product grading attempted"]',
        '',
    ])
    (BASE / 'registration.toml').write_text(registration)
    parsed = tomllib.loads(registration)
    print(json.dumps({'content_sha256': parsed['content_sha256'], 'prompt_sha256': parsed['prompt_sha256'],
                      'reference_sha256': parsed['reference_sha256'],
                      'content_files': len(files), 'content_bytes': parsed['content_bytes'],
                      'counts': parsed['counts']}, sort_keys=True))


if __name__ == '__main__':
    main()
