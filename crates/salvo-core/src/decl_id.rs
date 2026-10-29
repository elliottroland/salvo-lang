//! [decl-id] The identity of a declaration.
//!
//! Every declaration — a fn, struct, `type`, effect and each of its members,
//! handler and each of its members, qualifier and each of its fns, `fns`
//! group and each of its members, effect alias, and the declarations the IR
//! builder generates — is identified by a [`DeclId`]: its **module path**, its
//! **kind**, its **name path** (`["Counter", "get"]` for a handler's member),
//! for a fn-like declaration its **signature** (the written parameter types,
//! type parameters positional, and the return type: what tells overloads apart,
//! [fn-overload-duplicate] [fn-overload-return]), and an **ordinal** that only
//! two textually identical signatures need, which [fn-overload-duplicate]
//! already refuses in a module's top-level fns.
//!
//! The id is a function of the declaration's own text and module: it does not
//! change when a body is edited, when declarations are reordered, or when
//! another file is added or removed (PIPELINE.md §II.10 P1). It is interned
//! for the process, so it is a pointer: hashing and comparing it is one word,
//! and the same declaration has the same id in every program that contains it
//! (std's in the language server's std cache and in each analysis).
//!
//! Where a stage still needs the file and item a declaration is at (emission
//! order, the AST behind an id), it asks the program's [`DeclIndex`], built by
//! resolution (`Resolution::decls`) and carried by `Checked::decls`.

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::sync::{Arc, Mutex, Weak};

use salvo_syntax::ast::{FnDecl, Item, Module, NodeRange};
use salvo_syntax::Span;

use crate::source::{ModulePath, SourceFile};

/// What a declaration is. Part of its identity: one module may name a struct,
/// a handler and a `fns` group alike.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum DeclKind {
    Fn,
    Struct,
    /// A `type`: alias, union, `intrinsic type` or `platform type`.
    Type,
    Effect,
    EffectMember,
    Handler,
    /// A handler's member fn, or its `init` (named `init`).
    HandlerMember,
    Qualifier,
    QualifierFn,
    Group,
    GroupMember,
    EffectAlias,
    /// [ir-actor-glue] A declaration the IR builder generates (a message
    /// enum, a stub, a route, a dispatcher), by its generated name.
    Generated,
    /// [mod-use] A module-level `use`, as the IR's static.
    Static,
    /// [node-id] A file's own nodes, which no declaration holds (its
    /// imports, renames, refinements and module-level `use`s): what their
    /// facts are keyed by. Path empty.
    File,
}

/// The parts of a [`DeclId`].
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DeclPath {
    pub module: ModulePath,
    pub kind: DeclKind,
    /// The container's name, then the declaration's (one segment for a
    /// top-level declaration).
    pub path: Vec<String>,
    /// [fn-overload-duplicate] A fn-like declaration's signature as overload
    /// identity: `(T0, Str) -> Int`.
    pub sig: Option<String>,
    /// The tie-break among identical (module, kind, path, sig), in program
    /// order: 0 but in a program with a duplicate.
    pub ordinal: u32,
}

/// [decl-id] A declaration's identity. Interned: equal ids are one pointer.
#[derive(Clone, Copy)]
pub struct DeclId(&'static DeclPath);

impl PartialEq for DeclId {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.0, other.0)
    }
}
impl Eq for DeclId {}
impl std::hash::Hash for DeclId {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        (self.0 as *const DeclPath as usize).hash(state)
    }
}
impl PartialOrd for DeclId {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
/// By content, so an order over ids is the same in every run.
impl Ord for DeclId {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        if std::ptr::eq(self.0, other.0) {
            return std::cmp::Ordering::Equal;
        }
        self.0.cmp(other.0)
    }
}

static POOL: Mutex<Option<HashSet<&'static DeclPath>>> = Mutex::new(None);

impl DeclId {
    /// The id of `path`. Ids live for the process, like type keys
    /// (`typekey::keyed`): there are as many as declarations.
    pub fn intern(path: DeclPath) -> DeclId {
        let mut pool = POOL.lock().unwrap_or_else(|e| e.into_inner());
        let pool = pool.get_or_insert_with(HashSet::new);
        if let Some(found) = pool.get(&path) {
            return DeclId(found);
        }
        let leaked: &'static DeclPath = Box::leak(Box::new(path));
        pool.insert(leaked);
        DeclId(leaked)
    }

    /// [ir-actor-glue] The id of a declaration the IR generates in `module`,
    /// by its generated name path.
    pub fn generated(module: &ModulePath, path: &[&str]) -> DeclId {
        DeclId::intern(DeclPath {
            module: module.clone(),
            kind: DeclKind::Generated,
            path: path.iter().map(|s| s.to_string()).collect(),
            sig: None,
            ordinal: 0,
        })
    }

    /// [mod-use] The id of a module-level `use`'s static, by its local.
    pub fn static_use(module: &ModulePath, local: &str) -> DeclId {
        DeclId::intern(DeclPath { module: module.clone(), kind: DeclKind::Static, path: vec![local.to_string()], sig: None, ordinal: 0 })
    }

    pub fn parts(&self) -> &'static DeclPath {
        self.0
    }
    pub fn module(&self) -> &'static ModulePath {
        &self.0.module
    }
    pub fn kind(&self) -> DeclKind {
        self.0.kind
    }
    /// The declaration's own name: the last segment of its path.
    pub fn name(&self) -> &'static str {
        self.0.path.last().map(String::as_str).unwrap_or("")
    }
    pub fn path(&self) -> &'static [String] {
        &self.0.path
    }
    pub fn sig(&self) -> Option<&'static str> {
        self.0.sig.as_deref()
    }
    pub fn ordinal(&self) -> u32 {
        self.0.ordinal
    }
}

/// `Counter.get(Int) -> None@main`, `#1` after it for a second identical
/// signature: the `name@module` form dumps use (PIPELINE.md §II.7).
impl fmt::Display for DeclId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let p = self.0;
        write!(f, "{}", p.path.join("."))?;
        if let Some(sig) = &p.sig {
            write!(f, "{sig}")?;
        }
        write!(f, "@{}", p.module)?;
        if p.ordinal > 0 {
            write!(f, "#{}", p.ordinal)?;
        }
        Ok(())
    }
}

impl fmt::Debug for DeclId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DeclId({self})")
    }
}

/// Where a declaration is: its file, its item, and for a member the member's
/// position plus one (0 for the item itself).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DeclLoc {
    pub file: usize,
    pub item: usize,
    pub sub: u16,
}

/// [decl-id] Every declaration of one program: its id by position, and its
/// position by id.
#[derive(Default)]
pub struct DeclIndex {
    /// Per file, per item: the item's id (`None` for an import, rename,
    /// refinement or anything else that declares nothing).
    items: Vec<Vec<Option<DeclId>>>,
    /// Per file, per item: its members' ids, in declaration order (an effect's
    /// and a handler's fns — a handler's `init` last —, a qualifier's fns, a
    /// group's members).
    members: Vec<Vec<Vec<DeclId>>>,
    locs: HashMap<DeclId, DeclLoc>,
    /// [node-id] Per file: each declaration's run of node ordinals
    /// (`Module::node_ranges`), by its first ordinal, with the
    /// declaration's id — a file's own nodes under its `DeclKind::File` id.
    nodes: Vec<Vec<NodeRun>>,
    /// [node-id] Per file: its `DeclKind::File` id.
    files: Vec<DeclId>,
    /// [node-file] The file each tag is (`Module::file_tag`).
    tags: HashMap<u32, usize>,
}

/// [node-id] One declaration's node ordinals in its file.
#[derive(Clone, Copy, Debug)]
struct NodeRun {
    first: u32,
    end: u32,
    decl: DeclId,
}

/// [node-id] A node's identity: its ordinal within its declaration, in the
/// declaration's walk order (`salvo_syntax::node_id`). With the
/// declaration's id, what a fact is keyed by.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NodeId(pub u32);

impl fmt::Debug for DeclIndex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DeclIndex({} declarations)", self.locs.len())
    }
}

/// A fn's signature as overload identity (`refine::param_type_signature`
/// and `return_type_signature`, as [fn-overload-duplicate] compares them).
pub fn fn_sig(f: &FnDecl) -> String {
    let mut generics: Vec<&str> = f.generics.iter().map(|g| g.name.as_str()).collect();
    for (g, _) in &f.generic_canbe {
        if !generics.contains(&g.name.as_str()) {
            generics.push(g.name.as_str());
        }
    }
    let params = crate::refine::param_type_signature(&f.params, &generics);
    let ret = crate::refine::return_type_signature(f.return_type.as_ref(), &generics);
    format!("({}) -> {ret}", params.join(", "))
}

/// A module as resolution reads it: a shared one says so, which lets
/// [`DeclIndex::build`] reuse the ids it computed for that very module.
pub trait ModuleRef: std::borrow::Borrow<Module> {
    fn shared(&self) -> Option<&Arc<Module>> {
        None
    }
}
impl ModuleRef for Module {}
impl ModuleRef for &Module {}
impl ModuleRef for Arc<Module> {
    fn shared(&self) -> Option<&Arc<Module>> {
        Some(self)
    }
}

/// One file's ids at ordinal 0, per item and per member.
#[derive(Clone)]
struct FileIds {
    items: Vec<Option<DeclId>>,
    members: Vec<Vec<DeclId>>,
}

/// [decl-id] The ids of the modules seen last, by the module's allocation:
/// an id is a function of the module's text and path, so a module shared
/// between analyses (the language server's std, a cached parse) or between
/// the comptime resolution and the program's has its ids computed once. The
/// `Weak` makes a hit mean the very allocation is alive — and unchanged,
/// since `Arc::make_mut` on a module with only weak references left
/// disassociates them.
type Memo = HashMap<usize, (Weak<Module>, ModulePath, FileIds)>;
static MEMO: Mutex<Option<Memo>> = Mutex::new(None);

fn file_ids(path: &ModulePath, module: &Module) -> FileIds {
    let mut items = Vec::with_capacity(module.items.len());
    let mut members = Vec::with_capacity(module.items.len());
    for item in &module.items {
        let (own, subs): (Option<(DeclKind, Vec<String>, Option<String>)>, Vec<(DeclKind, Vec<String>, Option<String>)>) = match item {
            Item::Fn(f) => (Some((DeclKind::Fn, vec![f.name.name.clone()], Some(fn_sig(f)))), Vec::new()),
            Item::Struct(s) => (Some((DeclKind::Struct, vec![s.name.name.clone()], None)), Vec::new()),
            Item::Type(t) => (Some((DeclKind::Type, vec![t.name.name.clone()], None)), Vec::new()),
            Item::Effect(e) => (
                Some((DeclKind::Effect, vec![e.name.name.clone()], None)),
                e.fns.iter().map(|m| (DeclKind::EffectMember, vec![e.name.name.clone(), m.name.name.clone()], Some(fn_sig(m)))).collect(),
            ),
            Item::Handler(h) => (
                Some((DeclKind::Handler, vec![h.name.name.clone()], None)),
                h.fns
                    .iter()
                    .chain(h.init.as_ref())
                    .map(|m| (DeclKind::HandlerMember, vec![h.name.name.clone(), m.name.name.clone()], Some(fn_sig(m))))
                    .collect(),
            ),
            Item::Qualifier(q) => (
                // [qual-overload] One name over several subjects: the
                // subject tells them apart.
                Some((DeclKind::Qualifier, vec![q.name.name.clone()], Some(q.of.to_string()))),
                q.fns.iter().map(|m| (DeclKind::QualifierFn, vec![q.name.name.clone(), m.name.name.clone()], Some(fn_sig(m)))).collect(),
            ),
            Item::Fns(g) => (
                Some((DeclKind::Group, vec![g.name.name.clone()], None)),
                g.fns.iter().map(|m| (DeclKind::GroupMember, vec![g.name.name.clone(), m.name.name.clone()], Some(fn_sig(m)))).collect(),
            ),
            Item::EffectAlias(a) => (Some((DeclKind::EffectAlias, vec![a.name.name.clone()], None)), Vec::new()),
            Item::Import(_) | Item::Namespace(_) | Item::Refn(_) | Item::Rename(_) | Item::Test(_) => (None, Vec::new()),
        };
        let id = |(kind, p, sig): (DeclKind, Vec<String>, Option<String>)| DeclId::intern(DeclPath { module: path.clone(), kind, path: p, sig, ordinal: 0 });
        items.push(own.map(id));
        members.push(subs.into_iter().map(id).collect());
    }
    FileIds { items, members }
}

impl DeclIndex {
    /// Assigns every declaration of `files` its id.
    pub fn build<M: ModuleRef>(files: &[SourceFile], modules: &[M]) -> DeclIndex {
        let mut index = DeclIndex::default();
        let mut memo = MEMO.lock().unwrap_or_else(|e| e.into_inner());
        let memo = memo.get_or_insert_with(HashMap::new);
        for (file_idx, (file, slot)) in files.iter().zip(modules).enumerate() {
            let module: &Module = slot.borrow();
            let shared = slot.shared();
            let cached = shared.and_then(|arc| {
                let (weak, path, ids) = memo.get(&(Arc::as_ptr(arc) as usize))?;
                (weak.upgrade().is_some_and(|w| Arc::ptr_eq(&w, arc)) && *path == file.module).then(|| ids.clone())
            });
            let ids = match cached {
                Some(ids) => ids,
                None => {
                    let ids = file_ids(&file.module, module);
                    if let Some(arc) = shared {
                        memo.insert(Arc::as_ptr(arc) as usize, (Arc::downgrade(arc), file.module.clone(), ids.clone()));
                    }
                    ids
                }
            };
            let FileIds { mut items, mut members } = ids;
            for (item_idx, id) in items.iter_mut().enumerate() {
                if let Some(id) = id {
                    *id = index.assign(*id, DeclLoc { file: file_idx, item: item_idx, sub: 0 });
                }
                for (i, m) in members[item_idx].iter_mut().enumerate() {
                    *m = index.assign(*m, DeclLoc { file: file_idx, item: item_idx, sub: i as u16 + 1 });
                }
            }
            // [node-id] The file's own id, after its declarations' (an
            // ordinal only for two files of one module path).
            let own = DeclId::intern(DeclPath { module: file.module.clone(), kind: DeclKind::File, path: Vec::new(), sig: None, ordinal: 0 });
            let own = index.assign(own, DeclLoc { file: file_idx, item: usize::MAX, sub: 0 });
            let runs = module
                .node_ranges
                .iter()
                .map(|r: &NodeRange| {
                    let decl = if r.item == NodeRange::FILE {
                        own
                    } else if r.sub == 0 {
                        items.get(r.item as usize).copied().flatten().unwrap_or(own)
                    } else {
                        members.get(r.item as usize).and_then(|m| m.get(r.sub as usize - 1)).copied().unwrap_or(own)
                    };
                    NodeRun { first: r.first, end: r.first + r.len, decl }
                })
                .collect();
            index.items.push(items);
            index.members.push(members);
            index.nodes.push(runs);
            index.files.push(own);
            if module.file_tag != salvo_syntax::span::NO_FILE {
                index.tags.insert(module.file_tag, file_idx);
            }
        }
        // Forget the modules no longer alive, once they outnumber the rest.
        if memo.len() > 4 * files.len().max(64) {
            memo.retain(|_, (weak, _, _)| weak.strong_count() > 0);
        }
        index
    }

    /// Records `id` at `at`: the id itself, or, when another declaration of
    /// this program already has it (a duplicate), the next ordinal's.
    fn assign(&mut self, id: DeclId, at: DeclLoc) -> DeclId {
        if let std::collections::hash_map::Entry::Vacant(v) = self.locs.entry(id) {
            v.insert(at);
            return id;
        }
        let mut parts = id.0.clone();
        loop {
            parts.ordinal += 1;
            let id = DeclId::intern(parts.clone());
            if let std::collections::hash_map::Entry::Vacant(v) = self.locs.entry(id) {
                v.insert(at);
                return id;
            }
        }
    }

    /// The id of the item at (`file`, `item`), if it declares something.
    pub fn item(&self, file: usize, item: usize) -> Option<DeclId> {
        self.items.get(file)?.get(item).copied().flatten()
    }

    /// The id of a top-level fn the caller knows is at (`file`, `item`).
    pub fn fn_at(&self, file: usize, item: usize) -> DeclId {
        self.item(file, item).unwrap_or_else(|| panic!("no declaration at file {file}, item {item}"))
    }

    /// The id of the `sub - 1`th member of the item at (`file`, `item`).
    pub fn member(&self, file: usize, item: usize, sub: usize) -> Option<DeclId> {
        self.members.get(file)?.get(item)?.get(sub.checked_sub(1)?).copied()
    }

    /// [node-id] The declaration the node `span` holds is in, and the
    /// node's id within it, for a node of file `file`. `hint` remembers the
    /// last declaration found, since consecutive lookups are mostly in one.
    /// `None` for a span no numbered node holds.
    pub fn node_key(&self, file: usize, span: Span, hint: &std::cell::Cell<usize>) -> Option<(DeclId, NodeId)> {
        if !span.is_numbered() {
            return None;
        }
        let runs = self.nodes.get(file)?;
        let at = hint.get();
        let run = match runs.get(at) {
            Some(r) if r.first <= span.node && span.node < r.end => r,
            _ => {
                let i = runs.partition_point(|r| r.first <= span.node).checked_sub(1)?;
                hint.set(i);
                &runs[i]
            }
        };
        (span.node < run.end).then(|| (run.decl, NodeId(span.node - run.first)))
    }

    /// [node-file] The file whose tree holds the node `span` holds, by its
    /// tag: `None` for a span no tagged file holds.
    pub fn file_of(&self, span: Span) -> Option<usize> {
        self.tags.get(&span.file).copied()
    }

    /// [node-id] The `DeclKind::File` id of file `file`: what its own nodes
    /// are keyed by.
    pub fn file_id(&self, file: usize) -> Option<DeclId> {
        self.files.get(file).copied()
    }

    /// Where `id` is declared in this program.
    pub fn loc(&self, id: DeclId) -> Option<DeclLoc> {
        self.locs.get(&id).copied()
    }

    /// The file `id` is declared in. Panics for an id of another program:
    /// what a caller holding a key the checker recorded never has.
    pub fn file(&self, id: DeclId) -> usize {
        self.loc(id).map(|l| l.file).unwrap_or_else(|| panic!("{id} is not declared in this program"))
    }

    /// The item `id` is (or, for a member, is declared in).
    pub fn item_ast<'m, M: std::borrow::Borrow<Module>>(&self, modules: &'m [M], id: DeclId) -> Option<&'m Item> {
        let l = self.loc(id)?;
        modules.get(l.file)?.borrow().items.get(l.item)
    }

    /// The fn behind `id`: a top-level fn, or a qualifier's.
    pub fn fn_or_qualifier_fn<'m, M: std::borrow::Borrow<Module>>(&self, modules: &'m [M], id: DeclId) -> Option<&'m FnDecl> {
        let l = self.loc(id)?;
        match modules.get(l.file)?.borrow().items.get(l.item)? {
            Item::Fn(f) if l.sub == 0 => Some(f),
            Item::Qualifier(q) if l.sub > 0 => q.fns.get(l.sub as usize - 1),
            _ => None,
        }
    }

    /// The top-level fn behind `id`.
    pub fn fn_decl<'m, M: std::borrow::Borrow<Module>>(&self, modules: &'m [M], id: DeclId) -> Option<&'m FnDecl> {
        let l = self.loc(id)?;
        if l.sub != 0 {
            return None;
        }
        match modules.get(l.file)?.borrow().items.get(l.item)? {
            Item::Fn(f) => Some(f),
            _ => None,
        }
    }
}
