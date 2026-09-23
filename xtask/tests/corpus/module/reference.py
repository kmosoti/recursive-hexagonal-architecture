#!/usr/bin/env python3
"""Independent, deliberately finite Rust source extractor. See README.md.

No generator, corpus, expected-output, or production modules are imported.
"""
import json
import re
import sys
from dataclasses import dataclass, field
from pathlib import Path


@dataclass
class Group:
    opening: str
    tokens: list


def lex(text):
    """Tokenize strings/comments atomically, then build balanced token groups."""
    flat = []
    i = 0
    while i < len(text):
        if text[i].isspace():
            i += 1
        elif text.startswith('//', i):
            end = text.find('\n', i)
            i = len(text) if end < 0 else end + 1
        elif text.startswith('/*', i):
            depth = 1
            i += 2
            while depth and i < len(text):
                if text.startswith('/*', i):
                    depth += 1
                    i += 2
                elif text.startswith('*/', i):
                    depth -= 1
                    i += 2
                else:
                    i += 1
            if depth:
                raise ValueError('unterminated comment')
        elif text[i] == '"':
            start = i
            i += 1
            while i < len(text) and text[i] != '"':
                i += 2 if text[i] == '\\' else 1
            if i >= len(text):
                raise ValueError('unterminated string')
            i += 1
            flat.append(text[start:i])
        else:
            m = re.match(r'[A-Za-z_][A-Za-z_0-9]*|[0-9]+|::|->|=>', text[i:])
            token = m[0] if m else text[i]
            flat.append(token)
            i += len(token)
    stack = [[]]
    opens = []
    for token in flat:
        if token in ('(', '[', '{'):
            opens.append(token)
            stack.append([])
        elif token in (')', ']', '}'):
            if not opens or '([{'.index(opens[-1]) != ')]}'.index(token):
                raise ValueError('unbalanced delimiter')
            group = Group(opens.pop(), stack.pop())
            stack[-1].append(group)
        else:
            stack[-1].append(token)
    if opens:
        raise ValueError('unclosed delimiter')
    return stack[0]


def ident(token):
    return isinstance(token, str) and re.fullmatch(r'[A-Za-z_][A-Za-z_0-9]*', token)


def display(tokens):
    closes = {'(': ')', '[': ']', '{': '}'}
    return ' '.join(t.opening + display(t.tokens) + closes[t.opening]
                    if isinstance(t, Group) else t for t in tokens)


@dataclass
class Scope:
    module: tuple
    parent: object = None
    test: bool = False
    bindings: dict = field(default_factory=dict)
    globs: list = field(default_factory=list)
    locals: set = field(default_factory=set)
    definitions: set = field(default_factory=set)


@dataclass
class Ref:
    scope: Scope
    path: tuple
    test: bool
    extraction: str
    use: bool = False


PRIMITIVES = {'u8', 'u16', 'u32', 'u64', 'usize', 'i8', 'i32', 'i64',
              'bool', 'str', 'Self', 'self', 'true', 'false', '_'}


class Extractor:
    def __init__(self, root, crate_name, edition='2021', externals=()):
        self.root = Path(root).resolve()
        self.crate = crate_name.replace('-', '_')
        self.edition = edition
        self.externals = {'std', 'core', 'alloc', *externals}
        self.modules = {}
        self.refs = []
        self.limits = set()
        self.files_active = set()

    def limit(self, module, code, detail):
        self.limits.add(('::'.join(module), code, detail))

    def read(self, path, scope, directory):
        path = path.resolve()
        # The protocol permits reading only the fixture directory, never ../ product.
        if not path.is_relative_to(self.root.parent.parent):
            self.limit(scope.module, 'outside_fixture', path.name)
            return
        if path in self.files_active:
            self.limit(scope.module, 'recursive_module_file', path.name)
            return
        self.files_active.add(path)
        try:
            tokens = lex(path.read_text())
            self.items(tokens, scope, directory)
        except (ValueError, OSError, IndexError) as exc:
            self.limit(scope.module, 'parse_error', type(exc).__name__ + ': ' + str(exc).replace(str(path), path.name))
        finally:
            self.files_active.remove(path)

    def attributes(self, tokens, i, scope):
        test = scope.test
        path = None
        while i < len(tokens) and tokens[i] == '#':
            i += 1
            if i < len(tokens) and tokens[i] == '!':
                i += 1
            attr = tokens[i]
            if not isinstance(attr, Group) or attr.opening != '[':
                raise ValueError('attribute requires brackets')
            a = attr.tokens
            if len(a) == 2 and a[0] == 'cfg' and isinstance(a[1], Group) and a[1].tokens == ['test']:
                test = True
            elif len(a) == 3 and a[:2] == ['path', '=']:
                path = json.loads(a[2])
            elif a and a[0] in ('allow', 'test'):
                pass
            else:
                self.limit(scope.module, 'unsupported_attribute', display(a))
            i += 1
        return i, test, path

    def addref(self, scope, parts, test=None, extraction='syntax', use=False):
        if parts:
            self.refs.append(Ref(scope, tuple(parts), scope.test if test is None else test, extraction, use))

    def use_tree(self, tokens, prefix=()):
        """Recursive Rust use-tree subset: path/group/self/rename/glob."""
        result = []
        pos = 0
        while pos < len(tokens):
            path = list(prefix)
            while pos < len(tokens) and ident(tokens[pos]):
                path.append(tokens[pos])
                pos += 1
                if pos < len(tokens) and tokens[pos] == '::':
                    pos += 1
                else:
                    break
            if pos < len(tokens) and isinstance(tokens[pos], Group):
                if tokens[pos].opening != '{':
                    raise ValueError('unsupported use group')
                result.extend(self.use_tree(tokens[pos].tokens, tuple(path)))
                pos += 1
            else:
                glob = pos < len(tokens) and tokens[pos] == '*'
                if glob:
                    pos += 1
                if path and path[-1] == 'self' and len(path) > 1:
                    path.pop()
                alias = path[-1] if path else ''
                if pos < len(tokens) and tokens[pos] == 'as':
                    alias = tokens[pos + 1]
                    pos += 2
                if not path:
                    raise ValueError('empty use path')
                result.append((tuple(path), alias, glob))
            if pos < len(tokens):
                if tokens[pos] != ',':
                    raise ValueError('unsupported use tree: ' + display(tokens[pos:]))
                pos += 1
        return result

    def import_item(self, tokens, scope, test):
        for path, alias, glob in self.use_tree(tokens):
            self.addref(scope, path, test, use=True)
            if glob:
                scope.globs.append(path)
            elif alias != '_':
                if alias in scope.bindings:
                    self.limit(scope.module, 'duplicate_binding', alias)
                scope.bindings[alias] = path
            if test != scope.test:
                self.limit(scope.module, 'conditional_import_binding', alias)

    def path_tokens(self, tokens, pos):
        parts = [tokens[pos]]
        pos += 1
        while pos + 1 < len(tokens) and tokens[pos] == '::' and ident(tokens[pos + 1]):
            parts.append(tokens[pos + 1])
            pos += 2
        return parts, pos

    def scan(self, tokens, scope, test=None, extraction='syntax'):
        """Walk finite type/expression syntax, recursively preserving block scopes."""
        i = 0
        while i < len(tokens):
            t = tokens[i]
            if isinstance(t, Group):
                inner = Scope(scope.module, scope, scope.test if test is None else test)
                if extraction == 'heuristic':
                    self.scan(t.tokens, scope, test, extraction)
                elif t.opening == '{':
                    self.items(t.tokens, inner, None, block=True)
                else:
                    self.scan(t.tokens, scope, test, extraction)
                i += 1
            elif t == 'let':
                if i + 1 >= len(tokens) or not ident(tokens[i + 1]):
                    self.limit(scope.module, 'unsupported_binding_pattern', display(tokens[i:]))
                    return
                # Generated locals do not shadow paths used earlier in this block.
                scope.locals.add(tokens[i + 1])
                i += 2
            elif t == '.':
                if i + 1 < len(tokens) and ident(tokens[i + 1]):
                    self.limit(scope.module, 'method_type_inference', tokens[i + 1])
                    i += 2
                else:
                    self.limit(scope.module, 'unsupported_expression', '.')
                    i += 1
            elif ident(t):
                if t in ('return', 'mut', 'as'):
                    i += 1
                    continue
                parts, end = self.path_tokens(tokens, i)
                if end < len(tokens) and tokens[end] == '!':
                    if end + 1 >= len(tokens) or not isinstance(tokens[end + 1], Group):
                        raise ValueError('macro needs token group')
                    if parts[-1] == 'include':
                        self.limit(scope.module, 'include_expansion', 'include! contents are not read')
                    else:
                        if parts[-1] not in ('assert_eq', 'assert', 'debug_assert', 'vec'):
                            self.limit(scope.module, 'macro_expansion', '::'.join(parts))
                        self.scan(tokens[end + 1].tokens, scope, test, 'heuristic')
                    i = end + 2
                else:
                    if t not in PRIMITIVES or len(parts) > 1:
                        self.addref(scope, parts, test, extraction)
                    i = end
            elif t in (';', ',', ':', '=', '&', '+', '-', '*') or re.fullmatch(r'[0-9]+|".*"', t, re.S):
                i += 1
            else:
                self.limit(scope.module, 'unsupported_expression', str(t))
                i += 1

    def items(self, tokens, scope, directory, block=False):
        i = 0
        while i < len(tokens):
            if tokens[i] == ';':
                i += 1
                continue
            i, test, explicit_path = self.attributes(tokens, i, scope)
            if i == len(tokens):
                break
            if tokens[i] == 'pub':
                i += 1
                if i < len(tokens) and isinstance(tokens[i], Group) and tokens[i].opening == '(':
                    i += 1
            t = tokens[i]
            if t == 'mod':
                name = tokens[i + 1]
                scope.definitions.add(name)
                module = scope.module + (name,)
                child = Scope(module, None, test)
                self.modules[module] = child
                body = tokens[i + 2]
                if block or directory is None:
                    self.limit(scope.module, 'block_module', name)
                elif isinstance(body, Group) and body.opening == '{':
                    if explicit_path:
                        self.limit(module, 'path_on_inline_module', explicit_path)
                    self.items(body.tokens, child, directory / name)
                elif body == ';':
                    if explicit_path:
                        filename = directory / explicit_path
                    else:
                        choices = [p for p in (directory / (name + '.rs'), directory / name / 'mod.rs') if p.is_file()]
                        if len(choices) != 1:
                            self.limit(module, 'module_file_ambiguity', name)
                            i += 3
                            continue
                        filename = choices[0]
                    next_dir = filename.parent if filename.name == 'mod.rs' else filename.with_suffix('')
                    self.read(filename, child, next_dir)
                else:
                    raise ValueError('module needs body or semicolon')
                i += 3
            elif t == 'use':
                end = tokens.index(';', i)
                self.import_item(tokens[i + 1:end], scope, test)
                i = end + 1
            elif t == 'extern' and tokens[i + 1] == 'crate':
                end = tokens.index(';', i)
                name = tokens[i + 2]
                alias = tokens[i + 4] if end == i + 5 and tokens[i + 3] == 'as' else name
                self.externals.add(alias)
                i = end + 1
            elif t == 'macro_rules':
                if tokens[i + 1] != '!' or not isinstance(tokens[i + 3], Group):
                    raise ValueError('malformed macro_rules')
                self.limit(scope.module, 'macro_definition', tokens[i + 2])
                i += 4
            elif t == 'fn':
                name = tokens[i + 1]
                scope.definitions.add(name)
                params = tokens[i + 2]
                if not isinstance(params, Group) or params.opening != '(':
                    self.limit(scope.module, 'unsupported_function_header', name)
                    return
                fn_scope = Scope(scope.module, scope, test)
                p = params.tokens
                j = 0
                while j < len(p):
                    if p[j] in ('&', 'mut'):
                        j += 1
                        continue
                    if not ident(p[j]):
                        raise ValueError('unsupported parameter pattern')
                    fn_scope.locals.add(p[j])
                    j += 1
                    if j < len(p) and p[j] == ':':
                        end = next((n for n in range(j + 1, len(p)) if p[n] == ','), len(p))
                        self.scan(p[j + 1:end], fn_scope)
                        j = end
                    if j < len(p) and p[j] == ',':
                        j += 1
                end = i + 3
                if end < len(tokens) and tokens[end] == '->':
                    start = end + 1
                    end = start
                    while end < len(tokens) and tokens[end] != ';' and not (isinstance(tokens[end], Group) and tokens[end].opening == '{'):
                        end += 1
                    self.scan(tokens[start:end], fn_scope)
                if isinstance(tokens[end], Group) and tokens[end].opening == '{':
                    self.items(tokens[end].tokens, Scope(scope.module, fn_scope, test), None, block=True)
                elif tokens[end] != ';':
                    raise ValueError('unsupported function body')
                i = end + 1
            elif t in ('struct', 'trait'):
                name = tokens[i + 1]
                scope.definitions.add(name)
                end = i + 2
                if tokens[end] == ';':
                    pass
                elif t == 'trait' and isinstance(tokens[end], Group) and tokens[end].opening == '{':
                    self.items(tokens[end].tokens, Scope(scope.module, scope, test), None)
                else:
                    self.limit(scope.module, 'unsupported_declaration', t + ' ' + name)
                i = end + 1
            elif t == 'impl':
                end = i + 1
                while end < len(tokens) and not (isinstance(tokens[end], Group) and tokens[end].opening == '{'):
                    end += 1
                self.scan([x for x in tokens[i + 1:end] if x != 'for'], scope, test)
                self.items(tokens[end].tokens, Scope(scope.module, scope, test), None)
                i = end + 1
            elif block or (ident(t) and i + 1 < len(tokens) and tokens[i + 1] == '!'):
                # A statement ends at a top-level semicolon; nested groups are atomic.
                end = next((n for n in range(i, len(tokens)) if tokens[n] == ';'), len(tokens))
                self.scan(tokens[i:end], scope, test)
                i = end + 1
            else:
                self.limit(scope.module, 'unsupported_item', display(tokens[i:i + 4]))
                end = next((n for n in range(i, len(tokens)) if tokens[n] == ';' or isinstance(tokens[n], Group)), len(tokens))
                i = end + 1

    def canonical(self, scope, path, use=False, active=frozenset()):
        if not path:
            return None
        head, *tail = path
        if head in self.externals:
            return (head, *tail)
        if head in ('crate', self.crate):
            return (self.crate, *tail)
        if head == 'self':
            return (*scope.module, *tail)
        if head == 'super':
            base = list(scope.module)
            rest = list(path)
            while rest and rest[0] == 'super':
                rest.pop(0)
                if len(base) == 1:
                    return None
                base.pop()
            return (*base, *rest)
        if use and self.edition == '2015':
            return (self.crate, *path)
        cursor = scope
        while cursor is not None:
            if head in cursor.locals:
                return ()  # Local values are not module paths; no type inference.
            if head in cursor.bindings:
                key = (id(cursor), head)
                if key in active:
                    return None
                value = self.canonical(cursor, cursor.bindings[head], True, active | {key})
                return (*value, *tail) if value else value
            if head in cursor.definitions:
                return (*scope.module, *path)
            candidates = set()
            for glob in cursor.globs:
                key = (id(cursor), '*', glob)
                if key in active:
                    continue
                base = self.canonical(cursor, glob, True, active | {key})
                if base in self.modules:
                    provider = self.modules[base]
                    if head in provider.definitions or head in provider.bindings:
                        candidates.add((*base, *path))
            if len(candidates) == 1:
                return next(iter(candidates))
            if len(candidates) > 1:
                return None
            cursor = cursor.parent
        return None

    def run(self):
        root_scope = Scope((self.crate,))
        self.modules[root_scope.module] = root_scope
        self.read(self.root, root_scope, self.root.parent)
        edges = set()
        for ref in self.refs:
            target = self.canonical(ref.scope, ref.path, ref.use)
            if target == ():
                continue
            if target and target[0] in self.externals:
                continue
            if not target:
                self.limit(ref.scope.module, 'unresolved_path', '::'.join(ref.path))
                continue
            owner = target
            while owner and owner not in self.modules:
                owner = owner[:-1]
            if not owner or len(target) > len(owner) + 1:
                self.limit(ref.scope.module, 'unsupported_associated_path', '::'.join(target))
                continue
            if len(target) == len(owner) + 1:
                s = self.modules[owner]
                if target[-1] not in s.definitions and target[-1] not in s.bindings:
                    self.limit(ref.scope.module, 'unresolved_path', '::'.join(target))
                    continue
            if owner != ref.scope.module:
                edges.add(('::'.join(ref.scope.module), '::'.join(target), ref.test, ref.extraction))
        return {
            'schema_version': 1,
            'edges': [dict(source=a, target=b, test_only=c, extraction=d) for a, b, c, d in sorted(edges)],
            'limitations': [dict(source=a, code=b, detail=c) for a, b, c in sorted(self.limits)],
        }


def extract(request):
    return Extractor(request['root'], request['crate_name'], request.get('edition', '2021'),
                     request.get('externals', [])).run()


def main():
    try:
        request = json.load(sys.stdin)
        result = extract(request)
    except (KeyError, TypeError, ValueError, OSError) as exc:
        print(json.dumps({'error': str(exc)}, sort_keys=True))
        return 2
    print(json.dumps(result, sort_keys=True, separators=(',', ':')))
    return 0


if __name__ == '__main__':
    sys.exit(main())
