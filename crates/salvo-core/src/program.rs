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

use crate::source::{CompanionFile, SourceFile};

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
    pub qualifiers: HashMap<&'p str, &'p QualifierDecl>,
    pub type_aliases: HashMap<&'p str, &'p TypeDecl>,
    /// `intrinsic type` declarations (mapped natively by each backend)
    /// [backend-intrinsic].
    pub intrinsic_types: HashMap<&'p str, &'p TypeDecl>,
    /// Effect-member function name -> owning effect name.
    pub effect_of_fn: HashMap<&'p str, &'p str>,
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
                    Item::Struct(s) => {
                        symbols.structs.insert(&s.name.name, s);
                    }
                    Item::Effect(e) => {
                        symbols.effects.insert(&e.name.name, e);
                        for f in &e.fns {
                            symbols.effect_of_fn.insert(&f.name.name, &e.name.name);
                        }
                    }
                    Item::Handler(h) => {
                        symbols.handlers.insert(&h.name.name, h);
                    }
                    Item::Params(g) => {
                        symbols.param_groups.insert(&g.name.name, g);
                    }
                    Item::Qualifier(q) => {
                        symbols.qualifiers.insert(&q.name.name, q);
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
