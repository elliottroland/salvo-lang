//! Per-module name resolution.
//!
//! Each source file gets a [`ModuleScope`]: the names visible to code in
//! that file. Visibility rules per LANGUAGE.md [mod-visibility]:
//!
//! * everything declared in the same module,
//! * everything in `core.*` (implicitly imported),
//! * everything named by an `import` (with optional `as` alias)
//!   [mod-import].
//!
//! Unresolved and ambiguous imports are reported as errors [mod-import].

use std::collections::HashMap;

use salvo_syntax::ast::{
    ParamsDecl,
    EffectDecl, FnDecl, HandlerDecl, ImportDecl, Item, QualifierDecl, RefnDecl, RenameDecl,
    StructDecl, TypeDecl,
};

use crate::diag::FileDiagnostic;
use crate::program::Program;
use crate::source::ModulePath;
use salvo_syntax::Span;

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
    /// [fn-overload-scope] Which rung of the visibility ladder brought this
    /// overload into scope. Overload selection prefers the *most specific*
    /// rung that has a candidate matching the arguments, so a module's own
    /// `map` wins over `core`'s without either being an error.
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
    /// Implicitly visible `core.*`.
    Core,
    /// Named by an `import` in this file.
    Import,
    /// Declared by this file's own module.
    Own,
}

impl Rung {
    /// How the rung reads in a diagnostic.
    pub fn describe(self) -> &'static str {
        match self {
            Rung::Core => "the standard library",
            Rung::Import => "an import in this file",
            Rung::Own => "this module",
        }
    }
}

/// Where a declaration's *name* is written [lsp-definition]: the file it
/// lives in and the span of its identifier. Recorded for every visible
/// non-fn declaration (fns are identified precisely by [`FnKey`] through
/// the checker's `fn_refs` table) so tooling can jump to definitions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DefSite {
    /// Index into `Program::files`.
    pub file: usize,
    pub span: Span,
}

/// The names visible to one source file.
#[derive(Default)]
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
    /// `params` groups visible here [implicit-group].
    pub param_groups: HashMap<&'p str, &'p ParamsDecl>,
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
    /// `intrinsic type` declarations visible here.
    pub opaque_types: HashMap<&'p str, &'p TypeDecl>,
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

/// One module's own declarations, prior to visibility merging. Non-fn
/// items carry their declaring file index so collision diagnostics
/// [mod-collision] can point at the right file.
#[derive(Default)]
struct ModuleItems<'p> {
    fns: Vec<(FnKey, &'p FnDecl)>,
    structs: Vec<(usize, &'p StructDecl)>,
    effects: Vec<(usize, &'p EffectDecl)>,
    param_groups: Vec<(usize, &'p ParamsDecl)>,
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
}

impl<'p> ModuleItems<'p> {
    fn has_name(&self, name: &str) -> bool {
        self.name_ref(name).is_some()
    }

    /// The declaration's own `&'p str` for `name`, so a synthesized
    /// lookup key (a dot-name assembled from import path segments
    /// [name-dot]) can be exchanged for one that lives as long as the
    /// program.
    fn name_ref(&self, name: &str) -> Option<&'p str> {
        let hit = |n: &'p str| (n == name).then_some(n);
        self.fns
            .iter()
            .find_map(|(_, f)| hit(f.name.name.as_str()))
            .or_else(|| self.structs.iter().find_map(|(_, s)| hit(s.name.name.as_str())))
            .or_else(|| self.effects.iter().find_map(|(_, e)| hit(e.name.name.as_str())))
            .or_else(|| {
                self.param_groups
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
        for (f, g) in &self.param_groups {
            out.push((NameKind::ParamGroup, g.name.name.as_str(), None, *f, g.name.span));
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
    ParamGroup,
}

impl NameKind {
    fn describe(self) -> &'static str {
        match self {
            NameKind::Struct => "struct",
            NameKind::ParamGroup => "params group",
            NameKind::Effect => "effect",
            NameKind::Handler => "handler",
            NameKind::Qualifier => "qualifier",
            NameKind::TypeAlias => "type alias",
            NameKind::OpaqueType => "type",
        }
    }
}

/// Visibility precedence of a scope entry [mod-collision]: own-module
/// declarations override implicit `core.*` visibility, and explicit
/// imports override `core.*`; everything else is a collision.
#[derive(Clone, Copy, PartialEq)]
enum Level {
    Core,
    Own,
    Import,
}

impl Level {
    /// [fn-overload-scope] The same distinction, as the ladder rung an
    /// overload entry records.
    fn rung(self) -> Rung {
        match self {
            Level::Core => Rung::Core,
            Level::Own => Rung::Own,
            Level::Import => Rung::Import,
        }
    }
}

pub fn resolve(program: &Program) -> Resolution<'_> {
    // Pass 1: collect each module's own declarations (all files of the
    // module contribute).
    let mut by_module: HashMap<&ModulePath, ModuleItems<'_>> = HashMap::new();
    let mut unexpanded_iter_fns: Vec<FileDiagnostic> = Vec::new();
    for (file_idx, (file, ast)) in program.files.iter().zip(&program.modules).enumerate() {
        let items = by_module.entry(&file.module).or_default();
        for (item_idx, item) in ast.items.iter().enumerate() {
            match item {
                // [iter-fn] An `iter fn` must be expanded before resolution
                // (`parse_module` does it; `parse_module_deferred` hands the
                // duty to a program-level `expand_iter_fns_with`). Checking
                // it as an ordinary fn would be silently different behavior,
                // so a survivor is a loud integration error, never checked.
                Item::Fn(f) if f.is_iter => {
                    unexpanded_iter_fns.push(FileDiagnostic::error(
                        file_idx,
                        f.name.span,
                        "internal: this `iter fn` reached resolution unexpanded — \
                         the driver that parsed it skipped the `iter fn` expansion \
                         (`expand_iter_fns_with`); this is a compiler-integration \
                         bug, not a mistake in this program",
                    ));
                }
                Item::Fn(f) => items.fns.push((
                    FnKey {
                        file: file_idx,
                        item: item_idx,
                    },
                    f,
                )),
                Item::Struct(s) => items.structs.push((file_idx, s)),
                Item::Effect(e) => items.effects.push((file_idx, e)),
                Item::Params(g) => items.param_groups.push((file_idx, g)),
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
                _ => {}
            }
        }
    }

    let mut core_modules: Vec<&ModulePath> = by_module
        .keys()
        .filter(|m| m.0.first().is_some_and(|p| p == "core"))
        .copied()
        .collect();
    // Sorted for determinism: scope-entry and overload-candidate order
    // must not depend on hash-map iteration [mod-collision].
    core_modules.sort_by_key(|m| m.to_string());

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
        for (_, s) in &items.structs {
            record(&s.name.name, &s.name.name);
        }
        for (_, e) in &items.effects {
            record(&e.name.name, &e.name.name);
            for f in &e.fns {
                record(&f.name.name, &e.name.name);
            }
        }
        for (_, h) in &items.handlers {
            record(&h.name.name, &h.name.name);
        }
        for (_, q) in &items.qualifiers {
            record(&q.name.name, &q.name.name);
        }
        for (_, t) in &items.type_aliases {
            record(&t.name.name, &t.name.name);
        }
        for (_, t) in &items.opaque_types {
            record(&t.name.name, &t.name.name);
        }
    }

    let mut errors = unexpanded_iter_fns;

    // [mod-collision] Declaration-level collisions are reported once,
    // globally: a same-kind same-name duplicate within one module, and a
    // same-kind same-name collision between two implicitly visible
    // `core.*` modules (which would otherwise resolve by hash-map
    // ordering). Fns are exempt — same-name fns form overload sets.
    {
        let mut sorted_modules: Vec<&&ModulePath> = by_module.keys().collect();
        sorted_modules.sort_by_key(|m| m.to_string());
        // Keyed by kind, name **and subject** — see `non_fn_names`.
        let mut core_seen: HashMap<(NameKind, &str, Option<String>), (&ModulePath, usize, Span)> =
            HashMap::new();
        for module in sorted_modules {
            let items = &by_module[*module];
            let mut module_seen: HashMap<(NameKind, &str, Option<String>), Span> = HashMap::new();
            let is_core = module.0.first().is_some_and(|p| p == "core");
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

    // [fn-overload-duplicate] Two fns of one module with the same name *and*
    // the same parameter types are duplicates, not overloads: nothing at a
    // call site could tell them apart, since neither parameter names nor
    // return types take part in selection [fn-overload-rank]. Reported at the
    // second declaration (user decision 2026-09-07).
    {
        let mut sorted_modules: Vec<&&ModulePath> = by_module.keys().collect();
        sorted_modules.sort_by_key(|m| m.to_string());
        for module in sorted_modules {
            let items = &by_module[*module];
            let mut seen: HashMap<(&str, Vec<String>), Span> = HashMap::new();
            for (key, f) in &items.fns {
                let mut generics: Vec<&str> =
                    f.generics.iter().map(|g| g.name.as_str()).collect();
                for (g, _) in &f.generic_canbe {
                    if !generics.contains(&g.name.as_str()) {
                        generics.push(g.name.as_str());
                    }
                }
                let sig = crate::refine::param_type_signature(&f.params, &generics);
                if seen.contains_key(&(f.name.name.as_str(), sig.clone())) {
                    let shown = sig.join(", ");
                    errors.push(FileDiagnostic::error(
                        key.file,
                        f.name.span,
                        format!(
                            "duplicate fn `{}({shown})` in module `{module}`: an \
                             overload set is distinguished by parameter *types*, \
                             so this one could never be called. Parameter names \
                             and return types take no part in choosing an \
                             overload [fn-overload-rank]",
                            f.name.name
                        ),
                    ));
                    continue;
                }
                seen.insert((f.name.name.as_str(), sig), f.name.span);
            }
        }
    }

    // Pass 2: build one scope per file.
    let mut scopes = Vec::with_capacity(program.files.len());
    for (file_idx, (file, ast)) in program.files.iter().zip(&program.modules).enumerate() {
        let mut scope = ModuleScope::default();
        // Provenance of every non-fn scope entry [mod-collision]:
        // (level, module) per (kind, visible name), driving the
        // override-vs-collision decision below.
        // [qual-overload] Keyed by kind, name **and subject**: two core
        // modules may each declare a `NonEmpty` so long as their subjects
        // differ, so visibility has to be tracked per declaration rather than
        // per name (see `non_fn_names`).
        let mut provenance: HashMap<(NameKind, &str, Option<String>), (Level, &ModulePath)> =
            HashMap::new();
        let mut ctx = AddCtx {
            file_idx,
            provenance: &mut provenance,
            errors: &mut errors,
        };
        // core.* is implicitly visible everywhere — except in a core module's
        // *own* files, which add themselves at `Level::Own` just below. Adding
        // both put every one of that module's fns into the overload set
        // **twice** (the same declaration at two rungs). Ordinary resolution
        // survived it, since the duplicates are identical and `Own` outranks
        // `Core` either way; the refinement matcher did not — it counts
        // matches, so a `refn` inside a core module reported "matches more
        // than one `add` in scope" and no std module could carry a refinement
        // at all [qual-refn-match].
        for m in &core_modules {
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
            // from `core.*` (which is visible everywhere) and not through
            // an import.
            scope.refns.extend(items.refns.iter().copied());
            // [fn-rename] Module-scoped and order-independent, like every
            // other module-level declaration: a module-level rename is in
            // force for the whole module, in every file of it.
            scope.renames.extend(items.renames.iter().copied());
        }
        // Explicit imports.
        for item in &ast.items {
            let Item::Import(import) = item else { continue };
            resolve_import(&mut scope, &by_module, import, file_idx, &mut ctx);
        }
        // [name-dot] Dot-name rules, checked against everything visible
        // here (own module, core, imports).
        check_dot_names(&scope, ast, file, file_idx, &mut errors);
        scopes.push(scope);
    }

    Resolution {
        scopes,
        declared_in,
        errors,
    }
}

/// Per-file state threaded through scope building [mod-collision].
struct AddCtx<'e, 'p> {
    file_idx: usize,
    provenance: &'e mut HashMap<(NameKind, &'p str, Option<String>), (Level, &'p ModulePath)>,
    errors: &'e mut Vec<FileDiagnostic>,
}

impl<'e, 'p> AddCtx<'e, 'p> {
    /// Decides whether a non-fn entry may land in the scope
    /// [mod-collision]. Own-module declarations and explicit imports
    /// override implicit `core.*` visibility; an import colliding with an
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
///   [kt-qual-mangling] [rs-fn-mangling]. Checked against the whole
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
    // Structs declared in this file, with their generic arity.
    let mut local_structs: HashMap<&str, usize> = HashMap::new();
    for item in &ast.items {
        if let Item::Struct(s) = item {
            local_structs.insert(s.name.name.as_str(), s.generics.len());
        }
    }
    let mut dotted: Vec<(&str, Span)> = Vec::new();
    for item in &ast.items {
        match item {
            Item::Struct(s) => dotted.push((s.name.name.as_str(), s.name.span)),
            Item::Qualifier(q) => dotted.push((q.name.name.as_str(), q.name.span)),
            _ => continue,
        }
    }
    for (name, span) in dotted {
        let Some((ns, _member)) = name.split_once('.') else {
            continue;
        };
        match local_structs.get(ns) {
            None => errors.push(FileDiagnostic::error(
                file_idx,
                span,
                format!(
                    "`{ns}` in the dot-name `{name}` must be a struct declared in this \
                     file [name-dot]"
                ),
            )),
            Some(0) => {}
            Some(_) => errors.push(FileDiagnostic::error(
                file_idx,
                span,
                format!(
                    "`{ns}` is generic, so it cannot namespace `{name}`: Kotlin emits the \
                     member as a nested class, which cannot use the outer type parameters \
                     [name-dot]"
                ),
            )),
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
    for (key, f) in &items.fns {
        if want(&f.name.name) {
            let name = visible_as(&f.name.name);
            scope
                .fns
                .entry(name)
                .or_default()
                .push(FnEntry {
                    key: *key,
                    decl: f,
                    rung: level.rung(),
                    module,
                });
            origin(name, scope);
        }
    }
    for (file, s) in &items.structs {
        if want(&s.name.name) {
            let name = visible_as(&s.name.name);
            if ctx.admit(NameKind::Struct, name, module, level, import_span) {
                scope.structs.insert(name, s);
                scope.struct_files.insert(name, *file);
                origin(name, scope);
                def_site(name, *file, s.name.span, scope);
            }
        }
    }
    for (file, e) in &items.effects {
        if want(&e.name.name) {
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
    for (file, g) in &items.param_groups {
        if want(&g.name.name) {
            let name = visible_as(&g.name.name);
            if ctx.admit(NameKind::ParamGroup, name, module, level, import_span) {
                scope.param_groups.insert(name, g);
                origin(name, scope);
                def_site(name, *file, g.name.span, scope);
            }
        }
    }
    for (file, h) in &items.handlers {
        if want(&h.name.name) {
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
            let name = visible_as(&t.name.name);
            if ctx.admit(NameKind::TypeAlias, name, module, level, import_span) {
                scope.type_aliases.insert(name, t);
                origin(name, scope);
                def_site(name, *file, t.name.span, scope);
            }
        }
    }
    for (file, t) in &items.opaque_types {
        if want(&t.name.name) {
            let name = visible_as(&t.name.name);
            if ctx.admit(NameKind::OpaqueType, name, module, level, import_span) {
                scope.opaque_types.insert(name, t);
                origin(name, scope);
                def_site(name, *file, t.name.span, scope);
            }
        }
    }
}

fn resolve_import<'p>(
    scope: &mut ModuleScope<'p>,
    by_module: &HashMap<&'p ModulePath, ModuleItems<'p>>,
    import: &'p ImportDecl,
    file_idx: usize,
    ctx: &mut AddCtx<'_, 'p>,
) {
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
