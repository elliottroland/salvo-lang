//! [lsp-check-cache] The per-file checker rows, kept across a language
//! server's analyses (PIPELINE.md step 8, piece 3).
//!
//! A file's walks (its signature facts, its headers, its bodies) write rows
//! keyed by that file and read only its own text and other declarations'
//! interfaces [interface-pass]. So while a file's module is the very `Arc`
//! it was checked as, and no file's interface has changed, what its walks
//! would write is what they wrote last time: the cached check inserts those
//! rows, at the step where each walk would have run, and walks only the
//! files whose modules are new. Everything program-wide (the written keys,
//! the program checks, the deductions, the literal lowering) runs as in a
//! full check, over the merged rows.
//!
//! What stands in for a walk, per step, is exactly what it appended or
//! wrote there: its diagnostics, its rows of the tables kept in order
//! (`StepRows`), the `written_types` rows it wrote or found written, and the
//! handlers it marked parking. The keyed tables are inserted with the file's
//! first step: no walk reads another file's rows of them except the
//! signature facts, which every file's first step writes before any header.
//!
//! An edit to a plain fn body leaves every interface as it was but moves the
//! declarations after it in its file, so a cached file's go-to-definition
//! targets there are moved by the same amounts (`remap`).

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use salvo_syntax::ast::{self, Item, Module};
use salvo_syntax::visit::{self as v, Visitor};
use salvo_syntax::Span;

use crate::check::{Checked, Key};
use crate::diag::FileDiagnostic;
use crate::program::Program;
use crate::decl_id::DeclId;
use crate::types::Ty;

/// The walk steps a file goes through, in order [interface-pass].
pub(crate) const FACTS: usize = 0;
pub(crate) const HEADERS: usize = 1;
pub(crate) const BODIES: usize = 2;

/// [lsp-check-cache] What one walk step of one file appended to the tables
/// kept in order, and wrote to the address-keyed and set-valued ones.
#[derive(Clone, Default)]
pub(crate) struct StepRows {
    pub errors: Vec<FileDiagnostic>,
    /// `written_types` rows, in the order written: the node's address, the
    /// file whose module holds the node, the row, and whether it was written
    /// only where none was. A row for a node of another file follows that
    /// node into the file's new module after an edit (`node_map`).
    pub written: Vec<(usize, usize, Ty, bool)>,
    pub parking: Vec<String>,
    pub waitfor_sites: Vec<(String, Key)>,
    pub handler_constructs: Vec<(String, String, Key)>,
    pub actor_gates: Vec<(String, Key)>,
    pub actor_sends: Vec<(String, String, Key)>,
    pub task_sends: Vec<(String, String, Key)>,
    pub task_mints: Vec<(String, String, Key)>,
    pub fn_body_effects: Vec<(usize, Span, Vec<Ty>)>,
}

impl StepRows {
    /// Appends and writes what the step did, as if it had run.
    pub(crate) fn apply(&self, out: &mut Checked, parking: &mut HashSet<String>) {
        out.errors.extend(self.errors.iter().cloned());
        for (at, _, ty, if_absent) in &self.written {
            if *if_absent {
                out.written_types.entry(*at).or_insert_with(|| ty.clone());
            } else {
                out.written_types.insert(*at, ty.clone());
            }
        }
        for p in &self.parking {
            parking.insert(p.clone());
        }
        out.waitfor_sites.extend(self.waitfor_sites.iter().cloned());
        out.handler_constructs.extend(self.handler_constructs.iter().cloned());
        out.actor_gates.extend(self.actor_gates.iter().cloned());
        out.actor_sends.extend(self.actor_sends.iter().cloned());
        out.task_sends.extend(self.task_sends.iter().cloned());
        out.task_mints.extend(self.task_mints.iter().cloned());
        out.fn_body_effects.extend(self.fn_body_effects.iter().cloned());
    }
}

/// [lsp-check-cache] The lengths of the tables kept in order, before a step.
pub(crate) struct StepMarks {
    errors: usize,
    waitfor_sites: usize,
    handler_constructs: usize,
    actor_gates: usize,
    actor_sends: usize,
    task_sends: usize,
    task_mints: usize,
    fn_body_effects: usize,
}

impl StepMarks {
    pub(crate) fn of(out: &Checked) -> Self {
        StepMarks {
            errors: out.errors.len(),
            waitfor_sites: out.waitfor_sites.len(),
            handler_constructs: out.handler_constructs.len(),
            actor_gates: out.actor_gates.len(),
            actor_sends: out.actor_sends.len(),
            task_sends: out.task_sends.len(),
            task_mints: out.task_mints.len(),
            fn_body_effects: out.fn_body_effects.len(),
        }
    }

    /// What the step appended since the marks, with the `written_types` rows
    /// it wrote (their owners are found at the end of the check,
    /// `CheckCache::record`) and the handlers it marked parking.
    pub(crate) fn take(self, out: &Checked, written: Vec<(usize, Ty, bool)>, parking: Vec<String>) -> StepRows {
        StepRows {
            errors: out.errors[self.errors..].to_vec(),
            written: written.into_iter().map(|(at, ty, if_absent)| (at, usize::MAX, ty, if_absent)).collect(),
            parking,
            waitfor_sites: out.waitfor_sites[self.waitfor_sites..].to_vec(),
            handler_constructs: out.handler_constructs[self.handler_constructs..].to_vec(),
            actor_gates: out.actor_gates[self.actor_gates..].to_vec(),
            actor_sends: out.actor_sends[self.actor_sends..].to_vec(),
            task_sends: out.task_sends[self.task_sends..].to_vec(),
            task_mints: out.task_mints[self.task_mints..].to_vec(),
            fn_body_effects: out.fn_body_effects[self.fn_body_effects..].to_vec(),
        }
    }
}

/// [lsp-check-cache] One file's rows, as its last walk left them.
#[derive(Clone)]
pub(crate) struct FileRows {
    /// The module the rows were checked from: they stand in for a walk only
    /// while the program holds this very `Arc`.
    module: Arc<Module>,
    /// The file's text with its plain fn bodies cut out (`interface_of`).
    iface: u64,
    /// The plain fn bodies' byte ranges, in order: what an edit inside one
    /// moves the rest of the file by (`remap`).
    bodies: Vec<(u32, u32)>,
    /// The keyed rows (`Checked::split_files`), and the file's
    /// `param_mutations`.
    pub rows: Checked,
    pub steps: [StepRows; 3],
}

/// [lsp-check-cache] What a file is, apart from its text: the file list part
/// of the program's interface fingerprint.
#[derive(Clone, PartialEq)]
struct FileId {
    name: String,
    module: crate::source::ModulePath,
    is_std: bool,
    is_shadow: bool,
    is_test: bool,
    dependency: Option<String>,
}

impl FileId {
    fn of(f: &crate::source::SourceFile) -> Self {
        FileId {
            name: f.name.clone(),
            module: f.module.clone(),
            is_std: f.is_std,
            is_shadow: f.is_shadow,
            is_test: f.is_test,
            dependency: f.dependency.clone(),
        }
    }
}

/// [lsp-check-cache] The checker rows of every walked file of a program,
/// kept by the language server between analyses, the way its `ParseCache`
/// keeps parsed modules (`check_project_cached`).
#[derive(Default)]
pub struct CheckCache {
    files: Vec<FileId>,
    /// Every file's module as last checked: holding them keeps the addresses
    /// the rows are keyed by from being reused.
    modules: Vec<Arc<Module>>,
    /// The first file the walks cover (std's are the `StdCache`'s).
    first: usize,
    /// The syntactic parking handlers (PIPELINE.md 4.5), read off bodies.
    parking: HashSet<String>,
    entries: HashMap<usize, FileRows>,
    /// Which file's module holds each `ast::Type` node, by address, for every
    /// module in `modules`; `type_nodes[f]` is file `f`'s.
    owners: HashMap<usize, usize>,
    type_nodes: Vec<Vec<usize>>,
    /// The files the last check walked, in order.
    walked: Vec<usize>,
}

impl CheckCache {
    pub fn new() -> Self {
        Self::default()
    }

    /// [lsp-check-cache] The files the last check walked; every other file
    /// from the first project file on had its rows inserted.
    pub fn walked(&self) -> &[usize] {
        &self.walked
    }

    /// Decides which files' rows stand in for their walks: every file from
    /// `first` on whose module is the `Arc` its rows were checked from, when
    /// the program's interfaces are as they were — the same files, std's
    /// same modules, the same syntactic parking set, and every changed
    /// file's text the same outside its plain fn bodies. A cached file's
    /// rows are brought up to date with the changed files first: its
    /// `written_types` rows for their nodes move to the same nodes of their
    /// new modules (`node_map`), and its definition sites in them move with
    /// the text (`remap`).
    pub(crate) fn plan(&mut self, program: &Program, first: usize, parking: &HashSet<String>) -> HashSet<usize> {
        let n = program.files.len();
        let same_program = self.first == first
            && self.files.len() == n
            && self.files.iter().zip(&program.files).all(|(a, b)| *a == FileId::of(b))
            && self.parking == *parking
            && (0..first).all(|i| Arc::ptr_eq(&self.modules[i], &program.modules[i]));
        if !same_program {
            self.entries.clear();
            return HashSet::new();
        }
        let mut changed: HashMap<usize, (Vec<(u32, u32)>, Vec<(u32, u32)>)> = HashMap::new();
        let mut moved: HashMap<usize, HashMap<usize, usize>> = HashMap::new();
        for f in first..n {
            let Some(e) = self.entries.get(&f) else {
                self.entries.clear();
                return HashSet::new();
            };
            if Arc::ptr_eq(&e.module, &program.modules[f]) {
                continue;
            }
            let (bodies, iface) = interface_of(&program.files[f].content, &program.modules[f]);
            let nodes = (iface == e.iface && bodies.len() == e.bodies.len())
                .then(|| node_map(&e.module, &e.bodies, &program.modules[f], &bodies))
                .flatten();
            let Some(nodes) = nodes else {
                self.entries.clear();
                return HashSet::new();
            };
            moved.insert(f, nodes);
            changed.insert(f, (e.bodies.clone(), bodies));
        }
        for f in changed.keys() {
            self.entries.remove(f);
        }
        let mut sound = true;
        for e in self.entries.values_mut() {
            for step in &mut e.steps {
                for (at, owner, _, _) in &mut step.written {
                    if let Some(nodes) = moved.get(owner) {
                        match nodes.get(at) {
                            Some(new) => *at = *new,
                            None => sound = false,
                        }
                    }
                }
            }
            if !changed.is_empty() {
                sound &= remap_sites(&mut e.rows, &changed);
            }
        }
        if !sound {
            self.entries.clear();
            return HashSet::new();
        }
        self.entries.keys().copied().collect()
    }

    /// Not used for this check, which walks all of `files`: what it held
    /// goes, since it is of no use to the next check either.
    pub(crate) fn skip(&mut self, files: &[usize]) {
        *self = CheckCache::default();
        self.walked = files.to_vec();
    }

    /// The rows standing in for file `f`'s walks.
    pub(crate) fn rows(&self, f: usize) -> Option<&FileRows> {
        self.entries.get(&f)
    }

    /// Records the walked files' rows from a check of `program` (after every
    /// body, before the program checks): `out` is the check so far,
    /// `mutations` the walk's `param_mutations`, `steps` what each walked
    /// file's steps appended, and `decl_file` the declaring file of each
    /// handler and effect key a walked file declares.
    pub(crate) fn record(
        &mut self,
        program: &Program,
        first: usize,
        parking: &HashSet<String>,
        out: &Checked,
        mutations: &HashMap<DeclId, HashSet<String>>,
        steps: Vec<(usize, [StepRows; 3])>,
        decl_file: &HashMap<String, usize>,
    ) {
        self.sync_owners(program);
        self.files = program.files.iter().map(FileId::of).collect();
        self.modules = program.modules.clone();
        self.first = first;
        self.parking = parking.clone();
        let walked: Vec<usize> = steps.iter().map(|(f, _)| *f).collect();
        let mut parts = out.split_files(program.files.len(), &walked, decl_file);
        for (f, mut steps) in steps {
            for step in &mut steps {
                let owners = &self.owners;
                step.written.retain_mut(|(at, owner, _, _)| match owners.get(at) {
                    Some(o) => {
                        *owner = *o;
                        true
                    }
                    // A node no module holds (one the checker built and
                    // dropped): nothing a later check could look up.
                    None => false,
                });
            }
            let mut rows = parts[f].take().unwrap_or_default();
            rows.param_mutations = mutations
                .iter()
                .filter(|(k, _)| out.decls.loc(**k).is_some_and(|l| l.file == f))
                .map(|(k, v)| (*k, v.clone()))
                .collect();
            let (bodies, iface) = interface_of(&program.files[f].content, &program.modules[f]);
            self.entries.insert(
                f,
                FileRows { module: program.modules[f].clone(), iface, bodies, rows, steps },
            );
        }
        self.walked = walked;
    }

    /// Brings `owners` up to `program`'s modules: a file whose module is no
    /// longer the one recorded has its nodes replaced.
    fn sync_owners(&mut self, program: &Program) {
        let n = program.modules.len();
        for i in 0..self.type_nodes.len().max(n) {
            let same = i < n && self.modules.get(i).is_some_and(|m| Arc::ptr_eq(m, &program.modules[i]));
            if same && i < self.type_nodes.len() {
                continue;
            }
            if let Some(old) = self.type_nodes.get_mut(i) {
                for at in old.drain(..) {
                    if self.owners.get(&at) == Some(&i) {
                        self.owners.remove(&at);
                    }
                }
            }
            if i < n {
                let mut nodes = Vec::new();
                type_nodes(&program.modules[i], &mut nodes);
                for &at in &nodes {
                    self.owners.insert(at, i);
                }
                if i >= self.type_nodes.len() {
                    self.type_nodes.resize_with(i + 1, Vec::new);
                }
                self.type_nodes[i] = nodes;
            }
        }
        self.type_nodes.truncate(n);
    }
}

/// [lsp-check-cache] Whether a fn's body is read by no interface: an
/// ordinary hand-written body. A comptime fn's, a stamped one's, and an
/// `iter fn`'s generated ones are part of the interface (an inferring
/// minter's body is checked with the interfaces [state-infer]).
fn plain_body(f: &ast::FnDecl) -> Option<Span> {
    let body = f.body.as_ref()?;
    let plain = f.compfn.is_none() && f.stamped.is_none() && f.by.is_none() && !f.is_iter && f.iter_state_name.is_none();
    plain.then_some(body.span)
}

/// [lsp-check-cache] A file's plain fn bodies (top-level fns, struct fns,
/// handler members and qualifier fns), as disjoint byte ranges in order,
/// and the hash of its text with them cut out: what a body-only edit leaves
/// alone.
pub(crate) fn interface_of(content: &str, module: &Module) -> (Vec<(u32, u32)>, u64) {
    let mut spans: Vec<(u32, u32)> = Vec::new();
    let mut push = |f: &ast::FnDecl| {
        if let Some(s) = plain_body(f) {
            spans.push((s.start, s.end));
        }
    };
    for item in &module.items {
        match item {
            Item::Fn(f) => push(f),
            Item::Struct(s) => s.fns.iter().for_each(&mut push),
            Item::Handler(h) => h.fns.iter().for_each(&mut push),
            Item::Qualifier(q) => q.fns.iter().for_each(&mut push),
            _ => {}
        }
    }
    let len = content.len() as u32;
    spans.retain(|(s, e)| s < e && *e <= len);
    spans.sort_unstable();
    let mut bodies: Vec<(u32, u32)> = Vec::with_capacity(spans.len());
    for (s, e) in spans {
        match bodies.last_mut() {
            Some(last) if s < last.1 => last.1 = last.1.max(e),
            _ => bodies.push((s, e)),
        }
    }
    let bytes = content.as_bytes();
    let mut h: u64 = 0xcbf29ce484222325;
    let mut eat = |b: u8| {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    };
    let mut at = 0usize;
    for (s, e) in &bodies {
        bytes[at..*s as usize].iter().for_each(|b| eat(*b));
        eat(0xff);
        at = *e as usize;
    }
    bytes[at..].iter().for_each(|b| eat(*b));
    (bodies, h)
}

/// [lsp-check-cache] Moves a position of a changed file from its old text
/// to its new one: the text outside the plain bodies is the same, so a
/// position outside them moves by what the bodies before it grew. `None`
/// for a position inside an old body, which nothing outside it can be
/// pointing at.
fn remap(pos: u32, old: &[(u32, u32)], new: &[(u32, u32)]) -> Option<u32> {
    let mut delta: i64 = 0;
    for ((os, oe), (ns, ne)) in old.iter().zip(new) {
        if *oe <= pos {
            delta += (*ne as i64 - *ns as i64) - (*oe as i64 - *os as i64);
        } else if *os < pos {
            return None;
        } else {
            break;
        }
    }
    u32::try_from(pos as i64 + delta).ok()
}

/// [lsp-check-cache] Every `ast::Type` node of a changed file's old module
/// outside its plain bodies, mapped to the same node of its new module: the
/// two hold the same text there, so the same nodes in the same order, at
/// positions `remap` relates. `None` when they do not line up.
fn node_map(old: &Module, old_bodies: &[(u32, u32)], new: &Module, new_bodies: &[(u32, u32)]) -> Option<HashMap<usize, usize>> {
    struct Collect<'o> {
        bodies: &'o [(u32, u32)],
        out: Vec<(usize, u32)>,
    }
    impl Visitor for Collect<'_> {
        fn visit_type(&mut self, t: &ast::Type) {
            let at = t.span().start;
            if !self.bodies.iter().any(|(s, e)| *s <= at && at < *e) {
                self.out.push((t as *const ast::Type as usize, at));
            }
        }
    }
    let mut a = Collect { bodies: old_bodies, out: Vec::new() };
    walk_module(&mut a, old);
    let mut b = Collect { bodies: new_bodies, out: Vec::new() };
    walk_module(&mut b, new);
    if a.out.len() != b.out.len() {
        return None;
    }
    let mut map = HashMap::with_capacity(a.out.len());
    for ((oa, opos), (na, npos)) in a.out.into_iter().zip(b.out) {
        if remap(opos, old_bodies, new_bodies) != Some(npos) {
            return None;
        }
        map.insert(oa, na);
    }
    Some(map)
}

/// [lsp-check-cache] Moves a cached file's definition sites into the changed
/// files (`def_refs`, `field_refs`). False when one cannot be moved.
fn remap_sites(rows: &mut Checked, changed: &HashMap<usize, (Vec<(u32, u32)>, Vec<(u32, u32)>)>) -> bool {
    let mut ok = true;
    for site in rows.def_refs.values_mut().chain(rows.field_refs.values_mut()) {
        let Some((old, new)) = changed.get(&site.file) else { continue };
        match (remap(site.span.start, old, new), remap(site.span.end, old, new)) {
            (Some(s), Some(e)) => site.span = Span::new(s, e),
            _ => ok = false,
        }
    }
    ok
}

/// [lsp-check-cache] The address of every `ast::Type` node `module` holds:
/// what tells a `written_types` row which file's node it is for.
pub(crate) fn type_nodes(module: &Module, out: &mut Vec<usize>) {
    struct Collect<'o>(&'o mut Vec<usize>);
    impl Visitor for Collect<'_> {
        fn visit_type(&mut self, t: &ast::Type) {
            self.0.push(t as *const ast::Type as usize);
        }
    }
    walk_module(&mut Collect(out), module);
}

/// [lsp-check-cache] [type-identity] Walks every declaration of `module`
/// with `v`: every type, type reference and expression it holds, bodies
/// included. Exhaustive over the declarations by construction (every field
/// named), so a new field holding a type is a compile error here.
pub(crate) fn walk_module<V: Visitor>(c: &mut V, module: &Module) {
    for u in &module.uses {
        v::walk_expr(c, &u.handler);
    }
    for i in &module.items {
        walk_item(c, i);
    }
}

fn params<V: Visitor>(c: &mut V, ps: &[ast::Param]) {
    for p in ps {
        let ast::Param { name: _, ty, variadic: _, implicit: _, default, span: _ } = p;
        v::walk_type(c, ty);
        if let Some(d) = default {
            v::walk_expr(c, d);
        }
    }
}
fn func<V: Visitor>(c: &mut V, f: &ast::FnDecl) {
    // `walk_fn` covers the rest; it does not descend into these.
    let ast::FnDecl {
        docs: _,
        exported: _,
        intrinsic: _,
        platform: _,
        is_iter: _,
        iter_state: _,
        iter_state_name: _,
        is_send: _,
        name: _,
        type_name,
        scoped_to: _,
        compfn: _,
        by: _,
        stamped: _,
        generics: _,
        generic_canbe: _,
        derived_return: _,
        params: ps,
        implicit_groups: _,
        effects: _,
        deductions: _,
        return_type: _,
        response,
        constructs: _,
        body: _,
        span: _,
    } = f;
    v::walk_fn(c, f);
    if let Some(t) = type_name {
        v::walk_type(c, t);
    }
    for r in response.iter().flatten() {
        v::walk_type_ref(c, &r.qual);
    }
    for p in ps {
        if let Some(d) = &p.default {
            v::walk_expr(c, d);
        }
    }
}
fn field<V: Visitor>(c: &mut V, f: &ast::FieldDecl) {
    v::walk_field_decl(c, f);
    for a in &f.annotations {
        v::walk_type_ref(c, a);
    }
}
fn slots<V: Visitor>(c: &mut V, slots: &[ast::SlotDecl]) {
    for s in slots {
        match s {
            ast::SlotDecl::One(ast::FnSlot { name: _, ty, span: _ }) => v::walk_type(c, ty),
            ast::SlotDecl::Group(t) => v::walk_type_ref(c, t),
        }
    }
}
fn canbe<V: Visitor>(c: &mut V, gs: &[(ast::Ident, ast::TypeRef)]) {
    for (_, t) in gs {
        v::walk_type_ref(c, t);
    }
}
fn obligations<V: Visitor>(c: &mut V, os: &[ast::Obligation]) {
    for ast::Obligation { by: _, group } in os {
        v::walk_type_ref(c, group);
    }
}
fn refn<V: Visitor>(c: &mut V, r: &ast::RefnDecl) {
    let ast::RefnDecl { docs: _, name: _, at: _, generics: _, params: ps, wildcard: _, deductions, response, span: _ } = r;
    params(c, ps);
    for ast::RefnDeduction { param: _, add, remove, preserve, span: _ } in deductions {
        for t in add.iter().chain(remove).chain(preserve) {
            v::walk_type_ref(c, t);
        }
    }
    for r in response.iter().flatten() {
        v::walk_type_ref(c, &r.qual);
    }
}
pub(crate) fn walk_item<V: Visitor>(c: &mut V, it: &Item) {
    match it {
        Item::Import(_) => {}
        Item::Type(ast::TypeDecl {
            docs: _,
            exported: _,
            intrinsic: _,
            linear: _,
            noremote: _,
            comptime: _,
            platform: _,
            threadsafe: _,
            iterable: _,
            name: _,
            generics: _,
            generic_canbe,
            fn_slots,
            auto_qualifiers,
            alias,
            obligations: obs,
            span: _,
        }) => {
            canbe(c, generic_canbe);
            slots(c, fn_slots);
            for q in auto_qualifiers {
                v::walk_type_ref(c, q);
            }
            if let Some(a) = alias {
                v::walk_type(c, a);
            }
            obligations(c, obs);
        }
        Item::Struct(ast::StructDecl {
            docs: _,
            annotations,
            exported: _,
            name: _,
            generics: _,
            generic_canbe,
            fn_slots,
            obligations: obs,
            auto_qualifiers,
            fields,
            fns,
            linear: _,
            opaque: _,
            noremote: _,
            comptime: _,
            span: _,
        }) => {
            for a in annotations.iter().chain(auto_qualifiers) {
                v::walk_type_ref(c, a);
            }
            canbe(c, generic_canbe);
            slots(c, fn_slots);
            obligations(c, obs);
            for f in fields {
                field(c, f);
            }
            for f in fns {
                func(c, f);
            }
        }
        Item::Qualifier(ast::QualifierDecl {
            docs: _,
            exported: _,
            intrinsic: _,
            subject: _,
            name: _,
            generics: _,
            generic_canbe,
            fn_slots,
            value_slots,
            of,
            field_overrides,
            fns,
            refns,
            has_body: _,
            comptime: _,
            span: _,
        }) => {
            canbe(c, generic_canbe);
            slots(c, fn_slots);
            for ast::ValueSlot { name: _, ty, span: _ } in value_slots {
                v::walk_type(c, ty);
            }
            v::walk_type(c, of);
            for f in field_overrides {
                field(c, f);
            }
            for f in fns {
                func(c, f);
            }
            for r in refns {
                refn(c, r);
            }
        }
        Item::Effect(ast::EffectDecl { docs: _, exported: _, is_actor: _, name: _, generics: _, prereqs, fns, span: _ }) => {
            for r in prereqs {
                v::walk_effect_ref(c, r);
            }
            for f in fns {
                func(c, f);
            }
        }
        Item::Handler(ast::HandlerDecl {
            docs: _,
            exported: _,
            intrinsic: _,
            platform: _,
            threadsafe: _,
            name: _,
            generics: _,
            params: ps,
            effects,
            of,
            of_any: _,
            init,
            mailbox,
            state,
            state_name: _,
            fns,
            span: _,
        }) => {
            params(c, ps);
            for r in effects.iter().flatten() {
                v::walk_effect_ref(c, r);
            }
            for t in of {
                v::walk_type(c, t);
            }
            if let Some(f) = init {
                func(c, f);
            }
            if let Some(m) = mailbox {
                v::walk_expr(c, m);
            }
            for f in state {
                field(c, f);
            }
            for f in fns {
                func(c, f);
            }
        }
        Item::Params(ast::ParamsDecl { docs: _, exported: _, name: _, generics: _, fns, deductions, span: _ }) => {
            for f in fns {
                func(c, f);
            }
            for d in deductions {
                v::walk_deduction(c, d);
            }
        }
        Item::Fn(f) => func(c, f),
        Item::EffectAlias(ast::EffectAliasDecl { docs: _, exported: _, name: _, effects, span: _ }) => {
            for r in effects {
                v::walk_effect_ref(c, r);
            }
        }
        Item::Namespace(ast::NamespaceDecl { docs: _, exported: _, name: _, items, terminal: _, span: _ }) => {
            for i in items {
                walk_item(c, i);
            }
        }
        Item::Refn(r) => refn(c, r),
        Item::Rename(ast::RenameDecl { docs: _, name: _, target: _, generics: _, params: ps, span: _ }) => params(c, ps),
        Item::Test(ast::TestDecl { docs: _, name: _, name_span: _, kind: _, body, span: _ }) => v::walk_block(c, body),
    }
}

impl Checked {
    /// [lsp-check-cache] Clones every row keyed by one of [files] into a
    /// `Checked` of its own per file: the rows of the tables a walk writes
    /// per file, by the file of their key (`decl_file` for the tables keyed
    /// by a declaration's symbol key). Exhaustive by construction: a new
    /// table is a compile error here until it says where its rows go.
    pub(crate) fn split_files(&self, n_files: usize, files: &[usize], decl_file: &HashMap<String, usize>) -> Vec<Option<Checked>> {
        let mut parts: Vec<Option<Checked>> = (0..n_files).map(|_| None).collect();
        for &f in files {
            parts[f] = Some(Checked::default());
        }
        fn part(parts: &mut [Option<Checked>], f: usize) -> Option<&mut Checked> {
            parts.get_mut(f).and_then(Option::as_mut)
        }
        let Checked { decls, expr_ty, comparisons, arm_compares, promotions, repr_ty, coerce, is_tests, safe_navs, when_exhaustive, when_arms, predicate_tests, qual_member_calls, field_casts, call_fn, for_drivers, refinements, use_effects, route_uses, route_keys, parking_handlers, protocol_hashes, use_deps, use_with_items, spawn_effects, spawn_deps, spawn_dep_items, replyto_members, facade_sends, waitfor_sites, handler_constructs, replyto_tasks, task_mint_effects, use_addrs, use_addr_tuples, project_effects, merge_effects, addr_calls, self_sends, linear_moves, moved_reads, flow_moves, fate_mutations, bind_facts, derived_uses, virtual_candidates, by_value_loops, state_takes, actor_gates, actor_sends, task_sends, task_mints, effect_calls, effect_member_calls, member_calls, dot_calls, written_types, local_calls, local_call_names, call_effects, fn_effects, decl_effects, handler_deps, effect_prereqs, fn_body_effects, call_type_args, lambda_effects, fn_value_implicits, widen_targets, pick_left, elvis_picks, elvis_tests, elvis_widen, may_throw, deductions, fn_refs, def_refs, local_decls, type_ref_keys, type_ref_span_keys, visible_keys, visible_handler_keys, field_refs, module_refs, fate_reads, interp_to_str, interp_union, interp_implicit, temp_views, lending_calls, fn_lends, handle_muts, param_mutations, moved_projections, boundary_checks, factories, platform_closure_all, lambda_captures, derived_calls, distinct_pairs, mut_lend_calls, fn_value_calls, lambda_contracts, implicit_params, established_quals, carried_identities, implicit_args, use_handler_args, implicit_members, errors } = self;
        // [decl-id] The file a fn-keyed row belongs to.
        let fk = |k: &DeclId| decls.loc(*k).map_or(usize::MAX, |l| l.file);
        for (k, v) in expr_ty { if let Some(p) = part(&mut parts, k.0) { p.expr_ty.insert(k.clone(), v.clone()); } }
        for (k, v) in comparisons { if let Some(p) = part(&mut parts, k.0) { p.comparisons.insert(k.clone(), v.clone()); } }
        for (k, v) in arm_compares { if let Some(p) = part(&mut parts, k.0) { p.arm_compares.insert(k.clone(), v.clone()); } }
        for (k, v) in promotions { if let Some(p) = part(&mut parts, k.0) { p.promotions.insert(k.clone(), v.clone()); } }
        for (k, v) in repr_ty { if let Some(p) = part(&mut parts, k.0) { p.repr_ty.insert(k.clone(), v.clone()); } }
        for (k, v) in coerce { if let Some(p) = part(&mut parts, k.0) { p.coerce.insert(k.clone(), v.clone()); } }
        for (k, v) in is_tests { if let Some(p) = part(&mut parts, k.0) { p.is_tests.insert(k.clone(), v.clone()); } }
        for (k, v) in safe_navs { if let Some(p) = part(&mut parts, k.0) { p.safe_navs.insert(k.clone(), v.clone()); } }
        for k in when_exhaustive { if let Some(p) = part(&mut parts, k.0) { p.when_exhaustive.insert(*k); } }
        for (k, v) in when_arms { if let Some(p) = part(&mut parts, k.0) { p.when_arms.insert(k.clone(), v.clone()); } }
        for (k, v) in predicate_tests { if let Some(p) = part(&mut parts, k.0) { p.predicate_tests.insert(k.clone(), v.clone()); } }
        for (k, v) in qual_member_calls { if let Some(p) = part(&mut parts, k.0) { p.qual_member_calls.insert(k.clone(), v.clone()); } }
        for (k, v) in field_casts { if let Some(p) = part(&mut parts, k.0) { p.field_casts.insert(k.clone(), v.clone()); } }
        for (k, v) in call_fn { if let Some(p) = part(&mut parts, k.0) { p.call_fn.insert(k.clone(), v.clone()); } }
        for (k, v) in for_drivers { if let Some(p) = part(&mut parts, k.0) { p.for_drivers.insert(k.clone(), v.clone()); } }
        let _ = refinements; // program-wide: recomputed every check
        for (k, v) in use_effects { if let Some(p) = part(&mut parts, k.0) { p.use_effects.insert(k.clone(), v.clone()); } }
        for (k, v) in route_uses { if let Some(p) = part(&mut parts, k.0) { p.route_uses.insert(k.clone(), v.clone()); } }
        for (k, v) in route_keys { if let Some(p) = decl_file.get(k.as_str()).and_then(|f| part(&mut parts, *f)) { p.route_keys.insert(k.clone(), v.clone()); } }
        let _ = parking_handlers; // program-wide: recomputed every check
        let _ = protocol_hashes; // program-wide: recomputed every check
        for (k, v) in use_deps { if let Some(p) = part(&mut parts, k.0) { p.use_deps.insert(k.clone(), v.clone()); } }
        for (k, v) in use_with_items { if let Some(p) = part(&mut parts, k.0) { p.use_with_items.insert(k.clone(), v.clone()); } }
        for (k, v) in spawn_effects { if let Some(p) = part(&mut parts, k.0) { p.spawn_effects.insert(k.clone(), v.clone()); } }
        for (k, v) in spawn_deps { if let Some(p) = part(&mut parts, k.0) { p.spawn_deps.insert(k.clone(), v.clone()); } }
        for (k, v) in spawn_dep_items { if let Some(p) = part(&mut parts, k.0) { p.spawn_dep_items.insert(k.clone(), v.clone()); } }
        for (k, v) in replyto_members { if let Some(p) = part(&mut parts, k.0) { p.replyto_members.insert(k.clone(), v.clone()); } }
        for (k, v) in facade_sends { if let Some(p) = part(&mut parts, k.0) { p.facade_sends.insert(k.clone(), v.clone()); } }
        let _ = waitfor_sites; // per walk step (`StepRows`)
        let _ = handler_constructs; // per walk step (`StepRows`)
        for (k, v) in replyto_tasks { if let Some(p) = part(&mut parts, k.0) { p.replyto_tasks.insert(k.clone(), v.clone()); } }
        for (k, v) in task_mint_effects { if let Some(p) = part(&mut parts, k.0) { p.task_mint_effects.insert(k.clone(), v.clone()); } }
        for (k, v) in use_addrs { if let Some(p) = part(&mut parts, k.0) { p.use_addrs.insert(k.clone(), v.clone()); } }
        for (k, v) in use_addr_tuples { if let Some(p) = part(&mut parts, k.0) { p.use_addr_tuples.insert(k.clone(), v.clone()); } }
        for (k, v) in project_effects { if let Some(p) = part(&mut parts, k.0) { p.project_effects.insert(k.clone(), v.clone()); } }
        for (k, v) in merge_effects { if let Some(p) = part(&mut parts, k.0) { p.merge_effects.insert(k.clone(), v.clone()); } }
        for (k, v) in addr_calls { if let Some(p) = part(&mut parts, k.0) { p.addr_calls.insert(k.clone(), v.clone()); } }
        for (k, v) in self_sends { if let Some(p) = part(&mut parts, k.0) { p.self_sends.insert(k.clone(), v.clone()); } }
        for k in linear_moves { if let Some(p) = part(&mut parts, k.0) { p.linear_moves.insert(*k); } }
        for k in moved_reads { if let Some(p) = part(&mut parts, k.0) { p.moved_reads.insert(*k); } }
        for k in flow_moves { if let Some(p) = part(&mut parts, k.0) { p.flow_moves.insert(*k); } }
        for (k, v) in fate_mutations { if let Some(p) = part(&mut parts, k.0) { p.fate_mutations.insert(k.clone(), v.clone()); } }
        for (k, v) in bind_facts { if let Some(p) = part(&mut parts, k.0) { p.bind_facts.insert(k.clone(), v.clone()); } }
        for (k, v) in derived_uses { if let Some(p) = part(&mut parts, k.0) { p.derived_uses.insert(k.clone(), v.clone()); } }
        for (k, v) in virtual_candidates { if let Some(p) = part(&mut parts, k.0) { p.virtual_candidates.insert(k.clone(), v.clone()); } }
        for k in by_value_loops { if let Some(p) = part(&mut parts, k.0) { p.by_value_loops.insert(*k); } }
        for k in state_takes { if let Some(p) = part(&mut parts, k.0) { p.state_takes.insert(*k); } }
        let _ = actor_gates; // per walk step (`StepRows`)
        let _ = actor_sends; // per walk step (`StepRows`)
        let _ = task_sends; // per walk step (`StepRows`)
        let _ = task_mints; // per walk step (`StepRows`)
        for (k, v) in effect_calls { if let Some(p) = part(&mut parts, k.0) { p.effect_calls.insert(k.clone(), v.clone()); } }
        for (k, v) in effect_member_calls { if let Some(p) = part(&mut parts, k.0) { p.effect_member_calls.insert(k.clone(), v.clone()); } }
        for (k, v) in member_calls { if let Some(p) = part(&mut parts, k.0) { p.member_calls.insert(k.clone(), v.clone()); } }
        for k in dot_calls { if let Some(p) = part(&mut parts, k.0) { p.dot_calls.insert(*k); } }
        let _ = written_types; // per walk step (`StepRows`)
        for k in local_calls { if let Some(p) = part(&mut parts, k.0) { p.local_calls.insert(*k); } }
        for (k, v) in local_call_names { if let Some(p) = part(&mut parts, k.0) { p.local_call_names.insert(k.clone(), v.clone()); } }
        for (k, v) in call_effects { if let Some(p) = part(&mut parts, k.0) { p.call_effects.insert(k.clone(), v.clone()); } }
        for (k, v) in fn_effects { if let Some(p) = part(&mut parts, fk(k)) { p.fn_effects.insert(k.clone(), v.clone()); } }
        for (k, v) in decl_effects { if let Some(p) = part(&mut parts, k.0) { p.decl_effects.insert(k.clone(), v.clone()); } }
        for (k, v) in handler_deps { if let Some(p) = decl_file.get(k.as_str()).and_then(|f| part(&mut parts, *f)) { p.handler_deps.insert(k.clone(), v.clone()); } }
        for (k, v) in effect_prereqs { if let Some(p) = decl_file.get(k.as_str()).and_then(|f| part(&mut parts, *f)) { p.effect_prereqs.insert(k.clone(), v.clone()); } }
        let _ = fn_body_effects; // per walk step (`StepRows`)
        for (k, v) in call_type_args { if let Some(p) = part(&mut parts, k.0) { p.call_type_args.insert(k.clone(), v.clone()); } }
        for (k, v) in lambda_effects { if let Some(p) = part(&mut parts, k.0) { p.lambda_effects.insert(k.clone(), v.clone()); } }
        for (k, v) in fn_value_implicits { if let Some(p) = part(&mut parts, k.0) { p.fn_value_implicits.insert(k.clone(), v.clone()); } }
        for (k, v) in widen_targets { if let Some(p) = part(&mut parts, k.0) { p.widen_targets.insert(k.clone(), v.clone()); } }
        for (k, v) in pick_left { if let Some(p) = part(&mut parts, k.0) { p.pick_left.insert(k.clone(), v.clone()); } }
        for (k, v) in elvis_picks { if let Some(p) = part(&mut parts, k.0) { p.elvis_picks.insert(k.clone(), v.clone()); } }
        for (k, v) in elvis_tests { if let Some(p) = part(&mut parts, k.0) { p.elvis_tests.insert(k.clone(), v.clone()); } }
        for (k, v) in elvis_widen { if let Some(p) = part(&mut parts, k.0) { p.elvis_widen.insert(k.clone(), v.clone()); } }
        for (k, v) in may_throw { if let Some(p) = part(&mut parts, k.0) { p.may_throw.insert(k.clone(), v.clone()); } }
        let _ = deductions; // program-wide: recomputed every check
        for (k, v) in fn_refs { if let Some(p) = part(&mut parts, k.0) { p.fn_refs.insert(k.clone(), v.clone()); } }
        for (k, v) in def_refs { if let Some(p) = part(&mut parts, k.0) { p.def_refs.insert(k.clone(), v.clone()); } }
        for (k, v) in local_decls { if let Some(p) = part(&mut parts, *k) { p.local_decls.insert(k.clone(), v.clone()); } }
        let _ = type_ref_keys; // program-wide: recomputed every check
        let _ = type_ref_span_keys; // program-wide: recomputed every check
        let _ = visible_keys; // program-wide: recomputed every check
        let _ = visible_handler_keys; // program-wide: recomputed every check
        for (k, v) in field_refs { if let Some(p) = part(&mut parts, k.0) { p.field_refs.insert(k.clone(), v.clone()); } }
        for (k, v) in module_refs { if let Some(p) = part(&mut parts, k.0) { p.module_refs.insert(k.clone(), v.clone()); } }
        for (k, v) in fate_reads { if let Some(p) = part(&mut parts, k.0) { p.fate_reads.insert(k.clone(), v.clone()); } }
        for (k, v) in interp_to_str { if let Some(p) = part(&mut parts, k.0) { p.interp_to_str.insert(k.clone(), v.clone()); } }
        for (k, v) in interp_union { if let Some(p) = part(&mut parts, k.0) { p.interp_union.insert(k.clone(), v.clone()); } }
        for (k, v) in interp_implicit { if let Some(p) = part(&mut parts, k.0) { p.interp_implicit.insert(k.clone(), v.clone()); } }
        for k in temp_views { if let Some(p) = part(&mut parts, k.0) { p.temp_views.insert(*k); } }
        for (k, v) in lending_calls { if let Some(p) = part(&mut parts, k.0) { p.lending_calls.insert(k.clone(), v.clone()); } }
        for (k, v) in fn_lends { if let Some(p) = part(&mut parts, fk(k)) { p.fn_lends.insert(k.clone(), v.clone()); } }
        for k in handle_muts { if let Some(p) = part(&mut parts, k.0) { p.handle_muts.insert(*k); } }
        let _ = param_mutations; // the walk's own map, by the caller
        for k in moved_projections { if let Some(p) = part(&mut parts, k.0) { p.moved_projections.insert(*k); } }
        for (k, v) in boundary_checks { if let Some(p) = part(&mut parts, k.0) { p.boundary_checks.insert(k.clone(), v.clone()); } }
        for (k, v) in factories { if let Some(p) = part(&mut parts, k.0) { p.factories.insert(k.clone(), v.clone()); } }
        let _ = platform_closure_all; // program-wide: recomputed every check
        for (k, v) in lambda_captures { if let Some(p) = part(&mut parts, k.0) { p.lambda_captures.insert(k.clone(), v.clone()); } }
        for (k, v) in derived_calls { if let Some(p) = part(&mut parts, k.0) { p.derived_calls.insert(k.clone(), v.clone()); } }
        for (k, v) in distinct_pairs { if let Some(p) = part(&mut parts, k.0) { p.distinct_pairs.insert(k.clone(), v.clone()); } }
        for k in mut_lend_calls { if let Some(p) = part(&mut parts, k.0) { p.mut_lend_calls.insert(*k); } }
        for (k, v) in fn_value_calls { if let Some(p) = part(&mut parts, k.0) { p.fn_value_calls.insert(k.clone(), v.clone()); } }
        for (k, v) in lambda_contracts { if let Some(p) = part(&mut parts, k.0) { p.lambda_contracts.insert(k.clone(), v.clone()); } }
        for (k, v) in implicit_params { if let Some(p) = part(&mut parts, fk(k)) { p.implicit_params.insert(k.clone(), v.clone()); } }
        for (k, v) in established_quals { if let Some(p) = part(&mut parts, fk(&k.0)) { p.established_quals.insert(k.clone(), v.clone()); } }
        for (k, v) in carried_identities { if let Some(p) = part(&mut parts, k.0) { p.carried_identities.insert(k.clone(), v.clone()); } }
        for (k, v) in implicit_args { if let Some(p) = part(&mut parts, k.0) { p.implicit_args.insert(k.clone(), v.clone()); } }
        for (k, v) in use_handler_args { if let Some(p) = part(&mut parts, k.0) { p.use_handler_args.insert(k.clone(), v.clone()); } }
        for (k, v) in implicit_members { if let Some(p) = part(&mut parts, k.0) { p.implicit_members.insert(k.clone(), v.clone()); } }
        let _ = errors; // per walk step (`StepRows`)
        parts
    }

    /// [lsp-check-cache] Inserts one file's rows (`split_files`).
    pub(crate) fn insert_rows(&mut self, rows: &Checked) {
        let Checked { decls: _, expr_ty, comparisons, arm_compares, promotions, repr_ty, coerce, is_tests, safe_navs, when_exhaustive, when_arms, predicate_tests, qual_member_calls, field_casts, call_fn, for_drivers, refinements, use_effects, route_uses, route_keys, parking_handlers, protocol_hashes, use_deps, use_with_items, spawn_effects, spawn_deps, spawn_dep_items, replyto_members, facade_sends, waitfor_sites, handler_constructs, replyto_tasks, task_mint_effects, use_addrs, use_addr_tuples, project_effects, merge_effects, addr_calls, self_sends, linear_moves, moved_reads, flow_moves, fate_mutations, bind_facts, derived_uses, virtual_candidates, by_value_loops, state_takes, actor_gates, actor_sends, task_sends, task_mints, effect_calls, effect_member_calls, member_calls, dot_calls, written_types, local_calls, local_call_names, call_effects, fn_effects, decl_effects, handler_deps, effect_prereqs, fn_body_effects, call_type_args, lambda_effects, fn_value_implicits, widen_targets, pick_left, elvis_picks, elvis_tests, elvis_widen, may_throw, deductions, fn_refs, def_refs, local_decls, type_ref_keys, type_ref_span_keys, visible_keys, visible_handler_keys, field_refs, module_refs, fate_reads, interp_to_str, interp_union, interp_implicit, temp_views, lending_calls, fn_lends, handle_muts, param_mutations, moved_projections, boundary_checks, factories, platform_closure_all, lambda_captures, derived_calls, distinct_pairs, mut_lend_calls, fn_value_calls, lambda_contracts, implicit_params, established_quals, carried_identities, implicit_args, use_handler_args, implicit_members, errors } = rows;
        self.expr_ty.extend(expr_ty.iter().map(|(k, v)| (k.clone(), v.clone())));
        self.comparisons.extend(comparisons.iter().map(|(k, v)| (k.clone(), v.clone())));
        self.arm_compares.extend(arm_compares.iter().map(|(k, v)| (k.clone(), v.clone())));
        self.promotions.extend(promotions.iter().map(|(k, v)| (k.clone(), v.clone())));
        self.repr_ty.extend(repr_ty.iter().map(|(k, v)| (k.clone(), v.clone())));
        self.coerce.extend(coerce.iter().map(|(k, v)| (k.clone(), v.clone())));
        self.is_tests.extend(is_tests.iter().map(|(k, v)| (k.clone(), v.clone())));
        self.safe_navs.extend(safe_navs.iter().map(|(k, v)| (k.clone(), v.clone())));
        self.when_exhaustive.extend(when_exhaustive.iter().copied());
        self.when_arms.extend(when_arms.iter().map(|(k, v)| (k.clone(), v.clone())));
        self.predicate_tests.extend(predicate_tests.iter().map(|(k, v)| (k.clone(), v.clone())));
        self.qual_member_calls.extend(qual_member_calls.iter().map(|(k, v)| (k.clone(), v.clone())));
        self.field_casts.extend(field_casts.iter().map(|(k, v)| (k.clone(), v.clone())));
        self.call_fn.extend(call_fn.iter().map(|(k, v)| (k.clone(), v.clone())));
        self.for_drivers.extend(for_drivers.iter().map(|(k, v)| (k.clone(), v.clone())));
        let _ = refinements;
        self.use_effects.extend(use_effects.iter().map(|(k, v)| (k.clone(), v.clone())));
        self.route_uses.extend(route_uses.iter().map(|(k, v)| (k.clone(), v.clone())));
        self.route_keys.extend(route_keys.iter().map(|(k, v)| (k.clone(), v.clone())));
        let _ = parking_handlers;
        let _ = protocol_hashes;
        self.use_deps.extend(use_deps.iter().map(|(k, v)| (k.clone(), v.clone())));
        self.use_with_items.extend(use_with_items.iter().map(|(k, v)| (k.clone(), v.clone())));
        self.spawn_effects.extend(spawn_effects.iter().map(|(k, v)| (k.clone(), v.clone())));
        self.spawn_deps.extend(spawn_deps.iter().map(|(k, v)| (k.clone(), v.clone())));
        self.spawn_dep_items.extend(spawn_dep_items.iter().map(|(k, v)| (k.clone(), v.clone())));
        self.replyto_members.extend(replyto_members.iter().map(|(k, v)| (k.clone(), v.clone())));
        self.facade_sends.extend(facade_sends.iter().map(|(k, v)| (k.clone(), v.clone())));
        let _ = waitfor_sites;
        let _ = handler_constructs;
        self.replyto_tasks.extend(replyto_tasks.iter().map(|(k, v)| (k.clone(), v.clone())));
        self.task_mint_effects.extend(task_mint_effects.iter().map(|(k, v)| (k.clone(), v.clone())));
        self.use_addrs.extend(use_addrs.iter().map(|(k, v)| (k.clone(), v.clone())));
        self.use_addr_tuples.extend(use_addr_tuples.iter().map(|(k, v)| (k.clone(), v.clone())));
        self.project_effects.extend(project_effects.iter().map(|(k, v)| (k.clone(), v.clone())));
        self.merge_effects.extend(merge_effects.iter().map(|(k, v)| (k.clone(), v.clone())));
        self.addr_calls.extend(addr_calls.iter().map(|(k, v)| (k.clone(), v.clone())));
        self.self_sends.extend(self_sends.iter().map(|(k, v)| (k.clone(), v.clone())));
        self.linear_moves.extend(linear_moves.iter().copied());
        self.moved_reads.extend(moved_reads.iter().copied());
        self.flow_moves.extend(flow_moves.iter().copied());
        self.fate_mutations.extend(fate_mutations.iter().map(|(k, v)| (k.clone(), v.clone())));
        self.bind_facts.extend(bind_facts.iter().map(|(k, v)| (k.clone(), v.clone())));
        self.derived_uses.extend(derived_uses.iter().map(|(k, v)| (k.clone(), v.clone())));
        self.virtual_candidates.extend(virtual_candidates.iter().map(|(k, v)| (k.clone(), v.clone())));
        self.by_value_loops.extend(by_value_loops.iter().copied());
        self.state_takes.extend(state_takes.iter().copied());
        let _ = actor_gates;
        let _ = actor_sends;
        let _ = task_sends;
        let _ = task_mints;
        self.effect_calls.extend(effect_calls.iter().map(|(k, v)| (k.clone(), v.clone())));
        self.effect_member_calls.extend(effect_member_calls.iter().map(|(k, v)| (k.clone(), v.clone())));
        self.member_calls.extend(member_calls.iter().map(|(k, v)| (k.clone(), v.clone())));
        self.dot_calls.extend(dot_calls.iter().copied());
        let _ = written_types;
        self.local_calls.extend(local_calls.iter().copied());
        self.local_call_names.extend(local_call_names.iter().map(|(k, v)| (k.clone(), v.clone())));
        self.call_effects.extend(call_effects.iter().map(|(k, v)| (k.clone(), v.clone())));
        self.fn_effects.extend(fn_effects.iter().map(|(k, v)| (k.clone(), v.clone())));
        self.decl_effects.extend(decl_effects.iter().map(|(k, v)| (k.clone(), v.clone())));
        self.handler_deps.extend(handler_deps.iter().map(|(k, v)| (k.clone(), v.clone())));
        self.effect_prereqs.extend(effect_prereqs.iter().map(|(k, v)| (k.clone(), v.clone())));
        let _ = fn_body_effects;
        self.call_type_args.extend(call_type_args.iter().map(|(k, v)| (k.clone(), v.clone())));
        self.lambda_effects.extend(lambda_effects.iter().map(|(k, v)| (k.clone(), v.clone())));
        self.fn_value_implicits.extend(fn_value_implicits.iter().map(|(k, v)| (k.clone(), v.clone())));
        self.widen_targets.extend(widen_targets.iter().map(|(k, v)| (k.clone(), v.clone())));
        self.pick_left.extend(pick_left.iter().map(|(k, v)| (k.clone(), v.clone())));
        self.elvis_picks.extend(elvis_picks.iter().map(|(k, v)| (k.clone(), v.clone())));
        self.elvis_tests.extend(elvis_tests.iter().map(|(k, v)| (k.clone(), v.clone())));
        self.elvis_widen.extend(elvis_widen.iter().map(|(k, v)| (k.clone(), v.clone())));
        self.may_throw.extend(may_throw.iter().map(|(k, v)| (k.clone(), v.clone())));
        let _ = deductions;
        self.fn_refs.extend(fn_refs.iter().map(|(k, v)| (k.clone(), v.clone())));
        self.def_refs.extend(def_refs.iter().map(|(k, v)| (k.clone(), v.clone())));
        self.local_decls.extend(local_decls.iter().map(|(k, v)| (k.clone(), v.clone())));
        let _ = type_ref_keys;
        let _ = type_ref_span_keys;
        let _ = visible_keys;
        let _ = visible_handler_keys;
        self.field_refs.extend(field_refs.iter().map(|(k, v)| (k.clone(), v.clone())));
        self.module_refs.extend(module_refs.iter().map(|(k, v)| (k.clone(), v.clone())));
        self.fate_reads.extend(fate_reads.iter().map(|(k, v)| (k.clone(), v.clone())));
        self.interp_to_str.extend(interp_to_str.iter().map(|(k, v)| (k.clone(), v.clone())));
        self.interp_union.extend(interp_union.iter().map(|(k, v)| (k.clone(), v.clone())));
        self.interp_implicit.extend(interp_implicit.iter().map(|(k, v)| (k.clone(), v.clone())));
        self.temp_views.extend(temp_views.iter().copied());
        self.lending_calls.extend(lending_calls.iter().map(|(k, v)| (k.clone(), v.clone())));
        self.fn_lends.extend(fn_lends.iter().map(|(k, v)| (k.clone(), v.clone())));
        self.handle_muts.extend(handle_muts.iter().copied());
        let _ = param_mutations; // the walk's own map, by the caller
        self.moved_projections.extend(moved_projections.iter().copied());
        self.boundary_checks.extend(boundary_checks.iter().map(|(k, v)| (k.clone(), v.clone())));
        self.factories.extend(factories.iter().map(|(k, v)| (k.clone(), v.clone())));
        let _ = platform_closure_all;
        self.lambda_captures.extend(lambda_captures.iter().map(|(k, v)| (k.clone(), v.clone())));
        self.derived_calls.extend(derived_calls.iter().map(|(k, v)| (k.clone(), v.clone())));
        self.distinct_pairs.extend(distinct_pairs.iter().map(|(k, v)| (k.clone(), v.clone())));
        self.mut_lend_calls.extend(mut_lend_calls.iter().copied());
        self.fn_value_calls.extend(fn_value_calls.iter().map(|(k, v)| (k.clone(), v.clone())));
        self.lambda_contracts.extend(lambda_contracts.iter().map(|(k, v)| (k.clone(), v.clone())));
        self.implicit_params.extend(implicit_params.iter().map(|(k, v)| (k.clone(), v.clone())));
        self.established_quals.extend(established_quals.iter().map(|(k, v)| (k.clone(), v.clone())));
        self.carried_identities.extend(carried_identities.iter().map(|(k, v)| (k.clone(), v.clone())));
        self.implicit_args.extend(implicit_args.iter().map(|(k, v)| (k.clone(), v.clone())));
        self.use_handler_args.extend(use_handler_args.iter().map(|(k, v)| (k.clone(), v.clone())));
        self.implicit_members.extend(implicit_members.iter().map(|(k, v)| (k.clone(), v.clone())));
        let _ = errors;
    }
}
