//! Per-module name resolution.
//!
//! Each source file gets a [`ModuleScope`]: the names visible to code in
//! that file. Visibility rules per LANGUAGE.md [mod-visibility]:
//!
//! * everything declared in the same module (all files of that module,
//!   including the backend define file),
//! * everything in `core.*` (implicitly imported),
//! * everything named by an `import` (with optional `as` alias)
//!   [mod-import].
//!
//! Unresolved and ambiguous imports are reported as errors [mod-import].

use std::collections::HashMap;

use salvo_syntax::ast::{
    BackingMod, EffectDecl, FnDecl, HandlerDecl, ImportDecl, Item, QualifierDecl, StructDecl,
    TypeDecl,
};

use crate::diag::FileDiagnostic;
use crate::program::Program;
use crate::source::ModulePath;

/// Identifies a top-level `fn` declaration: (file index, item index).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FnKey {
    pub file: usize,
    pub item: usize,
}

/// A function together with its identity.
#[derive(Clone, Copy)]
pub struct FnEntry<'p> {
    pub key: FnKey,
    pub decl: &'p FnDecl,
}

/// The names visible to one source file.
#[derive(Default)]
pub struct ModuleScope<'p> {
    pub fns: HashMap<&'p str, Vec<FnEntry<'p>>>,
    pub structs: HashMap<&'p str, &'p StructDecl>,
    pub effects: HashMap<&'p str, &'p EffectDecl>,
    pub handlers: HashMap<&'p str, &'p HandlerDecl>,
    pub qualifiers: HashMap<&'p str, &'p QualifierDecl>,
    pub type_aliases: HashMap<&'p str, &'p TypeDecl>,
    /// `internal type` / `external type` declarations visible here.
    pub opaque_types: HashMap<&'p str, &'p TypeDecl>,
    /// Effect-member fn name -> (owning effect, member decl).
    pub effect_members: HashMap<&'p str, (&'p EffectDecl, &'p FnDecl)>,
    /// Visible name -> modules that declare it (under this name; aliased
    /// imports record the alias). Drives module reachability
    /// [mod-used-only] and generated backend imports.
    pub name_origins: HashMap<&'p str, Vec<&'p ModulePath>>,
}

impl<'p> ModuleScope<'p> {
    /// Whether `name` refers to a declared qualifier.
    pub fn is_qualifier(&self, name: &str) -> bool {
        self.qualifiers.contains_key(name)
    }
}

pub struct Resolution<'p> {
    /// One scope per file, aligned with `program.files`.
    pub scopes: Vec<ModuleScope<'p>>,
    /// Whole-program declaration index: name -> (declaring module,
    /// importable item). For most declarations the item is the name
    /// itself; for effect members it is the owning *effect* (importing
    /// the effect brings its members). Drives import suggestions on
    /// unresolved-name diagnostics [diag-import-suggest].
    pub declared_in: HashMap<&'p str, Vec<(&'p ModulePath, &'p str)>>,
    /// Resolution errors (unresolved/ambiguous imports) [diag-structured].
    pub errors: Vec<FileDiagnostic>,
}

impl Resolution<'_> {
    /// Import paths (`module.Item`) that would bring `name` into scope,
    /// sorted and deduplicated [diag-import-suggest]. Non-`core` modules
    /// only: `core.*` is implicitly visible, so an unknown name is never
    /// fixed by importing it from core.
    pub fn import_candidates(&self, name: &str) -> Vec<String> {
        let mut paths: Vec<String> = self
            .declared_in
            .get(name)
            .map(|entries| {
                entries
                    .iter()
                    .filter(|(module, _)| module.0.first().is_none_or(|p| p != "core"))
                    .map(|(module, item)| format!("{module}.{item}"))
                    .collect()
            })
            .unwrap_or_default();
        paths.sort();
        paths.dedup();
        paths
    }
}

/// One module's own declarations, prior to visibility merging.
#[derive(Default)]
struct ModuleItems<'p> {
    fns: Vec<(FnKey, &'p FnDecl)>,
    structs: Vec<&'p StructDecl>,
    effects: Vec<&'p EffectDecl>,
    handlers: Vec<&'p HandlerDecl>,
    qualifiers: Vec<&'p QualifierDecl>,
    type_aliases: Vec<&'p TypeDecl>,
    opaque_types: Vec<&'p TypeDecl>,
}

impl<'p> ModuleItems<'p> {
    fn has_name(&self, name: &str) -> bool {
        self.fns.iter().any(|(_, f)| f.name.name == name)
            || self.structs.iter().any(|s| s.name.name == name)
            || self.effects.iter().any(|e| e.name.name == name)
            || self.handlers.iter().any(|h| h.name.name == name)
            || self.qualifiers.iter().any(|q| q.name.name == name)
            || self.type_aliases.iter().any(|t| t.name.name == name)
            || self.opaque_types.iter().any(|t| t.name.name == name)
    }
}

pub fn resolve(program: &Program) -> Resolution<'_> {
    // Pass 1: collect each module's own declarations (all files of the
    // module contribute, including backend define files which may declare
    // `external fn` signatures).
    let mut by_module: HashMap<&ModulePath, ModuleItems<'_>> = HashMap::new();
    for (file_idx, (file, ast)) in program.files.iter().zip(&program.modules).enumerate() {
        let items = by_module.entry(&file.module).or_default();
        for (item_idx, item) in ast.items.iter().enumerate() {
            match item {
                Item::Fn(f) => items.fns.push((
                    FnKey {
                        file: file_idx,
                        item: item_idx,
                    },
                    f,
                )),
                Item::Struct(s) => items.structs.push(s),
                Item::Effect(e) => items.effects.push(e),
                Item::Handler(h) => items.handlers.push(h),
                Item::Qualifier(q) => items.qualifiers.push(q),
                Item::Type(t) => match (t.backing, &t.alias) {
                    (None, Some(_)) => items.type_aliases.push(t),
                    (Some(BackingMod::Internal) | Some(BackingMod::External), _)
                    | (None, None) => items.opaque_types.push(t),
                },
                _ => {}
            }
        }
    }

    let core_modules: Vec<&ModulePath> = by_module
        .keys()
        .filter(|m| m.0.first().is_some_and(|p| p == "core"))
        .copied()
        .collect();

    // Whole-program declaration index for import suggestions
    // [diag-import-suggest]. Effect members map to their owning effect:
    // importing the effect is what brings the member into scope.
    let mut declared_in: HashMap<&str, Vec<(&ModulePath, &str)>> = HashMap::new();
    for (module, items) in &by_module {
        let mut record = |name, item| {
            declared_in.entry(name).or_default().push((*module, item));
        };
        for (_, f) in &items.fns {
            record(&f.name.name, &f.name.name);
        }
        for s in &items.structs {
            record(&s.name.name, &s.name.name);
        }
        for e in &items.effects {
            record(&e.name.name, &e.name.name);
            for f in &e.fns {
                record(&f.name.name, &e.name.name);
            }
        }
        for h in &items.handlers {
            record(&h.name.name, &h.name.name);
        }
        for q in &items.qualifiers {
            record(&q.name.name, &q.name.name);
        }
        for t in &items.type_aliases {
            record(&t.name.name, &t.name.name);
        }
        for t in &items.opaque_types {
            record(&t.name.name, &t.name.name);
        }
    }

    // Pass 2: build one scope per file.
    let mut scopes = Vec::with_capacity(program.files.len());
    let mut errors = Vec::new();
    for (file_idx, (file, ast)) in program.files.iter().zip(&program.modules).enumerate() {
        let mut scope = ModuleScope::default();
        // core.* is implicitly visible everywhere.
        for m in &core_modules {
            if let Some(items) = by_module.get(*m) {
                add_items(&mut scope, items, m, None);
            }
        }
        // The file's own module (overrides core on collision).
        if let Some((own, items)) = by_module.get_key_value(&file.module) {
            add_items(&mut scope, items, own, None);
        }
        // Explicit imports.
        for item in &ast.items {
            let Item::Import(import) = item else { continue };
            resolve_import(&mut scope, &by_module, import, file_idx, &mut errors);
        }
        scopes.push(scope);
    }

    Resolution {
        scopes,
        declared_in,
        errors,
    }
}

/// Adds a module's items to a scope, optionally under a single-name filter
/// with an alias (for imports). Every inserted name records `module` as an
/// origin (under its visible name) for reachability [mod-used-only].
fn add_items<'p>(
    scope: &mut ModuleScope<'p>,
    items: &ModuleItems<'p>,
    module: &'p ModulePath,
    filter: Option<(&str, &'p str)>,
) {
    let want = |name: &str| filter.is_none_or(|(n, _)| n == name);
    let visible_as = |name: &'p str| filter.map_or(name, |(_, alias)| alias);
    let origin = |name: &'p str, scope: &mut ModuleScope<'p>| {
        let origins = scope.name_origins.entry(name).or_default();
        if !origins.contains(&module) {
            origins.push(module);
        }
    };
    for (key, f) in &items.fns {
        if want(&f.name.name) {
            let name = visible_as(&f.name.name);
            scope
                .fns
                .entry(name)
                .or_default()
                .push(FnEntry { key: *key, decl: f });
            origin(name, scope);
        }
    }
    for s in &items.structs {
        if want(&s.name.name) {
            let name = visible_as(&s.name.name);
            scope.structs.insert(name, s);
            origin(name, scope);
        }
    }
    for e in &items.effects {
        if want(&e.name.name) {
            let name = visible_as(&e.name.name);
            scope.effects.insert(name, e);
            origin(name, scope);
        }
        // Effect members become callable wherever the effect is visible.
        for f in &e.fns {
            scope.effect_members.insert(&f.name.name, (e, f));
            origin(&f.name.name, scope);
        }
    }
    for h in &items.handlers {
        if want(&h.name.name) {
            let name = visible_as(&h.name.name);
            scope.handlers.insert(name, h);
            origin(name, scope);
        }
    }
    for q in &items.qualifiers {
        if want(&q.name.name) {
            let name = visible_as(&q.name.name);
            scope.qualifiers.insert(name, q);
            origin(name, scope);
        }
    }
    for t in &items.type_aliases {
        if want(&t.name.name) {
            let name = visible_as(&t.name.name);
            scope.type_aliases.insert(name, t);
            origin(name, scope);
        }
    }
    for t in &items.opaque_types {
        if want(&t.name.name) {
            let name = visible_as(&t.name.name);
            scope.opaque_types.insert(name, t);
            origin(name, scope);
        }
    }
}

fn resolve_import<'p>(
    scope: &mut ModuleScope<'p>,
    by_module: &HashMap<&'p ModulePath, ModuleItems<'p>>,
    import: &'p ImportDecl,
    file_idx: usize,
    errors: &mut Vec<FileDiagnostic>,
) {
    if import.path.len() < 2 {
        errors.push(FileDiagnostic::error(
            file_idx,
            import.span,
            "import path must be `module.item`",
        ));
        return;
    }
    let item_name = &import.path.last().unwrap().name;
    let prefix: Vec<&str> = import.path[..import.path.len() - 1]
        .iter()
        .map(|i| i.name.as_str())
        .collect();

    // Candidate modules: path equals the prefix, or the prefix is a leading
    // path of the module (`import core.Str` finds `core.string`).
    let mut matches: Vec<&&ModulePath> = Vec::new();
    for (path, items) in by_module {
        let exact = path.0.len() == prefix.len()
            && path.0.iter().zip(&prefix).all(|(a, b)| a == b);
        let prefixed = path.0.len() > prefix.len()
            && path.0.iter().zip(&prefix).all(|(a, b)| a == b);
        if (exact || prefixed) && items.has_name(item_name) {
            matches.push(by_module.get_key_value(path).unwrap().0);
        }
    }
    match matches.len() {
        0 => {
            // Suggest modules that do declare the item, wherever they
            // live [diag-import-suggest].
            let mut suggestions: Vec<String> = by_module
                .iter()
                .filter(|(_, items)| items.has_name(item_name))
                .map(|(path, _)| format!("{path}.{item_name}"))
                .collect();
            suggestions.sort();
            suggestions.dedup();
            errors.push(
                FileDiagnostic::error(
                    file_idx,
                    import.span,
                    format!(
                        "unresolved import: no module matching `{}` declares `{item_name}`",
                        prefix.join(".")
                    ),
                )
                .with_imports(suggestions),
            );
        }
        1 => {
            let alias: &'p str = import
                .alias
                .as_ref()
                .map(|a| a.name.as_str())
                .unwrap_or(item_name);
            let module = *matches[0];
            let items = &by_module[module];
            add_items(scope, items, module, Some((item_name, alias)));
        }
        _ => {
            let mut names: Vec<String> = matches.iter().map(|m| m.to_string()).collect();
            names.sort();
            errors.push(FileDiagnostic::error(
                file_idx,
                import.span,
                format!(
                    "ambiguous import `{item_name}`: found in modules {}",
                    names.join(", ")
                ),
            ));
        }
    }
}
