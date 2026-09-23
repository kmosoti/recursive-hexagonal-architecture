use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use proc_macro2::TokenTree;
use serde::Serialize;
use syn::visit::Visit;
use syn::{
    Attribute, Expr, ExprLit, FnArg, GenericParam, Item, ItemMacro, ItemMod, ItemUse, Lit, Meta,
    Pat, Path as SynPath, Stmt, UseTree,
};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct Edge {
    pub source: String,
    pub target: String,
    pub test_only: bool,
    pub extraction: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct Limitation {
    pub source: String,
    pub code: String,
    pub detail: String,
}

#[derive(Debug, Clone)]
pub struct Extracted {
    pub modules: BTreeMap<String, PathBuf>,
    pub edges: Vec<Edge>,
    pub limitations: Vec<Limitation>,
}

#[derive(Clone)]
struct UseDecl {
    path: Vec<String>,
    alias: String,
    glob: bool,
    test: bool,
}

struct ModuleData {
    path: Vec<String>,
    items: Vec<Item>,
    definitions: BTreeSet<String>,
    imports: Vec<UseDecl>,
    test: bool,
}

struct ChildSpec {
    path: Vec<String>,
    name: String,
    file: PathBuf,
    inline_items: Option<Vec<Item>>,
    path_attr: Option<String>,
    test: bool,
}

#[derive(Clone)]
struct Binding {
    path: Vec<String>,
    origin: usize,
    test: bool,
}

struct Frame {
    module: Vec<String>,
    module_scope: bool,
    bindings: BTreeMap<String, Vec<Binding>>,
    globs: Vec<Binding>,
    locals: BTreeSet<String>,
    type_locals: BTreeSet<String>,
    definitions: BTreeSet<String>,
}

impl Frame {
    fn module(module: Vec<String>) -> Self {
        Self {
            module,
            module_scope: true,
            bindings: BTreeMap::new(),
            globs: Vec::new(),
            locals: BTreeSet::new(),
            type_locals: BTreeSet::new(),
            definitions: BTreeSet::new(),
        }
    }

    fn block(module: Vec<String>) -> Self {
        Self {
            module,
            module_scope: false,
            bindings: BTreeMap::new(),
            globs: Vec::new(),
            locals: BTreeSet::new(),
            type_locals: BTreeSet::new(),
            definitions: BTreeSet::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Resolution {
    Target(Vec<String>),
    External,
    Local,
    Unknown,
    Ambiguous,
}

struct Engine {
    crate_root: PathBuf,
    crate_name: String,
    edition: String,
    externals: BTreeSet<String>,
    modules: BTreeMap<String, ModuleData>,
    module_paths: BTreeMap<String, PathBuf>,
    edges: BTreeSet<Edge>,
    limitations: BTreeSet<Limitation>,
    active_files: BTreeSet<PathBuf>,
}

impl Engine {
    fn limit(&mut self, module: &[String], code: &str, detail: impl Into<String>) {
        self.limitations.insert(Limitation {
            source: module_name(module),
            code: code.to_owned(),
            detail: detail.into(),
        });
    }

    fn attr_info(&mut self, attrs: &[Attribute], module: &[String]) -> (bool, Option<String>) {
        let mut test = false;
        let mut path = None;
        for attr in attrs {
            if attr.path().is_ident("cfg") {
                if let Meta::List(list) = &attr.meta {
                    let detail = list.tokens.to_string();
                    if detail == "test" {
                        test = true;
                    } else {
                        self.limit(module, "conditional_cfg", detail);
                    }
                } else {
                    self.limit(
                        module,
                        "conditional_cfg",
                        syn_path_names(attr.meta.path()).join("::"),
                    );
                }
            } else if attr.path().is_ident("path") {
                match &attr.meta {
                    Meta::NameValue(value) => match &value.value {
                        Expr::Lit(ExprLit {
                            lit: Lit::Str(value),
                            ..
                        }) => path = Some(value.value()),
                        _ => self.limit(module, "unsupported_attribute", "path"),
                    },
                    _ => self.limit(module, "unsupported_attribute", "path"),
                }
            } else if attr.path().is_ident("cfg_attr") {
                self.limit(
                    module,
                    "conditional_cfg",
                    attr.meta.to_token_stream_string(),
                );
            } else if attr.path().is_ident("allow")
                || attr.path().is_ident("deny")
                || attr.path().is_ident("warn")
                || attr.path().is_ident("expect")
                || attr.path().is_ident("test")
            {
            }
        }
        (test, path)
    }

    fn install_module(
        &mut self,
        path: Vec<String>,
        file: PathBuf,
        items: Vec<Item>,
        test: bool,
        directory: PathBuf,
        attr_base: PathBuf,
    ) -> Result<(), String> {
        let key = module_name(&path);
        if self.modules.contains_key(&key) {
            self.limit(&path, "duplicate_module", key);
            return Ok(());
        }

        let mut definitions = BTreeSet::new();
        let mut imports = Vec::new();
        let mut children = Vec::new();

        for item in &items {
            let (item_test, path_attr) = self.item_attrs(item, &path);
            if let Item::Use(item_use) = item {
                for declaration in flatten_use(&item_use.tree, &[]) {
                    imports.push(UseDecl {
                        path: declaration.0,
                        alias: declaration.1,
                        glob: declaration.2,
                        test: test || item_test,
                    });
                }
            }
            if let Some(name) = item_definition(item) {
                definitions.insert(name);
            }
            if let Item::ExternCrate(item) = item {
                let external = item
                    .rename
                    .as_ref()
                    .map(|(_, ident)| ident.to_string())
                    .unwrap_or_else(|| item.ident.to_string());
                self.externals.insert(external);
            }
            if let Item::Mod(item_mod) = item {
                let child_path = {
                    let mut child = path.clone();
                    child.push(item_mod.ident.to_string());
                    child
                };
                let child_test = test || item_test;
                let inline_items = item_mod.content.as_ref().map(|(_, body)| body.clone());
                children.push(ChildSpec {
                    path: child_path,
                    name: item_mod.ident.to_string(),
                    file: file.clone(),
                    inline_items,
                    path_attr,
                    test: child_test,
                });
            }
        }

        self.module_paths.insert(key.clone(), file.clone());
        self.modules.insert(
            key,
            ModuleData {
                path: path.clone(),
                items,
                definitions,
                imports,
                test,
            },
        );

        for child in children {
            if let Some(items) = child.inline_items {
                let child_directory = if let Some(relative) = child.path_attr {
                    attr_base.join(relative)
                } else {
                    directory.join(&child.name)
                };
                let child_attr_base = child_directory.clone();
                self.install_module(
                    child.path,
                    child.file,
                    items,
                    child.test,
                    child_directory,
                    child_attr_base,
                )?;
                continue;
            }

            let selected = if let Some(relative) = child.path_attr.as_ref() {
                Some(attr_base.join(relative))
            } else {
                let file_candidate = directory.join(format!("{}.rs", child.name));
                let directory_candidate = directory.join(&child.name).join("mod.rs");
                let file_exists = file_candidate.is_file();
                let directory_exists = directory_candidate.is_file();
                match (file_exists, directory_exists) {
                    (true, false) => Some(file_candidate),
                    (false, true) => Some(directory_candidate),
                    (false, false) => {
                        return Err(format!(
                            "missing module source for {}: {} or {}",
                            module_name(&child.path),
                            file_candidate.display(),
                            directory_candidate.display()
                        ));
                    }
                    (true, true) => {
                        return Err(format!(
                            "ambiguous module source for {}: {} and {}",
                            module_name(&child.path),
                            file_candidate.display(),
                            directory_candidate.display()
                        ));
                    }
                }
            };

            if let Some(selected) = selected {
                let child_directory = if child.path_attr.is_some() {
                    selected.parent().unwrap_or(Path::new(".")).to_path_buf()
                } else {
                    directory.join(&child.name)
                };
                let child_attr_base = selected.parent().unwrap_or(Path::new(".")).to_path_buf();
                self.discover_file(
                    child.path,
                    selected,
                    child.test,
                    child_directory,
                    child_attr_base,
                )?;
            }
        }
        Ok(())
    }

    fn item_attrs(&mut self, item: &Item, module: &[String]) -> (bool, Option<String>) {
        match item {
            Item::Const(item) => self.attr_info(&item.attrs, module),
            Item::Enum(item) => self.attr_info(&item.attrs, module),
            Item::ExternCrate(item) => self.attr_info(&item.attrs, module),
            Item::Fn(item) => self.attr_info(&item.attrs, module),
            Item::ForeignMod(item) => self.attr_info(&item.attrs, module),
            Item::Impl(item) => self.attr_info(&item.attrs, module),
            Item::Macro(item) => self.attr_info(&item.attrs, module),
            Item::Mod(item) => self.attr_info(&item.attrs, module),
            Item::Static(item) => self.attr_info(&item.attrs, module),
            Item::Struct(item) => self.attr_info(&item.attrs, module),
            Item::Trait(item) => self.attr_info(&item.attrs, module),
            Item::TraitAlias(item) => self.attr_info(&item.attrs, module),
            Item::Type(item) => self.attr_info(&item.attrs, module),
            Item::Union(item) => self.attr_info(&item.attrs, module),
            Item::Use(item) => self.attr_info(&item.attrs, module),
            Item::Verbatim(_) => (false, None),
            _ => {
                self.limit(module, "unsupported_item", "unrecognized syntax item");
                (false, None)
            }
        }
    }

    fn discover_file(
        &mut self,
        path: Vec<String>,
        file: PathBuf,
        inherited_test: bool,
        directory: PathBuf,
        attr_base: PathBuf,
    ) -> Result<(), String> {
        let file = fs::canonicalize(&file)
            .map_err(|error| format!("failed to canonicalize {}: {}", file.display(), error))?;
        if !file.starts_with(&self.crate_root) {
            return Err(format!(
                "module file {} is outside crate root {}",
                file.display(),
                self.crate_root.display()
            ));
        }
        if self.active_files.contains(&file) {
            return Err(format!(
                "recursive module source {} at {}",
                file.display(),
                module_name(&path)
            ));
        }

        self.active_files.insert(file.clone());
        let source = match fs::read_to_string(&file) {
            Ok(source) => source,
            Err(error) => {
                self.active_files.remove(&file);
                return Err(format!("failed to read {}: {}", file.display(), error));
            }
        };
        let parsed = match syn::parse_file(&source) {
            Ok(parsed) => parsed,
            Err(error) => {
                self.active_files.remove(&file);
                return Err(format!("failed to parse {}: {}", file.display(), error));
            }
        };
        let (file_test, _) = self.attr_info(&parsed.attrs, &path);
        let result = self.install_module(
            path.clone(),
            file.clone(),
            parsed.items,
            inherited_test || file_test,
            directory,
            attr_base,
        );
        self.active_files.remove(&file);
        result
    }
}

fn item_definition(item: &Item) -> Option<String> {
    match item {
        Item::Const(item) => Some(item.ident.to_string()),
        Item::Enum(item) => Some(item.ident.to_string()),
        Item::Fn(item) => Some(item.sig.ident.to_string()),
        Item::Macro(item) => item.ident.as_ref().map(ToString::to_string),
        Item::Mod(item) => Some(item.ident.to_string()),
        Item::Static(item) => Some(item.ident.to_string()),
        Item::Struct(item) => Some(item.ident.to_string()),
        Item::Trait(item) => Some(item.ident.to_string()),
        Item::TraitAlias(item) => Some(item.ident.to_string()),
        Item::Type(item) => Some(item.ident.to_string()),
        Item::Union(item) => Some(item.ident.to_string()),
        Item::Use(item) => flatten_use(&item.tree, &[])
            .into_iter()
            .find(|(_, alias, glob)| !*glob && alias != "_")
            .map(|(_, alias, _)| alias),
        _ => None,
    }
}

fn module_name(path: &[String]) -> String {
    path.join("::")
}

fn flatten_use(tree: &UseTree, prefix: &[String]) -> Vec<(Vec<String>, String, bool)> {
    match tree {
        UseTree::Path(path) => {
            let mut next = prefix.to_vec();
            next.push(path.ident.to_string());
            flatten_use(&path.tree, &next)
        }
        UseTree::Name(name) => {
            let mut path = prefix.to_vec();
            let ident = name.ident.to_string();
            if ident == "self" {
                let alias = path.last().cloned().unwrap_or_else(|| "self".to_owned());
                vec![(path, alias, false)]
            } else {
                path.push(ident.clone());
                vec![(path, ident, false)]
            }
        }
        UseTree::Rename(rename) => {
            let mut path = prefix.to_vec();
            let ident = rename.ident.to_string();
            if ident != "self" {
                path.push(ident);
            }
            vec![(path, rename.rename.to_string(), false)]
        }
        UseTree::Glob(_) => vec![(prefix.to_vec(), "*".to_owned(), true)],
        UseTree::Group(group) => group
            .items
            .iter()
            .flat_map(|tree| flatten_use(tree, prefix))
            .collect(),
    }
}

struct Walker<'a> {
    engine: &'a mut Engine,
    module: Vec<String>,
    frames: Vec<Frame>,
    test: bool,
}

impl<'a> Walker<'a> {
    fn new(engine: &'a mut Engine, module: Vec<String>, test: bool) -> Self {
        let mut frame = Frame::module(module.clone());
        if let Some(data) = engine.modules.get(&module_name(&module)) {
            for import in &data.imports {
                let binding = Binding {
                    path: import.path.clone(),
                    origin: 0,
                    test: import.test,
                };
                if import.glob {
                    frame.globs.push(binding);
                } else if import.alias != "_" {
                    frame
                        .bindings
                        .entry(import.alias.clone())
                        .or_default()
                        .push(binding);
                }
            }
        }
        Self {
            engine,
            module,
            frames: vec![frame],
            test,
        }
    }

    fn walk_items(&mut self, items: &[Item]) {
        for item in items {
            self.walk_item(item);
        }
    }

    fn walk_item(&mut self, item: &Item) {
        let old_test = self.test;
        self.test = old_test || item_cfg_test(item);
        self.visit_item(item);
        self.test = old_test;
    }

    fn walk_use(&mut self, item: &ItemUse) {
        for (path, _, _) in flatten_use(&item.tree, &[]) {
            self.record_path(&path, "syntax", true, true);
        }
    }

    fn resolve(&self, path: &[String], for_use: bool) -> Resolution {
        let mut active = BTreeSet::new();
        self.resolve_from(
            path,
            self.frames.len().saturating_sub(1),
            for_use,
            &mut active,
        )
    }

    fn resolve_from(
        &self,
        path: &[String],
        start: usize,
        for_use: bool,
        active: &mut BTreeSet<(usize, String)>,
    ) -> Resolution {
        if path.is_empty() {
            return Resolution::Unknown;
        }
        let head = &path[0];
        let tail = &path[1..];

        if self.engine.externals.contains(head) {
            return Resolution::External;
        }
        if head == "crate" || head == &self.engine.crate_name {
            return Resolution::Target(with_suffix(vec![self.engine.crate_name.clone()], tail));
        }
        if head == "self" {
            if tail.is_empty() {
                return Resolution::Local;
            }
            return Resolution::Target(with_suffix(self.frames[start].module.clone(), tail));
        }
        if head == "super" {
            let mut base = self.frames[start].module.clone();
            let mut rest = path;
            while rest.first().is_some_and(|part| part == "super") {
                if base.len() <= 1 {
                    return Resolution::Unknown;
                }
                base.pop();
                rest = &rest[1..];
            }
            return Resolution::Target(with_suffix(base, rest));
        }
        if for_use && self.engine.edition == "2015" {
            return Resolution::Target(with_suffix(vec![self.engine.crate_name.clone()], path));
        }
        if is_nonmodule_name(head) {
            return Resolution::Local;
        }

        for index in (0..=start).rev() {
            let frame = &self.frames[index];
            if frame.type_locals.contains(head)
                || (frame.locals.contains(head) && path.len() == 1)
                || frame.definitions.contains(head)
            {
                return Resolution::Local;
            }
            if let Some(bindings) = frame.bindings.get(head) {
                let mut candidates = Vec::new();
                for binding in bindings {
                    if binding.test && !self.test {
                        continue;
                    }
                    let key = (binding.origin, head.clone());
                    if !active.insert(key.clone()) {
                        continue;
                    }
                    let resolution = self.resolve_from(&binding.path, binding.origin, true, active);
                    active.remove(&key);
                    candidates.push(append_resolution(resolution, tail));
                }
                candidates.sort();
                candidates.dedup();
                match candidates.as_slice() {
                    [candidate] => return candidate.clone(),
                    [] => {}
                    _ => return Resolution::Ambiguous,
                }
            }
            if frame.module_scope
                && let Some(data) = self.engine.modules.get(&module_name(&frame.module))
                && data.definitions.contains(head)
            {
                return Resolution::Target(with_suffix(frame.module.clone(), path));
            }
            let mut candidates = BTreeSet::new();
            for glob in &frame.globs {
                if glob.test && !self.test {
                    continue;
                }
                let base = self.resolve_from(&glob.path, glob.origin, true, active);
                let Resolution::Target(base) = base else {
                    continue;
                };
                let base_key = module_name(&base);
                if !self.engine.modules.contains_key(&base_key) {
                    continue;
                }
                if self.exported(&base, head, &mut BTreeSet::new()) {
                    candidates.insert(with_suffix(base, path));
                }
            }
            match candidates.len() {
                0 => {}
                1 => return Resolution::Target(candidates.into_iter().next().unwrap_or_default()),
                _ => return Resolution::Ambiguous,
            }
        }
        Resolution::Unknown
    }

    fn exported(
        &self,
        module: &[String],
        name: &str,
        active: &mut BTreeSet<(String, String)>,
    ) -> bool {
        let key = (module_name(module), name.to_owned());
        if !active.insert(key.clone()) {
            return false;
        }
        let Some(data) = self.engine.modules.get(&module_name(module)) else {
            active.remove(&key);
            return false;
        };
        if data.definitions.contains(name) {
            active.remove(&key);
            return true;
        }
        if data
            .imports
            .iter()
            .any(|import| !import.glob && import.alias == name && (!import.test || self.test))
        {
            active.remove(&key);
            return true;
        }
        let imports = data.imports.clone();
        for import in imports {
            if !import.glob || (import.test && !self.test) {
                continue;
            }
            if let Some(provider) = self.resolve_module_import(module, &import.path, active)
                && self.exported(&provider, name, active)
            {
                active.remove(&key);
                return true;
            }
        }
        active.remove(&key);
        false
    }

    fn resolve_module_import(
        &self,
        module: &[String],
        path: &[String],
        active: &mut BTreeSet<(String, String)>,
    ) -> Option<Vec<String>> {
        if path.is_empty() {
            return None;
        }
        let head = &path[0];
        let tail = &path[1..];
        if self.engine.externals.contains(head) {
            return None;
        }
        if head == "crate" || head == &self.engine.crate_name {
            return Some(with_suffix(vec![self.engine.crate_name.clone()], tail));
        }
        if head == "self" {
            return Some(with_suffix(module.to_vec(), tail));
        }
        if head == "super" {
            let mut base = module.to_vec();
            let mut rest = path;
            while rest.first().is_some_and(|part| part == "super") {
                if base.len() <= 1 {
                    return None;
                }
                base.pop();
                rest = &rest[1..];
            }
            return Some(with_suffix(base, rest));
        }
        if self.engine.edition == "2015" {
            return Some(with_suffix(vec![self.engine.crate_name.clone()], path));
        }
        let data = self.engine.modules.get(&module_name(module))?;
        let imports = data.imports.clone();
        for import in imports {
            if import.glob || import.alias != *head || (import.test && !self.test) {
                continue;
            }
            let guard = (module_name(module), head.clone());
            if active.contains(&guard) {
                return None;
            }
            active.insert(guard.clone());
            let result = self
                .resolve_module_import(module, &import.path, active)
                .map(|base| with_suffix(base, tail));
            active.remove(&guard);
            return result;
        }
        if data.definitions.contains(head) {
            return Some(with_suffix(module.to_vec(), path));
        }
        None
    }

    fn record_path(
        &mut self,
        path: &[String],
        extraction: &str,
        for_use: bool,
        report_unknown: bool,
    ) {
        if path.is_empty() {
            return;
        }
        let resolution = self.resolve(path, for_use);
        let target = match resolution {
            Resolution::External | Resolution::Local => return,
            Resolution::Target(target) => target,
            Resolution::Ambiguous => {
                if report_unknown {
                    self.engine
                        .limit(&self.module, "ambiguous_path", path.join("::"));
                }
                return;
            }
            Resolution::Unknown => {
                if report_unknown {
                    self.engine
                        .limit(&self.module, "unresolved_path", path.join("::"));
                }
                return;
            }
        };
        if target
            .first()
            .is_some_and(|head| self.engine.externals.contains(head))
        {
            return;
        }

        let mut owner = target.clone();
        while !owner.is_empty() && !self.engine.modules.contains_key(&module_name(&owner)) {
            owner.pop();
        }
        if owner.is_empty() {
            if report_unknown {
                self.engine
                    .limit(&self.module, "unresolved_path", target.join("::"));
            }
            return;
        }
        if target.len() > owner.len() {
            let name = target[owner.len()].clone();
            let mut active = BTreeSet::new();
            if !self.exported(&owner, &name, &mut active) {
                self.engine
                    .limit(&self.module, "unresolved_path", target.join("::"));
                return;
            }
        }
        if owner != self.module {
            self.engine.edges.insert(Edge {
                source: module_name(&self.module),
                target: target.join("::"),
                test_only: self.test,
                extraction: extraction.to_owned(),
            });
        }
    }

    fn visit_function(&mut self, signature: &syn::Signature, block: Option<&syn::Block>) {
        let frame = generic_frame(&self.module, &signature.generics);
        self.frames.push(frame);
        self.visit_signature(signature);
        for input in &signature.inputs {
            if let FnArg::Typed(input) = input {
                let mut names = BTreeSet::new();
                collect_pattern_names(&input.pat, &mut names);
                if let Some(frame) = self.frames.last_mut() {
                    frame.locals.extend(names);
                }
            }
        }
        if let Some(block) = block {
            self.visit_block(block);
        }
        self.frames.pop();
    }

    fn visit_condition_part(&mut self, expression: &Expr) {
        if let Expr::Binary(binary) = expression
            && matches!(binary.op, syn::BinOp::And(_))
        {
            self.visit_condition_part(&binary.left);
            self.visit_condition_part(&binary.right);
            return;
        }
        if let Expr::Let(let_expression) = expression {
            self.visit_expr(&let_expression.expr);
            self.visit_pat(&let_expression.pat);
            let mut names = BTreeSet::new();
            collect_pattern_names(&let_expression.pat, &mut names);
            if let Some(frame) = self.frames.last_mut() {
                frame.locals.extend(names);
            }
            return;
        }
        self.visit_expr(expression);
    }

    fn visit_condition_body(&mut self, condition: &Expr, body: &syn::Block) {
        self.frames.push(Frame::block(self.module.clone()));
        self.visit_condition_part(condition);
        self.visit_block(body);
        self.frames.pop();
    }

    fn scan_macro_tokens(&mut self, stream: proc_macro2::TokenStream) {
        let tokens: Vec<TokenTree> = stream.into_iter().collect();
        let mut index = 0;
        while index < tokens.len() {
            match &tokens[index] {
                TokenTree::Group(group) => {
                    self.scan_macro_tokens(group.stream());
                    index += 1;
                }
                TokenTree::Ident(ident) => {
                    let mut path = vec![ident.to_string()];
                    let mut end = index + 1;
                    while end + 2 < tokens.len()
                        && is_colon(&tokens[end])
                        && is_colon(&tokens[end + 1])
                    {
                        if let TokenTree::Ident(next) = &tokens[end + 2] {
                            path.push(next.to_string());
                            end += 3;
                        } else {
                            break;
                        }
                    }
                    if macro_candidate(&path, self) {
                        self.record_path(&path, "heuristic", false, true);
                    }
                    index = end;
                }
                _ => index += 1,
            }
        }
    }
}

impl<'ast> Visit<'ast> for Walker<'_> {
    fn visit_attribute(&mut self, _attribute: &'ast Attribute) {}

    fn visit_path(&mut self, path: &'ast SynPath) {
        let names = syn_path_names(path);
        let report_unknown = names.first().is_some_and(|head| {
            matches!(head.as_str(), "crate" | "self" | "super") || head == &self.engine.crate_name
        });
        self.record_path(&names, "syntax", false, report_unknown);
        syn::visit::visit_path(self, path);
    }

    fn visit_visibility(&mut self, _visibility: &'ast syn::Visibility) {}

    fn visit_item_mod(&mut self, _item: &'ast ItemMod) {}

    fn visit_item_use(&mut self, item: &'ast ItemUse) {
        self.walk_use(item);
    }

    fn visit_item_fn(&mut self, item: &'ast syn::ItemFn) {
        self.visit_visibility(&item.vis);
        self.visit_function(&item.sig, Some(&item.block));
    }

    fn visit_item_trait(&mut self, item: &'ast syn::ItemTrait) {
        self.visit_visibility(&item.vis);
        self.frames
            .push(generic_frame(&self.module, &item.generics));
        self.visit_generics(&item.generics);
        for bound in &item.supertraits {
            self.visit_type_param_bound(bound);
        }
        for trait_item in &item.items {
            self.visit_trait_item(trait_item);
        }
        self.frames.pop();
    }

    fn visit_item_impl(&mut self, item: &'ast syn::ItemImpl) {
        self.frames
            .push(generic_frame(&self.module, &item.generics));
        self.visit_generics(&item.generics);
        if let Some((path, _)) = &item.trait_ {
            self.visit_path(path);
        }
        self.visit_type(&item.self_ty);
        for impl_item in &item.items {
            self.visit_impl_item(impl_item);
        }
        self.frames.pop();
    }

    fn visit_item_macro(&mut self, item: &'ast ItemMacro) {
        if item.ident.is_some() {
            self.engine.limit(
                &self.module,
                "macro_definition",
                item.ident
                    .as_ref()
                    .map(ToString::to_string)
                    .unwrap_or_else(|| "macro".to_owned()),
            );
        } else {
            self.visit_macro(&item.mac);
        }
    }

    fn visit_expr_macro(&mut self, item: &'ast syn::ExprMacro) {
        self.visit_macro(&item.mac);
    }

    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        let path = syn_path_names(&mac.path);
        let macro_name = path.last().cloned().unwrap_or_default();
        if macro_name == "include" {
            self.engine.limit(
                &self.module,
                "include_expansion",
                "include! contents are not read",
            );
            return;
        }
        let known = matches!(
            macro_name.as_str(),
            "assert_eq" | "assert" | "debug_assert" | "vec"
        );
        if !known {
            self.engine
                .limit(&self.module, "macro_expansion", path.join("::"));
        }
        if macro_candidate(&path, self) {
            self.record_path(&path, "syntax", false, false);
        }
        self.scan_macro_tokens(mac.tokens.clone());
    }

    fn visit_block(&mut self, block: &'ast syn::Block) {
        let mut frame = Frame::block(self.module.clone());
        for statement in &block.stmts {
            if let Stmt::Item(Item::Use(item)) = statement {
                let (item_test, _) = self.engine.attr_info(&item.attrs, &self.module);
                for (path, alias, glob) in flatten_use(&item.tree, &[]) {
                    let declaration = UseDecl {
                        path,
                        alias,
                        glob,
                        test: self.test || item_test,
                    };
                    let binding = Binding {
                        path: declaration.path,
                        origin: self.frames.len(),
                        test: declaration.test,
                    };
                    if declaration.glob {
                        frame.globs.push(binding);
                    } else if declaration.alias != "_" {
                        frame
                            .bindings
                            .entry(declaration.alias)
                            .or_default()
                            .push(binding);
                    }
                }
            }
        }
        self.frames.push(frame);
        for statement in &block.stmts {
            self.visit_stmt(statement);
        }
        self.frames.pop();
    }

    fn visit_expr_closure(&mut self, closure: &'ast syn::ExprClosure) {
        if let Some(lifetimes) = &closure.lifetimes {
            self.visit_bound_lifetimes(lifetimes);
        }
        self.frames.push(Frame::block(self.module.clone()));
        for input in &closure.inputs {
            self.visit_pat(input);
        }
        self.visit_return_type(&closure.output);
        let mut names = BTreeSet::new();
        for input in &closure.inputs {
            collect_pattern_names(input, &mut names);
        }
        if let Some(frame) = self.frames.last_mut() {
            frame.locals.extend(names);
        }
        self.visit_expr(&closure.body);
        self.frames.pop();
    }

    fn visit_expr_for_loop(&mut self, expression: &'ast syn::ExprForLoop) {
        self.visit_expr(&expression.expr);
        self.visit_pat(&expression.pat);
        let mut frame = Frame::block(self.module.clone());
        collect_pattern_names(&expression.pat, &mut frame.locals);
        self.frames.push(frame);
        self.visit_block(&expression.body);
        self.frames.pop();
    }

    fn visit_expr_if(&mut self, expression: &'ast syn::ExprIf) {
        self.visit_condition_body(&expression.cond, &expression.then_branch);
        if let Some((_, otherwise)) = &expression.else_branch {
            self.visit_expr(otherwise);
        }
    }

    fn visit_expr_while(&mut self, expression: &'ast syn::ExprWhile) {
        self.visit_condition_body(&expression.cond, &expression.body);
    }

    fn visit_expr_match(&mut self, expression: &'ast syn::ExprMatch) {
        self.visit_expr(&expression.expr);
        for arm in &expression.arms {
            self.visit_arm(arm);
        }
    }

    fn visit_arm(&mut self, arm: &'ast syn::Arm) {
        let mut frame = Frame::block(self.module.clone());
        collect_pattern_names(&arm.pat, &mut frame.locals);
        self.frames.push(frame);
        self.visit_pat(&arm.pat);
        self.visit_expr(&arm.body);
        self.frames.pop();
    }

    fn visit_stmt(&mut self, statement: &'ast Stmt) {
        match statement {
            Stmt::Item(item) => {
                if !matches!(item, Item::Use(_))
                    && let Some(name) = item_definition(item)
                    && let Some(frame) = self.frames.last_mut()
                {
                    frame.definitions.insert(name);
                }
                self.walk_item(item);
            }
            Stmt::Local(local) => self.visit_local(local),
            Stmt::Expr(expr, _) => self.visit_expr(expr),
            Stmt::Macro(mac) => self.visit_stmt_macro(mac),
        }
    }

    fn visit_local(&mut self, local: &'ast syn::Local) {
        self.visit_pat(&local.pat);
        if let Some(init) = &local.init {
            self.visit_local_init(init);
        }
        let mut names = BTreeSet::new();
        collect_pattern_names(&local.pat, &mut names);
        if let Some(frame) = self.frames.last_mut() {
            frame.locals.extend(names);
        }
    }

    fn visit_trait_item_fn(&mut self, item: &'ast syn::TraitItemFn) {
        let old_test = self.test;
        self.test = old_test || attrs_cfg_test(&item.attrs);
        self.visit_function(&item.sig, item.default.as_ref());
        self.test = old_test;
    }

    fn visit_impl_item_fn(&mut self, item: &'ast syn::ImplItemFn) {
        let old_test = self.test;
        self.test = old_test || attrs_cfg_test(&item.attrs);
        self.visit_visibility(&item.vis);
        self.visit_function(&item.sig, Some(&item.block));
        self.test = old_test;
    }

    fn visit_expr_method_call(&mut self, item: &'ast syn::ExprMethodCall) {
        if self
            .frames
            .iter()
            .any(|frame| frame.globs.iter().any(|glob| !glob.test || self.test))
        {
            self.engine.limit(
                &self.module,
                "method_type_inference",
                item.method.to_string(),
            );
        }
        syn::visit::visit_expr_method_call(self, item);
    }
}

struct NameCollector<'a> {
    names: &'a mut BTreeSet<String>,
}

impl<'ast> Visit<'ast> for NameCollector<'_> {
    fn visit_pat_ident(&mut self, pat: &'ast syn::PatIdent) {
        self.names.insert(pat.ident.to_string());
        syn::visit::visit_pat_ident(self, pat);
    }

    fn visit_pat_guard(&mut self, pat: &'ast syn::PatGuard) {
        self.visit_pat(&pat.pat);
    }

    fn visit_expr_closure(&mut self, _closure: &'ast syn::ExprClosure) {}
}

fn generic_frame(module: &[String], generics: &syn::Generics) -> Frame {
    let mut frame = Frame::block(module.to_vec());
    for parameter in &generics.params {
        match parameter {
            GenericParam::Type(parameter) => {
                frame.type_locals.insert(parameter.ident.to_string());
            }
            GenericParam::Const(parameter) => {
                frame.locals.insert(parameter.ident.to_string());
            }
            GenericParam::Lifetime(_) => {}
        }
    }
    frame
}

fn collect_pattern_names(pattern: &Pat, names: &mut BTreeSet<String>) {
    let mut collector = NameCollector { names };
    collector.visit_pat(pattern);
}

fn syn_path_names(path: &SynPath) -> Vec<String> {
    path.segments
        .iter()
        .map(|segment| segment.ident.to_string())
        .collect()
}

fn item_cfg_test(item: &Item) -> bool {
    match item {
        Item::Const(item) => attrs_cfg_test(&item.attrs),
        Item::Enum(item) => attrs_cfg_test(&item.attrs),
        Item::ExternCrate(item) => attrs_cfg_test(&item.attrs),
        Item::Fn(item) => attrs_cfg_test(&item.attrs),
        Item::ForeignMod(item) => attrs_cfg_test(&item.attrs),
        Item::Impl(item) => attrs_cfg_test(&item.attrs),
        Item::Macro(item) => attrs_cfg_test(&item.attrs),
        Item::Mod(item) => attrs_cfg_test(&item.attrs),
        Item::Static(item) => attrs_cfg_test(&item.attrs),
        Item::Struct(item) => attrs_cfg_test(&item.attrs),
        Item::Trait(item) => attrs_cfg_test(&item.attrs),
        Item::TraitAlias(item) => attrs_cfg_test(&item.attrs),
        Item::Type(item) => attrs_cfg_test(&item.attrs),
        Item::Union(item) => attrs_cfg_test(&item.attrs),
        Item::Use(item) => attrs_cfg_test(&item.attrs),
        Item::Verbatim(_) => false,
        _ => false,
    }
}

fn attrs_cfg_test(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| {
        attr.path().is_ident("cfg")
            && matches!(&attr.meta, Meta::List(list) if list.tokens.to_string() == "test")
    })
}

fn is_nonmodule_name(name: &str) -> bool {
    matches!(
        name,
        "u8" | "u16"
            | "u32"
            | "u64"
            | "u128"
            | "usize"
            | "i8"
            | "i16"
            | "i32"
            | "i64"
            | "i128"
            | "isize"
            | "f32"
            | "f64"
            | "bool"
            | "char"
            | "str"
            | "Self"
            | "true"
            | "false"
            | "_"
            | "Option"
            | "Result"
            | "Vec"
            | "String"
            | "Box"
            | "Some"
            | "None"
            | "Ok"
            | "Err"
    )
}

fn with_suffix(mut base: Vec<String>, suffix: &[String]) -> Vec<String> {
    base.extend(suffix.iter().cloned());
    base
}

fn append_resolution(resolution: Resolution, suffix: &[String]) -> Resolution {
    match resolution {
        Resolution::Target(mut target) => {
            target.extend(suffix.iter().cloned());
            Resolution::Target(target)
        }
        other => other,
    }
}

fn is_colon(token: &TokenTree) -> bool {
    matches!(token, TokenTree::Punct(punct) if punct.as_char() == ':')
}

fn macro_candidate(path: &[String], walker: &Walker<'_>) -> bool {
    if path.is_empty() {
        return false;
    }
    if matches!(path[0].as_str(), "crate" | "self" | "super")
        || path[0] == walker.engine.crate_name
        || walker.engine.externals.contains(&path[0])
    {
        return true;
    }
    !matches!(walker.resolve(path, false), Resolution::Unknown)
}

trait TokenStreamString {
    fn to_token_stream_string(&self) -> String;
}

impl TokenStreamString for Meta {
    fn to_token_stream_string(&self) -> String {
        self.path()
            .segments
            .iter()
            .map(|segment| segment.ident.to_string())
            .collect::<Vec<_>>()
            .join("::")
    }
}

pub fn extract(
    crate_root: &std::path::Path,
    root_file: &std::path::Path,
    crate_name: &str,
    edition: &str,
    externals: &std::collections::BTreeSet<String>,
) -> Result<Extracted, String> {
    let crate_root = fs::canonicalize(crate_root)
        .map_err(|error| format!("failed to canonicalize {}: {}", crate_root.display(), error))?;
    let root_file = fs::canonicalize(root_file)
        .map_err(|error| format!("failed to canonicalize {}: {}", root_file.display(), error))?;
    if !root_file.starts_with(&crate_root) {
        return Err(format!(
            "root file {} is outside crate root {}",
            root_file.display(),
            crate_root.display()
        ));
    }

    let source = fs::read_to_string(&root_file)
        .map_err(|error| format!("failed to read {}: {}", root_file.display(), error))?;
    let parsed = syn::parse_file(&source)
        .map_err(|error| format!("failed to parse {}: {}", root_file.display(), error))?;

    let normalized_crate = crate_name.replace('-', "_");
    let mut all_externals = externals
        .iter()
        .map(|name| name.replace('-', "_"))
        .collect::<BTreeSet<_>>();
    all_externals.extend(["std", "core", "alloc"].into_iter().map(str::to_owned));

    let mut engine = Engine {
        crate_root,
        crate_name: normalized_crate.clone(),
        edition: edition.to_owned(),
        externals: all_externals,
        modules: BTreeMap::new(),
        module_paths: BTreeMap::new(),
        edges: BTreeSet::new(),
        limitations: BTreeSet::new(),
        active_files: BTreeSet::new(),
    };
    engine.active_files.insert(root_file.clone());
    let (root_test, _) = engine.attr_info(&parsed.attrs, std::slice::from_ref(&normalized_crate));
    engine.install_module(
        vec![normalized_crate.clone()],
        root_file.clone(),
        parsed.items,
        root_test,
        root_file.parent().unwrap_or(Path::new(".")).to_path_buf(),
        root_file.parent().unwrap_or(Path::new(".")).to_path_buf(),
    )?;
    engine.active_files.remove(&root_file);

    let module_paths = engine
        .modules
        .values()
        .map(|module| module.path.clone())
        .collect::<Vec<_>>();
    for module in module_paths {
        let key = module_name(&module);
        let items = engine
            .modules
            .get(&key)
            .map(|data| data.items.clone())
            .unwrap_or_default();
        let test = engine
            .modules
            .get(&key)
            .map(|data| data.test)
            .unwrap_or(false);
        let mut walker = Walker::new(&mut engine, module, test);
        walker.walk_items(&items);
    }

    let mut modules = BTreeMap::new();
    for (key, path) in engine.module_paths {
        modules.insert(key, path);
    }
    Ok(Extracted {
        modules,
        edges: engine.edges.into_iter().collect(),
        limitations: engine.limitations.into_iter().collect(),
    })
}
