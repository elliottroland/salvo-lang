//! [lsp-check-cache] The per-file checker rows, kept across a language
//! server's analyses (the pipeline record step 8, piece 3).
//!
//! A file's walks (its signature facts, its headers, its bodies) write rows
//! keyed by that file and read only its own text and other declarations'
//! interfaces [decl-interface-pass]. So while a file's module is the very `Arc`
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

use crate::check::{Checked, Key, Ref};
use crate::diag::FileDiagnostic;
use crate::program::Program;
use crate::types::Ty;

/// The walk steps a file goes through, in order [decl-interface-pass].
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
    pub waitfor_sites: Vec<(crate::DeclId, Key)>,
    pub handler_constructs: Vec<(crate::DeclId, crate::DeclId, Key)>,
    pub actor_gates: Vec<(crate::DeclId, Key)>,
    pub actor_sends: Vec<(crate::DeclId, String, Key)>,
    pub task_sends: Vec<(crate::DeclId, String, Key)>,
    pub task_mints: Vec<(crate::DeclId, crate::DeclId, Key)>,
}

impl StepRows {
    /// [lsp-check-cache] Moves what the step recorded to the files' new
    /// places (`Checked::remap_files`); false when something is placed in a
    /// file that is gone.
    fn remap_files(&mut self, map: &dyn Fn(usize) -> Option<usize>) -> bool {
        let mut ok = true;
        let mut at = |f: usize| -> usize {
            match map(f) {
                Some(n) => n,
                None => {
                    ok = false;
                    f
                }
            }
        };
        for d in &mut self.errors {
            d.file = at(d.file);
        }
        for (_, owner, _, _) in &mut self.written {
            *owner = at(*owner);
        }
        for (_, k) in self.waitfor_sites.iter_mut().chain(self.actor_gates.iter_mut()) {
            *k = k.with_file(at(k.file()));
        }
        for (_, _, k) in self.handler_constructs.iter_mut().chain(self.task_mints.iter_mut()) {
            *k = k.with_file(at(k.file()));
        }
        for (_, _, k) in self.actor_sends.iter_mut().chain(self.task_sends.iter_mut()) {
            *k = k.with_file(at(k.file()));
        }
        ok
    }

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
#[derive(Clone, PartialEq, Eq, Hash)]
struct FileId {
    name: String,
    module: crate::source::ModulePath,
    is_std: bool,
    is_shadow: bool,
    is_test: bool,
    dependency: Option<String>,
    /// [doc-example] What an example's file documents, and whether it sees
    /// that module whole: what its scope is built from.
    doc: Option<(crate::source::ModulePath, bool)>,
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
            doc: f.doc_example.as_ref().map(|d| (d.documents.clone(), d.own)),
        }
    }
}

/// [lsp-check-cache] The checker rows of every walked file of a program,
/// kept by the language server between analyses, the way its `ParseCache`
/// keeps parsed modules (`check_project_cached`).
#[derive(Default, Clone)]
pub struct CheckCache {
    files: Vec<FileId>,
    /// Every file's module as last checked: holding them keeps the addresses
    /// the rows are keyed by from being reused.
    modules: Vec<Arc<Module>>,
    /// The first file the walks cover.
    first: usize,
    /// The syntactic parking handlers (the pipeline record 4.5), read off bodies.
    parking: HashSet<String>,
    /// Shared, so that a cache cloned for another program (a warm one, for
    /// each of a test process's programs) shares its files' rows.
    entries: HashMap<usize, Arc<FileRows>>,
    /// Which file's module holds each `ast::Type` node, by address, for every
    /// module in `modules`; `type_nodes[f]` is file `f`'s.
    owners: Arc<HashMap<usize, usize>>,
    type_nodes: Arc<Vec<Vec<usize>>>,
    /// The modules each module's files could see in the last checked
    /// program: who could see a file that has left.
    sees: HashMap<crate::source::ModulePath, HashSet<crate::source::ModulePath>>,
    /// The files the last check walked, in order.
    walked: Vec<usize>,
    /// [lsp-ir-cache] The IR analyses' findings per file, which stand for a
    /// file only while its rows do.
    pub ir: crate::ir::analyze::IrCache,
    /// [lsp-scope-cache] Every file's scope by id, as the last expansion
    /// built it over the declarations as written, and as the last resolution
    /// built it over the stamped program.
    pub scopes: ScopeCache,
    pub stamped_scopes: ScopeCache,
    /// [lsp-check-cache] The keyed rows of every file, merged, as a check left
    /// them, and which file rows they were merged from: a check starts from a
    /// clone of them (taking out the files whose rows are no longer those)
    /// instead of inserting every cached file's rows one by one.
    base: Option<Arc<Checked>>,
    base_rows: HashMap<usize, Arc<FileRows>>,
    /// How many of the base's files the last check took out of it.
    base_misses: usize,
    /// What validating each file's deductions reported, with the module it
    /// was validated over: a file the walk took from the cache is not
    /// validated again.
    deduced: HashMap<usize, (Arc<Module>, crate::deduce::FileDeductions)>,
    /// The files whose rows the current check takes from the cache.
    cached: HashSet<usize>,
    /// A clone made for one program, used once: what it would record for
    /// the next is not recorded.
    pub(crate) transient: bool,
    /// [lsp-parse-cache] Parsed modules, by content.
    pub parse: crate::front::ParseCache,
    /// [lsp-expand-cache] Each stamped file's result.
    pub expansion: ExpandCache,
}

/// [lsp-expand-cache] Each stamped file's result, as the last expansion made
/// it, and what the stamping read: kept so that the next expansion of the
/// same files stamps again only what an edit can have changed.
#[derive(Default, Clone)]
pub struct ExpandCache {
    files: Vec<FileId>,
    /// Each file's module before expansion, and its interface hash.
    pre: Vec<Arc<Module>>,
    ifaces: Vec<u64>,
    stamped: Vec<Option<crate::comptime::StampedFile>>,
    /// The modules each module's files saw in the last expansion.
    sees: HashMap<crate::source::ModulePath, HashSet<crate::source::ModulePath>>,
    /// How many files the last expansion did not stamp again.
    kept: usize,
}

/// [lsp-expand-cache] Whether a file of module `own` (holding `module`) can
/// see module `other` by name: its own, an import of it or of one of its
/// declarations, the tested module of an annex, or `std.core`'s, which every
/// file sees.
fn names_module(module: &Module, own: &crate::source::ModulePath, other: &crate::source::ModulePath) -> bool {
    if own == other || other.0.starts_with(&["std".to_string(), "core".to_string()]) || own.0.starts_with(&other.0) {
        return true;
    }
    module.items.iter().any(|i| match i {
        Item::Import(imp) => {
            let path: Vec<&str> = imp.path.iter().map(|p| p.name.as_str()).collect();
            let m: Vec<&str> = other.0.iter().map(String::as_str).collect();
            path == m || (path.len() == m.len() + 1 && path.starts_with(&m))
        }
        _ => false,
    })
}

/// Whether a module holds a comptime fn: a template, which a stamping in
/// any file may copy.
fn holds_templates(module: &Module) -> bool {
    module.items.iter().any(|i| matches!(i, Item::Fn(f) if f.compfn.is_some()))
}

impl ExpandCache {
    /// How many stamped files the last expansion kept.
    pub fn kept(&self) -> usize {
        self.kept
    }

    /// Each file's module before the last expansion.
    pub fn pre_modules(&self) -> &[Arc<Module>] {
        &self.pre
    }

    /// The stamped files that stand for the files `modules` (each file's
    /// before expansion): a file's when its module is the one it was
    /// stamped from and nothing a stamping reads has changed — no file's
    /// interface (`interface_of`), no file holding a template (any edit of
    /// one: a copy carries the template's positions), and no file joined or
    /// left but a doc example (which nothing sees).
    pub(crate) fn plan(&mut self, files: &[crate::source::SourceFile], modules: &[Arc<Module>]) -> Vec<Option<crate::comptime::StampedFile>> {
        self.kept = 0;
        if self.pre.len() != self.files.len() {
            return Vec::new();
        }
        let ids: Vec<FileId> = files.iter().map(FileId::of).collect();
        let old: HashMap<&FileId, usize> = self.files.iter().enumerate().map(|(i, id)| (id, i)).collect();
        let new: HashSet<&FileId> = ids.iter().collect();
        // What a stamping may read differently now: a module with a changed
        // interface, any edit of a file holding a template, or a file that
        // joined or left (a doc example's aside, which nothing sees).
        let mut touched: HashSet<crate::source::ModulePath> = HashSet::new();
        let mut joined: Vec<&crate::source::ModulePath> = Vec::new();
        for (j, id) in ids.iter().enumerate() {
            match old.get(id) {
                None if id.doc.is_none() => {
                    touched.insert(id.module.clone());
                    joined.push(&id.module);
                }
                None => {}
                Some(&i) if !Arc::ptr_eq(&self.pre[i], &modules[j]) => {
                    if holds_templates(&self.pre[i]) || holds_templates(&modules[j]) || interface_of(&files[j].content, &modules[j]).1 != self.ifaces[i] {
                        touched.insert(id.module.clone());
                    }
                }
                Some(_) => {}
            }
        }
        for id in &self.files {
            if !new.contains(id) && id.doc.is_none() {
                touched.insert(id.module.clone());
            }
        }
        // Who could read them: what saw them in the last expansion, and what
        // names a module that joined (its imports did not resolve then).
        let stale = reach(&self.sees, &touched);
        let reuse: Vec<Option<crate::comptime::StampedFile>> = ids
            .iter()
            .enumerate()
            .map(|(j, id)| {
                let i = *old.get(id)?;
                if !Arc::ptr_eq(&self.pre[i], &modules[j]) || stale.contains(&id.module) {
                    return None;
                }
                if joined.iter().any(|m| names_module(&modules[j], &id.module, m)) {
                    return None;
                }
                self.stamped.get(i)?.clone()
            })
            .collect();
        self.kept = reuse.iter().filter(|r| r.is_some()).count();
        reuse
    }

    /// Records an expansion: `pre` the modules before it, `modules` after,
    /// what it kept (`reuse`, as `plan` answered) and what it stamped.
    pub(crate) fn record(
        &mut self,
        files: &[crate::source::SourceFile],
        pre: &[Arc<Module>],
        modules: &[Arc<Module>],
        reuse: Vec<Option<crate::comptime::StampedFile>>,
        fresh: Vec<crate::comptime::FreshStamp>,
        scope_ids: &[Arc<crate::scope_ids::ScopeIds>],
    ) {
        let mut sees: HashMap<crate::source::ModulePath, HashSet<crate::source::ModulePath>> = HashMap::new();
        for (f, file) in files.iter().enumerate() {
            let entry = sees.entry(file.module.clone()).or_default();
            entry.insert(file.module.clone());
            if let Some(ids) = scope_ids.get(f) {
                entry.extend(ids.modules.iter().map(|m| (*m).clone()));
            }
        }
        self.sees = sees;
        let ids: Vec<FileId> = files.iter().map(FileId::of).collect();
        let old: HashMap<&FileId, usize> = self.files.iter().enumerate().map(|(i, id)| (id, i)).collect();
        let ifaces: Vec<u64> = (0..files.len())
            .map(|j| match old.get(&ids[j]) {
                Some(&i) if self.pre.get(i).is_some_and(|m| Arc::ptr_eq(m, &pre[j])) => self.ifaces[i],
                _ => interface_of(&files[j].content, &pre[j]).1,
            })
            .collect();
        let mut stamped: Vec<Option<crate::comptime::StampedFile>> = reuse;
        stamped.resize(files.len(), None);
        for (j, diagnostics, hovers, when_arms, changed) in fresh {
            stamped[j] = Some(crate::comptime::StampedFile { module: modules[j].clone(), diagnostics, hovers, when_arms, changed });
        }
        self.files = ids;
        self.pre = pre.to_vec();
        self.ifaces = ifaces;
        self.stamped = stamped;
    }
}

/// [lsp-scope-cache] The scopes (by id [decl-id]) an expansion built over a
/// program's declarations as written, and what each was built from: kept so
/// that the next analysis of the same files builds again only the scopes an
/// edit can change.
#[derive(Default, Clone)]
pub struct ScopeCache {
    files: Vec<FileId>,
    /// Each file's module before expansion, and its interface hash
    /// (`interface_of`).
    modules: Vec<Arc<Module>>,
    ifaces: Vec<u64>,
    scopes: Vec<Option<crate::resolve::GivenScope>>,
    /// The files whose scopes the last expansion was given.
    given: usize,
}

impl ScopeCache {
    /// How many scopes the last expansion was given rather than built.
    pub fn given(&self) -> usize {
        self.given
    }

    /// The scopes that stand for `modules` (each file's before expansion):
    /// a file's when the files are the same, and neither the file nor any
    /// module its scope takes in has changed its interface — the text
    /// outside its plain bodies and comments, which is all a scope is built
    /// from. A file whose own text changed keeps its scope only when
    /// building it reported nothing (a report is placed in its text).
    pub fn plan(&mut self, files: &[crate::source::SourceFile], modules: &[Arc<Module>]) -> Vec<Option<crate::resolve::GivenScope>> {
        self.plan_with(files, modules, &|_| false)
    }

    /// [`ScopeCache::plan`] where the files `opaque` says hold declarations
    /// their text does not show (a module comptime stamped into) count as
    /// changed whenever their module is a new one.
    pub fn plan_with(&mut self, files: &[crate::source::SourceFile], modules: &[Arc<Module>], opaque: &dyn Fn(usize) -> bool) -> Vec<Option<crate::resolve::GivenScope>> {
        self.given = 0;
        if self.modules.len() != self.files.len() || self.scopes.len() != self.files.len() {
            *self = ScopeCache::default();
            return Vec::new();
        }
        let ids: Vec<FileId> = files.iter().map(FileId::of).collect();
        let old: HashMap<&FileId, usize> = self.files.iter().enumerate().map(|(i, id)| (id, i)).collect();
        let new: HashSet<&FileId> = ids.iter().collect();
        // The modules whose declarations may differ from what the scopes
        // took in: a changed interface, a file that joined or left (a doc
        // example's aside, which nothing sees).
        let mut touched: HashSet<crate::source::ModulePath> = HashSet::new();
        let mut edited: HashSet<usize> = HashSet::new();
        let mut joined = false;
        for (j, id) in ids.iter().enumerate() {
            match old.get(id) {
                None => {
                    joined = true;
                    if id.doc.is_none() {
                        touched.insert(id.module.clone());
                    }
                }
                Some(&i) if !Arc::ptr_eq(&self.modules[i], &modules[j]) => {
                    edited.insert(j);
                    if opaque(j) || interface_of(&files[j].content, &modules[j]).1 != self.ifaces[i] {
                        touched.insert(id.module.clone());
                    }
                }
                Some(_) => {}
            }
        }
        for id in &self.files {
            if !new.contains(id) && id.doc.is_none() {
                touched.insert(id.module.clone());
            }
        }
        let still: Vec<bool> = (0..self.files.len()).map(|i| ids.get(i).is_some_and(|id| old.get(id) == Some(&i))).collect();
        let all_still = still.iter().all(|s| *s) && ids.len() == self.files.len();
        let stays = |i: usize| all_still || still.get(i).copied().unwrap_or(false);
        let given: Vec<Option<crate::resolve::GivenScope>> = (0..ids.len())
            .map(|j| {
                // The file is where it was, and so is every file its scope names.
                let i = *old.get(&ids[j])?;
                if i != j {
                    return None;
                }
                let g = self.scopes.get(i)?.as_ref()?;
                if touched.contains(&ids[j].module) || g.ids.modules.iter().any(|m| touched.contains(*m)) {
                    return None;
                }
                // A report is placed in the file's text, and one may be about
                // a name a file that joined now declares.
                if !g.errors.is_empty() && (edited.contains(&j) || joined) {
                    return None;
                }
                if !all_still && !g.ids.files().all(stays) {
                    return None;
                }
                Some(g.clone())
            })
            .collect();
        self.given = given.iter().filter(|g| g.is_some()).count();
        given
    }

    /// Records the scopes an expansion built (or was given) over `modules`.
    pub fn record(&mut self, files: &[crate::source::SourceFile], modules: &[Arc<Module>], interfaces: &crate::resolve::Interfaces) {
        self.record_scopes(files, modules, &interfaces.scope_ids, &interfaces.scope_errors);
    }

    /// Records scopes by id and what building each reported, over `modules`.
    pub fn record_scopes(
        &mut self,
        files: &[crate::source::SourceFile],
        modules: &[Arc<Module>],
        scope_ids: &[Arc<crate::scope_ids::ScopeIds>],
        scope_errors: &[Arc<Vec<FileDiagnostic>>],
    ) {
        let n = files.len();
        if scope_ids.len() != n || scope_errors.len() != n || modules.len() != n {
            *self = ScopeCache::default();
            return;
        }
        let ids: Vec<FileId> = files.iter().map(FileId::of).collect();
        let old: HashMap<&FileId, usize> = self.files.iter().enumerate().map(|(i, id)| (id, i)).collect();
        let ifaces: Vec<u64> = (0..n)
            .map(|j| match old.get(&ids[j]) {
                Some(&i) if self.modules.get(i).is_some_and(|m| Arc::ptr_eq(m, &modules[j])) && i < self.ifaces.len() => self.ifaces[i],
                _ => interface_of(&files[j].content, &modules[j]).1,
            })
            .collect();
        self.files = files.iter().map(FileId::of).collect();
        self.modules = modules.to_vec();
        self.ifaces = ifaces;
        self.scopes = scope_ids
            .iter()
            .zip(scope_errors)
            .map(|(ids, errors)| Some(crate::resolve::GivenScope { ids: ids.clone(), errors: errors.clone() }))
            .collect();
    }
}

impl CheckCache {
    pub fn new() -> Self {
        Self::default()
    }

    /// [lsp-warm-std] A clone for one program, used once and dropped (a warm
    /// std cache's, for each program a test checks): what it reads is the
    /// cache's, and nothing is recorded for a next program.
    pub fn transient(&self) -> CheckCache {
        let mut c = self.clone();
        c.transient = true;
        c
    }

    pub(crate) fn is_transient(&self) -> bool {
        self.transient
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
    pub(crate) fn plan(
        &mut self,
        program: &Program,
        first: usize,
        parking: &HashSet<String>,
        scopes: &[crate::resolve::ModuleScope<'_>],
    ) -> HashSet<usize> {
        let n = program.files.len();
        let same_base = self.first == first
            && first <= n
            && self.modules.len() >= first
            && (0..first).all(|i| Arc::ptr_eq(&self.modules[i], &program.modules[i]));
        if !same_base {
            self.entries.clear();
            return HashSet::new();
        }
        // [lsp-check-cache] Rows are a file's, not a position's: when the file
        // list changed, each kept file's rows move to where the file is now,
        // and a file that joined is walked with what can see its module. A
        // file gone clears the cache — what could see it is not known now —
        // unless it was a doc example's, which nothing sees.
        if !(self.files.len() == n && self.files.iter().zip(&program.files).all(|(a, b)| *a == FileId::of(b))) {
            let ids: Vec<FileId> = program.files.iter().map(FileId::of).collect();
            let index: HashMap<&FileId, usize> = ids.iter().enumerate().map(|(j, id)| (id, j)).collect();
            let moves: Vec<Option<usize>> = self.files.iter().map(|id| index.get(id).copied()).collect();
            if (0..first).any(|i| moves.get(i).copied().flatten() != Some(i)) {
                self.entries.clear();
                return HashSet::new();
            }
            // A file that left: what could see its module, in the program
            // that had it, is walked.
            let left: HashSet<crate::source::ModulePath> =
                self.files.iter().zip(&moves).filter(|(id, to)| to.is_none() && id.doc.is_none()).map(|(id, _)| id.module.clone()).collect();
            let blind = reach(&self.sees, &left);
            let old_files = std::mem::take(&mut self.files);
            self.entries.retain(|f, _| old_files.get(*f).is_some_and(|id| !blind.contains(&id.module)));
            let map = |f: usize| moves.get(f).copied().flatten();
            let mut entries = HashMap::with_capacity(self.entries.len());
            for (f, mut e) in std::mem::take(&mut self.entries) {
                let Some(to) = map(f) else { continue };
                // A file whose rows place nothing in a moved or departed
                // file keeps them as they are (and shared).
                let still = |g: usize| map(g) == Some(g);
                if !(to == f && rows_place_only(&e, &still)) {
                    let rows = Arc::make_mut(&mut e);
                    let rows_ok = rows.rows.remap_files(&map);
                    let steps_ok = rows.steps.iter_mut().all(|s| s.remap_files(&map));
                    if !(rows_ok && steps_ok) {
                        continue;
                    }
                }
                entries.insert(to, e);
            }
            self.entries = entries;
            // The base holds keys placed in their files: it stands while none
            // of its files moved.
            if self.base_rows.keys().any(|&f| map(f) != Some(f)) {
                self.base = None;
                self.base_rows.clear();
            }
            // What validating the deductions reported moves the same way.
            let deduced = std::mem::take(&mut self.deduced);
            for (f, (module, mut d)) in deduced {
                let Some(to) = map(f) else { continue };
                let mut ok = true;
                for diag in d.iter_mut().flatten() {
                    match map(diag.file) {
                        Some(n) => diag.file = n,
                        None => ok = false,
                    }
                }
                if ok {
                    self.deduced.insert(to, (module, d));
                }
            }
            // The node owners are found again from the modules at the next
            // record.
            let mut modules: Vec<Arc<Module>> = program.modules.clone();
            for (f, to) in moves.iter().enumerate() {
                if let (Some(to), Some(m)) = (to, self.modules.get(f)) {
                    modules[*to] = m.clone();
                }
            }
            self.modules = modules;
            self.files = ids;
            self.owners = Arc::default();
            self.type_nodes = Arc::default();
        }
        let mut changed: HashMap<usize, (Vec<(u32, u32)>, Vec<(u32, u32)>)> = HashMap::new();
        let mut moved: HashMap<usize, HashMap<usize, usize>> = HashMap::new();
        // The files whose interface changed: walked, with every file that
        // can depend on them.
        let mut reshaped: HashSet<usize> = HashSet::new();
        // A handler whose parking changed (the syntactic set is read off
        // bodies) changes its file's interface: what can see it reads it.
        if self.parking != *parking {
            let flipped: Vec<&String> = self.parking.symmetric_difference(parking).collect();
            for (f, (file, module)) in program.files.iter().zip(&program.modules).enumerate() {
                let declares = module.items.iter().any(|i| {
                    matches!(i, Item::Handler(h) if flipped.iter().any(|k| {
                        crate::typekey::plain(k) == h.name.name
                            && crate::typekey::module_of(k).is_none_or(|m| m == file.module.to_string())
                    }))
                });
                if declares {
                    reshaped.insert(f);
                }
            }
        }
        for f in first..n {
            // A file with no rows (one that joined, or whose rows could not
            // move) is walked, and counted as changed.
            let Some(e) = self.entries.get(&f) else {
                reshaped.insert(f);
                continue;
            };
            if Arc::ptr_eq(&e.module, &program.modules[f]) {
                continue;
            }
            let (bodies, iface) = interface_of(&program.files[f].content, &program.modules[f]);
            let nodes = (iface == e.iface && bodies.len() == e.bodies.len())
                .then(|| node_map(&e.module, &e.bodies, &program.modules[f], &bodies))
                .flatten();
            match nodes {
                Some(nodes) => {
                    moved.insert(f, nodes);
                    changed.insert(f, (e.bodies.clone(), bodies));
                }
                None => {
                    reshaped.insert(f);
                }
            }
        }
        for f in changed.keys().chain(&reshaped) {
            self.entries.remove(f);
        }
        if !reshaped.is_empty() {
            let stale = dependents(program, first, scopes, &reshaped);
            self.entries.retain(|f, e| {
                // A file that can see a reshaped one; one whose rows point
                // into one (a `written_types` row for its node, a definition
                // site there), which no node map can move; and one holding a
                // diagnostic, whose text may read what the whole program
                // declares (an unresolved name's import suggestions).
                !stale.contains(f)
                    && !e.steps.iter().any(|s| {
                        s.written.iter().any(|(_, owner, _, _)| reshaped.contains(owner))
                            || s.errors.iter().any(|d| d.is_error() || !d.suggested_imports.is_empty())
                    })
                    && !e.rows.refs.values().filter_map(Ref::site).any(|site| reshaped.contains(&site.file))
            });
        }
        let mut sound = true;
        for e in self.entries.values_mut() {
            let touches = e.steps.iter().any(|s| s.written.iter().any(|(_, owner, _, _)| moved.contains_key(owner)))
                || (!changed.is_empty() && e.rows.refs.values().filter_map(Ref::site).any(|site| changed.contains_key(&site.file)));
            if !touches {
                continue;
            }
            let e = Arc::make_mut(e);
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
        if !self.transient {
            self.sees = sees_by_module(program, scopes);
        }
        self.cached = self.entries.keys().copied().collect();
        self.cached.clone()
    }

    /// [lsp-check-cache] The base's rows for a check that takes the rows of
    /// `cached` from the cache: a clone, without the files whose rows are no
    /// longer the base's, and the files whose rows it holds.
    pub(crate) fn seed(&mut self, cached: &HashSet<usize>) -> Option<(Checked, HashSet<usize>)> {
        let base = self.base.as_ref()?;
        let mut keyed = (**base).clone();
        let mut held = HashSet::new();
        self.base_misses = 0;
        for (f, rows) in &self.base_rows {
            if cached.contains(f) && self.entries.get(f).is_some_and(|e| Arc::ptr_eq(e, rows)) {
                held.insert(*f);
            } else {
                keyed.remove_rows(&rows.rows);
                self.base_misses += 1;
            }
        }
        Some((keyed, held))
    }

    /// [lsp-check-cache] What validating file `f`'s deductions reported last
    /// time, when the last check did not walk it and it is the same module.
    pub(crate) fn deduced(&self, f: usize, program: &Program) -> Option<crate::deduce::FileDeductions> {
        if !self.cached.contains(&f) {
            return None;
        }
        let (module, d) = self.deduced.get(&f)?;
        Arc::ptr_eq(module, program.modules.get(f)?).then(|| d.clone())
    }

    /// Records what validating each file's deductions reported.
    pub(crate) fn record_deduced(&mut self, program: &Program, deduced: HashMap<usize, crate::deduce::FileDeductions>) {
        self.deduced = deduced.into_iter().filter_map(|(f, d)| Some((f, (program.modules.get(f)?.clone(), d)))).collect();
    }

    /// Not used for this check, which walks all of `files`: what it held
    /// goes, since it is of no use to the next check either.
    pub(crate) fn skip(&mut self, files: &[usize]) {
        // The scopes and the IR findings answer to their own checks.
        let ir = std::mem::take(&mut self.ir);
        let scopes = std::mem::take(&mut self.scopes);
        let stamped_scopes = std::mem::take(&mut self.stamped_scopes);
        let parse = std::mem::take(&mut self.parse);
        let expansion = std::mem::take(&mut self.expansion);
        *self = CheckCache { ir, scopes, stamped_scopes, parse, expansion, ..CheckCache::default() };
        self.walked = files.to_vec();
    }

    /// The rows standing in for file `f`'s walks.
    pub(crate) fn rows(&self, f: usize) -> Option<&FileRows> {
        self.entries.get(&f).map(|e| &**e)
    }

    /// Records the walked files' rows from a check of `program` (after every
    /// body, before the program checks): `out` is the check so far,
    /// `mutations` the walk's `param_mutations`, `steps` what each walked
    /// file's steps appended, and `owners` the walked files' declarations
    /// [decl-rows].
    pub(crate) fn record(
        &mut self,
        program: &Program,
        first: usize,
        parking: &HashSet<String>,
        out: &Checked,
        mutations: &HashMap<crate::check::Key, HashSet<String>>,
        steps: Vec<(usize, [StepRows; 3])>,
        owners: &crate::partition::Owners,
    ) {
        self.sync_owners(program);
        self.files = program.files.iter().map(FileId::of).collect();
        self.modules = program.modules.clone();
        self.first = first;
        self.parking = parking.clone();
        let walked: Vec<usize> = steps.iter().map(|(f, _)| *f).collect();
        let mut parts = out.split_files(program.files.len(), &walked, owners);
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
            for step in &mut steps {
                step.written = compress_written(std::mem::take(&mut step.written));
            }
            let mut rows = parts[f].take().unwrap_or_default();
            rows.param_mutations = mutations
                .iter()
                .filter(|(k, _)| out.decls.loc(k.decl).is_some_and(|l| l.file == f))
                .map(|(k, v)| (*k, v.clone()))
                .collect();
            let (bodies, iface) = interface_of(&program.files[f].content, &program.modules[f]);
            self.entries.insert(
                f,
                Arc::new(FileRows { module: program.modules[f].clone(), iface, bodies, rows, steps }),
            );
        }
        self.walked = walked;
        // A base most of whose files left it is made again, from this check.
        if self.base.is_none() || self.base_misses * 2 > self.base_rows.len() {
            self.base = Some(Arc::new(out.keyed_clone()));
            self.base_rows = self.entries.clone();
            self.base_misses = 0;
        }
    }

    /// Brings `owners` up to `program`'s modules: a file whose module is no
    /// longer the one recorded has its nodes replaced.
    fn sync_owners(&mut self, program: &Program) {
        let n = program.modules.len();
        let stale: Vec<usize> = (0..self.type_nodes.len().max(n))
            .filter(|&i| !(i < n && i < self.type_nodes.len() && self.modules.get(i).is_some_and(|m| Arc::ptr_eq(m, &program.modules[i]))))
            .collect();
        if stale.is_empty() {
            return;
        }
        let owners = Arc::make_mut(&mut self.owners);
        let type_nodes = Arc::make_mut(&mut self.type_nodes);
        for i in stale {
            if let Some(old) = type_nodes.get_mut(i) {
                for at in old.drain(..) {
                    if owners.get(&at) == Some(&i) {
                        owners.remove(&at);
                    }
                }
            }
            if i < n {
                let mut nodes = Vec::new();
                type_nodes_of(&program.modules[i], &mut nodes);
                for &at in &nodes {
                    owners.insert(at, i);
                }
                if i >= type_nodes.len() {
                    type_nodes.resize_with(i + 1, Vec::new);
                }
                type_nodes[i] = nodes;
            }
        }
        type_nodes.truncate(n);
    }
}

/// [lsp-check-cache] Whether everything a file's rows place (definition
/// sites, the files whose nodes its `written_types` rows are for, its
/// diagnostics) is in a file `still` says has not moved.
fn rows_place_only(e: &FileRows, still: &dyn Fn(usize) -> bool) -> bool {
    e.rows.refs.values().filter_map(Ref::site).all(|site| still(site.file))
        && e.steps.iter().all(|s| s.written.iter().all(|(_, owner, _, _)| still(*owner)) && s.errors.iter().all(|d| still(d.file)))
}

/// [lsp-check-cache] What each module's files see: their module, and every
/// module their scopes take in.
fn sees_by_module(program: &Program, scopes: &[crate::resolve::ModuleScope<'_>]) -> HashMap<crate::source::ModulePath, HashSet<crate::source::ModulePath>> {
    let mut out: HashMap<crate::source::ModulePath, HashSet<crate::source::ModulePath>> = HashMap::new();
    for (f, file) in program.files.iter().enumerate() {
        let entry = out.entry(file.module.clone()).or_default();
        entry.insert(file.module.clone());
        if let Some(scope) = scopes.get(f) {
            entry.extend(scope.modules.iter().map(|m| (*m).clone()));
        }
    }
    out
}

/// [lsp-check-cache] The modules that reach one of `from` through `sees`
/// (they see it, or see a module that does), `from` included.
fn reach(
    sees: &HashMap<crate::source::ModulePath, HashSet<crate::source::ModulePath>>,
    from: &HashSet<crate::source::ModulePath>,
) -> HashSet<crate::source::ModulePath> {
    let mut out = from.clone();
    if from.is_empty() {
        return out;
    }
    loop {
        let before = out.len();
        for (m, seen) in sees {
            if !out.contains(m) && seen.iter().any(|s| out.contains(s)) {
                out.insert(m.clone());
            }
        }
        if out.len() == before {
            return out;
        }
    }
}

/// [lsp-check-cache] A step's `written_types` rows with the same effect on
/// any table they are applied to, one per node: the last plain write when
/// there is one (it overwrites whatever came before), else the first
/// write-where-absent. A body walk lowers a callee's signature at every
/// call, so a std tree's body steps wrote ~210k rows for ~5k nodes, and
/// applying them was most of a cached check (2026-10-31, 32ms of 44).
fn compress_written(rows: Vec<(usize, usize, Ty, bool)>) -> Vec<(usize, usize, Ty, bool)> {
    let mut at: HashMap<usize, usize> = HashMap::with_capacity(rows.len() / 8);
    let mut out: Vec<(usize, usize, Ty, bool)> = Vec::new();
    for row in rows {
        match at.get(&row.0) {
            None => {
                at.insert(row.0, out.len());
                out.push(row);
            }
            // A plain write replaces what the node had.
            Some(&i) if !row.3 => out[i] = row,
            // A write where absent after another write does nothing.
            Some(_) => {}
        }
    }
    out
}

/// [lsp-check-cache] Whether a fn's body is read by no interface: an
/// ordinary hand-written body. A comptime fn's, a stamped one's, and an
/// `iter fn`'s generated ones are part of the interface (an inferring
/// minter's body is checked with the interfaces [handler-state-infer]).
fn plain_body(f: &ast::FnDecl) -> Option<Span> {
    let body = f.body.as_ref()?;
    let plain = f.compfn.is_none() && f.stamped.is_none() && f.by.is_none() && !f.is_iter && f.iter_state_name.is_none();
    plain.then_some(body.span)
}

/// [lsp-check-cache] A file's plain fn bodies (top-level fns, struct fns,
/// handler members and qualifier fns) and its comment runs, as disjoint byte
/// ranges in order, and the hash of its text with them cut out: what a
/// body-or-comment edit leaves alone.
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
    // Comments are no interface either: no other file's checking reads
    // them, so an edit in a doc comment (an example's code included
    // [doc-example]) walks its own file (and an example its own) only. Two
    // exceptions keep them: a run holding a `@whynot`, the template a
    // `qualifies` failure answers with, which a call site's interpolation
    // reads [qual-whynot]; and every comment of a file comptime stamps into,
    // since a stamping may read a doc comment [comptime-docs].
    if !crate::comptime::stamps_anything(module) {
        spans.extend(comment_runs(content));
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

/// [lsp-check-cache] The files from `first` on whose checking can read an
/// interface of a file in `reshaped`: those whose scope takes in its module,
/// followed through the modules each module's files see (a signature read
/// through an import names types of modules the reader never imports).
fn dependents(program: &Program, first: usize, scopes: &[crate::resolve::ModuleScope<'_>], reshaped: &HashSet<usize>) -> HashSet<usize> {
    use crate::source::ModulePath;
    // What each module's files see, in one step.
    let mut sees: HashMap<&ModulePath, HashSet<&ModulePath>> = HashMap::new();
    for (f, file) in program.files.iter().enumerate() {
        let entry = sees.entry(&file.module).or_default();
        entry.insert(&file.module);
        if let Some(scope) = scopes.get(f) {
            entry.extend(scope.modules.iter().copied());
        }
    }
    let target: HashSet<&ModulePath> = reshaped.iter().map(|&f| &program.files[f].module).collect();
    // A module reaches a reshaped one when it sees one, or sees a module
    // that reaches one: grown to a fixed point.
    let mut reaches: HashSet<&ModulePath> = target.clone();
    loop {
        let before = reaches.len();
        for (m, seen) in &sees {
            if !reaches.contains(m) && seen.iter().any(|s| reaches.contains(s)) {
                reaches.insert(m);
            }
        }
        if reaches.len() == before {
            break;
        }
    }
    (first..program.files.len())
        .filter(|&f| {
            let own = reaches.contains(&program.files[f].module);
            own || scopes.get(f).is_some_and(|s| s.modules.iter().any(|m| reaches.contains(m)))
        })
        .collect()
}

/// [lsp-check-cache] The byte ranges of `content`'s runs of own-line `//`
/// comments, each from its first line's start to its last line's end, but a
/// run holding a `@whynot` tag.
fn comment_runs(content: &str) -> Vec<(u32, u32)> {
    let mut out = Vec::new();
    let mut at = 0u32;
    let mut run: Option<(u32, u32, bool)> = None;
    for line in content.split_inclusive('\n') {
        let text = line.trim_end_matches('\n');
        let comment = text.trim_start().starts_with("//");
        let end = at + text.len() as u32;
        match (&mut run, comment) {
            (Some((_, e, whynot)), true) => {
                *e = end;
                *whynot |= text.contains("@whynot");
            }
            (None, true) => run = Some((at, end, text.contains("@whynot"))),
            (Some(_), false) => {
                if let Some((s, e, false)) = run.take() {
                    out.push((s, e));
                }
            }
            (None, false) => {}
        }
        at += line.len() as u32;
    }
    if let Some((s, e, false)) = run {
        out.push((s, e));
    }
    out
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
        out: Vec<(usize, Span)>,
    }
    impl Visitor for Collect<'_> {
        fn visit_type(&mut self, t: &ast::Type) {
            let at = t.span();
            // [node-origin] A copied node is at its template's range, which no
            // body of this file holds.
            if at.is_expanded() || !self.bodies.iter().any(|(s, e)| *s <= at.start && at.start < *e) {
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
        // [node-origin] A copied node's range is its template's, which a body-only
        // edit of this file leaves where it was.
        let same = if opos.is_expanded() {
            opos == npos
        } else {
            npos.is_written() && remap(opos.start, old_bodies, new_bodies) == Some(npos.start)
        };
        if !same {
            return None;
        }
        map.insert(oa, na);
    }
    Some(map)
}

/// [lsp-check-cache] Moves a cached file's definition sites into the changed
/// files (`refs`). False when one cannot be moved.
fn remap_sites(rows: &mut Checked, changed: &HashMap<usize, (Vec<(u32, u32)>, Vec<(u32, u32)>)>) -> bool {
    let mut ok = true;
    for site in rows.refs.values_mut().filter_map(Ref::site_mut) {
        let Some((old, new)) = changed.get(&site.file) else { continue };
        // [node-origin] A copied node's range is its template's: it does not move.
        if site.span.is_expanded() {
            continue;
        }
        match (remap(site.span.start, old, new), remap(site.span.end, old, new)) {
            (Some(s), Some(e)) => site.span = Span { start: s, end: e, ..site.span },
            _ => ok = false,
        }
    }
    ok
}

/// [lsp-check-cache] The address of every `ast::Type` node `module` holds:
/// what tells a `written_types` row which file's node it is for.
pub(crate) fn type_nodes_of(module: &Module, out: &mut Vec<usize>) {
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
        let ast::Param { annotations, name: _, ty, variadic: _, implicit: _, default, span: _ } = p;
        for a in annotations {
            v::walk_type_ref(c, a);
        }
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
        effect_generics,
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
    c.enter_generics(&f.generics, &f.generic_canbe);
    // [effect-var] Which of them are effect variables.
    c.enter_effect_generics(effect_generics);
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
    c.leave_generics();
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
    let ast::RefnDecl { docs: _, name: _, at: _, generics, params: ps, wildcard: _, deductions, response, span: _ } = r;
    c.enter_generics(generics, &[]);
    params(c, ps);
    for ast::RefnDeduction { param: _, add, remove, preserve, span: _ } in deductions {
        for t in add.iter().chain(remove).chain(preserve) {
            v::walk_type_ref(c, t);
        }
    }
    for r in response.iter().flatten() {
        v::walk_type_ref(c, &r.qual);
    }
    c.leave_generics();
}
pub(crate) fn walk_item<V: Visitor>(c: &mut V, it: &Item) {
    // [name-table] The type parameters the item brings into scope, around
    // everything it holds (its member fns add their own in `func`).
    let (generics, canbe_list): (&[ast::Ident], &[(ast::Ident, ast::TypeRef)]) = match it {
        Item::Type(d) => (&d.generics, &d.generic_canbe),
        Item::Struct(d) => (&d.generics, &d.generic_canbe),
        Item::Qualifier(d) => (&d.generics, &d.generic_canbe),
        Item::Effect(d) => (&d.generics, &[]),
        Item::Handler(d) => (&d.generics, &[]),
        Item::Fns(d) => (&d.generics, &[]),
        Item::Rename(d) => (&d.generics, &[]),
        _ => (&[], &[]),
    };
    c.enter_generics(generics, canbe_list);
    // [effect-var] A struct's or a group's effect variables.
    match it {
        Item::Struct(d) => c.enter_effect_generics(&d.effect_generics),
        Item::Fns(d) => c.enter_effect_generics(&d.effect_generics),
        _ => {}
    }
    walk_item_in(c, it);
    c.leave_generics();
}

fn walk_item_in<V: Visitor>(c: &mut V, it: &Item) {
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
            refns,
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
            for r in refns {
                refn(c, r);
            }
        }
        Item::Struct(ast::StructDecl {
            docs: _,
            annotations,
            exported: _,
            name: _,
            generics: _,
            generic_canbe,
            fn_slots,
            effect_generics: _,
            obligations: obs,
            auto_qualifiers,
            fields,
            fns,
            refns,
            linear: _,
            opaque: _,
            noremote: _,
            comptime: _,
            message_arm: _,
            span: _,
        }) => {
            for r in refns {
                refn(c, r);
            }
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
            of_claims: _,
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
        Item::Fns(ast::FnsDecl { docs: _, exported: _, name: _, generics: _, effect_generics: _, fns, deductions, span: _ }) => {
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
