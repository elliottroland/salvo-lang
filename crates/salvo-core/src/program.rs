//! The parsed program and its global symbol tables.
//!
//! For now Salvo uses a flat global namespace: imports are parsed but
//! cross-module name resolution simply merges every module's declarations.
//! Proper per-module scoping (honoring `import` and aliases) comes with the
//! full resolver.

use std::collections::HashMap;

use salvo_syntax::ast::{
    ParamsDecl,
    EffectDecl, FnDecl, HandlerDecl, Item, Module, QualifierDecl, StructDecl, TypeDecl, TypeRef,
};

use crate::source::{CompanionFile, ModulePath, SourceFile};

/// A parsed compilation: one AST per source file, in `SourceSet` order.
pub struct Program {
    pub files: Vec<SourceFile>,
    /// [lsp-std-front] Shared, so a language server can hand every analysis
    /// the same parsed and expanded std modules without cloning them (and
    /// at the same addresses, which the address-keyed checker tables of the
    /// `StdCache` rely on). A driver building its own program owns every
    /// `Arc` uniquely, and an expansion mutating one (`Arc::make_mut`) does
    /// not clone it.
    pub modules: Vec<std::sync::Arc<Module>>,
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
            .map(|(file, ast)| Unit { file, ast: &**ast })
    }

    /// [lsp-std-front] Modules a driver parsed and owns, as a `Program`
    /// holds them.
    pub fn share(modules: Vec<Module>) -> Vec<std::sync::Arc<Module>> {
        modules.into_iter().map(std::sync::Arc::new).collect()
    }
}

/// [type-identity] A declaration with a key: what `Symbols::key_of` takes.
/// A trait rather than any `T`, so passing a reference to one (`&&EffectDecl`
/// from an iterator) is a type error instead of the address of the
/// reference.
pub trait TypeDeclaration {}
impl TypeDeclaration for StructDecl {}
impl TypeDeclaration for EffectDecl {}
impl TypeDeclaration for HandlerDecl {}
impl TypeDeclaration for TypeDecl {}

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
    /// [platform-check] The module each qualifier is declared in, by
    /// declaration: a boundary check calls its `qualifies` by full path.
    pub qualifier_modules: Vec<(&'p QualifierDecl, &'p ModulePath)>,
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
    /// [type-identity] The key of every type declaration (struct, effect,
    /// alias, intrinsic or platform type, handler), by the declaration's
    /// address: what `key_of` answers. See `typekey`.
    pub decl_keys: HashMap<usize, &'p str>,
    /// [type-identity] The module every key is declared in.
    pub key_modules: HashMap<&'p str, &'p ModulePath>,
    /// [type-identity] The names more than one module declares: an emitter
    /// spells every declaration of one with its module's path, since a
    /// glob or star import of two would be ambiguous in the host.
    pub clashing: std::collections::HashSet<&'p str>,
    /// [type-identity] The program the tables were collected from, for
    /// [`Symbols::ref_key`].
    program: Option<&'p Program>,
    /// [type-identity] The key of every written type reference of `program`
    /// whose key is not its name, by the reference's address
    /// (`resolve::written_keys`), computed the first time a reference that
    /// may need it is asked about.
    ref_keys: std::sync::OnceLock<HashMap<usize, String>>,
}

impl<'p> Symbols<'p> {
    /// [platform-check] The module declaring `decl`.
    pub fn qualifier_module(&self, decl: &QualifierDecl) -> Option<&'p ModulePath> {
        self.qualifier_modules.iter().find(|(q, _)| std::ptr::eq(*q, decl)).map(|(_, m)| *m)
    }

    /// [type-identity] The key of a type declaration: its name, unless
    /// another module declared that name first.
    pub fn key_of<D: TypeDeclaration>(&self, decl: &D) -> Option<&'p str> {
        self.decl_keys.get(&(decl as *const D as usize)).copied()
    }

    /// [type-identity] The key of a declaration, or [name] (its written
    /// name) for one the tables do not hold.
    pub fn clashes(&self, key: &str) -> bool {
        self.clashing.contains(crate::typekey::plain(key))
    }

    pub fn key_or<'a, D: TypeDeclaration>(&self, decl: &D, name: &'a str) -> &'a str
    where
        'p: 'a,
    {
        self.key_of(decl).unwrap_or(name)
    }

    /// [type-identity] Hands [`Symbols::ref_key`] the program's written keys
    /// when a caller already has them (`resolve::written_keys` over a
    /// resolution of the same program), so the first clashing or undeclared
    /// name asked about does not resolve the program a second time
    /// (PIPELINE.md §6, prerequisite 5). A no-op once they are known.
    pub fn seed_ref_keys(&self, keys: &HashMap<usize, String>) {
        if self.ref_keys.get().is_none() {
            let _ = self.ref_keys.set(keys.clone());
        }
    }

    /// [type-identity] The key of the declaration a written type reference
    /// of the collected program names, in the scope of the file that wrote
    /// it: what a reading of a declaration's own field and parameter types
    /// must use, since the tables are keyed. A declared, unclashing name is
    /// its own key; anything else (a clashing name, an import alias, a
    /// generic parameter) is looked up in the program's resolved references,
    /// resolved once on first need (~1% of a check). A reference from
    /// another tree (a copy) is answered by its written name.
    pub fn ref_key<'a>(&'a self, r: &'a TypeRef) -> &'a str {
        let name = r.name.name.as_str();
        if !self.clashing.contains(name) && self.key_modules.contains_key(name) {
            return name;
        }
        let Some(program) = self.program else { return name };
        let keys = self.ref_keys.get_or_init(|| {
            let resolution = crate::resolve::resolve(program);
            crate::resolve::written_keys(program, &resolution, self)
        });
        keys.get(&(r as *const TypeRef as usize)).map(String::as_str).unwrap_or(name)
    }

    /// Collects symbols from every module of the program.
    pub fn collect(program: &'p Program) -> Self {
        let mut symbols = Symbols { program: Some(program), ..Symbols::default() };
        // [type-identity] Which module first declared each type name, and
        // which handler name: one namespace for types and effects, one for
        // handlers.
        let mut first_type: HashMap<&'p str, &'p ModulePath> = HashMap::new();
        let mut first_handler: HashMap<&'p str, &'p ModulePath> = HashMap::new();
        for unit in program.units() {
            let module = &unit.file.module;
            for item in &unit.ast.items {
                let (name, table) = match item {
                    Item::Struct(s) => (s.name.name.as_str(), &mut first_type),
                    Item::Effect(e) => (e.name.name.as_str(), &mut first_type),
                    Item::Type(t) => (t.name.name.as_str(), &mut first_type),
                    Item::Handler(h) => (h.name.name.as_str(), &mut first_handler),
                    _ => continue,
                };
                if table.get(name).is_some_and(|m| *m != module) {
                    symbols.clashing.insert(name);
                }
                table.entry(name).or_insert(module);
            }
        }
        // [type-identity] A handler named like a type of another module: two
        // namespaces in Salvo, one in each host's imports (0c item 13), so
        // the name clashes and the handler is the one keyed.
        let cross: Vec<&'p str> = first_handler
            .iter()
            .filter(|(n, m)| first_type.get(*n).is_some_and(|tm| tm != *m))
            .map(|(n, _)| *n)
            .collect();
        for name in &cross {
            symbols.clashing.insert(name);
        }
        for unit in program.units() {
            let module: &'p ModulePath = &unit.file.module;
            let key_in = |name: &'p str, first: &HashMap<&'p str, &'p ModulePath>| -> &'p str {
                match first.get(name) {
                    Some(m) if *m != module => crate::typekey::keyed(name, &module.to_string()),
                    _ => name,
                }
            };
            for item in &unit.ast.items {
                let (addr, key) = match item {
                    Item::Struct(s) => (s as *const StructDecl as usize, key_in(&s.name.name, &first_type)),
                    Item::Effect(e) => (e as *const EffectDecl as usize, key_in(&e.name.name, &first_type)),
                    Item::Type(t) => (t as *const TypeDecl as usize, key_in(&t.name.name, &first_type)),
                    Item::Handler(h) if first_type.get(h.name.name.as_str()).is_some_and(|m| *m != module) => (
                        h as *const HandlerDecl as usize,
                        crate::typekey::keyed(&h.name.name, &module.to_string()),
                    ),
                    Item::Handler(h) => (h as *const HandlerDecl as usize, key_in(&h.name.name, &first_handler)),
                    _ => continue,
                };
                symbols.decl_keys.insert(addr, key);
                symbols.key_modules.insert(key, module);
            }
        }
        let keys = symbols.decl_keys.clone();
        let key = |addr: usize, name: &'p str| -> &'p str { keys.get(&addr).copied().unwrap_or(name) };
        for unit in program.units() {
            for item in &unit.ast.items {
                match item {
                    Item::Fn(f) => symbols.fns.entry(&f.name.name).or_default().push(f),
                    // [fn-rename] A rename introduces no declaration: it is a
                    // scope-local name for one that already exists, and it is
                    // erased before emission.
                    Item::Rename(_) => {}
                    // [namespace] Flattened away before symbol collection.
                    Item::Namespace(_) => {}
                    Item::Struct(s) => {
                        symbols.structs.insert(key(s as *const StructDecl as usize, &s.name.name), s);
                    }
                    Item::Effect(e) => {
                        let k = key(e as *const EffectDecl as usize, &e.name.name);
                        symbols.effects.insert(k, e);
                        for f in &e.fns {
                            symbols.effect_of_fn.entry(&f.name.name).or_default().push(k);
                        }
                    }
                    Item::Handler(h) => {
                        let k = key(h as *const HandlerDecl as usize, &h.name.name);
                        symbols.handlers.insert(k, h);
                        symbols.handler_modules.insert(k, &unit.file.module);
                    }
                    Item::Params(g) => {
                        symbols.param_groups.insert(&g.name.name, g);
                    }
                    Item::Qualifier(q) => {
                        symbols.qualifiers.entry(&q.name.name).or_default().push(q);
                        symbols.qualifier_modules.push((q, &unit.file.module));
                    }
                    // An `intrinsic type` is the compiler's; anything else
                    // is an alias, since a bodiless non-intrinsic `type` is
                    // a parse error [decl-body].
                    // [platform-type] A platform type is opaque the same way:
                    // a name with no Salvo representation.
                    Item::Type(t) if t.intrinsic || t.platform => {
                        symbols.intrinsic_types.insert(key(t as *const TypeDecl as usize, &t.name.name), t);
                    }
                    Item::Type(t) => {
                        symbols.type_aliases.insert(key(t as *const TypeDecl as usize, &t.name.name), t);
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
                    // [effect-alias] Expanded away before symbol collection.
                    Item::EffectAlias(_) => {}
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
