//! The parsed program and its global symbol tables.
//!
//! For now Salvo uses a flat global namespace: imports are parsed but
//! cross-module name resolution simply merges every module's declarations.
//! Proper per-module scoping (honoring `import` and aliases) comes with the
//! full resolver.

use std::collections::HashMap;

use salvo_syntax::ast::{
    ParamsDecl,
    EffectDecl, FnDecl, HandlerDecl, Item, Module, QualifierDecl, StructDecl, TypeDecl,
};

use crate::source::{CompanionFile, ModulePath, SourceFile};

/// A parsed compilation: one AST per source file, in `SourceSet` order.
pub struct Program {
    pub files: Vec<SourceFile>,
    pub modules: Vec<Module>,
    /// Backend-native companion files, copied into the output when their
    /// module is reachable [backend-companion].
    pub companions: Vec<CompanionFile>,
}

/// A single parsed unit (file + AST).
pub struct Unit<'p> {
    pub file: &'p SourceFile,
    pub ast: &'p Module,
}

impl Program {
    pub fn units(&self) -> impl Iterator<Item = Unit<'_>> {
        self.files
            .iter()
            .zip(&self.modules)
            .map(|(file, ast)| Unit { file, ast })
    }

    /// [cmp-auto] Whether a **structural** implementation of `member` exists for
    /// the type named `ty` — an `auto fn member@Ty`, written or expanded from an
    /// `auto Group<self>` clause.
    ///
    /// This is what a backend's derive hangs on: a generated member is defined
    /// in terms of the host's own derived operation, so the derive must be
    /// present exactly when the member is. Before `auto` moved to the function
    /// level (user decision 2026-09-22) both emitters asked the *obligation
    /// clause* instead, which now answers the wrong question — a struct may have
    /// `auto fn cmp@Person` and no clause at all.
    pub fn has_auto_member(&self, ty: &str, member: &str) -> bool {
        self.modules.iter().flat_map(|m| &m.items).any(|item| {
            matches!(item, Item::Fn(f)
                if f.structural
                    && f.name.name == member
                    && f.scoped_to.as_ref().is_some_and(|t| t.name == ty))
        })
    }
}

/// Global symbol tables, keyed by simple name.
#[derive(Default)]
pub struct Symbols<'p> {
    /// Top-level functions (including bodiless `intrinsic fn`
    /// signatures), by name.
    /// Multiple entries are overloads.
    pub fns: HashMap<&'p str, Vec<&'p FnDecl>>,
    pub structs: HashMap<&'p str, &'p StructDecl>,
    pub effects: HashMap<&'p str, &'p EffectDecl>,
    pub handlers: HashMap<&'p str, &'p HandlerDecl>,
    /// [platform-handler] [platform-tree] The module a handler is declared
    /// in. Only a `platform handler` needs it — the host class implementing
    /// it lives in *that* module's `platform/` companion, so a `use` site in
    /// another module still has to name the right package (Kotlin) or mount
    /// (Rust).
    pub handler_modules: HashMap<&'p str, &'p ModulePath>,
    /// [qual-overload] Overload sets: one name may be declared over several
    /// subject types, and a backend picks by the subject in hand.
    pub qualifiers: HashMap<&'p str, Vec<&'p QualifierDecl>>,
    pub type_aliases: HashMap<&'p str, &'p TypeDecl>,
    /// `intrinsic type` declarations (mapped natively by each backend)
    /// [backend-intrinsic].
    pub intrinsic_types: HashMap<&'p str, &'p TypeDecl>,
    /// Effect-member function name -> owning effect name(s)
    /// [effect-member-overload]: several effects may declare the same
    /// member name (user decision 2026-09-14), so this is a multimap; the
    /// checker records which effect each call resolved to
    /// (`Checked::effect_calls`).
    pub effect_of_fn: HashMap<&'p str, Vec<&'p str>>,
    /// `params` groups by name [implicit-group]: bundles of implicit
    /// parameters, spread into a signature as `?Name<T>`.
    pub param_groups: HashMap<&'p str, &'p ParamsDecl>,
}

impl<'p> Symbols<'p> {
    /// Collects symbols from every module of the program.
    pub fn collect(program: &'p Program) -> Self {
        let mut symbols = Symbols::default();
        for unit in program.units() {
            for item in &unit.ast.items {
                match item {
                    Item::Fn(f) => symbols.fns.entry(&f.name.name).or_default().push(f),
                    // [fn-rename] A rename introduces no declaration: it is a
                    // scope-local name for one that already exists, and it is
                    // erased before emission.
                    Item::Rename(_) => {}
                    Item::Struct(s) => {
                        symbols.structs.insert(&s.name.name, s);
                    }
                    Item::Effect(e) => {
                        symbols.effects.insert(&e.name.name, e);
                        for f in &e.fns {
                            symbols
                                .effect_of_fn
                                .entry(&f.name.name)
                                .or_default()
                                .push(&e.name.name);
                        }
                    }
                    Item::Handler(h) => {
                        symbols.handlers.insert(&h.name.name, h);
                        symbols
                            .handler_modules
                            .insert(&h.name.name, &unit.file.module);
                    }
                    Item::Params(g) => {
                        symbols.param_groups.insert(&g.name.name, g);
                    }
                    Item::Qualifier(q) => {
                        symbols.qualifiers.entry(&q.name.name).or_default().push(q);
                    }
                    // An `intrinsic type` is the compiler's; anything else
                    // is an alias, since a bodiless non-intrinsic `type` is
                    // a parse error [decl-body].
                    Item::Type(t) if t.intrinsic => {
                        symbols.intrinsic_types.insert(&t.name.name, t);
                    }
                    Item::Type(t) => {
                        symbols.type_aliases.insert(&t.name.name, t);
                    }
                    Item::Import(_) => {}
                    // [qual-refn] A refinement declares no symbol of its
                    // own: it names a function declared elsewhere. Its
                    // index is built by `refine::collect`, which needs
                    // per-file *visibility* rather than the flat table.
                    Item::Refn(_) => {}
                    // [test-decl] Likewise: a `test` declares no symbol. In
                    // an annex it is an ordinary fn by now [test-run]; here
                    // it can only be one resolution is about to refuse
                    // [test-file].
                    Item::Test(_) => {}
                }
            }
        }
        symbols
    }

    /// Finds the best `fn` overload for a call with `arg_count` arguments.
    /// Falls back to the first overload when none matches exactly (variadic
    /// functions accept any remaining arity).
    pub fn resolve_fn(&self, name: &str, arg_count: usize) -> Option<&'p FnDecl> {
        let overloads = self.fns.get(name)?;
        overloads
            .iter()
            .find(|f| arity_matches(&f.params, arg_count))
            .or_else(|| overloads.first())
            .copied()
    }

    /// All `fn` overloads whose arity accepts `arg_count` arguments —
    /// emitters use this in unchecked contexts to detect ambiguous
    /// dispatch [backend-never-wrong].
    pub fn fns_matching_arity(&self, name: &str, arg_count: usize) -> Vec<&'p FnDecl> {
        self.fns
            .get(name)
            .map(|overloads| {
                overloads
                    .iter()
                    .filter(|f| arity_matches(&f.params, arg_count))
                    .copied()
                    .collect()
            })
            .unwrap_or_default()
    }

}

/// Whether a parameter list accepts `arg_count` arguments (variadics
/// accept any remaining arity).
fn arity_matches(params: &[salvo_syntax::ast::Param], arg_count: usize) -> bool {
    let required = params.iter().filter(|p| !p.variadic).count();
    let variadic = params.iter().any(|p| p.variadic);
    if variadic {
        arg_count >= required
    } else {
        arg_count == required
    }
}
