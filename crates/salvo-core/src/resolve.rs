//! Per-module name resolution.
//!
//! Each source file gets a [`ModuleScope`]: the names visible to code in
//! that file. Visibility rules per LANGUAGE.md:
//!
//! * everything declared in the same module (all files of that module,
//!   including the backend define file),
//! * everything in `core.*` (implicitly imported),
//! * everything named by an `import` (with optional `as` alias).
//!
//! Unresolved and ambiguous imports are reported as errors.

use std::collections::HashMap;

use salvo_syntax::ast::{
    BackingMod, EffectDecl, FnDecl, HandlerDecl, ImportDecl, Item, QualifierDecl, StructDecl,
    TypeDecl,
};
use salvo_syntax::diag::Diagnostic;

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
    /// Rendered resolution errors (unresolved/ambiguous imports).
    pub errors: Vec<String>,
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

    // Pass 2: build one scope per file.
    let mut scopes = Vec::with_capacity(program.files.len());
    let mut errors = Vec::new();
    for (file, ast) in program.files.iter().zip(&program.modules) {
        let mut scope = ModuleScope::default();
        // core.* is implicitly visible everywhere.
        for m in &core_modules {
            if let Some(items) = by_module.get(*m) {
                add_items(&mut scope, items, None);
            }
        }
        // The file's own module (overrides core on collision).
        if let Some(items) = by_module.get(&file.module) {
            add_items(&mut scope, items, None);
        }
        // Explicit imports.
        for item in &ast.items {
            let Item::Import(import) = item else { continue };
            resolve_import(&mut scope, &by_module, import, file, &mut errors, program);
        }
        scopes.push(scope);
    }

    Resolution { scopes, errors }
}

/// Adds a module's items to a scope, optionally under a single-name filter
/// with an alias (for imports).
fn add_items<'p>(
    scope: &mut ModuleScope<'p>,
    items: &ModuleItems<'p>,
    filter: Option<(&str, &'p str)>,
) {
    let want = |name: &str| filter.is_none_or(|(n, _)| n == name);
    let visible_as = |name: &'p str| filter.map_or(name, |(_, alias)| alias);
    for (key, f) in &items.fns {
        if want(&f.name.name) {
            scope
                .fns
                .entry(visible_as(&f.name.name))
                .or_default()
                .push(FnEntry { key: *key, decl: f });
        }
    }
    for s in &items.structs {
        if want(&s.name.name) {
            scope.structs.insert(visible_as(&s.name.name), s);
        }
    }
    for e in &items.effects {
        if want(&e.name.name) {
            scope.effects.insert(visible_as(&e.name.name), e);
        }
        // Effect members become callable wherever the effect is visible.
        for f in &e.fns {
            scope.effect_members.insert(&f.name.name, (e, f));
        }
    }
    for h in &items.handlers {
        if want(&h.name.name) {
            scope.handlers.insert(visible_as(&h.name.name), h);
        }
    }
    for q in &items.qualifiers {
        if want(&q.name.name) {
            scope.qualifiers.insert(visible_as(&q.name.name), q);
        }
    }
    for t in &items.type_aliases {
        if want(&t.name.name) {
            scope.type_aliases.insert(visible_as(&t.name.name), t);
        }
    }
    for t in &items.opaque_types {
        if want(&t.name.name) {
            scope.opaque_types.insert(visible_as(&t.name.name), t);
        }
    }
}

fn resolve_import<'p>(
    scope: &mut ModuleScope<'p>,
    by_module: &HashMap<&'p ModulePath, ModuleItems<'p>>,
    import: &'p ImportDecl,
    file: &crate::source::SourceFile,
    errors: &mut Vec<String>,
    program: &'p Program,
) {
    if import.path.len() < 2 {
        errors.push(render_error(
            program,
            file,
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
        0 => errors.push(render_error(
            program,
            file,
            import.span,
            format!(
                "unresolved import: no module matching `{}` declares `{item_name}`",
                prefix.join(".")
            ),
        )),
        1 => {
            let alias: &'p str = import
                .alias
                .as_ref()
                .map(|a| a.name.as_str())
                .unwrap_or(item_name);
            let items = &by_module[*matches[0]];
            add_items(scope, items, Some((item_name, alias)));
        }
        _ => {
            let mut names: Vec<String> = matches.iter().map(|m| m.to_string()).collect();
            names.sort();
            errors.push(render_error(
                program,
                file,
                import.span,
                format!(
                    "ambiguous import `{item_name}`: found in modules {}",
                    names.join(", ")
                ),
            ));
        }
    }
}

fn render_error(
    program: &Program,
    file: &crate::source::SourceFile,
    span: salvo_syntax::Span,
    msg: impl Into<String>,
) -> String {
    let _ = program;
    Diagnostic::error(msg, span).render(&file.name, &file.content)
}
