//! [decl-id] [mod-visibility] A file's scope by declaration id.
//!
//! A [`ModuleScope`] answers what a name means in one file, with pointers into
//! the AST: a scope is good for exactly the modules it was built over. A
//! [`ScopeIds`] says the same with [`DeclId`]s, interned names and interned
//! module paths, so it outlives any one program: the same declarations have
//! the same ids in every program holding them (`decl_id`). [`ScopeIds::view`]
//! turns it back into a `ModuleScope` over a program's modules — one lookup
//! per entry, none of the visibility work that built it (PIPELINE.md C4).

use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

use salvo_syntax::ast::{EffectAliasDecl, EffectDecl, FnDecl, FnsDecl, HandlerDecl, Item, Module, QualifierDecl, RefnDecl, RenameDecl, StructDecl, TypeDecl};
use salvo_syntax::Span;

use crate::decl_id::{DeclId, DeclIndex};
use crate::resolve::{DefSite, FnEntry, ModuleScope, Rung};
use crate::source::{ModulePath, SourceFile};

/// One overload in scope, by id: [`FnEntry`] without its pointer.
#[derive(Clone, Debug, PartialEq)]
pub struct FnEntryId {
    pub key: DeclId,
    pub file: usize,
    pub rung: Rung,
    pub module: &'static ModulePath,
}

/// [`ModuleScope`] by declaration id. Field for field the same, except that a
/// declaration is its id, a top-level refinement or rename is the module
/// whose ones the scope takes (they have no id), and a definition site is
/// the declaration whose name it is (its span is read from the program the
/// view is over, which may have numbered it again).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ScopeIds {
    pub fns: HashMap<&'static str, Vec<FnEntryId>>,
    pub structs: HashMap<&'static str, DeclId>,
    pub struct_files: HashMap<&'static str, usize>,
    pub effects: HashMap<&'static str, DeclId>,
    pub effect_files: HashMap<&'static str, usize>,
    pub fn_groups: HashMap<&'static str, DeclId>,
    pub handlers: HashMap<&'static str, DeclId>,
    pub qualifiers: HashMap<&'static str, Vec<DeclId>>,
    /// The modules whose top-level refinements the scope takes, in order.
    pub refn_modules: Vec<&'static ModulePath>,
    /// The modules whose `rename fn`s the scope takes, in order.
    pub rename_modules: Vec<&'static ModulePath>,
    pub type_aliases: HashMap<&'static str, DeclId>,
    pub effect_aliases: HashMap<&'static str, (DeclId, usize)>,
    pub opaque_types: HashMap<&'static str, DeclId>,
    pub opaque_type_files: HashMap<&'static str, usize>,
    pub effect_members: HashMap<&'static str, Vec<(DeclId, DeclId)>>,
    pub name_origins: HashMap<&'static str, Vec<&'static ModulePath>>,
    pub modules: Vec<&'static ModulePath>,
    pub def_sites: HashMap<&'static str, DeclId>,
}

/// The process's interned names and module paths: what a [`ScopeIds`] holds
/// in place of the AST's. As many as there are distinct names, like
/// declaration ids.
#[derive(Default)]
struct Pool {
    names: HashSet<&'static str>,
    modules: HashSet<&'static ModulePath>,
}

static POOL: Mutex<Option<Pool>> = Mutex::new(None);

impl Pool {
    fn name(&mut self, s: &str) -> &'static str {
        if let Some(found) = self.names.get(s) {
            return found;
        }
        let leaked: &'static str = Box::leak(s.to_string().into_boxed_str());
        self.names.insert(leaked);
        leaked
    }

    fn module(&mut self, m: &ModulePath) -> &'static ModulePath {
        if let Some(found) = self.modules.get(m) {
            return found;
        }
        let leaked: &'static ModulePath = Box::leak(Box::new(m.clone()));
        self.modules.insert(leaked);
        leaked
    }
}

fn with_pool<R>(f: impl FnOnce(&mut Pool) -> R) -> R {
    let mut pool = POOL.lock().unwrap_or_else(|e| e.into_inner());
    f(pool.get_or_insert_with(Pool::default))
}

/// [decl-id] Every declaration of one program by the address of its AST
/// node and by where its name is written: what [`ScopeIds::of`] reads a
/// pointer scope's ids off.
pub struct Addrs {
    by_addr: HashMap<usize, DeclId>,
    by_name: HashMap<(usize, Span), DeclId>,
}

fn addr<T>(t: &T) -> usize {
    t as *const T as usize
}

impl Addrs {
    pub fn of<M: std::borrow::Borrow<Module>>(decls: &DeclIndex, modules: &[M]) -> Addrs {
        let mut by_addr = HashMap::new();
        let mut by_name = HashMap::new();
        for (file, module) in modules.iter().enumerate() {
            for (i, item) in module.borrow().items.iter().enumerate() {
                let Some(id) = decls.item(file, i) else { continue };
                let (a, name) = match item {
                    Item::Fn(f) => (addr(f), f.name.span),
                    Item::Struct(s) => (addr(s), s.name.span),
                    Item::Type(t) => (addr(t), t.name.span),
                    Item::Effect(e) => {
                        for (j, m) in e.fns.iter().enumerate() {
                            if let Some(mid) = decls.member(file, i, j + 1) {
                                by_addr.insert(addr(m), mid);
                                by_name.insert((file, m.name.span), mid);
                            }
                        }
                        (addr(e), e.name.span)
                    }
                    Item::Handler(h) => (addr(h), h.name.span),
                    Item::Qualifier(q) => (addr(q), q.name.span),
                    Item::Fns(g) => (addr(g), g.name.span),
                    Item::EffectAlias(a) => (addr(a), a.name.span),
                    _ => continue,
                };
                by_addr.insert(a, id);
                by_name.insert((file, name), id);
            }
        }
        Addrs { by_addr, by_name }
    }

    fn id<T>(&self, t: &T) -> DeclId {
        *self.by_addr.get(&addr(t)).unwrap_or_else(|| panic!("[decl-id] a declaration in scope has no id"))
    }
}

impl ScopeIds {
    /// The ids of `scope`, a scope of the program `addrs` was built over.
    /// `refn_modules` and `rename_modules` say where its refinements and
    /// renames came from, which the pointer scope does not record.
    pub fn of(scope: &ModuleScope<'_>, addrs: &Addrs, refn_modules: &[&ModulePath], rename_modules: &[&ModulePath]) -> ScopeIds {
        with_pool(|pool| {
            let names = |m: &HashMap<&str, usize>, pool: &mut Pool| m.iter().map(|(k, v)| (pool.name(k), *v)).collect();
            fn ids<T>(m: &HashMap<&str, &T>, addrs: &Addrs, pool: &mut Pool) -> HashMap<&'static str, DeclId> {
                m.iter().map(|(k, d)| (pool.name(k), addrs.id(*d))).collect()
            }
            ScopeIds {
                fns: scope
                    .fns
                    .iter()
                    .map(|(k, es)| {
                        let es = es.iter().map(|e| FnEntryId { key: e.key, file: e.file, rung: e.rung, module: pool.module(e.module) }).collect();
                        (pool.name(k), es)
                    })
                    .collect(),
                structs: ids(&scope.structs, addrs, pool),
                struct_files: names(&scope.struct_files, pool),
                effects: ids(&scope.effects, addrs, pool),
                effect_files: names(&scope.effect_files, pool),
                fn_groups: ids(&scope.fn_groups, addrs, pool),
                handlers: ids(&scope.handlers, addrs, pool),
                qualifiers: scope.qualifiers.iter().map(|(k, qs)| (pool.name(k), qs.iter().map(|q| addrs.id(*q)).collect())).collect(),
                refn_modules: refn_modules.iter().map(|m| pool.module(m)).collect(),
                rename_modules: rename_modules.iter().map(|m| pool.module(m)).collect(),
                type_aliases: ids(&scope.type_aliases, addrs, pool),
                effect_aliases: scope.effect_aliases.iter().map(|(k, (a, f))| (pool.name(k), (addrs.id(*a), *f))).collect(),
                opaque_types: ids(&scope.opaque_types, addrs, pool),
                opaque_type_files: names(&scope.opaque_type_files, pool),
                effect_members: scope
                    .effect_members
                    .iter()
                    .map(|(k, ms)| (pool.name(k), ms.iter().map(|(e, f)| (addrs.id(*e), addrs.id(*f))).collect()))
                    .collect(),
                name_origins: scope.name_origins.iter().map(|(k, ms)| (pool.name(k), ms.iter().map(|m| pool.module(m)).collect())).collect(),
                modules: scope.modules.iter().map(|m| pool.module(m)).collect(),
                def_sites: scope
                    .def_sites
                    .iter()
                    .map(|(k, d)| {
                        let id = *addrs.by_name.get(&(d.file, d.span)).unwrap_or_else(|| panic!("[decl-id] a definition site of `{k}` names no declaration"));
                        (pool.name(k), id)
                    })
                    .collect(),
            }
        })
    }

    /// The pointer scope over `decls`' program.
    pub fn view<'p>(&self, decls: &Decls<'_, 'p>) -> ModuleScope<'p> {
        fn map<'p, T: ?Sized>(m: &HashMap<&'static str, DeclId>, get: impl Fn(DeclId) -> &'p T) -> HashMap<&'p str, &'p T> {
            m.iter().map(|(k, id)| (*k, get(*id))).collect()
        }
        let files = |m: &HashMap<&'static str, usize>| -> HashMap<&'p str, usize> { m.iter().map(|(k, f)| (*k as &'p str, *f)).collect() };
        ModuleScope {
            fns: self
                .fns
                .iter()
                .map(|(k, es)| {
                    let es = es.iter().map(|e| FnEntry { key: e.key, file: e.file, decl: decls.fun(e.key), rung: e.rung, module: e.module }).collect();
                    (*k as &'p str, es)
                })
                .collect(),
            structs: map(&self.structs, |id| decls.structure(id)),
            struct_files: files(&self.struct_files),
            effects: map(&self.effects, |id| decls.effect(id)),
            effect_files: files(&self.effect_files),
            fn_groups: map(&self.fn_groups, |id| decls.group(id)),
            handlers: map(&self.handlers, |id| decls.handler(id)),
            qualifiers: self.qualifiers.iter().map(|(k, qs)| (*k as &'p str, qs.iter().map(|q| decls.qualifier(*q)).collect())).collect(),
            refns: self.refn_modules.iter().flat_map(|m| decls.refns(m)).collect(),
            renames: self.rename_modules.iter().flat_map(|m| decls.renames(m)).collect(),
            type_aliases: map(&self.type_aliases, |id| decls.type_decl(id)),
            effect_aliases: self.effect_aliases.iter().map(|(k, (a, f))| (*k as &'p str, (decls.effect_alias(*a), *f))).collect(),
            opaque_types: map(&self.opaque_types, |id| decls.type_decl(id)),
            opaque_type_files: files(&self.opaque_type_files),
            effect_members: self
                .effect_members
                .iter()
                .map(|(k, ms)| (*k as &'p str, ms.iter().map(|(e, f)| (decls.effect(*e), decls.member(*f))).collect()))
                .collect(),
            name_origins: self.name_origins.iter().map(|(k, ms)| (*k as &'p str, ms.iter().map(|m| *m as &'p ModulePath).collect())).collect(),
            modules: self.modules.iter().map(|m| *m as &'p ModulePath).collect(),
            def_sites: self.def_sites.iter().map(|(k, id)| (*k as &'p str, decls.def_site(*id))).collect(),
        }
    }
}

/// [decl-id] One program's declarations by id, for [`ScopeIds::view`]: the
/// AST behind each id, and each module's refinements and renames.
pub struct Decls<'d, 'p> {
    index: &'d DeclIndex,
    modules: Vec<&'p Module>,
    refns: HashMap<&'p ModulePath, Vec<&'p RefnDecl>>,
    renames: HashMap<&'p ModulePath, Vec<&'p RenameDecl>>,
}

impl<'d, 'p> Decls<'d, 'p> {
    pub fn new<M: std::borrow::Borrow<Module>>(index: &'d DeclIndex, files: &'p [SourceFile], modules: &'p [M]) -> Decls<'d, 'p> {
        let modules: Vec<&'p Module> = modules.iter().map(|m| m.borrow()).collect();
        let mut refns: HashMap<&'p ModulePath, Vec<&'p RefnDecl>> = HashMap::new();
        let mut renames: HashMap<&'p ModulePath, Vec<&'p RenameDecl>> = HashMap::new();
        for (file, module) in files.iter().zip(&modules) {
            for item in &module.items {
                match item {
                    Item::Refn(r) => refns.entry(&file.module).or_default().push(r),
                    Item::Rename(r) => renames.entry(&file.module).or_default().push(r),
                    _ => {}
                }
            }
        }
        Decls { index, modules, refns, renames }
    }

    fn item(&self, id: DeclId) -> &'p Item {
        let loc = self.index.loc(id).unwrap_or_else(|| panic!("[decl-id] `{id}` in scope is not declared in this program"));
        let module: &'p Module = self.modules[loc.file];
        &module.items[loc.item]
    }

    fn fun(&self, id: DeclId) -> &'p FnDecl {
        match self.item(id) {
            Item::Fn(f) => f,
            _ => panic!("[decl-id] `{id}` is not a fn"),
        }
    }

    fn structure(&self, id: DeclId) -> &'p StructDecl {
        match self.item(id) {
            Item::Struct(s) => s,
            _ => panic!("[decl-id] `{id}` is not a struct"),
        }
    }

    fn effect(&self, id: DeclId) -> &'p EffectDecl {
        match self.item(id) {
            Item::Effect(e) => e,
            _ => panic!("[decl-id] `{id}` is not an effect"),
        }
    }

    fn member(&self, id: DeclId) -> &'p FnDecl {
        let loc = self.index.loc(id).unwrap_or_else(|| panic!("[decl-id] `{id}` in scope is not declared in this program"));
        match self.item(id) {
            Item::Effect(e) if loc.sub > 0 => &e.fns[loc.sub as usize - 1],
            _ => panic!("[decl-id] `{id}` is not an effect member"),
        }
    }

    fn group(&self, id: DeclId) -> &'p FnsDecl {
        match self.item(id) {
            Item::Fns(g) => g,
            _ => panic!("[decl-id] `{id}` is not a group"),
        }
    }

    fn handler(&self, id: DeclId) -> &'p HandlerDecl {
        match self.item(id) {
            Item::Handler(h) => h,
            _ => panic!("[decl-id] `{id}` is not a handler"),
        }
    }

    fn qualifier(&self, id: DeclId) -> &'p QualifierDecl {
        match self.item(id) {
            Item::Qualifier(q) => q,
            _ => panic!("[decl-id] `{id}` is not a qualifier"),
        }
    }

    fn type_decl(&self, id: DeclId) -> &'p TypeDecl {
        match self.item(id) {
            Item::Type(t) => t,
            _ => panic!("[decl-id] `{id}` is not a type"),
        }
    }

    fn effect_alias(&self, id: DeclId) -> &'p EffectAliasDecl {
        match self.item(id) {
            Item::EffectAlias(a) => a,
            _ => panic!("[decl-id] `{id}` is not an effect alias"),
        }
    }

    fn refns(&self, module: &ModulePath) -> Vec<&'p RefnDecl> {
        self.refns.get(module).cloned().unwrap_or_default()
    }

    fn renames(&self, module: &ModulePath) -> Vec<&'p RenameDecl> {
        self.renames.get(module).cloned().unwrap_or_default()
    }

    /// Where the name of `id` is written, in this program.
    fn def_site(&self, id: DeclId) -> DefSite {
        let loc = self.index.loc(id).unwrap_or_else(|| panic!("[decl-id] `{id}` in scope is not declared in this program"));
        let span = match self.item(id) {
            Item::Effect(e) if loc.sub > 0 => e.fns[loc.sub as usize - 1].name.span,
            Item::Fn(f) => f.name.span,
            Item::Struct(s) => s.name.span,
            Item::Type(t) => t.name.span,
            Item::Effect(e) => e.name.span,
            Item::Handler(h) => h.name.span,
            Item::Qualifier(q) => q.name.span,
            Item::Fns(g) => g.name.span,
            Item::EffectAlias(a) => a.name.span,
            _ => panic!("[decl-id] `{id}` has no name"),
        };
        DefSite { file: loc.file, span }
    }
}

/// [decl-id] Whether two scopes are the same: the same names, meaning the
/// same declarations (by address) in the same order. What the debug build
/// checks a view against the scope it stands for.
pub fn same(a: &ModuleScope<'_>, b: &ModuleScope<'_>) -> Result<(), String> {
    fn ptrs<K: std::fmt::Debug + Eq + std::hash::Hash, T>(what: &str, a: &HashMap<K, &T>, b: &HashMap<K, &T>) -> Result<(), String> {
        if a.len() != b.len() {
            return Err(format!("{what}: {} names against {}", a.len(), b.len()));
        }
        for (k, x) in a {
            match b.get(k) {
                Some(y) if std::ptr::eq(*x, *y) => {}
                _ => return Err(format!("{what}: `{k:?}` differs")),
            }
        }
        Ok(())
    }
    fn eq<K: std::fmt::Debug + Eq + std::hash::Hash, V: PartialEq>(what: &str, a: &HashMap<K, V>, b: &HashMap<K, V>) -> Result<(), String> {
        if a != b {
            return Err(format!("{what} differ"));
        }
        Ok(())
    }
    let fn_entries = |a: &[FnEntry<'_>], b: &[FnEntry<'_>]| {
        a.len() == b.len()
            && a.iter().zip(b).all(|(x, y)| x.key == y.key && x.file == y.file && x.rung == y.rung && std::ptr::eq(x.decl, y.decl) && x.module == y.module)
    };
    if a.fns.len() != b.fns.len() || a.fns.iter().any(|(k, es)| b.fns.get(k).is_none_or(|fs| !fn_entries(es, fs))) {
        return Err("fns differ".to_string());
    }
    ptrs("structs", &a.structs, &b.structs)?;
    eq("struct_files", &a.struct_files, &b.struct_files)?;
    ptrs("effects", &a.effects, &b.effects)?;
    eq("effect_files", &a.effect_files, &b.effect_files)?;
    ptrs("fn_groups", &a.fn_groups, &b.fn_groups)?;
    ptrs("handlers", &a.handlers, &b.handlers)?;
    let same_list = |x: &[&QualifierDecl], y: &[&QualifierDecl]| x.len() == y.len() && x.iter().zip(y).all(|(p, q)| std::ptr::eq(*p, *q));
    if a.qualifiers.len() != b.qualifiers.len() || a.qualifiers.iter().any(|(k, qs)| b.qualifiers.get(k).is_none_or(|rs| !same_list(qs, rs))) {
        return Err("qualifiers differ".to_string());
    }
    if a.refns.len() != b.refns.len() || a.refns.iter().zip(&b.refns).any(|(x, y)| !std::ptr::eq(*x, *y)) {
        return Err("refns differ".to_string());
    }
    if a.renames.len() != b.renames.len() || a.renames.iter().zip(&b.renames).any(|(x, y)| !std::ptr::eq(*x, *y)) {
        return Err("renames differ".to_string());
    }
    ptrs("type_aliases", &a.type_aliases, &b.type_aliases)?;
    let aliases = |x: &HashMap<&str, (&EffectAliasDecl, usize)>, y: &HashMap<&str, (&EffectAliasDecl, usize)>| {
        x.len() == y.len() && x.iter().all(|(k, (d, f))| y.get(k).is_some_and(|(e, g)| std::ptr::eq(*d, *e) && f == g))
    };
    if !aliases(&a.effect_aliases, &b.effect_aliases) {
        return Err("effect_aliases differ".to_string());
    }
    ptrs("opaque_types", &a.opaque_types, &b.opaque_types)?;
    eq("opaque_type_files", &a.opaque_type_files, &b.opaque_type_files)?;
    let members = |x: &[(&EffectDecl, &FnDecl)], y: &[(&EffectDecl, &FnDecl)]| {
        x.len() == y.len() && x.iter().zip(y).all(|((e, f), (g, h))| std::ptr::eq(*e, *g) && std::ptr::eq(*f, *h))
    };
    if a.effect_members.len() != b.effect_members.len() || a.effect_members.iter().any(|(k, ms)| b.effect_members.get(k).is_none_or(|ns| !members(ms, ns))) {
        return Err("effect_members differ".to_string());
    }
    eq("name_origins", &a.name_origins, &b.name_origins)?;
    if a.modules != b.modules {
        return Err("modules differ".to_string());
    }
    eq("def_sites", &a.def_sites, &b.def_sites)?;
    Ok(())
}
