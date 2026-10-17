//! [effect-alias] Effect-group aliases: `effects IO = [Console, Fs, Streams]`.
//!
//! An alias is shorthand for its member list anywhere an effect list appears —
//! a fn's effects, a handler's dependency list, an effect's prerequisites, a fn
//! type's effects. It may name other aliases; the expansion closes them
//! transitively (cycles refused). Done as an **expansion before resolution**,
//! the same way prerequisites are [effect-prereq]: each alias entry in every
//! effect list is replaced in place by the effects it stands for, so the
//! checker and both emitters see an ordinary program with no aliases in it and
//! agree by construction [backend-never-wrong]. An expanded effect the module
//! cannot name gets a synthetic `import` (span 0), since the name has to
//! resolve where it is now written.
//!
//! Runs *before* `prereq::expand_prerequisites`, so a prerequisite written with
//! an alias is expanded first, and the prerequisite pass then adds whatever the
//! resulting effects imply. The `effects IO = …` declarations themselves are
//! dropped once every list is rewritten — nothing downstream knows the form.

use std::collections::{HashMap, HashSet};

use salvo_syntax::ast::{
    EffectRef, FnDecl, HandlerDecl, Item, Module, TypeRef,
};
use salvo_syntax::visit_mut::{self, MutVisitor};
use salvo_syntax::{Diagnostic, Span};

use crate::source::{ModulePath, SourceFile};

/// One declared alias, owned so the modules can be rewritten while it is read.
#[derive(Clone)]
struct AliasInfo {
    file: usize,
    members: Vec<TypeRef>,
    exported: bool,
    span: Span,
}

/// Every alias by (file, name), plus module paths and imports, so a name
/// written in one file can be followed to the file that declares it — the
/// same shape `prereq::Index` uses.
struct Index {
    aliases: HashMap<(usize, String), AliasInfo>,
    /// Every (file, effect-name) an effect is declared in — so an expanded
    /// member name can be followed to its real declaring file for the import.
    effects: HashSet<(usize, String)>,
    modules: Vec<ModulePath>,
    is_test: Vec<bool>,
    imports: Vec<Vec<salvo_syntax::ast::ImportDecl>>,
}

impl Index {
    fn file_of(&self, path: &[String], name: &str) -> Option<usize> {
        (0..self.modules.len()).find(|&i| {
            !self.is_test[i]
                && self.modules[i].0.as_slice() == path
                && self.aliases.contains_key(&(i, name.to_string()))
        })
    }

    /// The file an *effect* `name` declared, following the same ladder as
    /// `resolve`: own module, named import, whole-module import, then
    /// `core.*`. Used to import an expanded member where the consumer needs
    /// it. Returns `None` when the name is not an indexed effect anywhere —
    /// resolution then reports it precisely.
    fn effect_file(&self, at: usize, name: &str) -> Option<usize> {
        let has = |i: usize| self.effects.contains(&(i, name.to_string()));
        let own = &self.modules[at].0;
        if let Some(f) = (0..self.modules.len())
            .find(|&i| !self.is_test[i] && self.modules[i].0.as_slice() == own.as_slice() && has(i))
        {
            return Some(f);
        }
        if self.is_test[at] {
            if let Some(tested) = own.split_last().map(|(_, p)| p) {
                if let Some(f) = (0..self.modules.len())
                    .find(|&i| !self.is_test[i] && self.modules[i].0.as_slice() == tested && has(i))
                {
                    return Some(f);
                }
            }
        }
        for imp in &self.imports[at] {
            let Some(last) = imp.path.last() else { continue };
            let matches = imp
                .alias
                .as_ref()
                .map(|a| a.name == name)
                .unwrap_or(last.name == name);
            if matches {
                let prefix: Vec<String> =
                    imp.path[..imp.path.len() - 1].iter().map(|s| s.name.clone()).collect();
                if let Some(f) = (0..self.modules.len()).find(|&i| {
                    !self.is_test[i] && self.modules[i].0.as_slice() == prefix.as_slice() && has(i)
                }) {
                    return Some(f);
                }
            }
            let all_lower = imp
                .path
                .iter()
                .all(|s| !s.name.starts_with(|c: char| c.is_uppercase()));
            if all_lower {
                let path: Vec<String> = imp.path.iter().map(|s| s.name.clone()).collect();
                if let Some(f) = (0..self.modules.len()).find(|&i| {
                    !self.is_test[i] && self.modules[i].0.as_slice() == path.as_slice() && has(i)
                }) {
                    return Some(f);
                }
            }
        }
        (0..self.modules.len())
            .find(|&i| !self.is_test[i] && self.modules[i].0.first().is_some_and(|p| p == "core") && has(i))
    }

    /// The file whose alias `name`, written in file `at`, refers to: own
    /// module, a named import (last segment or alias), a whole-module import,
    /// then `core.*`. Mirrors the resolver's ladder, reduced to what an alias
    /// name needs (an alias is always a plain capitalized name).
    fn resolve(&self, at: usize, name: &str) -> Option<usize> {
        let own = &self.modules[at].0;
        if let Some(f) = self.file_of(own, name) {
            return Some(f);
        }
        // A test annex shares its tested module's declarations.
        if self.is_test[at] {
            if let Some(tested) = own.split_last().map(|(_, p)| p) {
                if let Some(f) = self.file_of(tested, name) {
                    return Some(f);
                }
            }
        }
        for imp in &self.imports[at] {
            let last = imp.path.last()?;
            let matches = imp
                .alias
                .as_ref()
                .map(|a| a.name == name)
                .unwrap_or(last.name == name);
            if matches {
                let prefix: Vec<String> =
                    imp.path[..imp.path.len() - 1].iter().map(|s| s.name.clone()).collect();
                if let Some(f) = self.file_of(&prefix, &last.name) {
                    return Some(f);
                }
            }
            // A whole-module import: `import mod` brings `mod`'s aliases.
            let all_lower = imp
                .path
                .iter()
                .all(|s| !s.name.starts_with(|c: char| c.is_uppercase()));
            if all_lower {
                let path: Vec<String> = imp.path.iter().map(|s| s.name.clone()).collect();
                if let Some(f) = self.file_of(&path, name) {
                    return Some(f);
                }
            }
        }
        // `core.*` is implicitly visible everywhere.
        (0..self.modules.len()).find(|&i| {
            !self.is_test[i]
                && self.modules[i].0.first().is_some_and(|p| p == "core")
                && self.aliases.contains_key(&(i, name.to_string()))
        })
    }

    /// The transitive member list of alias `name` written in `file`: each
    /// member effect, with the file it is declared in so an import can be
    /// synthesized. An alias member expands to its own members; a plain
    /// effect member stays. Returns `Err(chain)` on a cycle.
    fn closure(
        &self,
        file: usize,
        name: &str,
    ) -> Result<Vec<(usize, TypeRef)>, Vec<String>> {
        let mut out: Vec<(usize, TypeRef)> = Vec::new();
        let mut stack: Vec<String> = vec![name.to_string()];
        self.walk(file, name, &mut out, &mut stack)?;
        Ok(out)
    }

    fn walk(
        &self,
        file: usize,
        name: &str,
        out: &mut Vec<(usize, TypeRef)>,
        stack: &mut Vec<String>,
    ) -> Result<(), Vec<String>> {
        let Some(info) = self.aliases.get(&(file, name.to_string())) else {
            return Ok(());
        };
        for member in &info.members {
            let mname = member.name.name.clone();
            // An alias member: recurse into its members instead of emitting it.
            if let Some(target) = self.resolve(info.file, &mname) {
                if self.aliases.contains_key(&(target, mname.clone())) {
                    if stack.contains(&mname) {
                        let mut chain = stack.clone();
                        chain.push(mname);
                        return Err(chain);
                    }
                    stack.push(mname.clone());
                    self.walk(target, &mname, out, stack)?;
                    stack.pop();
                    continue;
                }
            }
            // A plain effect member: emit it, deduped by name. Its declaring
            // file is where the member name resolves (for the import), or the
            // alias's own file when it resolves locally.
            if out.iter().any(|(_, r)| r.name.name == member.name.name) {
                continue;
            }
            let decl_file = self
                .effect_file(info.file, &member.name.name)
                .unwrap_or(info.file);
            out.push((decl_file, member.clone()));
        }
        Ok(())
    }
}

const SYNTHETIC: Span = Span { start: 0, end: 0 };

fn implied_ref(r: &TypeRef) -> TypeRef {
    let mut r = r.clone();
    r.name.span = SYNTHETIC;
    r.span = SYNTHETIC;
    r
}

/// Rewrites one file's effect lists, expanding alias entries.
struct Rewrite<'a> {
    index: &'a Index,
    file: usize,
    imports: Vec<(Vec<String>, String)>,
    diags: Vec<Diagnostic>,
}

impl Rewrite<'_> {
    fn ensure_visible(&mut self, target: usize, name: &str, site: Span, because: &str) -> bool {
        // Already visible from the consuming file — because it resolves to
        // the same declaring file (own module, an existing import, or `core.*`
        // which is implicit everywhere). No synthetic import needed.
        match self.index.effect_file(self.file, name) {
            Some(f) if f == target => return true,
            Some(other) => {
                // A different `{name}` is in scope here than the one the alias
                // meant — report it, as prereq does for its implied effects.
                self.diags.push(Diagnostic::error(
                    format!(
                        "the effect alias expands to `{}.{name}` [effect-alias], but `{name}` here \
                         names `{}.{name}` — rename one of them",
                        self.index.modules[target], self.index.modules[other]
                    ),
                    site,
                ));
                let _ = because;
                return false;
            }
            None => {}
        }
        if target == self.file {
            return true;
        }
        let path = self.index.modules[target].0.clone();
        if !self.imports.iter().any(|(p, n)| *p == path && n == name) {
            self.imports.push((path, name.to_string()));
        }
        let _ = (site, because);
        true
    }

    /// Replaces every alias entry in one effect list with its expansion,
    /// in place, preserving the order of non-alias entries.
    fn expand_list(&mut self, list: &mut Vec<EffectRef>) {
        // Nothing to do unless a list entry names an alias.
        let names_alias = list.iter().any(|e| match e {
            EffectRef::Effect(r) | EffectRef::AnyEffect(r) => {
                self.index.resolve(self.file, &r.name.name).is_some_and(|f| {
                    self.index.aliases.contains_key(&(f, r.name.name.clone()))
                })
            }
            _ => false,
        });
        if !names_alias {
            return;
        }
        let mut out: Vec<EffectRef> = Vec::new();
        let mut present: HashSet<String> = HashSet::new();
        for entry in std::mem::take(list) {
            match &entry {
                EffectRef::Effect(r) => {
                    let name = r.name.name.clone();
                    let alias_file = self
                        .index
                        .resolve(self.file, &name)
                        .filter(|f| self.index.aliases.contains_key(&(*f, name.clone())));
                    if let Some(af) = alias_file {
                        match self.index.closure(af, &name) {
                            Ok(members) => {
                                for (target, r) in members {
                                    if !present.insert(r.name.name.clone()) {
                                        continue;
                                    }
                                    if self.ensure_visible(target, &r.name.name, r.span, &name) {
                                        out.push(EffectRef::Effect(implied_ref(&r)));
                                    }
                                }
                            }
                            Err(chain) => self.diags.push(Diagnostic::error(
                                format!(
                                    "the effect alias `{name}` is cyclic: {} [effect-alias]",
                                    chain.join(" -> ")
                                ),
                                r.span,
                            )),
                        }
                        continue;
                    }
                    if present.insert(name) {
                        out.push(entry);
                    }
                }
                EffectRef::AnyEffect(r) => {
                    // `any Alias` makes no sense — an alias is several effects,
                    // and `any` is about one instance of one effect.
                    let name = r.name.name.clone();
                    let is_alias = self
                        .index
                        .resolve(self.file, &name)
                        .is_some_and(|f| self.index.aliases.contains_key(&(f, name.clone())));
                    if is_alias {
                        self.diags.push(Diagnostic::error(
                            format!(
                                "`any {name}` is not allowed: `{name}` is an effect alias (several \
                                 effects), and `any` is about one instance of one effect — write \
                                 `any` on the individual effect [effect-alias]"
                            ),
                            r.span,
                        ));
                        continue;
                    }
                    if present.insert(name) {
                        out.push(entry);
                    }
                }
                _ => out.push(entry),
            }
        }
        *list = out;
    }

    fn expand_fn(&mut self, f: &mut FnDecl) {
        if let Some(list) = &mut f.effects {
            self.expand_list(list);
        }
        let mut types = FnTypes(self);
        visit_mut::walk_fn(&mut types, f);
    }

    fn expand_handler(&mut self, h: &mut HandlerDecl) {
        if let Some(list) = &mut h.effects {
            self.expand_list(list);
        }
        for f in &mut h.fns {
            let mut types = FnTypes(self);
            visit_mut::walk_fn(&mut types, f);
        }
    }
}

/// Expands alias entries in fn *types* inside a declaration.
struct FnTypes<'r, 'a>(&'r mut Rewrite<'a>);

impl MutVisitor for FnTypes<'_, '_> {
    fn visit_type(&mut self, ty: &mut salvo_syntax::ast::Type) {
        if let salvo_syntax::ast::Type::Fn { effects: Some(list), .. } = ty {
            self.0.expand_list(list);
        }
    }
}

/// [effect-alias] Expands every effect alias in the program, then removes the
/// `effects …` declarations. Returns diagnostics by file index: cyclic
/// aliases, a malformed member list, and `any Alias`.
pub fn expand_effect_aliases(
    files: &[SourceFile],
    modules: &mut [Module],
) -> Vec<(usize, Diagnostic)> {
    let mut out: Vec<(usize, Diagnostic)> = Vec::new();
    let mut aliases = HashMap::new();
    let mut any_alias = false;
    for (file, module) in modules.iter().enumerate() {
        for item in &module.items {
            let Item::EffectAlias(a) = item else { continue };
            let mut members = Vec::new();
            for m in &a.effects {
                match m {
                    EffectRef::Effect(r) => members.push(r.clone()),
                    EffectRef::Use(span) | EffectRef::Spawn(span) => out.push((
                        file,
                        Diagnostic::error(
                            "an effect alias names effects: `use` and `spawn` are per-function \
                             capabilities, not effects a group can stand for [effect-alias]",
                            *span,
                        ),
                    )),
                    EffectRef::AnyEffect(r) => out.push((
                        file,
                        Diagnostic::error(
                            "`any` has no meaning in an effect alias: an alias is a set of \
                             effects, and `any` is about one instance of one effect — write it \
                             where the effect is required [effect-alias]",
                            r.span,
                        ),
                    )),
                }
            }
            any_alias = true;
            aliases.insert(
                (file, a.name.name.clone()),
                AliasInfo { file, members, exported: a.exported, span: a.name.span },
            );
        }
    }
    if !any_alias {
        return out;
    }
    let index = Index {
        aliases,
        effects: modules
            .iter()
            .enumerate()
            .flat_map(|(file, m)| {
                m.items.iter().filter_map(move |i| match i {
                    Item::Effect(e) => Some((file, e.name.name.clone())),
                    _ => None,
                })
            })
            .collect(),
        modules: files.iter().map(|f| f.module.clone()).collect(),
        is_test: files.iter().map(|f| f.is_test).collect(),
        imports: modules
            .iter()
            .map(|m| {
                m.items
                    .iter()
                    .filter_map(|i| match i {
                        Item::Import(d) => Some(d.clone()),
                        _ => None,
                    })
                    .collect()
            })
            .collect(),
    };

    // Declaration checks: an exported alias may not expand to a member a
    // consumer cannot see; a cycle is reported at the first list that hits it.
    let mut keys: Vec<&(usize, String)> = index.aliases.keys().collect();
    keys.sort();
    for key in keys {
        let info = &index.aliases[key];
        if info.exported {
            // Every member an exported alias names must itself be exported or
            // resolvable, else a consumer gets an unresolved effect. Reported
            // softly — resolution will say precisely — so this stays quiet
            // unless the closure cycles.
            if let Err(chain) = index.closure(key.0, &key.1) {
                out.push((
                    key.0,
                    Diagnostic::error(
                        format!(
                            "the effect alias `{}` is cyclic: {} [effect-alias]",
                            key.1,
                            chain.join(" -> ")
                        ),
                        info.span,
                    ),
                ));
            }
        }
    }

    // Rewrite every file's effect lists, then drop the alias declarations and
    // add synthetic imports for expanded members.
    for (file, module) in modules.iter_mut().enumerate() {
        let mut rw = Rewrite { index: &index, file, imports: Vec::new(), diags: Vec::new() };
        for item in &mut module.items {
            match item {
                Item::Fn(f) => rw.expand_fn(f),
                Item::Handler(h) => rw.expand_handler(h),
                Item::Effect(e) => {
                    let mut types = FnTypes(&mut rw);
                    for f in &mut e.fns {
                        visit_mut::walk_fn(&mut types, f);
                    }
                    // An effect's prerequisite list may name an alias too.
                    rw.expand_list(&mut e.prereqs);
                }
                _ => {}
            }
        }
        let imports = std::mem::take(&mut rw.imports);
        for d in std::mem::take(&mut rw.diags) {
            out.push((file, d));
        }
        // Drop the alias declarations — they have done their job.
        module.items.retain(|i| !matches!(i, Item::EffectAlias(_)));
        // Drop imports that name an alias: the alias is gone, and its members
        // were expanded in place (each with its own synthetic import). A
        // leftover `import mod.App` would resolve to nothing [effect-alias].
        let is_alias_import = |d: &salvo_syntax::ast::ImportDecl| -> bool {
            let Some(last) = d.path.last() else { return false };
            let prefix: Vec<String> =
                d.path[..d.path.len() - 1].iter().map(|s| s.name.clone()).collect();
            index.file_of(&prefix, &last.name).is_some()
        };
        module.items.retain(|i| match i {
            Item::Import(d) => !is_alias_import(d),
            _ => true,
        });
        // Synthetic imports for every expanded member the file cannot name.
        for (path, name) in imports {
            let already = module.items.iter().any(|i| match i {
                Item::Import(d) => {
                    d.path.last().is_some_and(|l| l.name == name)
                        && d.path[..d.path.len().saturating_sub(1)]
                            .iter()
                            .map(|s| s.name.clone())
                            .collect::<Vec<_>>()
                            == path
                }
                _ => false,
            });
            if already {
                continue;
            }
            let mut segs: Vec<salvo_syntax::ast::Ident> = path
                .iter()
                .map(|p| salvo_syntax::ast::Ident { name: p.clone(), span: SYNTHETIC })
                .collect();
            segs.push(salvo_syntax::ast::Ident { name: name.clone(), span: SYNTHETIC });
            module.items.insert(
                0,
                Item::Import(salvo_syntax::ast::ImportDecl { path: segs, alias: None, span: SYNTHETIC }),
            );
        }
    }
    out
}
