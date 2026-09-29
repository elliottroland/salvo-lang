//! [effect-prereq] Effect prerequisites: `effect Fs [Streams] { … }`.
//!
//! Wherever `Fs` is, `Streams` is too (user decision 2026-09-29, R2 — a
//! *prerequisite*, not inheritance: inheritance would give every handler of
//! `Fs` its own `Streams`, and the point of the stream layering is one stream
//! table every producer mints into). Concretely:
//!
//! * an **effect list** naming `Fs` — a fn's, or a fn *type*'s — implies
//!   `Streams`, transitively, so a function declaring `[Fs]` may call stream
//!   members;
//! * every **handler of `Fs`** reaches `Streams` as an implicit dependency,
//!   supplied from the enclosing scope like any handler dependency — so a `use`
//!   of it needs a `Streams` already bound, and says so the way a dependent
//!   handler does;
//! * cycles are refused, and so is a prerequisite on (or of) an `actor effect`
//!   — a mailbox protocol with a prerequisite raises questions nothing needs
//!   yet.
//!
//! Done as an **expansion before resolution**: the implied entries are written
//! into the AST — each fn's effect list, each handler's dependency list — so
//! the checker and both emitters see an ordinary program and agree by
//! construction [backend-never-wrong]. An implied effect the module cannot
//! name gets a synthetic `import` of it (span 0, never hovered or reported),
//! since the name has to resolve where it is now written.

use std::collections::{HashMap, HashSet};

use salvo_syntax::ast::{
    EffectRef, FnDecl, HandlerDecl, Ident, ImportDecl, Item, Module, Type, TypeRef,
};
use salvo_syntax::visit_mut::{self, MutVisitor};
use salvo_syntax::{Diagnostic, Span};

use crate::source::{ModulePath, SourceFile};

/// One declared effect, as the expansion needs it: owned, so the modules can
/// be rewritten while it is read.
#[derive(Clone)]
struct EffectInfo {
    /// Index of the declaring file.
    file: usize,
    generics: Vec<String>,
    prereqs: Vec<TypeRef>,
    is_actor: bool,
    exported: bool,
    span: Span,
}

/// Every effect by (file, name), plus the module paths, so a name written in
/// one file can be followed to the file that declares it.
struct Index {
    effects: HashMap<(usize, String), EffectInfo>,
    modules: Vec<ModulePath>,
    is_test: Vec<bool>,
    imports: Vec<Vec<ImportDecl>>,
}

impl Index {
    /// The file declaring effect `name` of module `path` (a module may be
    /// split over a file and its test annex; only the non-annex one declares).
    fn file_of(&self, path: &[String], name: &str) -> Option<usize> {
        (0..self.modules.len()).find(|&i| {
            !self.is_test[i]
                && self.modules[i].0.as_slice() == path
                && self.effects.contains_key(&(i, name.to_string()))
        })
    }

    /// The file whose effect `name`, written in file `at`, refers to — the
    /// same ladder the resolver uses, reduced to what an effect name needs:
    /// own module (and, for a test annex, the module it tests), a named import
    /// (by last segment or alias), a whole-module import, then `core.*`.
    fn resolve(&self, at: usize, name: &str) -> Option<usize> {
        let own = &self.modules[at].0;
        if let Some(f) = self.file_of(own, name) {
            return Some(f);
        }
        if self.is_test[at] {
            let tested = &own[..own.len().saturating_sub(1)];
            if let Some(f) = self.file_of(tested, name) {
                return Some(f);
            }
        }
        for import in &self.imports[at] {
            let segs: Vec<String> = import.path.iter().map(|p| p.name.clone()).collect();
            let Some((last, module)) = segs.split_last() else { continue };
            let local = import.alias.as_ref().map(|a| a.name.as_str()).unwrap_or(last);
            if local == name {
                if let Some(f) = self.file_of(module, last) {
                    return Some(f);
                }
            }
        }
        for import in &self.imports[at] {
            if import.alias.is_some() {
                continue;
            }
            let segs: Vec<String> = import.path.iter().map(|p| p.name.clone()).collect();
            if let Some(f) = self.file_of(&segs, name) {
                return Some(f);
            }
        }
        (0..self.modules.len()).find(|&i| {
            self.modules[i].0.first().map(String::as_str) == Some("core")
                && !self.is_test[i]
                && self.effects.contains_key(&(i, name.to_string()))
        })
    }

    /// Every effect `(file, name)` implies, transitively, with its type
    /// arguments substituted: the prerequisites of `name` as written at a site
    /// whose arguments are `args`. `Err` carries the chain of a cycle.
    fn closure(
        &self,
        file: usize,
        name: &str,
        args: &[Type],
    ) -> Result<Vec<(usize, TypeRef)>, Vec<String>> {
        let mut out: Vec<(usize, TypeRef)> = Vec::new();
        let mut stack: Vec<String> = vec![name.to_string()];
        self.walk(file, name, args, &mut out, &mut stack)?;
        Ok(out)
    }

    fn walk(
        &self,
        file: usize,
        name: &str,
        args: &[Type],
        out: &mut Vec<(usize, TypeRef)>,
        stack: &mut Vec<String>,
    ) -> Result<(), Vec<String>> {
        let Some(info) = self.effects.get(&(file, name.to_string())) else {
            return Ok(());
        };
        let subst: HashMap<&str, &Type> = info
            .generics
            .iter()
            .map(String::as_str)
            .zip(args.iter())
            .collect();
        for pre in &info.prereqs {
            let Some(target) = self.resolve(info.file, &pre.name.name) else {
                continue; // reported at the declaration
            };
            let mut pre = pre.clone();
            substitute(&mut pre, &subst);
            if stack.contains(&pre.name.name) {
                let mut chain = stack.clone();
                chain.push(pre.name.name.clone());
                return Err(chain);
            }
            if out.iter().any(|(f, r)| *f == target && r.name.name == pre.name.name) {
                continue;
            }
            out.push((target, pre.clone()));
            stack.push(pre.name.name.clone());
            let args = pre.args.clone();
            self.walk(target, &pre.name.name, &args, out, stack)?;
            stack.pop();
        }
        Ok(())
    }
}

/// Replaces an effect's generic parameters in a prerequisite by the site's
/// arguments (`effect Log<T> [Sink<T>]` at `Log<Str>` implies `Sink<Str>`).
fn substitute(r: &mut TypeRef, subst: &HashMap<&str, &Type>) {
    for arg in &mut r.args {
        substitute_type(arg, subst);
    }
}

fn substitute_type(t: &mut Type, subst: &HashMap<&str, &Type>) {
    match t {
        Type::Named { base, qualifiers } => {
            if qualifiers.is_empty() && base.args.is_empty() {
                if let Some(rep) = subst.get(base.name.name.as_str()) {
                    *t = (*rep).clone();
                    return;
                }
            }
            substitute(base, subst);
        }
        Type::Union { arms, .. } => arms.iter_mut().for_each(|a| substitute_type(a, subst)),
        Type::Tuple { elems, .. } => elems.iter_mut().for_each(|a| substitute_type(a, subst)),
        Type::Array { elem, .. } | Type::Nullable { inner: elem, .. } => substitute_type(elem, subst),
        _ => {}
    }
}

/// The synthetic span an injected entry carries: empty, at the start of the
/// file, so it never overlaps anything a cursor can sit on.
const SYNTHETIC: Span = Span { start: 0, end: 0 };

fn implied_ref(r: &TypeRef) -> TypeRef {
    let mut r = r.clone();
    r.name.span = SYNTHETIC;
    r.span = SYNTHETIC;
    r
}

/// What rewriting one file needs to know and record.
struct Rewrite<'a> {
    index: &'a Index,
    file: usize,
    /// Synthetic imports to add, as `(module path, name)`.
    imports: Vec<(Vec<String>, String)>,
    diags: Vec<Diagnostic>,
}

impl Rewrite<'_> {
    /// Makes `name` (declared in `target`) nameable in this file, or reports
    /// that the name already means something else here. `false` if it cannot
    /// be written.
    fn ensure_visible(&mut self, target: usize, name: &str, site: Span, because: &str) -> bool {
        match self.index.resolve(self.file, name) {
            Some(f) if f == target => true,
            Some(other) => {
                self.diags.push(Diagnostic::error(
                    format!(
                        "`{because}` needs effect `{}.{name}` in scope [effect-prereq], but `{name}` \
                         here names `{}.{name}` — rename one of them",
                        self.index.modules[target], self.index.modules[other]
                    ),
                    site,
                ));
                false
            }
            None => {
                let path = self.index.modules[target].0.clone();
                if !self.imports.iter().any(|(p, n)| *p == path && n == name) {
                    self.imports.push((path, name.to_string()));
                }
                true
            }
        }
    }

    /// Appends every implied effect to one effect list.
    fn expand_list(&mut self, list: &mut Vec<EffectRef>) {
        let mut added: Vec<EffectRef> = Vec::new();
        let named: Vec<(String, Vec<Type>, Span)> = list
            .iter()
            .filter_map(|e| match e {
                EffectRef::Effect(r) | EffectRef::AnyEffect(r) => {
                    Some((r.name.name.clone(), r.args.clone(), r.span))
                }
                _ => None,
            })
            .collect();
        let mut present: HashSet<String> = named.iter().map(|(n, _, _)| n.clone()).collect();
        for (name, args, span) in named {
            let Some(file) = self.index.resolve(self.file, &name) else { continue };
            let Ok(implied) = self.index.closure(file, &name, &args) else {
                continue; // reported at the declaration
            };
            for (target, r) in implied {
                if !present.insert(r.name.name.clone()) {
                    continue;
                }
                if self.ensure_visible(target, &r.name.name, span, &name) {
                    added.push(EffectRef::Effect(implied_ref(&r)));
                }
            }
        }
        list.extend(added);
    }

    fn expand_fn(&mut self, f: &mut FnDecl) {
        if let Some(list) = &mut f.effects {
            self.expand_list(list);
        }
        let mut types = FnTypes(self);
        visit_mut::walk_fn(&mut types, f);
    }

    /// A handler of an effect with prerequisites reaches them as
    /// dependencies. Not a platform or intrinsic handler: the host constructs
    /// those and has nothing to pass them; the prerequisite still holds in
    /// every effect list naming the effect.
    fn expand_handler(&mut self, h: &mut HandlerDecl) {
        if h.platform || h.intrinsic {
            return;
        }
        let faces: Vec<(String, Vec<Type>, Span)> = h
            .of
            .iter()
            .filter_map(|t| match t {
                Type::Named { base, .. } => Some((base.name.name.clone(), base.args.clone(), base.span)),
                _ => None,
            })
            .collect();
        let face_names: HashSet<String> = faces.iter().map(|(n, _, _)| n.clone()).collect();
        let mut present: HashSet<String> = h
            .effects
            .iter()
            .flatten()
            .filter_map(|e| match e {
                EffectRef::Effect(r) | EffectRef::AnyEffect(r) => Some(r.name.name.clone()),
                _ => None,
            })
            .collect();
        let mut added = Vec::new();
        for (name, args, span) in faces {
            let Some(file) = self.index.resolve(self.file, &name) else { continue };
            let Ok(implied) = self.index.closure(file, &name, &args) else { continue };
            for (target, r) in implied {
                // A handler that implements the prerequisite itself (`of Fs,
                // Streams`) supplies it; nothing to depend on.
                if face_names.contains(&r.name.name) || !present.insert(r.name.name.clone()) {
                    continue;
                }
                if self.ensure_visible(target, &r.name.name, span, &name) {
                    added.push(EffectRef::Effect(implied_ref(&r)));
                }
            }
        }
        if !added.is_empty() {
            h.effects.get_or_insert_with(Vec::new).extend(added);
        }
        for f in &mut h.fns {
            let mut types = FnTypes(self);
            visit_mut::walk_fn(&mut types, f);
        }
    }
}

/// Expands the effect lists of fn *types* inside a declaration.
struct FnTypes<'r, 'a>(&'r mut Rewrite<'a>);

impl MutVisitor for FnTypes<'_, '_> {
    fn visit_type(&mut self, ty: &mut Type) {
        if let Type::Fn { effects: Some(list), .. } = ty {
            self.0.expand_list(list);
        }
    }
}

/// [effect-prereq] Writes every implied effect into the program — see the
/// module docs. Returns diagnostics by file index: malformed prerequisite
/// lists, cycles, and a name clash that keeps an implied effect unwritable.
pub fn expand_prerequisites(
    files: &[SourceFile],
    modules: &mut [Module],
) -> Vec<(usize, Diagnostic)> {
    let mut out: Vec<(usize, Diagnostic)> = Vec::new();
    let mut effects = HashMap::new();
    let mut any_prereq = false;
    for (file, module) in modules.iter().enumerate() {
        for item in &module.items {
            let Item::Effect(e) = item else { continue };
            let mut prereqs = Vec::new();
            for p in &e.prereqs {
                match p {
                    EffectRef::Effect(r) => prereqs.push(r.clone()),
                    EffectRef::Use(span) | EffectRef::Spawn(span) => out.push((
                        file,
                        Diagnostic::error(
                            "a prerequisite names an effect: `use` and `spawn` are capabilities \
                             of a function, not something an effect can require [effect-prereq]",
                            *span,
                        ),
                    )),
                    EffectRef::AnyEffect(r) => out.push((
                        file,
                        Diagnostic::error(
                            "`any` has no meaning in a prerequisite: the prerequisite is the one \
                             instance bound around every use of the effect [effect-prereq]",
                            r.span,
                        ),
                    )),
                }
            }
            any_prereq |= !prereqs.is_empty();
            effects.insert(
                (file, e.name.name.clone()),
                EffectInfo {
                    file,
                    generics: e.generics.iter().map(|g| g.name.clone()).collect(),
                    prereqs,
                    is_actor: e.is_actor,
                    exported: e.exported,
                    span: e.name.span,
                },
            );
        }
    }
    if !any_prereq {
        return out;
    }
    let index = Index {
        effects,
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

    // Declarations: every prerequisite resolves, names a plain effect, is
    // visible to whoever may name the effect, and closes without a cycle.
    let mut keys: Vec<&(usize, String)> = index.effects.keys().collect();
    keys.sort();
    for key in keys {
        let info = &index.effects[key];
        if info.prereqs.is_empty() {
            continue;
        }
        let (file, name) = key;
        if info.is_actor {
            out.push((
                *file,
                Diagnostic::error(
                    format!(
                        "`actor effect {name}` cannot have prerequisites: a mailbox protocol that \
                         needs another effect in scope is not supported yet [effect-prereq]"
                    ),
                    info.span,
                ),
            ));
            continue;
        }
        for pre in &info.prereqs {
            let Some(target) = index.resolve(*file, &pre.name.name) else {
                out.push((
                    *file,
                    Diagnostic::error(
                        format!(
                            "unknown effect `{}` in the prerequisites of `{name}` [effect-prereq]",
                            pre.name.name
                        ),
                        pre.span,
                    ),
                ));
                continue;
            };
            let target_info = &index.effects[&(target, pre.name.name.clone())];
            if target_info.is_actor {
                out.push((
                    *file,
                    Diagnostic::error(
                        format!(
                            "`{}` is an actor effect, which cannot be a prerequisite yet \
                             [effect-prereq]",
                            pre.name.name
                        ),
                        pre.span,
                    ),
                ));
            }
            if info.exported && !target_info.exported {
                out.push((
                    *file,
                    Diagnostic::error(
                        format!(
                            "`{name}` is exported but its prerequisite `{}` is not: every module \
                             that names `{name}` needs `{}` too — export it [effect-prereq]",
                            pre.name.name, pre.name.name
                        ),
                        pre.span,
                    ),
                ));
            }
        }
        let generics: Vec<Type> = Vec::new();
        if let Err(chain) = index.closure(*file, name, &generics) {
            out.push((
                *file,
                Diagnostic::error(
                    format!(
                        "effect prerequisites form a cycle: {} [effect-prereq]",
                        chain.iter().map(|n| format!("`{n}`")).collect::<Vec<_>>().join(" → ")
                    ),
                    info.span,
                ),
            ));
        }
    }

    // Sites: every file's fns, handlers and fn types.
    for (file, module) in modules.iter_mut().enumerate() {
        let mut rw = Rewrite {
            index: &index,
            file,
            imports: Vec::new(),
            diags: Vec::new(),
        };
        for item in &mut module.items {
            match item {
                Item::Fn(f) => rw.expand_fn(f),
                Item::Handler(h) => rw.expand_handler(h),
                Item::Struct(s) => {
                    for field in &mut s.fields {
                        let mut types = FnTypes(&mut rw);
                        visit_mut::walk_field_decl(&mut types, field);
                    }
                }
                _ => {}
            }
        }
        let Rewrite { imports, diags, .. } = rw;
        for (path, name) in imports {
            let mut segs: Vec<Ident> = path
                .into_iter()
                .map(|s| Ident { name: s, span: SYNTHETIC })
                .collect();
            segs.push(Ident { name, span: SYNTHETIC });
            module.items.insert(
                0,
                Item::Import(ImportDecl {
                    path: segs,
                    alias: None,
                    span: SYNTHETIC,
                }),
            );
        }
        out.extend(diags.into_iter().map(|d| (file, d)));
    }
    out
}
