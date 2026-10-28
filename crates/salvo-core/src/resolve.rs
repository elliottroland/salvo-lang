//! Per-module name resolution.
//!
//! Each source file gets a [`ModuleScope`]: the names visible to code in
//! that file. Visibility rules per docs/language/ [mod-visibility]:
//!
//! * everything declared in the same module,
//! * everything in `std.core.*` (implicitly imported),
//! * everything named by an `import` (with optional `as` alias)
//!   [mod-import].
//!
//! Unresolved and ambiguous imports are reported as errors [mod-import].

use std::collections::HashMap;

use salvo_syntax::ast::{
    Obligation, FnsDecl, EffectAliasDecl,
    EffectDecl, FnDecl, HandlerDecl, ImportDecl, Item, QualifierDecl, RefnDecl, RenameDecl,
    StructDecl, TypeDecl,
};

use crate::decl_id::DeclId;
use crate::diag::FileDiagnostic;
use crate::program::Program;
use crate::source::ModulePath;
use salvo_syntax::Span;

/// A function together with its identity.
#[derive(Clone, Copy)]
pub struct FnEntry<'p> {
    pub key: DeclId,
    /// The file it is declared in.
    pub file: usize,
    pub decl: &'p FnDecl,
    /// [fn-overload-scope] Which rung of the visibility ladder brought this
    /// overload into scope. Overload selection prefers the *most specific*
    /// rung that has a candidate matching the arguments, so a module's own
    /// `map` wins over `std.core`'s without either being an error.
    pub rung: Rung,
    /// The module that declares it — what `f@core.list(...)` names
    /// [fn-overload-at], and what the scope-override warning points at.
    pub module: &'p ModulePath,
}

/// [fn-overload-scope] The visibility ladder, least specific first (user
/// decision 2026-09-07). The rungs above `Own` are not overload sets and so
/// are not listed here: a fn-typed local, parameter or implicit shadows a
/// name outright (it *is* the function the caller chose), an effect member
/// takes the name before any fn does, and a `rename` introduces a fresh
/// name [fn-rename].
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Rung {
    /// [comptime-hygiene] Visible to a stamped body only because the
    /// template's author could see it: the last resort, so a name the
    /// stamping site resolves itself always wins.
    Template,
    /// Implicitly visible `std.core.*`.
    Core,
    /// Swept in by a whole-module `import std.time` [mod-import-module]: less
    /// specific than naming the item, because nothing in the import
    /// mentions this declaration.
    ModuleImport,
    /// Named by an `import` in this file.
    Import,
    /// Declared by this file's own module.
    Own,
}

impl Rung {
    /// How the rung reads in a diagnostic.
    pub fn describe(self) -> &'static str {
        match self {
            Rung::Template => "the module the stamped code was written in",
            Rung::Core => "the standard library",
            Rung::ModuleImport => "a whole-module import in this file",
            Rung::Import => "an import in this file",
            Rung::Own => "this module",
        }
    }
}

/// Where a declaration's *name* is written [lsp-definition]: the file it
/// lives in and the span of its identifier. Recorded for every visible
/// non-fn declaration (fns are identified precisely by [`DeclId`] through
/// the checker's `fn_refs` table) so tooling can jump to definitions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DefSite {
    /// Index into `Program::files`.
    pub file: usize,
    pub span: Span,
}

/// The names visible to one source file.
#[derive(Default, Clone)]
pub struct ModuleScope<'p> {
    pub fns: HashMap<&'p str, Vec<FnEntry<'p>>>,
    pub structs: HashMap<&'p str, &'p StructDecl>,
    /// The file index each visible struct was declared in — what the
    /// same-file discharger rule keys on [linear-group].
    pub struct_files: HashMap<&'p str, usize>,
    pub effects: HashMap<&'p str, &'p EffectDecl>,
    /// The file index each visible effect was declared in — what the
    /// same-file discharger rule keys on for *members* [linear-group]: a
    /// consuming member joins a linear type's discharge set when its effect
    /// is declared in the type's own file.
    pub effect_files: HashMap<&'p str, usize>,
    /// `fns` groups visible here [implicit-group].
    pub fn_groups: HashMap<&'p str, &'p FnsDecl>,
    pub handlers: HashMap<&'p str, &'p HandlerDecl>,
    /// [qual-overload] Visible qualifiers, as **overload sets**: one name may
    /// be declared over several *subject types* (`NonEmpty of List<T>` and
    /// `NonEmpty of Set<T>`), and which one a use means is decided by the
    /// subject, the way a fn overload is decided by its arguments. Almost
    /// every name has exactly one, which is the fast path the checker takes.
    pub qualifiers: HashMap<&'p str, Vec<&'p QualifierDecl>>,
    /// Top-level refinements declared by this file's *own module*
    /// [qual-refn-scope]. Module-scoped and deliberately not importable:
    /// reconciling two qualifiers' conflicting refinements is the
    /// consumer's call, and a library shipping its own reconciliation
    /// would only move the conflict one level up. (A refinement declared
    /// *inside* a qualifier needs no entry here — it travels with the
    /// qualifier, so `qualifiers` already carries it.)
    pub refns: Vec<&'p RefnDecl>,
    /// [fn-rename] `rename fn` declarations of this file's *own module*,
    /// module-scoped and deliberately not importable: a rename removes an
    /// overload from a name, and a library shipping that decision would take
    /// it away from the consumer who has to live with it (the same reasoning
    /// as a top-level `refn`).
    pub renames: Vec<&'p RenameDecl>,
    pub type_aliases: HashMap<&'p str, &'p TypeDecl>,
    /// [effect-alias] `effects IO = […]` groups visible here, with the file
    /// each is declared in: an alias's members resolve where it is written,
    /// like a type alias's target.
    pub effect_aliases: HashMap<&'p str, (&'p EffectAliasDecl, usize)>,
    /// `intrinsic type` declarations visible here.
    pub opaque_types: HashMap<&'p str, &'p TypeDecl>,
    /// The file index each visible opaque type was declared in — what the
    /// same-file discharger rule keys on for a `linear intrinsic type`
    /// [linear-group], exactly as `struct_files` does for a `linear struct`.
    pub opaque_type_files: HashMap<&'p str, usize>,
    /// Effect-member fn name -> (owning effect, member decl).
    /// [effect-member-overload] Effect members callable in this scope, by
    /// name. Several effects may declare the same member name (user
    /// decision 2026-09-14); a call disambiguates by which effect is
    /// available, or explicitly with `member@Effect(…)` [effect-at].
    pub effect_members: HashMap<&'p str, Vec<(&'p EffectDecl, &'p FnDecl)>>,
    /// Visible name -> modules that declare it (under this name; aliased
    /// imports record the alias). Drives module reachability
    /// [mod-used-only] and generated backend imports.
    pub name_origins: HashMap<&'p str, Vec<&'p ModulePath>>,
    /// [obligation-by] Every module this file can see: its own, `std.core.*`, a
    /// test annex's tested module and implicit ones, and each import's
    /// (whole or by item) — every module whose declarations the scope took
    /// in, whether or not any name of it survived. A `by` path names only
    /// these.
    pub modules: Vec<&'p ModulePath>,
    /// Visible name -> where its declaration's identifier is written
    /// [lsp-definition]. Non-fn declarations and effect members; fns are
    /// resolved through the checker's `fn_refs` (overload-precise).
    pub def_sites: HashMap<&'p str, DefSite>,
}

impl<'p> ModuleScope<'p> {
    /// Whether `name` refers to a declared qualifier.
    pub fn is_qualifier(&self, name: &str) -> bool {
        self.qualifiers.contains_key(name)
    }

    /// [mod-export] Whether *anything* in this scope goes by `name` — a fn, a
    /// type, an effect and its members, a handler, a qualifier, a group.
    ///
    /// Used to decide whether an unresolved-name diagnostic has a visibility
    /// story to tell. A name that is here but of the wrong *kind* — a qualifier
    /// written where a type belongs — must not be told it is unexported, even
    /// when some other module happens to declare it privately.
    pub fn declares_name(&self, name: &str) -> bool {
        self.fns.contains_key(name)
            || self.structs.contains_key(name)
            || self.effects.contains_key(name)
            || self.effect_members.contains_key(name)
            || self.handlers.contains_key(name)
            || self.qualifiers.contains_key(name)
            || self.fn_groups.contains_key(name)
            || self.type_aliases.contains_key(name)
            || self.opaque_types.contains_key(name)
            || self.effect_aliases.contains_key(name)
    }

}

pub struct Resolution<'p> {
    /// One scope per file, aligned with `program.files`.
    pub scopes: Vec<ModuleScope<'p>>,
    /// [effect-handler-deps] The file each handler is declared in, keyed by
    /// the declaration's address: a handler's dependency list is written in
    /// *its* file and resolves there, whatever file binds it.
    pub handler_files: HashMap<usize, usize>,
    /// [type-identity] The file every written type reference is in, by its
    /// address (`type_ref_files`).
    pub type_ref_files: HashMap<usize, usize>,
    /// [type-identity] The file every struct, type and effect is declared
    /// in, by the declaration's address.
    pub decl_files: HashMap<usize, usize>,
    /// Whole-program declaration index: name -> (declaring module,
    /// importable item). For most declarations the item is the name
    /// itself; for effect members it is the owning *effect* (importing
    /// the effect brings its members). Drives import suggestions on
    /// unresolved-name diagnostics [diag-import-suggest].
    pub declared_in: HashMap<&'p str, Vec<(&'p ModulePath, &'p str)>>,
    /// [mod-export] The same index for declarations that are *not* exported:
    /// name -> the modules that declare it privately. Nothing resolves through
    /// it. It exists so an unresolved name can be told it exists and is
    /// private, instead of being told it does not exist and offered an import
    /// that could not work.
    pub private_in: HashMap<&'p str, Vec<&'p ModulePath>>,
    /// [fn-attached] Which type each function is **declared on**, for the
    /// functions that are: those written inside a struct body (the AST carries
    /// it, set when they were hoisted) and those that are their file's
    /// fulfilment of one of a struct's obligations. Computed here because the
    /// import rule needs it — a function declared on a type travels with the
    /// type — and imports are resolved before anything is checked.
    pub attached: HashMap<DeclId, String>,
    /// Resolution errors (unresolved/ambiguous imports) [diag-structured].
    pub errors: Vec<FileDiagnostic>,
    /// [decl-id] Every declaration's id and where it is.
    pub decls: std::sync::Arc<crate::decl_id::DeclIndex>,
    /// [comptime-hygiene] The scopes stamped bodies are checked in: the
    /// stamping file's, with what the template's own file sees added
    /// underneath (privates included) — one per (stamping file, template
    /// file) pair.
    pub stamp_scopes: Vec<ModuleScope<'p>>,
    /// [comptime-hygiene] Which of `stamp_scopes` a stamped fn is checked
    /// in, by the fn declaration's address.
    pub fn_stamp_scope: HashMap<usize, usize>,
    /// [comptime-hygiene] The same for every written type reference inside
    /// such a fn, by the reference's address.
    pub type_ref_stamp: HashMap<usize, usize>,
}

impl<'p> Resolution<'p> {
    /// [comptime-hygiene] The scope a type reference resolves in, given the
    /// file it is written in: a stamp's own when it is inside one.
    pub fn scope_for_ref(&self, file: usize, r: &salvo_syntax::ast::TypeRef) -> &ModuleScope<'p> {
        match self.type_ref_stamp.get(&(r as *const salvo_syntax::ast::TypeRef as usize)) {
            Some(&i) => &self.stamp_scopes[i],
            None => &self.scopes[file],
        }
    }
}

/// [comptime-hygiene] A stamped body's scope: everything the stamping file
/// sees, and under it what the template's file sees. Non-fn names fill only
/// the gaps — the site's own meaning of a name always stands — and the
/// template's fns join each overload set on the lowest rung, so a call the
/// site can answer is the site's, and only one it cannot falls back.
fn stamp_scope<'p>(site: &ModuleScope<'p>, template: &ModuleScope<'p>) -> ModuleScope<'p> {
    let mut out = site.clone();
    for (name, entries) in &template.fns {
        let list = out.fns.entry(name).or_default();
        for e in entries {
            if !list.iter().any(|x| x.key == e.key) {
                list.push(FnEntry { rung: Rung::Template, ..*e });
            }
        }
    }
    fn fill<K: std::hash::Hash + Eq + Copy, V: Clone>(out: &mut HashMap<K, V>, from: &HashMap<K, V>) {
        for (k, v) in from {
            out.entry(*k).or_insert_with(|| v.clone());
        }
    }
    fill(&mut out.structs, &template.structs);
    fill(&mut out.struct_files, &template.struct_files);
    fill(&mut out.effects, &template.effects);
    fill(&mut out.effect_files, &template.effect_files);
    fill(&mut out.fn_groups, &template.fn_groups);
    fill(&mut out.handlers, &template.handlers);
    fill(&mut out.qualifiers, &template.qualifiers);
    fill(&mut out.type_aliases, &template.type_aliases);
    fill(&mut out.effect_aliases, &template.effect_aliases);
    fill(&mut out.opaque_types, &template.opaque_types);
    fill(&mut out.opaque_type_files, &template.opaque_type_files);
    fill(&mut out.effect_members, &template.effect_members);
    fill(&mut out.name_origins, &template.name_origins);
    fill(&mut out.def_sites, &template.def_sites);
    out
}

/// [comptime-hygiene] The stamp scopes, and which fn and type reference is
/// checked in which: one per (stamping file, template file) pair whose
/// modules differ (a template of the stamping module sees nothing more).
#[allow(clippy::type_complexity)]
fn stamp_scopes<'p>(
    program: Sources<'_, 'p>,
    scopes: &[ModuleScope<'p>],
) -> (Vec<ModuleScope<'p>>, HashMap<usize, usize>, HashMap<usize, usize>) {
    use salvo_syntax::visit::{self as v, Visitor};
    struct Refs<'m>(&'m mut Vec<usize>);
    impl Visitor for Refs<'_> {
        fn visit_type_ref(&mut self, t: &salvo_syntax::ast::TypeRef) {
            self.0.push(t as *const salvo_syntax::ast::TypeRef as usize);
        }
    }
    let mut out: Vec<ModuleScope<'p>> = Vec::new();
    let mut by_pair: HashMap<(usize, usize), usize> = HashMap::new();
    let mut fns: HashMap<usize, usize> = HashMap::new();
    let mut refs: HashMap<usize, usize> = HashMap::new();
    for (site, ast) in program.modules.iter().enumerate() {
        for item in &ast.items {
            let Item::Fn(f) = item else { continue };
            let Some(t) = f.stamped.as_ref().and_then(|s| s.template_file) else { continue };
            if t >= scopes.len() || program.files[t].module == program.files[site].module {
                continue;
            }
            let idx = *by_pair.entry((site, t)).or_insert_with(|| {
                out.push(stamp_scope(&scopes[site], &scopes[t]));
                out.len() - 1
            });
            fns.insert(f as *const FnDecl as usize, idx);
            let mut addrs = Vec::new();
            v::walk_fn(&mut Refs(&mut addrs), f);
            for a in addrs {
                refs.insert(a, idx);
            }
        }
    }
    (out, fns, refs)
}

impl Resolution<'_> {
    /// [mod-export] What to add to an unresolved-name diagnostic when the name
    /// does exist elsewhere in the program, unexported. `None` when the name is
    /// genuinely nowhere, which leaves that diagnostic exactly as it was.
    pub fn export_note(&self, name: &str) -> Option<String> {
        let modules = self.private_in.get(name)?;
        let mut names: Vec<String> = modules.iter().map(|m| m.to_string()).collect();
        names.sort();
        names.dedup();
        let first = names.first()?;
        Some(match names.len() {
            1 => format!(
                ": it is declared in module `{first}` but not exported — write `export` \
                 in front of that declaration to let it out of its own file [mod-export]"
            ),
            n => format!(
                ": it is declared privately in {n} modules (`{first}` among them) — \
                 write `export` in front of the declaration you mean [mod-export]"
            ),
        })
    }

    /// Import paths (`module.Item`) that would bring `name` into scope,
    /// sorted and deduplicated [diag-import-suggest]. Non-`std.core` modules
    /// only: `std.core.*` is implicitly visible, so an unknown name is never
    /// fixed by importing it from core.
    pub fn import_candidates(&self, name: &str) -> Vec<String> {
        let mut paths: Vec<String> = self
            .declared_in
            .get(name)
            .map(|entries| {
                entries
                    .iter()
                    .filter(|(module, _)| !module.is_std_core())
                    .map(|(module, item)| format!("{module}.{item}"))
                    .collect()
            })
            .unwrap_or_default();
        paths.sort();
        paths.dedup();
        paths
    }
}

/// One module's own declarations, prior to visibility merging. Non-fn
/// items carry their declaring file index so collision diagnostics
/// [mod-collision] can point at the right file.
#[derive(Default)]
struct ModuleItems<'p> {
    fns: Vec<(DeclId, usize, &'p FnDecl)>,
    structs: Vec<(usize, &'p StructDecl)>,
    effects: Vec<(usize, &'p EffectDecl)>,
    fn_groups: Vec<(usize, &'p FnsDecl)>,
    handlers: Vec<(usize, &'p HandlerDecl)>,
    qualifiers: Vec<(usize, &'p QualifierDecl)>,
    /// Top-level refinements [qual-refn-scope]: module-scoped, so they are
    /// collected per module and added to the scope of every file of that
    /// module — and to no other.
    refns: Vec<&'p RefnDecl>,
    /// [fn-rename] Module-scoped, like `refns`.
    renames: Vec<&'p RenameDecl>,
    type_aliases: Vec<(usize, &'p TypeDecl)>,
    opaque_types: Vec<(usize, &'p TypeDecl)>,
    /// [effect-alias] `effects X = […]` declarations.
    effect_aliases: Vec<(usize, &'p EffectAliasDecl)>,
    /// [namespace] The dotted namespace paths declared anywhere in this
    /// module's files (`AttributeValue`, `Outer`, `Outer.Inner`). A whole-
    /// namespace import (`import mod.X`) brings every member under one of
    /// these prefixes; a pure namespace has no declaration of its own, so
    /// this is how the import resolver knows the name is a namespace.
    namespaces: Vec<&'p str>,
}

impl<'p> ModuleItems<'p> {
    fn has_name(&self, name: &str) -> bool {
        self.name_ref(name).is_some()
    }

    /// [mod-export] Whether this module declares `name` **and** lets it out.
    /// An import names a declaration in someone else's module, so this — not
    /// `has_name` — is what an import may resolve to. Effect *members* count
    /// as their effect's: importing an effect brings its members, and a member
    /// has no visibility of its own.
    fn exports_name(&self, name: &str) -> bool {
        let hit = |n: &str, exported: bool| n == name && exported;
        self.fns.iter().any(|(_, _, f)| hit(&f.name.name, f.exported))
            || self.structs.iter().any(|(_, s)| hit(&s.name.name, s.exported))
            || self.effects.iter().any(|(_, e)| {
                hit(&e.name.name, e.exported)
                    || (e.exported && e.fns.iter().any(|f| f.name.name == name))
            })
            || self
                .fn_groups
                .iter()
                .any(|(_, g)| hit(&g.name.name, g.exported))
            || self.handlers.iter().any(|(_, h)| hit(&h.name.name, h.exported))
            || self
                .qualifiers
                .iter()
                .any(|(_, q)| hit(&q.name.name, q.exported))
            || self
                .type_aliases
                .iter()
                .any(|(_, t)| hit(&t.name.name, t.exported))
            || self
                .opaque_types
                .iter()
                .any(|(_, t)| hit(&t.name.name, t.exported))
            || self
                .effect_aliases
                .iter()
                .any(|(_, a)| hit(&a.name.name, a.exported))
    }

    /// The declaration's own `&'p str` for `name`, so a synthesized
    /// lookup key (a dot-name assembled from import path segments
    /// [name-dot]) can be exchanged for one that lives as long as the
    /// program.
    fn name_ref(&self, name: &str) -> Option<&'p str> {
        let hit = |n: &'p str| (n == name).then_some(n);
        self.fns
            .iter()
            .find_map(|(_, _, f)| hit(f.name.name.as_str()))
            .or_else(|| self.structs.iter().find_map(|(_, s)| hit(s.name.name.as_str())))
            .or_else(|| self.effects.iter().find_map(|(_, e)| hit(e.name.name.as_str())))
            .or_else(|| {
                self.fn_groups
                    .iter()
                    .find_map(|(_, g)| hit(g.name.name.as_str()))
            })
            .or_else(|| self.handlers.iter().find_map(|(_, h)| hit(h.name.name.as_str())))
            .or_else(|| {
                self.qualifiers
                    .iter()
                    .find_map(|(_, q)| hit(q.name.name.as_str()))
            })
            .or_else(|| {
                self.type_aliases
                    .iter()
                    .find_map(|(_, t)| hit(t.name.name.as_str()))
            })
            .or_else(|| {
                self.opaque_types
                    .iter()
                    .find_map(|(_, t)| hit(t.name.name.as_str()))
            })
            .or_else(|| {
                self.effect_aliases
                    .iter()
                    .find_map(|(_, a)| hit(a.name.name.as_str()))
            })
    }

    /// `(kind, name, file, span)` of every non-fn item, for collision
    /// detection [mod-collision]. Fns are exempt: same-name fns form
    /// overload sets.
    /// [mod-collision] Non-fn declarations, with the *subject* that
    /// distinguishes same-named qualifiers [qual-overload]: `NonEmpty of
    /// List<T>` and `NonEmpty of Set<T>` are two declarations, not a
    /// duplicate, so the collision key carries the `of` type's base name.
    /// A generic `of` (`qualifier Ok<T> of T`) has no base name and so
    /// collides with everything of its name — which is the conservative
    /// answer, since it would accept the same subjects.
    fn non_fn_names(&self) -> Vec<(NameKind, &'p str, Option<String>, usize, Span)> {
        let mut out = Vec::new();
        for (f, s) in &self.structs {
            out.push((NameKind::Struct, s.name.name.as_str(), None, *f, s.name.span));
        }
        for (f, e) in &self.effects {
            out.push((NameKind::Effect, e.name.name.as_str(), None, *f, e.name.span));
        }
        for (f, g) in &self.fn_groups {
            out.push((NameKind::FnGroup, g.name.name.as_str(), None, *f, g.name.span));
        }
        for (f, h) in &self.handlers {
            out.push((NameKind::Handler, h.name.name.as_str(), None, *f, h.name.span));
        }
        for (f, q) in &self.qualifiers {
            out.push((
                NameKind::Qualifier,
                q.name.name.as_str(),
                crate::refine::of_base(&q.of, &q.generics),
                *f,
                q.name.span,
            ));
        }
        for (f, t) in &self.type_aliases {
            out.push((NameKind::TypeAlias, t.name.name.as_str(), None, *f, t.name.span));
        }
        for (f, t) in &self.opaque_types {
            out.push((NameKind::OpaqueType, t.name.name.as_str(), None, *f, t.name.span));
        }
        // [effect-alias] One namespace with the effects: an effect list names
        // either.
        for (f, a) in &self.effect_aliases {
            out.push((NameKind::Effect, a.name.name.as_str(), None, *f, a.name.span));
        }
        out
    }
}

/// The namespace of a non-fn declaration, for collision detection
/// [mod-collision]: same-kind same-name declarations collide; different
/// kinds live in different lookup tables.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum NameKind {
    Struct,
    Effect,
    Handler,
    Qualifier,
    TypeAlias,
    OpaqueType,
    FnGroup,
}

impl NameKind {
    fn describe(self) -> &'static str {
        match self {
            NameKind::Struct => "struct",
            NameKind::FnGroup => "fns group",
            NameKind::Effect => "effect",
            NameKind::Handler => "handler",
            NameKind::Qualifier => "qualifier",
            NameKind::TypeAlias => "type alias",
            NameKind::OpaqueType => "type",
        }
    }
}

/// Visibility precedence of a scope entry [mod-collision]: own-module
/// declarations override implicit `std.core.*` visibility, and explicit
/// imports override `std.core.*`; everything else is a collision. A
/// whole-module import [mod-import-module] sits between the two: it
/// overrides `std.core.*` and is overridden, silently, by anything more
/// specific.
#[derive(Clone, Copy, PartialEq)]
enum Level {
    Core,
    ModuleImport,
    Own,
    Import,
}

impl Level {
    /// [fn-overload-scope] The same distinction, as the ladder rung an
    /// overload entry records.
    fn rung(self) -> Rung {
        match self {
            Level::Core => Rung::Core,
            Level::ModuleImport => Rung::ModuleImport,
            Level::Own => Rung::Own,
            Level::Import => Rung::Import,
        }
    }
}

/// [test-implicit-import] The standard library's test module, `std.test`
/// (this is the segment after `std`). Implicitly available in
/// every test annex and nowhere else (user decision 2026-09-23), which is why
/// no test file ever writes an import for the assertions it uses.
pub const TEST_MODULE: &str = "test";

/// [test-implicit-import] [throw] The non-resumption module, `std.throw`. Also
/// implicitly available in a test annex, because the harness puts
/// `[Throw<Failure>]` on a test body [test-body] and an assertion throws: a
/// test file importing it would be importing something it never wrote
/// (2026-09-26, when `throw` left `std.core`).
pub const THROW_MODULE: &str = "throw";

pub fn resolve(program: &Program) -> Resolution<'_> {
    resolve_sources(&program.files, &program.modules)
}

/// The files and their ASTs, aligned: what resolution reads of a program.
#[derive(Clone, Copy)]
struct Sources<'s, 'p> {
    files: &'p [crate::source::SourceFile],
    modules: &'s [&'p salvo_syntax::ast::Module],
}

/// [comptime-instantiate] [type-identity] Resolution over files and ASTs not
/// (yet) a `Program`: the comptime expansion resolves the unstamped program
/// to look its names up through each file's scope.
pub fn resolve_sources<'p, M: crate::decl_id::ModuleRef>(
    files: &'p [crate::source::SourceFile],
    modules: &'p [M],
) -> Resolution<'p> {
    resolve_impl(files, modules, &|_| true, false)
}

/// [lsp-std-front] The scopes of the files `scoped` accepts, and nothing
/// else: every other file's scope is left empty, and of the rest of a
/// `Resolution` only what building a scope reads is computed (no
/// program-wide diagnostics, no reference tables). Every file still
/// contributes its declarations, so a scope that is built is the one
/// [`resolve_sources`] builds. What the comptime expansion looks names up
/// by, for the files it stamps.
pub fn resolve_scopes_where<'p, M: crate::decl_id::ModuleRef>(
    files: &'p [crate::source::SourceFile],
    modules: &'p [M],
    scoped: &dyn Fn(usize) -> bool,
) -> Resolution<'p> {
    resolve_impl(files, modules, scoped, true)
}

fn resolve_impl<'p, M: crate::decl_id::ModuleRef>(
    files: &'p [crate::source::SourceFile],
    modules: &'p [M],
    scoped: &dyn Fn(usize) -> bool,
    scopes_only: bool,
) -> Resolution<'p> {
    let modules_in = modules;
    let modules: Vec<&'p salvo_syntax::ast::Module> = modules.iter().map(|m| m.borrow()).collect();
    let program = Sources { files, modules: &modules };
    // [decl-id] Every declaration's id, before anything refers to one.
    let t = crate::timing::phase("    resolve: decl ids");
    let decls = std::sync::Arc::new(crate::decl_id::DeclIndex::build(files, modules_in));
    drop(t);
    // Pass 1: collect each module's own declarations (all files of the
    // module contribute).
    let mut by_module: HashMap<&ModulePath, ModuleItems<'_>> = HashMap::new();
    let mut unexpanded_iter_fns: Vec<FileDiagnostic> = Vec::new();
    // [std-root] Whether a file is the standard library's: std's own, or a
    // test annex of an on-disk std file of this tree (a `std = true` tree's
    // annexes are never `is_std` [test-file]).
    let std_owned = |file: &crate::source::SourceFile| {
        file.is_std
            || (file.is_test && {
                let tested = &file.module.0[..file.module.0.len().saturating_sub(1)];
                program
                    .files
                    .iter()
                    .any(|f| f.is_shadow && f.dependency.is_none() && f.module.0 == tested)
            })
    };
    for (file_idx, (file, ast)) in program.files.iter().zip(program.modules).enumerate() {
        // [std-root] `std` is a reserved root: only the standard library —
        // the embedded copy, a `std = true` tree, a `std = true` dependency —
        // declares modules under it.
        if file.module.is_std_rooted() && !std_owned(file) {
            unexpanded_iter_fns.push(FileDiagnostic::error(
                file_idx,
                Span::new(0, 0),
                format!(
                    "module `{}` is under `{root}`, which is reserved for the standard \
                     library: only a tree whose manifest says `[project] std = true` declares \
                     modules there — move this file out of its `{root}/` directory [std-root]",
                    file.module,
                    root = crate::STD_ROOT
                ),
            ));
        }
        let items = by_module.entry(&file.module).or_default();
        for ns in &ast.namespaces {
            if !items.namespaces.contains(&ns.as_str()) {
                items.namespaces.push(ns.as_str());
            }
        }
        for (item_idx, item) in ast.items.iter().enumerate() {
            match item {
                // [test-file] A `test` block reaching resolution was neither
                // expanded (in an annex) nor refused (in a production file),
                // which `expand` does to every one of them — so this is a
                // driver that skipped the expansion, the same integration
                // error an unexpanded `iter fn` is.
                Item::Test(t) => {
                    unexpanded_iter_fns.push(FileDiagnostic::error(
                        file_idx,
                        t.name_span,
                        "internal: this `test` block reached resolution unexpanded — \
                         the driver that parsed it skipped the program-level \
                         expansion (`salvo_core::expand`); this is a \
                         compiler-integration bug, not a mistake in this program",
                    ));
                }
                // [iter-fn] An `iter fn` must be expanded before resolution
                // (`parse_module` does it; `parse_module_deferred` hands the
                // duty to the program-level `salvo_core::expand`). Checking
                // it as an ordinary fn would be silently different behavior,
                // so a survivor is a loud integration error, never checked.
                Item::Fn(f) if f.is_iter => {
                    unexpanded_iter_fns.push(FileDiagnostic::error(
                        file_idx,
                        f.name.span,
                        "internal: this `iter fn` reached resolution unexpanded — \
                         the driver that parsed it skipped the `iter fn` expansion \
                         (`expand_iter_fns`); this is a compiler-integration \
                         bug, not a mistake in this program",
                    ));
                }
                Item::Fn(f) => items.fns.push((decls.fn_at(file_idx, item_idx), file_idx, f)),
                Item::Struct(s) => items.structs.push((file_idx, s)),
                Item::Effect(e) => items.effects.push((file_idx, e)),
                Item::Fns(g) => items.fn_groups.push((file_idx, g)),
                Item::Handler(h) => items.handlers.push((file_idx, h)),
                Item::Qualifier(q) => items.qualifiers.push((file_idx, q)),
                // [qual-refn-scope] A top-level refinement belongs to its
                // module, not to a name: there is nothing to import.
                Item::Refn(r) => items.refns.push(r),
                // [fn-rename] Module-scoped too, and not importable.
                Item::Rename(r) => items.renames.push(r),
                // An `intrinsic type` is opaque (the backend maps it);
                // anything else is an alias, since a bodiless
                // non-intrinsic `type` is a parse error [decl-body].
                Item::Type(t) if t.intrinsic || t.alias.is_none() => {
                    items.opaque_types.push((file_idx, t))
                }
                Item::Type(t) => items.type_aliases.push((file_idx, t)),
                Item::EffectAlias(a) => items.effect_aliases.push((file_idx, a)),
                _ => {}
            }
        }
    }

    // [mod-visibility] [std-root] What every file sees implicitly: std's
    // modules under `std.core` — declared by std files, so a project module
    // misplaced under the reserved root (refused above) is not one, and std
    // sees only std.
    let std_files: std::collections::HashSet<&ModulePath> =
        program.files.iter().filter(|f| std_owned(f)).map(|f| &f.module).collect();
    let mut core_modules: Vec<&ModulePath> = by_module
        .keys()
        .filter(|m| m.is_std_core() && std_files.contains(*m))
        .copied()
        .collect();
    // Sorted for determinism: scope-entry and overload-candidate order
    // must not depend on hash-map iteration [mod-collision].
    core_modules.sort_by_key(|m| m.to_string());

    // Whole-program declaration index for import suggestions
    // [diag-import-suggest]. Effect members map to their owning effect:
    // importing the effect is what brings the member into scope.
    //
    // [mod-export] Only *exported* declarations go in: suggesting
    // `import std.time.earliest_due` for a private name would be a help line that
    // cannot work. The private ones go in `private_in` instead, which is what
    // turns "no such name" into "not exported".
    let mut declared_in: HashMap<&str, Vec<(&ModulePath, &str)>> = HashMap::new();
    let mut private_in: HashMap<&str, Vec<&ModulePath>> = HashMap::new();
    for (module, items) in &by_module {
        let mut record = |name, item, exported: bool| {
            if exported {
                declared_in.entry(name).or_default().push((*module, item));
            } else {
                private_in.entry(name).or_default().push(*module);
            }
        };
        for (_, _, f) in &items.fns {
            record(&f.name.name, &f.name.name, f.exported);
        }
        for (_, s) in &items.structs {
            record(&s.name.name, &s.name.name, s.exported);
        }
        for (_, e) in &items.effects {
            record(&e.name.name, &e.name.name, e.exported);
            for f in &e.fns {
                record(&f.name.name, &e.name.name, e.exported);
            }
        }
        for (_, h) in &items.handlers {
            record(&h.name.name, &h.name.name, h.exported);
        }
        for (_, q) in &items.qualifiers {
            record(&q.name.name, &q.name.name, q.exported);
        }
        for (_, t) in &items.type_aliases {
            record(&t.name.name, &t.name.name, t.exported);
        }
        for (_, t) in &items.opaque_types {
            record(&t.name.name, &t.name.name, t.exported);
        }
        for (_, a) in &items.effect_aliases {
            record(&a.name.name, &a.name.name, a.exported);
        }
    }

    let mut errors = unexpanded_iter_fns;

    // [mod-collision] Declaration-level collisions are reported once,
    // globally: a same-kind same-name duplicate within one module, and a
    // same-kind same-name collision between two implicitly visible
    // `std.core.*` modules (which would otherwise resolve by hash-map
    // ordering). Fns are exempt — same-name fns form overload sets.
    if !scopes_only {
        let mut sorted_modules: Vec<&&ModulePath> = by_module.keys().collect();
        sorted_modules.sort_by_key(|m| m.to_string());
        // Keyed by kind, name **and subject** — see `non_fn_names`.
        let mut core_seen: HashMap<(NameKind, &str, Option<String>), (&ModulePath, usize, Span)> =
            HashMap::new();
        for module in sorted_modules {
            let items = &by_module[*module];
            let mut module_seen: HashMap<(NameKind, &str, Option<String>), Span> = HashMap::new();
            let is_core = module.is_std_core();
            for (kind, name, subject, file, span) in items.non_fn_names() {
                if let Some(_first) = module_seen.get(&(kind, name, subject.clone())) {
                    errors.push(FileDiagnostic::error(
                        file,
                        span,
                        format!(
                            "duplicate {} `{name}` in module `{module}`",
                            kind.describe()
                        ),
                    ));
                    continue;
                }
                module_seen.insert((kind, name, subject.clone()), span);
                if is_core {
                    if let Some((other, _, _)) = core_seen.get(&(kind, name, subject.clone())) {
                        errors.push(FileDiagnostic::error(
                            file,
                            span,
                            format!(
                                "{} `{name}` is declared in multiple implicitly \
                                 visible core modules: `{other}` and `{module}`",
                                kind.describe()
                            ),
                        ));
                    } else {
                        core_seen.insert((kind, name, subject), (module, file, span));
                    }
                }
            }
        }
    }

    // [fn-overload-duplicate] Two fns of one module with the same name, the
    // same parameter types *and* the same return type are duplicates, not
    // overloads: nothing at a call site could tell them apart. Parameter
    // names take no part in selection; the return type does, among
    // candidates whose parameters are identical [fn-overload-return] (user
    // decision 2026-10-20, the CLI design record round 2). Reported at the second
    // declaration (user decision 2026-09-07).
    if !scopes_only {
        let mut sorted_modules: Vec<&&ModulePath> = by_module.keys().collect();
        sorted_modules.sort_by_key(|m| m.to_string());
        for module in sorted_modules {
            let items = &by_module[*module];
            // The span it was first declared at, and whether that one was
            // *generated* [obligation-by].
            let mut seen: HashMap<(&str, Vec<String>, String), (Span, bool)> = HashMap::new();
            let mut erased_seen: HashMap<(&str, Vec<String>), Vec<String>> = HashMap::new();
            for (_, key_file, f) in &items.fns {
                let mut generics: Vec<&str> =
                    f.generics.iter().map(|g| g.name.as_str()).collect();
                for (g, _) in &f.generic_canbe {
                    if !generics.contains(&g.name.as_str()) {
                        generics.push(g.name.as_str());
                    }
                }
                let sig = crate::refine::param_type_signature(&f.params, &generics);
                let ret = crate::refine::return_type_signature(f.return_type.as_ref(), &generics);
                if let Some((_, other_stamped)) =
                    seen.get(&(f.name.name.as_str(), sig.clone(), ret.clone()))
                {
                    let shown = sig.join(", ");
                    // [obligation-by] One of the two is *stamped*: the remedy
                    // is the clause, not the fn. A `by` clause and a hand-written
                    // member of the same shape are the ordinary duplicate —
                    // nothing at a bare call site could tell them apart — with a
                    // message that names the half the author can delete.
                    let msg = if f.stamped.is_some() || *other_stamped {
                        format!(
                            "`{}({shown})` is declared twice in module `{module}`: a \
                             `by` clause stamps it, and this file also declares it by \
                             hand. Keep one — remove the member's group from the `by` \
                             clause, or delete the hand-written fn [obligation-by]",
                            f.name.name
                        )
                    } else {
                        format!(
                            "duplicate fn `{}({shown}) -> {ret}` in module `{module}`: \
                             an overload set is distinguished by parameter *types*, \
                             or by the return type among overloads whose parameters \
                             agree, so this one could never be called. Parameter \
                             names take no part in choosing an overload \
                             [fn-overload-rank] [fn-overload-return]",
                            f.name.name
                        )
                    };
                    errors.push(FileDiagnostic::error(
                        *key_file,
                        f.name.span,
                        msg,
                    ));
                    continue;
                }
                seen.insert((f.name.name.as_str(), sig.clone(), ret), (f.name.span, f.stamped.is_some()));
                // [type-literal] Two overloads apart only by literal types of
                // one base erase to the same host signature (a literal *is*
                // its base at run time), so neither backend could declare both.
                let erased: Vec<salvo_syntax::ast::Param> = f
                    .params
                    .iter()
                    .map(|p| salvo_syntax::ast::Param { ty: crate::literal::collapse_ast(&p.ty), ..p.clone() })
                    .collect();
                let erased_sig = crate::refine::param_type_signature(&erased, &generics);
                if erased_sig != sig {
                    if let Some(first) = erased_seen.get(&(f.name.name.as_str(), erased_sig.clone())) {
                        if *first != sig {
                            errors.push(FileDiagnostic::error(
                                *key_file,
                                f.name.span,
                                format!(
                                    "`{}` is overloaded on literal types of one base, and a literal \
                                     is its base at run time, so both would be `{}({})` on either \
                                     backend; take the union of the literals in one fn and `when` \
                                     over it [type-literal]",
                                    f.name.name,
                                    f.name.name,
                                    erased_sig.join(", ")
                                ),
                            ));
                        }
                    } else {
                        erased_seen.insert((f.name.name.as_str(), erased_sig), sig);
                    }
                }
            }
        }
    }

    // [fn-attached] The export rule for a function declared on a type. A
    // function written **inside** the body needs no rule — it carries the
    // struct's own visibility by construction, and writing `export` on one is a
    // parse error. A **detached** fulfilment is a separate declaration, so it
    // has to say so: an exported type whose capability is private would hide
    // that capability from every module that can use the type, and
    // [mod-export]'s "the public surface is exactly what the module writes
    // down" is what stops the compiler from silently widening it (user decision
    // 2026-09-26).
    //
    // Computed after the attachment table below, which is why the loop reads it.
    // [fn-attached] Attachment by **obligation fulfilment**: a struct that
    // declares `: Hashed<self>` names the capability it provides, and the
    // function in its own file that fulfils one of the group's members is
    // thereby declared on the type — so it travels with it, exactly as a
    // function written inside the body does. The two routes are one rule:
    // everything declared on a struct is imported with it.
    //
    // Matching is by member name and by a parameter naming the struct, not by
    // full signature: the *checker* judges whether the obligation is really
    // fulfilled (`check_obligations`, with the group's substituted member type)
    // and reports it where the obligation is written. Resolution only needs to
    // know which function the claim is about.
    let groups_by_name: HashMap<&str, &FnsDecl> = by_module
        .values()
        .flat_map(|items| items.fn_groups.iter())
        .map(|(_, g)| (g.name.name.as_str(), *g))
        .collect();
    let mut attached: HashMap<DeclId, String> = HashMap::new();
    for items in by_module.values() {
        // [group-obligation] A `type` declaration carries a clause too
        // (comptime round 2, R-1), so a named union's fulfilments attach the
        // same way a struct's do.
        let declaring: Vec<(usize, &str, &[Obligation])> = items
            .structs
            .iter()
            .map(|(f, st)| (*f, st.name.name.as_str(), st.obligations.as_slice()))
            .chain(
                items
                    .type_aliases
                    .iter()
                    .chain(items.opaque_types.iter())
                    .map(|(f, t)| (*f, t.name.name.as_str(), t.obligations.as_slice())),
            )
            .collect();
        for (file_idx, type_name, obligations) in declaring {
            for ob in obligations {
                let Some(group) = groups_by_name.get(ob.group.name.name.as_str()) else {
                    continue;
                };
                for member in &group.fns {
                    // [obligation-return] Which half of the member's signature
                    // the declaring type appears in: a member taking the
                    // group's `self` parameter (`eq(a: T, b: T)`) is
                    // fulfilled by a fn *taking* the type; only one that
                    // takes none (`from_args(…) -> Ok T | …`) by a fn
                    // returning it. Without the distinction `eq(Int, Int) ->
                    // Bool` in a file declaring both `Int` and `Bool` was
                    // attached to whichever was looked at last (found
                    // 2026-10-27 [unique-shape], when std's scalar `eq`s moved
                    // beside their types).
                    let self_param = ob
                        .group
                        .args
                        .iter()
                        .position(|a| {
                            matches!(a, salvo_syntax::ast::Type::Named { qualifiers, base }
                                if qualifiers.is_empty() && base.args.is_empty() && base.name.name == "self")
                        })
                        .and_then(|i| group.generics.get(i))
                        .map(|g| g.name.as_str());
                    let by_params = self_param
                        .is_none_or(|g| member.params.iter().any(|p| mentions_type(&p.ty, g)));
                    for (key, key_file, f) in &items.fns {
                        if *key_file != file_idx
                            || f.name.name != member.name.name
                            || f.scoped_to.is_some()
                        {
                            continue;
                        }
                        let in_params = f.params.iter().any(|p| mentions_type(&p.ty, type_name));
                        let in_return =
                            f.return_type.as_ref().is_some_and(|r| mentions_type(r, type_name));
                        if (by_params && in_params) || (!by_params && in_return) {
                            attached.insert(*key, type_name.to_string());
                        }
                    }
                }
            }
        }
    }

    // [fn-attached] The detached-fulfilment export rule (see the note above).
    if !scopes_only {
        let mut sorted_modules: Vec<&&ModulePath> = by_module.keys().collect();
        sorted_modules.sort_by_key(|m| m.to_string());
        for module in sorted_modules {
            let items = &by_module[*module];
            for (key, key_file, f) in &items.fns {
                let Some(target) = attached.get(key) else {
                    continue;
                };
                let exported_type = items
                    .structs
                    .iter()
                    .any(|(_, st)| st.name.name == *target && st.exported)
                    || items
                        .type_aliases
                        .iter()
                        .chain(items.opaque_types.iter())
                        .any(|(_, t)| t.name.name == *target && t.exported);
                if exported_type && !f.exported {
                    errors.push(FileDiagnostic::error(
                        *key_file,
                        f.name.span,
                        format!(
                            "`{}` fulfils an obligation of `{target}`, which is \
                             `export`ed, so it is declared on an exported type and has \
                             to say so: write `export fn {}(…)`. A function declared on \
                             a type is imported with the type, and a private one would \
                             be unreachable wherever the type is usable — or write it \
                             inside `{target}`'s body, where the visibility is the \
                             type's [fn-attached] [mod-export]",
                            f.name.name, f.name.name
                        ),
                    ));
                }
            }
        }
    }

    // Pass 2: build one scope per file.
    //
    // [mod-visibility] The `std.core.*` part of a scope depends only on which core
    // module (if any) the file is in: added at `Level::Core` into an empty
    // scope, with no diagnostic (core-core collisions were reported
    // globally). So it is built once per such module and cloned into each
    // file's scope (PIPELINE.md step 8: building it per file was most of
    // resolution). Should building it ever report something, every file
    // builds its own, so each diagnostic stays with its file.
    type CoreBase<'p> = (ModuleScope<'p>, HashMap<(NameKind, &'p str, Option<String>), (Level, &'p ModulePath)>);
    let mut core_bases: HashMap<Option<&ModulePath>, Option<CoreBase<'_>>> = HashMap::new();
    let mut scopes = Vec::with_capacity(program.files.len());
    for (file_idx, (file, ast)) in program.files.iter().zip(program.modules).enumerate() {
        if !scoped(file_idx) {
            scopes.push(ModuleScope::default());
            continue;
        }
        let own_core = core_modules.iter().copied().find(|m| **m == file.module);
        let base = core_bases.entry(own_core).or_insert_with(|| {
            let mut scope = ModuleScope::default();
            let mut provenance = HashMap::new();
            let mut base_errors: Vec<FileDiagnostic> = Vec::new();
            let mut ctx = AddCtx {
                file_idx,
                provenance: &mut provenance,
                errors: &mut base_errors,
                attached: &attached,
            };
            for m in &core_modules {
                if Some(*m) == own_core {
                    continue;
                }
                if let Some(items) = by_module.get(*m) {
                    add_items(&mut scope, items, m, None, Level::Core, None, &mut ctx);
                }
            }
            base_errors.is_empty().then_some((scope, provenance))
        });
        // Provenance of every non-fn scope entry [mod-collision]:
        // (level, module) per (kind, visible name), driving the
        // override-vs-collision decision below.
        // [qual-overload] Keyed by kind, name **and subject**: two core
        // modules may each declare a `NonEmpty` so long as their subjects
        // differ, so visibility has to be tracked per declaration rather than
        // per name (see `non_fn_names`).
        let shared = base.clone();
        let has_core = shared.is_some();
        let (mut scope, mut provenance) = shared.unwrap_or_default();
        let mut ctx = AddCtx {
            file_idx,
            provenance: &mut provenance,
            errors: &mut errors,
            attached: &attached,
        };
        // std.core.* is implicitly visible everywhere — except in a core module's
        // *own* files, which add themselves at `Level::Own` just below. Adding
        // both put every one of that module's fns into the overload set
        // **twice** (the same declaration at two rungs). Ordinary resolution
        // survived it, since the duplicates are identical and `Own` outranks
        // `Core` either way; the refinement matcher did not — it counts
        // matches, so a `refn` inside a core module reported "matches more
        // than one `add` in scope" and no std module could carry a refinement
        // at all [qual-refn-match].
        for m in core_modules.iter().filter(|_| !has_core) {
            if **m == file.module {
                continue;
            }
            if let Some(items) = by_module.get(*m) {
                add_items(&mut scope, items, m, None, Level::Core, None, &mut ctx);
            }
        }
        // The file's own module (overrides core on collision).
        if let Some((own, items)) = by_module.get_key_value(&file.module) {
            add_items(&mut scope, items, own, None, Level::Own, None, &mut ctx);
            // [qual-refn-scope] Top-level refinements are module-scoped and
            // not importable, so they are added here and nowhere else — not
            // from `std.core.*` (which is visible everywhere) and not through
            // an import.
            scope.refns.extend(items.refns.iter().copied());
            // [fn-rename] Module-scoped and order-independent, like every
            // other module-level declaration: a module-level rename is in
            // force for the whole module, in every file of it.
            scope.renames.extend(items.renames.iter().copied());
        }
        // [test-visibility] A test annex sees the module it tests **whole**,
        // private declarations included: that is what whitebox testing is,
        // and it is the reason the companion exists (user decision
        // 2026-09-23). The reverse never holds — the annex is its own module
        // (`heap.test`), nothing imports it, and a production build does not
        // even load it — so the visibility is one-way by construction rather
        // than by a rule someone has to check.
        //
        // Added at `Own`, so a call in a test resolves exactly as the same
        // call written in the module would: same rung, same overload ranking
        // [fn-overload-scope].
        if file.is_test {
            let mut tested = file.module.0.clone();
            tested.pop();
            let tested = ModulePath(tested);
            if let Some((module, items)) = by_module.get_key_value(&tested) {
                add_items(&mut scope, items, module, None, Level::Own, None, &mut ctx);
                scope.refns.extend(items.refns.iter().copied());
                scope.renames.extend(items.renames.iter().copied());
            }
            // [test-implicit-import] And it sees the standard test module
            // without importing it: every test file behaves as if it wrote
            // the whole-module import, which is why no test ever spells one
            // (user decision 2026-09-23). It enters at the bulk-import rung,
            // so a test file's own `expect` silently wins — as anything beats
            // a whole-module import [mod-import-module].
            //
            // `throw` rides along, for the same reason: the harness — not the
            // author — puts `[Throw<Failure>]` on a test body [test-body], and
            // an assertion is a function that throws, so a test file that had
            // to import `throw` would be importing something it never wrote
            // (2026-09-26, with `throw` leaving `std.core`).
            for implicit in [TEST_MODULE, THROW_MODULE] {
                let path = ModulePath::std(&[implicit]);
                if let Some((module, items)) = by_module.get_key_value(&path) {
                    add_items(
                        &mut scope,
                        items,
                        module,
                        None,
                        Level::ModuleImport,
                        None,
                        &mut ctx,
                    );
                }
            }
        }
        // Explicit imports.
        for item in &ast.items {
            let Item::Import(import) = item else { continue };
            // [mod-std-internal] The runtime module is std's own: a program
            // cannot import it, whole or by name.
            // The test harness (`salvo test`'s synthesized module) imports a
            // test annex by path to run it, std's own included.
            let harness = file.module.0.first().is_some_and(|m| m == "__salvo_test_main");
            let segs: Vec<&str> = import.path.iter().map(|seg| seg.name.as_str()).collect();
            // [std-root] The bare spelling of a std module (`import std.time`,
            // `import std.core.list`) is refused with the corrected path named:
            // it applies when the first segment is no module's root and
            // `std.<first>` is std's. Not a fallback — the import resolves
            // nothing.
            let bare = segs.first().is_some_and(|first| {
                *first != crate::STD_ROOT
                    && !by_module.keys().any(|m| m.0.first().is_some_and(|s| s == first))
                    && by_module.keys().any(|m| m.is_std_rooted() && m.0.get(1).is_some_and(|s| s == first))
            });
            let internal = if bare {
                segs.first() == Some(&STD_INTERNAL)
            } else {
                segs.len() >= 2 && segs[0] == crate::STD_ROOT && segs[1] == STD_INTERNAL
            };
            if !file.is_std && !harness && internal {
                ctx.errors.push(FileDiagnostic::error(
                    file_idx,
                    import.span,
                    format!(
                        "`{}.{STD_INTERNAL}` is the standard library's own module: the runtime \
                         every program runs on, reachable only from std itself \
                         [mod-std-internal]",
                        crate::STD_ROOT
                    ),
                ));
                continue;
            }
            if bare {
                ctx.errors.push(FileDiagnostic::error(
                    file_idx,
                    import.span,
                    format!(
                        "the standard library's modules are under `{root}`: write `import {root}.{}` \
                         [std-root]",
                        segs.join("."),
                        root = crate::STD_ROOT
                    ),
                ));
                continue;
            }
            resolve_import(&mut scope, &by_module, import, file_idx, &file.module, &mut ctx);
        }
        // [name-dot] Dot-name rules, checked against everything visible
        // here (own module, core, imports).
        check_dot_names(&scope, ast, file, file_idx, &mut errors);
        scopes.push(scope);
    }

    let mut handler_files: HashMap<usize, usize> = HashMap::new();
    let mut decl_files: HashMap<usize, usize> = HashMap::new();
    for (file_idx, ast) in program.modules.iter().enumerate() {
        for item in &ast.items {
            match item {
                Item::Handler(h) => {
                    handler_files.insert(h as *const HandlerDecl as usize, file_idx);
                }
                Item::Struct(d) => {
                    decl_files.insert(d as *const StructDecl as usize, file_idx);
                }
                Item::Effect(d) => {
                    decl_files.insert(d as *const EffectDecl as usize, file_idx);
                }
                Item::Type(d) => {
                    decl_files.insert(d as *const salvo_syntax::ast::TypeDecl as usize, file_idx);
                }
                _ => {}
            }
        }
    }

    // [unique-shape] Each operator (unique function shape) on a type is
    // declared in the type's file: a per-declaration interface fact, checked
    // once the scopes say which declaration each written type names.
    if !scopes_only {
        crate::unique_shape::check_program(files, &modules, &scopes, &decl_files, &mut errors);
    }

    if scopes_only {
        return Resolution {
            scopes,
            handler_files,
            type_ref_files: HashMap::new(),
            decl_files,
            declared_in,
            private_in,
            attached,
            errors,
            decls,
            stamp_scopes: Vec::new(),
            fn_stamp_scope: HashMap::new(),
            type_ref_stamp: HashMap::new(),
        };
    }
    let (stamp_scopes, fn_stamp_scope, type_ref_stamp) = stamp_scopes(program, &scopes);
    Resolution {
        scopes,
        handler_files,
        type_ref_files: type_ref_files_in(program.modules),
        decl_files,
        declared_in,
        private_in,
        attached,
        errors,
        decls,
        stamp_scopes,
        fn_stamp_scope,
        type_ref_stamp,
    }
}

/// Per-file state threaded through scope building [mod-collision].
/// [mod-std-internal] The segment after `std` of std's internal modules:
/// `std.runtime` and anything under it (`lib/std/runtime.sv`,
/// `lib/std/runtime/**`).
pub const STD_INTERNAL: &str = "runtime";

struct AddCtx<'e, 'p> {
    file_idx: usize,
    provenance: &'e mut HashMap<(NameKind, &'p str, Option<String>), (Level, &'p ModulePath)>,
    errors: &'e mut Vec<FileDiagnostic>,
    /// [fn-attached] Which type each attached fn is declared on.
    attached: &'e HashMap<DeclId, String>,
}

impl<'e, 'p> AddCtx<'e, 'p> {
    /// [type-name-shadow] A **type name** is one name whatever kind declares
    /// it: a struct, a type alias or an opaque type. So an own declaration
    /// or an import of `name` (any of the three) shadows an implicit `std.core`
    /// one of the *other* two kinds too, the way it already shadowed one of
    /// its own kind. Before 2026-10-20 a user `type Field = …` sat beside
    /// `std.core.comptime`'s `comptime struct Field`, and which one a use meant
    /// depended on which table the lookup consulted first (ROADMAP §2d item
    /// 4). Core is added first, so removing its entries here is enough.
    fn shadow_core_type(&mut self, scope: &mut ModuleScope<'p>, kind: NameKind, name: &'p str, level: Level) {
        if level == Level::Core {
            return;
        }
        for other in [NameKind::Struct, NameKind::TypeAlias, NameKind::OpaqueType] {
            if other == kind {
                continue;
            }
            if matches!(self.provenance.get(&(other, name, None)), Some((Level::Core, _))) {
                self.provenance.remove(&(other, name, None));
                match other {
                    NameKind::Struct => {
                        scope.structs.remove(name);
                        scope.struct_files.remove(name);
                    }
                    NameKind::TypeAlias => {
                        scope.type_aliases.remove(name);
                    }
                    _ => {
                        scope.opaque_types.remove(name);
                        scope.opaque_type_files.remove(name);
                    }
                }
            }
        }
    }

    /// Decides whether a non-fn entry may land in the scope
    /// [mod-collision]. Own-module declarations and explicit imports
    /// override implicit `std.core.*` visibility; an import colliding with an
    /// own-module declaration or another import is an error (declaration
    /// -level collisions were already reported globally).
    fn admit(
        &mut self,
        kind: NameKind,
        name: &'p str,
        module: &'p ModulePath,
        level: Level,
        import_span: Option<Span>,
    ) -> bool {
        self.admit_subject(kind, name, None, module, level, import_span)
    }

    /// [qual-overload] `admit` with the *subject* that distinguishes
    /// same-named qualifiers. Everything else passes `None`, which restores
    /// the by-name behavior exactly.
    fn admit_subject(
        &mut self,
        kind: NameKind,
        name: &'p str,
        subject: Option<String>,
        module: &'p ModulePath,
        level: Level,
        import_span: Option<Span>,
    ) -> bool {
        let key = (kind, name, subject);
        match self.provenance.get(&key) {
            None => {
                self.provenance.insert(key, (level, module));
                true
            }
            Some((_, existing)) if **existing == *module => {
                // Same module re-added (a core module that is also the
                // own module, or a redundant import): harmless.
                self.provenance.insert(key, (level, module));
                true
            }
            Some((Level::Core, _)) if level != Level::Core => {
                // Own declarations and imports deliberately shadow
                // implicit core visibility.
                self.provenance.insert(key, (level, module));
                true
            }
            Some((Level::Core, _)) => {
                // Core-core collisions were reported globally; keep the
                // first (deterministic: modules are added in sorted
                // order... core order is the collection order here, but
                // the global check already made this an error).
                false
            }
            // [mod-import-module] A whole-module import never *fights*
            // anything: it loses to an own declaration and to a named
            // import in silence (user decision 2026-09-18 — a bulk import
            // is a convenience, so a name it happens to carry must not
            // break a file that declares its own), and where two bulk
            // imports carry one type name the first wins with a warning.
            // Functions need neither: the ladder ranks them and `@module`
            // picks [fn-overload-scope].
            Some((Level::ModuleImport, existing_module)) if level == Level::ModuleImport => {
                let existing_module = *existing_module;
                let span = import_span.unwrap_or_default();
                self.errors.push(FileDiagnostic::warning(
                    self.file_idx,
                    span,
                    format!(
                        "{} `{name}` is carried by two whole-module imports \
                         (`{existing_module}` and `{module}`); `{existing_module}`'s wins \
                         — import the other by name (`import {module}.{name} as …`) to \
                         pick it [mod-import-module]",
                        kind.describe(),
                    ),
                ));
                false
            }
            Some(_) if level == Level::ModuleImport => false,
            Some((Level::ModuleImport, _)) => {
                // A named import or an own declaration overrides a bulk
                // one, silently and deliberately.
                self.provenance.insert(key, (level, module));
                true
            }
            Some((existing_level, existing_module)) => {
                let existing_module = *existing_module;
                let what = match existing_level {
                    Level::Own => "a declaration in this module".to_string(),
                    _ => format!("the import from `{existing_module}`"),
                };
                let span = import_span.unwrap_or_default();
                self.errors.push(FileDiagnostic::error(
                    self.file_idx,
                    span,
                    format!(
                        "{} `{name}` (imported from `{module}`) conflicts with {what}; \
                         rename it with `as`",
                        kind.describe(),
                    ),
                ));
                false
            }
        }
    }
}

/// Dot-name validation for one file [name-dot]:
///
/// * the namespace must be a struct declared in the *same file*, and it
///   must not be generic (Kotlin nests the member as a plain nested class,
///   which cannot reference the outer class's type parameters);
/// * nothing visible here may carry the *concatenated* name, because the
///   Rust backend flattens `Ns.Name` to `NsName` — and the same
///   concatenation is what overload mangling embeds
///   [fn-emit-name] [rs-fn-mangling]. Checked against the whole
///   scope, so an imported `NsName` counts.
/// * module path segments must be lowercase, so an import path splits
///   into module prefix and item name unambiguously [name-casing].
fn check_dot_names(
    scope: &ModuleScope<'_>,
    ast: &salvo_syntax::ast::Module,
    file: &crate::source::SourceFile,
    file_idx: usize,
    errors: &mut Vec<FileDiagnostic>,
) {
    // Names visible here, excluding fns (which are lowercase, so they can
    // never spell a concatenated type name).
    let visible = |name: &str| {
        scope.structs.contains_key(name)
            || scope.qualifiers.contains_key(name)
            || scope.effects.contains_key(name)
            || scope.handlers.contains_key(name)
            || scope.type_aliases.contains_key(name)
            || scope.opaque_types.contains_key(name)
    };
    // Namespaces: an explicit `namespace X { … }` (recorded on the module
    // when its body was flattened), plus the legacy rule that a struct or a
    // defined `type` in this file namespaces its dot-named members — an enum's
    // arms live under the enum (`StorageClass.Standard`), user decision
    // 2026-09-29. A struct/type namespace must not be generic (Kotlin nests
    // the member as a plain nested class, which cannot reference the outer
    // type parameters); an explicit `namespace` has no generics to worry
    // about. Keyed by the full dotted prefix so nesting works [name-dot] [namespace].
    let mut ns_decls: std::collections::HashSet<&str> =
        ast.namespaces.iter().map(|s| s.as_str()).collect();
    let mut generic_ns: HashMap<&str, usize> = HashMap::new();
    for item in &ast.items {
        match item {
            Item::Struct(s) if !s.name.name.contains('.') => {
                if s.generics.is_empty() {
                    ns_decls.insert(s.name.name.as_str());
                } else {
                    generic_ns.insert(s.name.name.as_str(), s.generics.len());
                }
            }
            Item::Type(t) if t.alias.is_some() && !t.name.name.contains('.') => {
                if t.generics.is_empty() {
                    ns_decls.insert(t.name.name.as_str());
                } else {
                    generic_ns.insert(t.name.name.as_str(), t.generics.len());
                }
            }
            _ => {}
        }
    }
    let mut dotted: Vec<(&str, Span)> = Vec::new();
    for item in &ast.items {
        match item {
            Item::Struct(s) => dotted.push((s.name.name.as_str(), s.name.span)),
            Item::Qualifier(q) => dotted.push((q.name.name.as_str(), q.name.span)),
            Item::Type(t) if t.name.name.contains('.') => dotted.push((t.name.name.as_str(), t.name.span)),
            Item::Effect(e) if e.name.name.contains('.') => dotted.push((e.name.name.as_str(), e.name.span)),
            Item::Handler(h) if h.name.name.contains('.') => dotted.push((h.name.name.as_str(), h.name.span)),
            Item::Fn(f) if f.name.name.contains('.') => dotted.push((f.name.name.as_str(), f.name.span)),
            _ => continue,
        }
    }
    for (name, span) in dotted {
        // The namespace prefix is everything before the final segment, so
        // `A.B.C`'s namespace is `A.B` — which must itself be a known
        // namespace (explicit, or a same-file struct/type for a two-segment
        // legacy name).
        let Some((ns, _member)) = name.rsplit_once('.') else {
            continue;
        };
        if let Some(arity) = generic_ns.get(ns) {
            let _ = arity;
            errors.push(FileDiagnostic::error(
                file_idx,
                span,
                format!(
                    "`{ns}` is generic, so it cannot namespace `{name}`: Kotlin emits the \
                     member as a nested class, which cannot use the outer type parameters \
                     [name-dot]"
                ),
            ));
        } else if !ns_decls.contains(ns) {
            errors.push(FileDiagnostic::error(
                file_idx,
                span,
                format!(
                    "`{ns}` in the dot-name `{name}` is not a namespace here: declare \
                     `namespace {ns} {{ … }}`, or a struct or defined `type {ns}` in this \
                     file [name-dot] [namespace]"
                ),
            ));
        }
        let concatenated = name.replace('.', "");
        if visible(&concatenated) {
            errors.push(FileDiagnostic::error(
                file_idx,
                span,
                format!(
                    "the dot-name `{name}` collides with `{concatenated}`, which is also \
                     visible here: the Rust backend flattens dot-names and overload \
                     mangling uses the same spelling — rename one of them [name-dot]"
                ),
            ));
        }
    }
    // [name-casing] Module paths come from file paths [mod-file], so this
    // is a constraint on file and directory names.
    if let Some(seg) = file
        .module
        .0
        .iter()
        .find(|seg| seg.starts_with(|c: char| c.is_uppercase()))
    {
        errors.push(FileDiagnostic::error(
            file_idx,
            Span::default(),
            format!(
                "module path segment `{seg}` must start with a lowercase letter: module \
                 paths are file paths, so rename the file or directory [name-casing]"
            ),
        ));
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
    level: Level,
    import_span: Option<Span>,
    ctx: &mut AddCtx<'_, 'p>,
) {
    if !scope.modules.contains(&module) {
        scope.modules.push(module);
    }
    // An unaliased import of a namespace struct also brings its dot-named
    // members: `import a.b.Environment` makes `Environment.Id` visible
    // [name-dot-import]. Aliased imports rename exactly one name, so
    // members do not ride along (there is no sensible partial rename).
    let want = |name: &str| {
        filter.is_none_or(|(n, alias)| {
            name == n || (alias == n && name.len() > n.len() && name.starts_with(n) && name.as_bytes()[n.len()] == b'.')
        })
    };
    let visible_as = |name: &'p str| match filter {
        Some((n, alias)) if name == n => alias,
        _ => name,
    };
    let origin = |name: &'p str, scope: &mut ModuleScope<'p>| {
        let origins = scope.name_origins.entry(name).or_default();
        if !origins.contains(&module) {
            origins.push(module);
        }
    };
    // Where the declaration's identifier is written [lsp-definition].
    let def_site = |name: &'p str, file: usize, span: Span, scope: &mut ModuleScope<'p>| {
        scope.def_sites.insert(name, DefSite { file, span });
    };
    // [mod-export] A declaration leaves its own module only if it says
    // `export`. `Level::Own` *is* the declaring module, which sees all of its
    // own names; every other level filters. A name that fails the filter is
    // remembered under `hidden`, so an unresolved use can be told it exists
    // and is private rather than being told it does not exist — the difference
    // between a rule and a mystery.
    let visible = |exported: bool| level == Level::Own || exported;
    for (key, file, f) in &items.fns {
        // [fn-attached] An `@`-scoped fn **travels with its type**: an import
        // that names `Person` brings every `fn …@Person` of that file along, so
        // the canonical is in scope wherever the type is usable. Without it
        // [implicit-resolve]'s per-call-site locality would be a hazard — a
        // module importing `Person` but not its file's `cmp` could resolve
        // `?cmp` to some *other* visible `cmp(Person, Person)`, or to nothing.
        // The fn keeps its own name: an `as` alias renames the type it was
        // written on, and there is no sensible partial rename of a capability.
        // [fn-attached] Either route attaches: written inside the body (the
        // AST says so) or the file's fulfilment of an obligation (resolution
        // worked it out).
        let target: Option<&str> = f
            .scoped_to
            .as_ref()
            .map(|t| t.name.as_str())
            .or_else(|| ctx.attached.get(key).map(|t| t.as_str()));
        let via_type = target.is_some_and(|t| !want(&f.name.name) && want(t));
        if want(&f.name.name) || via_type {
            if !visible(f.exported) {
                continue;
            }
            let name = if via_type {
                f.name.name.as_str()
            } else {
                visible_as(&f.name.name)
            };
            scope
                .fns
                .entry(name)
                .or_default()
                .push(FnEntry {
                    key: *key,
                    file: *file,
                    decl: f,
                    rung: level.rung(),
                    module,
                });
            origin(name, scope);
        }
    }
    for (file, s) in &items.structs {
        if want(&s.name.name) {
            if !visible(s.exported) {
                continue;
            }
            let name = visible_as(&s.name.name);
            if ctx.admit(NameKind::Struct, name, module, level, import_span) {
                ctx.shadow_core_type(scope, NameKind::Struct, name, level);
                scope.structs.insert(name, s);
                scope.struct_files.insert(name, *file);
                origin(name, scope);
                def_site(name, *file, s.name.span, scope);
            }
        }
    }
    for (file, e) in &items.effects {
        if want(&e.name.name) {
            if !visible(e.exported) {
                continue;
            }
            let name = visible_as(&e.name.name);
            if ctx.admit(NameKind::Effect, name, module, level, import_span) {
                scope.effects.insert(name, e);
                scope.effect_files.insert(name, *file);
                origin(name, scope);
                def_site(name, *file, e.name.span, scope);
                // Effect members become callable wherever the effect is
                // visible — and *only* there. Inserting them regardless of
                // visibility let a user effect's member name reach modules
                // that never imported the effect, std's included: a program
                // declaring `effect Sink { fn keep(...) }` made std's
                // `filter(it, keep: (T) -> Bool)` resolve its own parameter
                // as an effect call, which the emitters then reported as
                // "no handler for effect `Sink`" from inside `core/seq.sv`
                // [mod-collision].
                for f in &e.fns {
                    scope
                        .effect_members
                        .entry(&f.name.name)
                        .or_default()
                        .push((e, f));
                    origin(&f.name.name, scope);
                    def_site(&f.name.name, *file, f.name.span, scope);
                }
            }
        }
    }
    // [implicit-group] A group is a *type-level* name, visible like an
    // effect. Its members are not brought into scope: they are resolved at
    // the call site by name and type [implicit-resolve], not called through
    // the group.
    for (file, g) in &items.fn_groups {
        if want(&g.name.name) {
            if !visible(g.exported) {
                continue;
            }
            let name = visible_as(&g.name.name);
            if ctx.admit(NameKind::FnGroup, name, module, level, import_span) {
                scope.fn_groups.insert(name, g);
                origin(name, scope);
                def_site(name, *file, g.name.span, scope);
            }
        }
    }
    for (file, h) in &items.handlers {
        if want(&h.name.name) {
            if !visible(h.exported) {
                continue;
            }
            let name = visible_as(&h.name.name);
            if ctx.admit(NameKind::Handler, name, module, level, import_span) {
                scope.handlers.insert(name, h);
                origin(name, scope);
                def_site(name, *file, h.name.span, scope);
            }
        }
    }
    for (file, q) in &items.qualifiers {
        if want(&q.name.name) {
            if !visible(q.exported) {
                continue;
            }
            let name = visible_as(&q.name.name);
            let subject = crate::refine::of_base(&q.of, &q.generics);
            if ctx.admit_subject(
                NameKind::Qualifier,
                name,
                subject.clone(),
                module,
                level,
                import_span,
            ) {
                // [qual-overload] Overloading is by **subject**: a
                // declaration joins the set only when its subject is new.
                // Same name *and* same subject is the old single-entry case,
                // where a later level replaces an earlier one — which is what
                // lets a module declare its own `NonEmpty of List<T>` and
                // shadow std's rather than becoming ambiguous with it.
                let entry = scope.qualifiers.entry(name).or_default();
                match entry
                    .iter()
                    .position(|d| crate::refine::of_base(&d.of, &d.generics) == subject)
                {
                    Some(i) => entry[i] = q,
                    None => entry.push(q),
                }
                origin(name, scope);
                def_site(name, *file, q.name.span, scope);
            }
        }
    }
    for (file, t) in &items.type_aliases {
        if want(&t.name.name) {
            if !visible(t.exported) {
                continue;
            }
            let name = visible_as(&t.name.name);
            if ctx.admit(NameKind::TypeAlias, name, module, level, import_span) {
                ctx.shadow_core_type(scope, NameKind::TypeAlias, name, level);
                scope.type_aliases.insert(name, t);
                origin(name, scope);
                def_site(name, *file, t.name.span, scope);
            }
        }
    }
    for (file, t) in &items.opaque_types {
        if want(&t.name.name) {
            if !visible(t.exported) {
                continue;
            }
            let name = visible_as(&t.name.name);
            if ctx.admit(NameKind::OpaqueType, name, module, level, import_span) {
                ctx.shadow_core_type(scope, NameKind::OpaqueType, name, level);
                scope.opaque_types.insert(name, t);
                scope.opaque_type_files.insert(name, *file);
                origin(name, scope);
                def_site(name, *file, t.name.span, scope);
            }
        }
    }
    // [effect-alias] Resolved like a type alias (PIPELINE.md R6): visible
    // by declaration, import and `std.core.*`, expanded where a list is lowered.
    for (file, a) in &items.effect_aliases {
        if want(&a.name.name) {
            if !visible(a.exported) {
                continue;
            }
            let name = visible_as(&a.name.name);
            if ctx.admit(NameKind::Effect, name, module, level, import_span) {
                scope.effect_aliases.insert(name, (a, *file));
                origin(name, scope);
                def_site(name, *file, a.name.span, scope);
            }
        }
    }
}

fn resolve_import<'p>(
    scope: &mut ModuleScope<'p>,
    by_module: &HashMap<&'p ModulePath, ModuleItems<'p>>,
    import: &'p ImportDecl,
    file_idx: usize,
    own_module: &ModulePath,
    ctx: &mut AddCtx<'_, 'p>,
) {
    // [mod-import-module] `import std.time` — a whole *module*, every name in
    // it (user decision 2026-09-18). Tried first, and only when every
    // segment is lowercase, so it can never capture a name import: module
    // path segments are lowercase [name-casing] while a type is uppercase,
    // and a *fn* import is distinguished by its module prefix still
    // matching a module. The path names **one** module and not the tree
    // under it (user decision 2026-09-26): `import std.fs` brings `std.fs`,
    // and `std.fs.mem` takes a second line. Importing less is what an import
    // is for, and importing more is always one more statement.
    let all_lower = import
        .path
        .iter()
        .all(|seg| !seg.name.starts_with(|c: char| c.is_uppercase()));
    if all_lower {
        let prefix: Vec<&str> = import.path.iter().map(|i| i.name.as_str()).collect();
        // An import is **fully qualified** — it says where a name comes from,
        // so it writes the whole path; the suffix rule is for selectors
        // (`@`, `by`) only [mod-suffix] (user decision 2026-09-29).
        let named: Option<&&ModulePath> = by_module
            .keys()
            .find(|path| path.0.len() == prefix.len() && path.0.iter().eq(prefix.iter()));
        if let Some(module) = named {
            if let Some(alias) = &import.alias {
                ctx.errors.push(FileDiagnostic::error(
                    file_idx,
                    import.span,
                    format!(
                        "a whole-module import has no alias: `as {}` renames one \
                         imported *name*, and Salvo has no module-qualified type \
                         references to rename a module for. Import the names you \
                         want to alias individually [mod-import-module]",
                        alias.name
                    ),
                ));
                return;
            }
            // Already visible at another level: `std.core.*` is implicit
            // everywhere and the file's own module adds itself. Adding
            // them again would put every fn in the overload set twice —
            // the same declaration at two rungs, which the refinement
            // matcher counts [qual-refn-match].
            let is_core = module.is_std_core();
            if is_core || **module == *own_module {
                ctx.errors.push(FileDiagnostic::warning(
                    file_idx,
                    import.span,
                    format!(
                        "redundant import: everything in `{}` is already visible here \
                         (`core.*` is implicit, and a module sees itself) \
                         [mod-visibility]",
                        prefix.join(".")
                    ),
                ));
            } else {
                let items = &by_module[*module];
                add_items(
                    scope,
                    items,
                    module,
                    None,
                    Level::ModuleImport,
                    Some(import.span),
                    ctx,
                );
            }
            return;
        }
        // [mod-import-module] A path that names no module but has modules
        // *under* it is the one mistake the exact reading introduces, so it
        // names them: `import std.fs` where only `std.fs.mem` exists.
        let mut under: Vec<String> = by_module
            .keys()
            .filter(|path| {
                path.0.len() > prefix.len() && path.0.iter().zip(&prefix).all(|(a, b)| a == b)
            })
            .map(|m| m.to_string())
            .collect();
        if !under.is_empty() {
            under.sort();
            under.dedup();
            // `import std.core` names no module of its own — `std.core.list`,
            // `std.core.string` and the rest are separate modules — but
            // everything under it is visible anyway, so the honest answer is
            // the redundancy warning rather than "no such module"
            // [mod-visibility].
            if prefix == [crate::STD_ROOT, crate::source::CORE] {
                ctx.errors.push(FileDiagnostic::warning(
                    file_idx,
                    import.span,
                    format!(
                        "redundant import: everything in `{}` is already visible here \
                         (`core.*` is implicit) [mod-visibility]",
                        prefix.join(".")
                    ),
                ));
                return;
            }
            ctx.errors.push(FileDiagnostic::error(
                file_idx,
                import.span,
                format!(
                    "no module `{}` — it is a path prefix, not a module, and a \
                     whole-module import names one module rather than the tree \
                     under it. Import the ones you want: {} [mod-import-module]",
                    prefix.join("."),
                    under
                        .iter()
                        .map(|m| format!("`import {m}`"))
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            ));
            return;
        }
        // [mod-import-module] A single-segment path can only ever have been
        // a module: there is no `module.item` split to fall back to, so say
        // what is wrong rather than reporting the shape.
        if import.path.len() == 1 {
            let mut known: Vec<String> = by_module
                .keys()
                .filter(|m| !m.is_std_core())
                .map(|m| m.to_string())
                .collect();
            known.sort();
            known.dedup();
            ctx.errors.push(FileDiagnostic::error(
                file_idx,
                import.span,
                format!(
                    "unknown module `{}`: `import <module>` imports every name in a \
                     module [mod-import-module]{}",
                    prefix.join("."),
                    if known.is_empty() {
                        String::new()
                    } else {
                        format!(" (importable modules: {})", known.join(", "))
                    }
                ),
            ));
            return;
        }
    }
    if import.path.len() < 2 {
        ctx.errors.push(FileDiagnostic::error(
            file_idx,
            import.span,
            "import path must be `module.item`",
        ));
        return;
    }
    // Split the path into module prefix and item name. Module segments are
    // lowercase [name-casing], so *trailing* uppercase segments are the
    // item: one for a plain name, two for a dot-name `Ns.Name`
    // [name-dot]. A lowercase last segment is a value (a fn).
    let trailing_upper = import
        .path
        .iter()
        .rev()
        .take_while(|seg| seg.name.starts_with(|c: char| c.is_uppercase()))
        .count();
    if trailing_upper > 2 {
        ctx.errors.push(FileDiagnostic::error(
            file_idx,
            import.span,
            "a dot-name has exactly two segments (`module.Ns.Name`) [name-dot]",
        ));
        return;
    }
    let name_segs = trailing_upper.max(1);
    if name_segs >= import.path.len() {
        ctx.errors.push(FileDiagnostic::error(
            file_idx,
            import.span,
            "import path must be `module.item`",
        ));
        return;
    }
    let split = import.path.len() - name_segs;
    let item_owned: String = import.path[split..]
        .iter()
        .map(|i| i.name.as_str())
        .collect::<Vec<_>>()
        .join(".");
    // Dot-names are stored as one dotted name, so the borrowed key is the
    // declaration's own `Ident` when there are two segments.
    let item_name: &str = if name_segs == 1 {
        &import.path[split].name
    } else {
        &item_owned
    };
    let prefix: Vec<&str> = import.path[..split]
        .iter()
        .map(|i| i.name.as_str())
        .collect();

    // [namespace] A whole-namespace import: `import mod.X` where `X` is a
    // namespace of the target module (not a declared name of its own). It
    // brings every exported member under the `X.` prefix, keeping them
    // written `X.member` — not a flat dump like a whole-module import. An
    // alias rewrites the `X.` prefix to `Y.` (D5).
    {
        let ns_modules: Vec<&&'p ModulePath> = by_module
            .iter()
            .filter(|(path, items)| {
                let exact = path.0.len() == prefix.len()
                    && path.0.iter().zip(&prefix).all(|(a, b)| a == b);
                let prefixed = path.0.len() > prefix.len()
                    && path.0.iter().zip(&prefix).all(|(a, b)| a == b);
                (exact || prefixed) && items.namespaces.contains(&item_name)
            })
            .map(|(path, _)| by_module.get_key_value(path).unwrap().0)
            .collect();
        if !ns_modules.is_empty() {
            // Only one module should own a namespace path under a fully
            // qualified import; if several match, it is the same ambiguity a
            // name import would report.
            if ns_modules.len() > 1 {
                let mut names: Vec<String> = ns_modules.iter().map(|m| m.to_string()).collect();
                names.sort();
                ctx.errors.push(FileDiagnostic::error(
                    file_idx,
                    import.span,
                    format!(
                        "ambiguous namespace import `{item_name}`: found in modules {} \
                         [namespace]",
                        names.join(", ")
                    ),
                ));
                return;
            }
            let module = *ns_modules[0];
            let items = &by_module[module];
            add_namespace_items(
                scope,
                items,
                module,
                item_name,
                import.alias.as_ref().map(|a| a.name.as_str()),
                import.span,
                ctx,
            );
            return;
        }
    }

    // Candidate modules: path equals the prefix, or the prefix is a leading
    // path of the module (`import std.core.Str` finds `std.core.string`). Never a
    // suffix: an import is fully qualified [mod-suffix].
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
    // [mod-export] An import may only name a declaration its module *exports*.
    // Matching on "declares it" first and filtering here is deliberate: it lets
    // the diagnostic say the name exists and is private, where filtering during
    // the match would have produced "no module declares it" — true of no module
    // and helpful to nobody.
    let exported: Vec<&&'p ModulePath> = matches
        .iter()
        .copied()
        .filter(|m| by_module[*m].exports_name(item_name))
        .collect();
    if exported.is_empty() && !matches.is_empty() {
        let mut names: Vec<String> = matches.iter().map(|m| m.to_string()).collect();
        names.sort();
        names.dedup();
        ctx.errors.push(FileDiagnostic::error(
            file_idx,
            import.span,
            format!(
                "module `{}` declares `{item_name}` but does not export it, so it \
                 cannot be imported: write `export` in front of that declaration \
                 [mod-export]",
                names.join("`, `")
            ),
        ));
        return;
    }
    let matches = exported;
    match matches.len() {
        0 => {
            // Suggest modules that do declare the item, wherever they
            // live [diag-import-suggest]. Private declarations are not
            // suggested: importing one is the error just above.
            let mut suggestions: Vec<String> = by_module
                .iter()
                .filter(|(_, items)| items.exports_name(item_name))
                .map(|(path, _)| format!("{path}.{item_name}"))
                .collect();
            suggestions.sort();
            suggestions.dedup();
            ctx.errors.push(
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
            let module = *matches[0];
            let items = &by_module[module];
            // Exchange the (possibly synthesized) dotted key for the
            // declaration's own long-lived name [name-dot].
            let item_name: &'p str = match items.name_ref(item_name) {
                Some(n) => n,
                None => return,
            };
            let alias: &'p str = import
                .alias
                .as_ref()
                .map(|a| a.name.as_str())
                .unwrap_or(item_name);
            add_items(
                scope,
                items,
                module,
                Some((item_name, alias)),
                Level::Import,
                Some(import.span),
                ctx,
            );
        }
        _ => {
            let mut names: Vec<String> = matches.iter().map(|m| m.to_string()).collect();
            names.sort();
            ctx.errors.push(FileDiagnostic::error(
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

/// [namespace] Brings a whole namespace's exported members into `scope`:
/// every exported declaration of `module` whose name is `ns` itself or starts
/// with `ns.` (`AttributeValue.S`, `Outer.Inner.Thing`). Without an alias the
/// members keep their names; with one, the `ns.` prefix is rewritten to
/// `alias.` (D5). A rewritten name is interned for the program's life (a
/// single compile), since it has no AST home.
fn add_namespace_items<'p>(
    scope: &mut ModuleScope<'p>,
    items: &ModuleItems<'p>,
    module: &'p ModulePath,
    ns: &str,
    alias: Option<&str>,
    import_span: Span,
    ctx: &mut AddCtx<'_, 'p>,
) {
    let dot_prefix = format!("{ns}.");
    // Every exported member name under the namespace, de-duplicated.
    let mut members: Vec<&'p str> = Vec::new();
    let consider = |name: &'p str, members: &mut Vec<&'p str>| {
        if (name == ns || name.starts_with(&dot_prefix))
            && items.exports_name(name)
            && !members.contains(&name)
        {
            members.push(name);
        }
    };
    for (_, s) in &items.structs {
        consider(s.name.name.as_str(), &mut members);
    }
    for (_, t) in &items.type_aliases {
        consider(t.name.name.as_str(), &mut members);
    }
    for (_, t) in &items.opaque_types {
        consider(t.name.name.as_str(), &mut members);
    }
    for (_, e) in &items.effects {
        consider(e.name.name.as_str(), &mut members);
    }
    for (_, h) in &items.handlers {
        consider(h.name.name.as_str(), &mut members);
    }
    for (_, q) in &items.qualifiers {
        consider(q.name.name.as_str(), &mut members);
    }
    for (_, _, f) in &items.fns {
        consider(f.name.name.as_str(), &mut members);
    }
    if members.is_empty() {
        ctx.errors.push(FileDiagnostic::warning(
            ctx.file_idx,
            import_span,
            format!(
                "namespace `{ns}` of `{module}` exports nothing, so this import brings \
                 no names [namespace]"
            ),
        ));
        return;
    }
    for name in members {
        // The name this member is visible as here: itself, or with the `ns.`
        // prefix rewritten to the alias.
        let visible: &'p str = match alias {
            Some(a) => {
                let rewritten = if name == ns {
                    a.to_string()
                } else {
                    format!("{a}.{}", &name[dot_prefix.len()..])
                };
                // Interned for the program's life (one compile) — a rewritten
                // name has no AST home to borrow from.
                &*Box::leak(rewritten.into_boxed_str())
            }
            None => name,
        };
        add_items(
            scope,
            items,
            module,
            Some((name, visible)),
            Level::Import,
            Some(import_span),
            ctx,
        );
    }
}

/// [fn-attached] Whether a written type mentions `name` at its top level or
/// inside a qualifier group, union arm or type argument — enough to tell that a
/// parameter is *about* the struct whose obligation is being fulfilled. The
/// signature itself is judged by the checker.
fn mentions_type(ty: &salvo_syntax::ast::Type, name: &str) -> bool {
    use salvo_syntax::ast::Type as T;
    match ty {
        T::Literal { .. } => false,
        T::Named { base, .. } => {
            base.name.name == name || base.args.iter().any(|a| mentions_type(a, name))
        }
        T::QualifiedGroup { base, .. } => mentions_type(base, name),
        T::Union { arms, .. } => arms.iter().any(|a| mentions_type(a, name)),
        T::Nullable { inner, .. } => mentions_type(inner, name),
        T::Spread { inner, .. } => mentions_type(inner, name),
        T::Tuple { elems, .. } => elems.iter().any(|e| mentions_type(e, name)),
        T::Array { elem, .. } => mentions_type(elem, name),
        T::Fn { params, ret, .. } => {
            params.iter().any(|p| mentions_type(p, name))
                || mentions_type(ret, name)
        }
    }
}

/// [type-identity] The file every written type reference is in, by the
/// reference's address: a type is resolved in the scope of the file that
/// wrote it, wherever it is lowered (a callee's signature is lowered at
/// every call).
pub fn type_ref_files(program: &Program) -> HashMap<usize, usize> {
    type_ref_files_in(&program.modules)
}

///
/// Every reference, wherever in a declaration it is written (a qualifier's
/// slots, a rename's parameters, a parameter's default …): a reference this
/// missed was taken for the lowering file's own, so its definition site and
/// its names were recorded and resolved there (found 2026-10-20, by the
/// check cache's differential [lsp-check-cache]).
fn type_ref_files_in<M: std::borrow::Borrow<salvo_syntax::ast::Module>>(modules: &[M]) -> HashMap<usize, usize> {
    use salvo_syntax::visit::Visitor;
    struct Collect<'o> {
        file: usize,
        out: &'o mut HashMap<usize, usize>,
    }
    impl Visitor for Collect<'_> {
        fn visit_type_ref(&mut self, t: &salvo_syntax::ast::TypeRef) {
            self.out.insert(t as *const salvo_syntax::ast::TypeRef as usize, self.file);
        }
    }
    let mut out = HashMap::new();
    for (file, ast) in modules.iter().enumerate() {
        crate::check_cache::walk_module(&mut Collect { file, out: &mut out }, ast.borrow());
    }
    out
}

/// [type-identity] The key of the declaration a written name resolves to
/// in [scope] (through an import alias too), or `None` when it names no
/// declaration there.
pub fn key_in<'p>(symbols: &crate::program::Symbols<'p>, scope: &ModuleScope<'p>, name: &str) -> Option<&'p str> {
    let addr = scope
        .structs
        .get(name)
        .map(|d| *d as *const StructDecl as usize)
        .or_else(|| scope.effects.get(name).map(|d| *d as *const EffectDecl as usize))
        .or_else(|| scope.opaque_types.get(name).map(|d| *d as *const salvo_syntax::ast::TypeDecl as usize))
        .or_else(|| scope.type_aliases.get(name).map(|d| *d as *const salvo_syntax::ast::TypeDecl as usize))
        .or_else(|| scope.handlers.get(name).map(|d| *d as *const HandlerDecl as usize))?;
    symbols.decl_keys.get(&addr).copied()
}

/// [type-identity] Every written type reference whose declaration's key is
/// not the name written — a clashing name, or an alias — by the
/// reference's address: what an emitter rendering a written type reads.
pub fn written_keys<'p>(
    program: &'p Program,
    resolution: &Resolution<'p>,
    symbols: &crate::program::Symbols<'p>,
) -> HashMap<usize, String> {
    written_keys_from(program, resolution, symbols, 0)
}

/// [lsp-std-front] [`written_keys`] of the files from `first` on.
pub fn written_keys_from<'p>(
    program: &'p Program,
    resolution: &Resolution<'p>,
    symbols: &crate::program::Symbols<'p>,
    first: usize,
) -> HashMap<usize, String> {
    let mut out = HashMap::new();
    for_each_type_ref_from(&program.modules, first, &mut |file, t| {
        let name = t.name.name.as_str();
        if let Some(key) = key_in(symbols, resolution.scope_for_ref(file, t), name) {
            if key != name {
                out.insert(t as *const salvo_syntax::ast::TypeRef as usize, key.to_string());
            }
        }
    });
    out
}

/// [type-identity] The same, by the reference's span and written name: what
/// a *copy* of a reference is found by (an emitter clones types freely).
/// Every reference at one span with one name resolves alike in practice;
/// where two do not, the entry is dropped rather than guessed.
pub fn written_keys_by_span<'p>(
    program: &'p Program,
    resolution: &Resolution<'p>,
    symbols: &crate::program::Symbols<'p>,
) -> HashMap<(Span, String), String> {
    let mut raw = SpanKeys::new();
    span_keys_from(program, resolution, symbols, 0, &mut raw);
    span_keys_final(raw)
}

/// [lsp-std-front] What [`written_keys_by_span`] gathers before it keeps
/// only the agreed entries that differ from the written name: every
/// (span, written name) with the one key all its references resolve to, or
/// `None` where two disagree. Gathering is order-free (a disagreement
/// anywhere is final), so the files may be gathered in parts — std's once,
/// a project's per analysis — and merged (`check_once`).
pub type SpanKeys = HashMap<(Span, String), Option<String>>;

/// [lsp-std-front] Gathers the references of the files from `first` on into
/// `out`.
pub fn span_keys_from<'p>(
    program: &'p Program,
    resolution: &Resolution<'p>,
    symbols: &crate::program::Symbols<'p>,
    first: usize,
    out: &mut SpanKeys,
) {
    for_each_type_ref_from(&program.modules, first, &mut |file, t| {
        let name = t.name.name.as_str();
        let key = key_in(symbols, resolution.scope_for_ref(file, t), name).unwrap_or(name).to_string();
        span_keys_add(out, (t.name.span, name.to_string()), Some(key));
    });
}

fn span_keys_add(out: &mut SpanKeys, at: (Span, String), key: Option<String>) {
    match out.get(&at) {
        None => {
            out.insert(at, key);
        }
        Some(Some(k)) if Some(k) == key.as_ref() => {}
        Some(_) => {
            out.insert(at, None);
        }
    }
}

/// [lsp-std-front] The agreed entries that differ from the written name.
pub fn span_keys_final(raw: SpanKeys) -> HashMap<(Span, String), String> {
    raw.into_iter()
        .filter_map(|(at, k)| k.filter(|k| *k != at.1).map(|k| (at, k)))
        .collect()
}

/// Every written type reference of the files from `first` on, by their
/// index in `modules`.
fn for_each_type_ref_from<M: std::borrow::Borrow<salvo_syntax::ast::Module>>(
    modules: &[M],
    first: usize,
    f: &mut dyn FnMut(usize, &salvo_syntax::ast::TypeRef),
) {
    use salvo_syntax::visit::{self as v, Visitor};
    struct Collect<'m> {
        file: usize,
        f: &'m mut dyn FnMut(usize, &salvo_syntax::ast::TypeRef),
    }
    impl Visitor for Collect<'_> {
        fn visit_type_ref(&mut self, t: &salvo_syntax::ast::TypeRef) {
            (self.f)(self.file, t);
        }
    }
    let out = f;
    for (file, ast) in modules.iter().enumerate().skip(first) {
        let ast: &salvo_syntax::ast::Module = ast.borrow();
        let mut c = Collect { file, f: &mut *out };
        for u in &ast.uses {
            v::walk_expr(&mut c, &u.handler);
        }
        for item in &ast.items {
            match item {
                Item::Fn(f) => v::walk_fn(&mut c, f),
                Item::Struct(s) => {
                    for f in &s.fields {
                        v::walk_field_decl(&mut c, f);
                    }
                    for f in &s.fns {
                        v::walk_fn(&mut c, f);
                    }
                    for q in &s.auto_qualifiers {
                        v::walk_type_ref(&mut c, q);
                    }
                }
                Item::Effect(e) => {
                    for r in &e.prereqs {
                        v::walk_effect_ref(&mut c, r);
                    }
                    for f in &e.fns {
                        v::walk_fn(&mut c, f);
                    }
                }
                Item::Handler(h) => {
                    for p in &h.params {
                        v::walk_type(&mut c, &p.ty);
                    }
                    for r in h.effects.iter().flatten() {
                        v::walk_effect_ref(&mut c, r);
                    }
                    for t in &h.of {
                        v::walk_type(&mut c, t);
                    }
                    if let Some(f) = &h.init {
                        v::walk_fn(&mut c, f);
                    }
                    if let Some(m) = &h.mailbox {
                        v::walk_expr(&mut c, m);
                    }
                    for f in &h.state {
                        v::walk_field_decl(&mut c, f);
                    }
                    for f in &h.fns {
                        v::walk_fn(&mut c, f);
                    }
                }
                Item::Type(t) => {
                    if let Some(a) = &t.alias {
                        v::walk_type(&mut c, a);
                    }
                }
                Item::Qualifier(q) => {
                    v::walk_type(&mut c, &q.of);
                    for f in &q.field_overrides {
                        v::walk_field_decl(&mut c, f);
                    }
                    for f in &q.fns {
                        v::walk_fn(&mut c, f);
                    }
                }
                Item::Fns(g) => {
                    for f in &g.fns {
                        v::walk_fn(&mut c, f);
                    }
                }
                Item::Test(t) => v::walk_block(&mut c, &t.body),
                // [effect-alias] Its members resolve in its own file.
                Item::EffectAlias(a) => {
                    for r in &a.effects {
                        v::walk_effect_ref(&mut c, r);
                    }
                }
                _ => {}
            }
        }
    }
}
