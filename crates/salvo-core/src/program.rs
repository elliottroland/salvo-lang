//! The parsed program and its global symbol tables.
//!
//! For now Salvo uses a flat global namespace: imports are parsed but
//! cross-module name resolution simply merges every module's declarations.
//! Proper per-module scoping (honoring `import` and aliases) comes with the
//! full resolver.

use std::collections::HashMap;

use salvo_syntax::ast::{
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

/// Program-wide symbol tables: types by key [type-identity], which a
/// declaration determines alone, and the few by name that a name settles —
/// std's intrinsics [intrinsic-std-only], and qualifiers, which types carry
/// by name. Everything a *written* name means is a file's scope's question
/// (`resolve::ModuleScope`), never these tables' (PIPELINE.md C1).
#[derive(Default)]
pub struct Symbols<'p> {
    /// [intrinsic-std-only] std's `intrinsic fn`s, by name: an intrinsic is
    /// the compiler's, declared once by std, so its name is its identity.
    /// Multiple entries are overloads.
    pub intrinsic_fns: HashMap<&'p str, Vec<&'p FnDecl>>,
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
    /// [type-identity] The key of every type declaration (struct, effect,
    /// alias, intrinsic or platform type, handler), by the declaration's
    /// address: what `key_of` answers. See `typekey`.
    pub decl_keys: HashMap<usize, &'p str>,
    /// [decl-id] The key of every type declaration, by its id: what the IR,
    /// which names declarations by id, reads a key from.
    pub id_keys: HashMap<crate::decl_id::DeclId, &'p str>,
    /// [type-identity] The module every key is declared in.
    pub key_modules: HashMap<&'p str, &'p ModulePath>,
    /// [type-identity] The names more than one module declares: an emitter
    /// spells every declaration of one with its module's path, since a
    /// glob or star import of two would be ambiguous in the host.
    pub clashing: std::collections::HashSet<&'p str>,
    /// [type-identity] The program the tables were collected from, for
    /// [`Symbols::ref_key`].
    program: Option<&'p Program>,
    /// [type-identity] [name-table] The key of every written type reference
    /// of `program` whose key is not its name, by the reference's address
    /// (`resolve::written_keys`) and, for a copy, by its node, span and name
    /// (`resolve::written_keys_by_span`): read off the resolved names and
    /// handed over by whoever resolved them (`seed_ref_keys`).
    ref_keys: std::sync::OnceLock<(HashMap<usize, String>, HashMap<crate::resolve::CopyKey, String>)>,
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

    /// [decl-id] The key of the type declaration `id` names.
    pub fn key_of_id(&self, id: crate::decl_id::DeclId) -> Option<&'p str> {
        self.id_keys.get(&id).copied()
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

    /// [name-table] Symbols for a program nothing has checked (the ABI
    /// stamp of a platform root, a dependency's stamp): collected, with the
    /// program's names resolved and seeded for [`Symbols::ref_key`].
    pub fn collect_resolved(program: &'p Program) -> Self {
        let symbols = Symbols::collect(program);
        let resolution = crate::resolve::resolve(program);
        symbols.seed_ref_keys(
            &crate::resolve::written_keys(program, &resolution, &symbols),
            &crate::resolve::written_keys_by_span(program, &resolution, &symbols),
        );
        symbols
    }

    /// [type-identity] [name-table] Hands [`Symbols::ref_key`] the program's
    /// written keys (`resolve::written_keys` and `written_keys_by_span` over a
    /// resolution of the same program). A no-op once they are known.
    pub fn seed_ref_keys(&self, keys: &HashMap<usize, String>, copies: &HashMap<crate::resolve::CopyKey, String>) {
        if self.ref_keys.get().is_none() {
            let _ = self.ref_keys.set((keys.clone(), copies.clone()));
        }
    }

    /// [type-identity] [name-table] The key of the declaration a written type
    /// reference of the collected program names, in the scope of the file
    /// that wrote it: what a reading of a declaration's own field and
    /// parameter types must use, since the tables are keyed. A declared,
    /// unclashing name is its own key; anything else (a clashing name, an
    /// import alias, a generic parameter) is read off the program's resolved
    /// names, which the checker hands over (`seed_ref_keys`) — by address, or
    /// for a copy by node, span and name.
    ///
    /// Unseeded (a caller that never checked the program), the names are
    /// resolved here, once: a debug build refuses that, so a path that
    /// forgets to seed is found by the suite rather than paying a second
    /// resolution in release.
    pub fn ref_key<'a>(&'a self, r: &'a TypeRef) -> &'a str {
        let name = r.name.name.as_str();
        if !self.clashing.contains(name) && self.key_modules.contains_key(name) {
            return name;
        }
        let Some(program) = self.program else { return name };
        let (keys, copies) = self.ref_keys.get_or_init(|| {
            debug_assert!(false, "Symbols::ref_key before seed_ref_keys: resolve the names once and seed them");
            let resolution = crate::resolve::resolve(program);
            (
                crate::resolve::written_keys(program, &resolution, self),
                crate::resolve::written_keys_by_span(program, &resolution, self),
            )
        });
        if let Some(k) = keys.get(&(r as *const TypeRef as usize)) {
            return k;
        }
        crate::resolve::copy_key(r).and_then(|at| copies.get(&at)).map(String::as_str).unwrap_or(name)
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
        // [decl-id] Which std module first declared each name: std's own
        // text is fixed, so its first declarer is too.
        let mut first_std_type: HashMap<&'p str, &'p ModulePath> = HashMap::new();
        let mut first_std_handler: HashMap<&'p str, &'p ModulePath> = HashMap::new();
        for unit in program.units().filter(|u| u.file.is_std) {
            for item in &unit.ast.items {
                let (name, table) = match item {
                    Item::Struct(s) => (s.name.name.as_str(), &mut first_std_type),
                    Item::Effect(e) => (e.name.name.as_str(), &mut first_std_type),
                    Item::Type(t) => (t.name.name.as_str(), &mut first_std_type),
                    Item::Handler(h) => (h.name.name.as_str(), &mut first_std_handler),
                    _ => continue,
                };
                table.entry(name).or_insert(&unit.file.module);
            }
        }
        for unit in program.units() {
            let module: &'p ModulePath = &unit.file.module;
            let std = unit.file.is_std;
            // [decl-id] A key is a function of the declaration alone: std's
            // first declaration of a name is the name, every other
            // declaration `Name§module`, whatever else the program declares.
            let key_in = |name: &'p str, first: &HashMap<&'p str, &'p ModulePath>| -> &'p str {
                match first.get(name) {
                    Some(m) if std && *m == module => name,
                    _ => crate::typekey::keyed(name, &module.to_string()),
                }
            };
            let first_type = &first_std_type;
            let first_handler = &first_std_handler;
            for item in &unit.ast.items {
                use crate::decl_id::DeclKind;
                let (addr, key, kind, name) = match item {
                    Item::Struct(s) => (s as *const StructDecl as usize, key_in(&s.name.name, first_type), DeclKind::Struct, &s.name.name),
                    Item::Effect(e) => (e as *const EffectDecl as usize, key_in(&e.name.name, first_type), DeclKind::Effect, &e.name.name),
                    Item::Type(t) => (t as *const TypeDecl as usize, key_in(&t.name.name, first_type), DeclKind::Type, &t.name.name),
                    Item::Handler(h) if first_type.get(h.name.name.as_str()).is_some_and(|m| *m != module) => (
                        h as *const HandlerDecl as usize,
                        crate::typekey::keyed(&h.name.name, &module.to_string()),
                        DeclKind::Handler,
                        &h.name.name,
                    ),
                    Item::Handler(h) => (h as *const HandlerDecl as usize, key_in(&h.name.name, first_handler), DeclKind::Handler, &h.name.name),
                    _ => continue,
                };
                symbols.decl_keys.insert(addr, key);
                symbols.key_modules.insert(key, module);
                // [decl-id] The id the key spells (a type is not overloaded:
                // no signature, and the ordinal a duplicate would take is
                // not a key of its own).
                let id = crate::decl_id::DeclId::intern(crate::decl_id::DeclPath { module: module.clone(), kind, path: vec![name.clone()], sig: None, ordinal: 0 });
                symbols.id_keys.entry(id).or_insert(key);
            }
        }
        let keys = symbols.decl_keys.clone();
        let key = |addr: usize, name: &'p str| -> &'p str { keys.get(&addr).copied().unwrap_or(name) };
        for unit in program.units() {
            for item in &unit.ast.items {
                match item {
                    Item::Fn(f) if f.intrinsic => symbols.intrinsic_fns.entry(&f.name.name).or_default().push(f),
                    Item::Fn(_) => {}
                    // [fn-rename] A rename introduces no declaration: it is a
                    // scope-local name for one that already exists, and it is
                    // erased before emission.
                    Item::Rename(_) => {}
                    // [mod-namespace] Flattened away before symbol collection.
                    Item::Namespace(_) => {}
                    Item::Struct(s) => {
                        symbols.structs.insert(key(s as *const StructDecl as usize, &s.name.name), s);
                    }
                    Item::Effect(e) => {
                        let k = key(e as *const EffectDecl as usize, &e.name.name);
                        symbols.effects.insert(k, e);
                    }
                    Item::Handler(h) => {
                        let k = key(h as *const HandlerDecl as usize, &h.name.name);
                        symbols.handlers.insert(k, h);
                        symbols.handler_modules.insert(k, &unit.file.module);
                    }
                    Item::Fns(_) => {}
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
}

