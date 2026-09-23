#!/usr/bin/env python3
"""Hand-derived checks of our own finite reference, never production grading."""
import json
import tempfile
import unittest
from pathlib import Path
from generate import BASE, materialize, reference


class ReferenceTests(unittest.TestCase):
    def extract(self, source, extra=None, edition='2021'):
        with tempfile.TemporaryDirectory(dir=BASE, prefix='.reference-test-') as tmp:
            directory = Path(tmp)
            materialize({'src/lib.rs': source, **(extra or {})}, directory)
            return reference(directory, 'probe', edition)

    def tuples(self, result):
        return {(e['source'], e['target'], e['test_only'], e['extraction']) for e in result['edges']}

    def test_local_alias_chain_and_group(self):
        r = self.extract('''pub mod a { pub struct Item; pub fn f() {} }
            mod b { use crate::a as dep; use dep::{self as alias, Item as T};
              fn g(_: T) { alias::f(); } }''')
        self.assertEqual(r['limitations'], [])
        self.assertEqual(self.tuples(r), {
            ('probe::b', 'probe::a', False, 'syntax'),
            ('probe::b', 'probe::a::Item', False, 'syntax'),
            ('probe::b', 'probe::a::f', False, 'syntax')})

    def test_disjoint_function_imports(self):
        r = self.extract('''mod a { pub struct Item; } mod b { pub struct Item; }
            mod c { fn f() { use crate::a::Item as T; let _ = T; }
              fn g() { use crate::b::Item as T; let _ = T; } }''')
        self.assertEqual(r['limitations'], [])
        self.assertEqual({e['target'] for e in r['edges']}, {'probe::a::Item', 'probe::b::Item'})

    def test_glob_actual_binding_use(self):
        r = self.extract('''mod a { pub struct Item; pub fn f() {} }
            mod b { use crate::a::*; fn f(_: Item) {} }''')
        self.assertEqual(r['limitations'], [])
        self.assertEqual({e['target'] for e in r['edges']}, {'probe::a', 'probe::a::Item'})

    def test_nested_file_and_explicit_path(self):
        r = self.extract('mod a; #[path="elsewhere.rs"] mod b;', {
            'src/a/mod.rs': 'pub struct Item; pub mod nested;',
            'src/a/nested.rs': 'fn f(_: super::Item) {}',
            'src/elsewhere.rs': 'fn f(_: crate::a::Item) {}'})
        self.assertEqual(r['limitations'], [])
        self.assertEqual(self.tuples(r), {
            ('probe::a::nested', 'probe::a::Item', False, 'syntax'),
            ('probe::b', 'probe::a::Item', False, 'syntax')})

    def test_cfg_module_and_item_flags(self):
        r = self.extract('''mod a { pub fn f() {} }
          mod b { #[cfg(test)] fn test() { crate::a::f(); }
            #[cfg(test)] mod tests { fn f() { crate::a::f(); } }
            fn production() { crate::a::f(); } }''')
        self.assertEqual(r['limitations'], [])
        self.assertEqual(self.tuples(r), {
            ('probe::b', 'probe::a::f', True, 'syntax'),
            ('probe::b', 'probe::a::f', False, 'syntax'),
            ('probe::b::tests', 'probe::a::f', True, 'syntax')})

    def test_macro_args_heuristic_through_groups(self):
        r = self.extract('''mod a { pub fn f() -> u8 { 1 } }
            mod b { fn f() { assert_eq!({ crate::a::f() }, 1); } }''')
        self.assertEqual(r['limitations'], [])
        self.assertEqual(self.tuples(r), {('probe::b', 'probe::a::f', False, 'heuristic')})

    def test_macro_and_include_expected_holes(self):
        r = self.extract('''macro_rules! m { () => { crate::hidden::f() } }
            mod b { include!("../absent.in"); fn f() { m!(); } }''')
        self.assertEqual(r['edges'], [])
        self.assertEqual({x['code'] for x in r['limitations']},
                         {'macro_definition', 'macro_expansion', 'include_expansion'})

    def test_m19_explicit_facade_kept(self):
        r = self.extract('''pub mod a { pub fn score() {} }
            pub use a::score;
            mod b { fn f() { crate::score(); } }''')
        self.assertEqual(r['limitations'], [])
        self.assertEqual(self.tuples(r), {
            ('probe', 'probe::a::score', False, 'syntax'),
            ('probe::b', 'probe::score', False, 'syntax')})

    def test_uniform_paths_editions(self):
        source = 'mod a { pub struct Item; } mod b { use a::Item; }'
        a = self.extract(source, edition='2015')
        self.assertEqual(a['limitations'], [])
        self.assertEqual(self.tuples(a), {('probe::b', 'probe::a::Item', False, 'syntax')})
        b = self.extract(source)
        self.assertEqual(b['edges'], [])
        self.assertEqual(b['limitations'][0]['code'], 'unresolved_path')
        c = self.extract(source.replace('use a::Item;', 'use super::a; use a::Item;'))
        self.assertEqual(c['limitations'], [])
        self.assertEqual({e['target'] for e in c['edges']}, {'probe::a', 'probe::a::Item'})

    def test_comments_strings_and_external_paths(self):
        r = self.extract('''// crate::ghost::Miss
          /* crate::ghost::Miss /* nested */ */
          mod a { fn f() { let _ = "crate::ghost::Miss"; std::hint::black_box(1); } }''')
        self.assertEqual(r, {'schema_version': 1, 'edges': [], 'limitations': []})

    def test_unknown_and_unsupported_are_explicit(self):
        r = self.extract('''#[cfg(feature="x")] mod a { use crate::missing::Item; }
            enum Unsupported { A }''')
        self.assertEqual(r['edges'], [])
        codes = {x['code'] for x in r['limitations']}
        self.assertIn('unsupported_attribute', codes)
        self.assertIn('unsupported_item', codes)
        self.assertIn('unsupported_associated_path', codes)

    def test_mutation_changes_parsed_edges(self):
        source = 'mod a { pub struct Item; } mod b { pub struct Item; } mod c { use crate::a::Item; }'
        a = self.extract(source)
        b = self.extract(source.replace('use crate::a::Item', 'use crate::b::Item'))
        self.assertNotEqual(a, b)
        self.assertEqual(a['edges'][0]['target'], 'probe::a::Item')
        self.assertEqual(b['edges'][0]['target'], 'probe::b::Item')

    def test_missing_module_file_not_silent(self):
        r = self.extract('mod absent;')
        self.assertEqual(r['limitations'][0]['code'], 'module_file_ambiguity')

    def test_method_inference_hole(self):
        r = self.extract('fn f() { let value = 1; value.touch(); }')
        self.assertEqual(r['edges'], [])
        self.assertEqual(r['limitations'], [{'source': 'probe', 'code': 'method_type_inference', 'detail': 'touch'}])


if __name__ == '__main__':
    unittest.main()
