//! The parsed program and its global symbol tables.
//!
//! For now Salvo uses a flat global namespace: imports are parsed but
//! cross-module name resolution simply merges every module's declarations.
//! Proper per-module scoping (honoring `import` and aliases) comes with the
//! full resolver.

use std::collections::HashMap;

use salvo_syntax::ast::{
    DefineFn, DefineHandler, DefineType, EffectDecl, FnDecl, HandlerDecl, Item, Module,
    QualifierDecl, StructDecl, TypeDecl,
};

use crate::source::{CompanionFile, SourceFile, SourceKind};

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
    /// Top-level functions (including `external fn` signatures), by name.
    /// Multiple entries are overloads.
    pub fns: HashMap<&'p str, Vec<&'p FnDecl>>,
    pub structs: HashMap<&'p str, &'p StructDecl>,
    pub effects: HashMap<&'p str, &'p EffectDecl>,
    pub handlers: HashMap<&'p str, &'p HandlerDecl>,
    pub qualifiers: HashMap<&'p str, &'p QualifierDecl>,
    pub type_aliases: HashMap<&'p str, &'p TypeDecl>,
    /// `internal type` declarations (mapped natively by each backend).
    pub internal_types: HashMap<&'p str, &'p TypeDecl>,
    /// `external type` declarations (mapped via `define type`).
    pub external_types: HashMap<&'p str, &'p TypeDecl>,
    /// Backend `define fn` templates, by function name.
    pub define_fns: HashMap<&'p str, Vec<&'p DefineFn>>,
    /// Backend `define type` templates, by type name.
    pub define_types: HashMap<&'p str, &'p DefineType>,
    /// Backend `define handler` templates, by handler name.
    pub define_handlers: HashMap<&'p str, &'p DefineHandler>,
    /// Effect-member function name -> owning effect name.
    pub effect_of_fn: HashMap<&'p str, &'p str>,
}

impl<'p> Symbols<'p> {
    /// Collects symbols from every module of the program.
    pub fn collect(program: &'p Program) -> Self {
        let mut symbols = Symbols::default();
        for unit in program.units() {
            let is_define_file = unit.file.kind == SourceKind::BackendDefine;
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
                    Item::Qualifier(q) => {
                        symbols.qualifiers.insert(&q.name.name, q);
                    }
                    Item::Type(t) => {
                        use salvo_syntax::ast::BackingMod;
                        match (t.backing, &t.alias) {
                            (Some(BackingMod::Internal), _) => {
                                symbols.internal_types.insert(&t.name.name, t);
                            }
                            (Some(BackingMod::External), _) => {
                                symbols.external_types.insert(&t.name.name, t);
                            }
                            (None, Some(_)) => {
                                symbols.type_aliases.insert(&t.name.name, t);
                            }
                            (None, None) => {
                                // A bodiless `type` outside std: treat like
                                // an external type.
                                symbols.external_types.insert(&t.name.name, t);
                            }
                        }
                    }
                    Item::DefineFn(d) if is_define_file => {
                        symbols
                            .define_fns
                            .entry(&d.sig.name.name)
                            .or_default()
                            .push(d);
                    }
                    Item::DefineType(d) if is_define_file => {
                        symbols.define_types.insert(&d.name.name, d);
                    }
                    Item::DefineHandler(d) if is_define_file => {
                        symbols.define_handlers.insert(&d.name.name, d);
                    }
                    Item::DefineFn(_) | Item::DefineType(_) | Item::DefineHandler(_) => {}
                    Item::Import(_) => {}
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

    /// Finds the best `define fn` template for a call with `arg_count`
    /// arguments.
    pub fn resolve_define_fn(&self, name: &str, arg_count: usize) -> Option<&'p DefineFn> {
        let overloads = self.define_fns.get(name)?;
        overloads
            .iter()
            .find(|d| arity_matches(&d.sig.params, arg_count))
            .or_else(|| overloads.first())
            .copied()
    }

    /// All `define fn` templates whose arity accepts `arg_count`
    /// arguments — emitters use this in unchecked contexts to detect
    /// ambiguous dispatch [backend-never-wrong].
    pub fn defines_matching_arity(&self, name: &str, arg_count: usize) -> Vec<&'p DefineFn> {
        self.define_fns
            .get(name)
            .map(|overloads| {
                overloads
                    .iter()
                    .filter(|d| arity_matches(&d.sig.params, arg_count))
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

/// [decl-explicit] Validates the `define fn` ↔ `external fn` pairing for a
/// backend: every define must implement exactly one external declaration,
/// and no external may be implemented twice. The external carries the
/// contract (effects, deductions, return type) — the define supplies only
/// the native template — so a define with no external has no contract at
/// all, and two defines for one external make dispatch ambiguous.
///
/// Matching is by name, then by arity, then by parameter *base type names*
/// (the same key `define_for_decl` uses in the emitters, so overloaded
/// externals like `size(Str)` / `size(List<T>)` pair up correctly).
/// `backend` names the backend in the messages.
pub fn check_define_pairing(
    program: &Program,
    symbols: &Symbols<'_>,
    backend: &str,
) -> Vec<String> {
    let mut errors = Vec::new();
    // define -> the externals it could implement.
    for unit in program.units() {
        if unit.file.kind != SourceKind::BackendDefine {
            continue;
        }
        for item in &unit.ast.items {
            let Item::DefineFn(d) = item else { continue };
            let name = d.sig.name.name.as_str();
            let candidates: Vec<&FnDecl> = symbols
                .fns
                .get(name)
                .map(|decls| {
                    decls
                        .iter()
                        .copied()
                        .filter(|f| {
                            f.body.is_none()
                                && f.params.len() == d.sig.params.len()
                                && params_pair(&f.params, &d.sig.params)
                        })
                        .collect()
                })
                .unwrap_or_default();
            match candidates.len() {
                1 => {}
                0 => errors.push(format!(
                    "{}: {backend} `define fn {name}` implements no `external fn` \
                     declaration — a define supplies the native template, the \
                     external declares the contract (effects, deductions, \
                     return type), so every define needs one",
                    unit.file.name
                )),
                _ => errors.push(format!(
                    "{}: {backend} `define fn {name}` matches {} `external fn` \
                     declarations; the pairing must be one-to-one (make the \
                     parameter types distinguish them)",
                    unit.file.name,
                    candidates.len()
                )),
            }
        }
    }
    // Two defines for one external.
    for (name, defines) in &symbols.define_fns {
        for (i, a) in defines.iter().enumerate() {
            for b in &defines[i + 1..] {
                if a.sig.params.len() == b.sig.params.len()
                    && params_pair(&a.sig.params, &b.sig.params)
                {
                    errors.push(format!(
                        "{backend} `define fn {name}` is defined twice for the \
                         same signature; the `define`/`external` pairing must \
                         be one-to-one"
                    ));
                }
            }
        }
    }
    errors
}

/// Whether two parameter lists agree on variadic-ness and base type names
/// (`Mut List<T>` and `List<T>` pair; `Str` and `List<T>` do not).
fn params_pair(a: &[salvo_syntax::ast::Param], b: &[salvo_syntax::ast::Param]) -> bool {
    a.iter().zip(b).all(|(x, y)| {
        x.variadic == y.variadic
            && match (base_name(&x.ty), base_name(&y.ty)) {
                (Some(x), Some(y)) => x == y,
                // Shapes without a single base name (unions, tuples, fn
                // types) are not distinguished — lenient, like the
                // emitters' own pairing.
                _ => true,
            }
    })
}

/// The base type name of an AST type, ignoring qualifiers and nullability.
fn base_name(ty: &salvo_syntax::ast::Type) -> Option<&str> {
    use salvo_syntax::ast::Type;
    match ty {
        Type::Named { base, .. } => Some(base.name.name.as_str()),
        Type::Nullable { inner, .. } => base_name(inner),
        Type::QualifiedGroup { base, .. } => base_name(base),
        Type::Array { .. } => Some("[]"),
        _ => None,
    }
}
