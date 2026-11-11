//! [lsp-check-cache] The front half of the compiler, through one cache: a
//! program parsed, expanded, resolved and checked with a [`CheckCache`] keeps
//! every file's results for the next program of the same files, so the next
//! one parses, stamps, scopes and checks again only what an edit can have
//! changed. std is no special case: it sees only std, so a project's edits
//! never reach it, and a cache warmed with std alone (the language server
//! holds one, the repository's tests one per process) stands in for std in
//! every program over it, cloned for each.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use salvo_syntax::ast::Module;
use salvo_syntax::diag::Diagnostic;

use crate::check_cache::CheckCache;
use crate::expand::FileTest;
use crate::source::{SourceFile, SourceSet};
use crate::{Checked, Expansion, FileDiagnostic, Program, Resolution, Symbols};

/// One file parsed and through its per-file expansions.
pub type Parsed = (Arc<Module>, Vec<Diagnostic>, Vec<FileTest>);

/// A file parsed and through its per-file expansions
/// (`expand::expand_file`): what a program's module for it is before the
/// program-wide expansion.
pub fn parse_file(file: &SourceFile) -> Parsed {
    let (mut module, mut diags) = salvo_syntax::parse_module(&file.content);
    let (tests, more) = crate::expand::expand_file(file, &mut module);
    diags.extend(more);
    (Arc::new(module), diags, tests)
}

/// [lsp-parse-cache] Parsed modules, keyed by the **content** of the file
/// that produced them: parsing is per-file and local, so a file whose text
/// has not changed since it was last parsed reuses its module (and its parse
/// diagnostics). Only the edited file misses on a keystroke.
///
/// The cached module is the parse **with its per-file expansions** (the
/// struct-body hoist, `iter fn`s, `test` blocks), which depend on the file's
/// text, its module path and whether it is an annex — the key. Diagnostics
/// here carry no file index: that is assigned at use, so an entry is valid
/// wherever the file lands. A 64-bit hash over editor-sized files makes a
/// collision no practical risk.
#[derive(Default, Clone)]
pub struct ParseCache {
    entries: HashMap<u64, Parsed>,
    /// The keys the current program has asked for: what `sweep` keeps.
    live: HashSet<u64>,
    /// [lsp-check-cache] Each file's module as the last expansion left it, by
    /// file name: a stamping that made a module equal to the last one hands
    /// the last one back, so what is keyed by its `Arc` sees it unchanged
    /// (`keep_stamped`).
    stamped: HashMap<String, Arc<Module>>,
    /// Each file's last text and key, by name: a file of one name has one
    /// module and is an annex or not, so the same text is the same key, and
    /// comparing texts is much cheaper than hashing them.
    keys: HashMap<String, (String, u64)>,
}

/// What a cached parse is keyed by: the text, and the two facts about the
/// file its per-file expansions read.
fn parse_key(file: &SourceFile) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    let mut eat = |b: u8| {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    };
    eat(file.is_test as u8);
    for b in file.module.to_string().bytes().chain([0]).chain(file.content.bytes()) {
        eat(b);
    }
    h
}

impl ParseCache {
    pub fn new() -> Self {
        Self::default()
    }

    /// The parsed module for `file`: the cached one when its key is known,
    /// otherwise freshly parsed and remembered.
    pub fn parse(&mut self, file: &SourceFile) -> Parsed {
        let key = match self.keys.get(&file.name) {
            Some((text, key)) if *text == file.content => *key,
            _ => {
                let key = parse_key(file);
                self.keys.insert(file.name.clone(), (file.content.clone(), key));
                key
            }
        };
        self.live.insert(key);
        if let Some(hit) = self.entries.get(&key) {
            return hit.clone();
        }
        let parsed = parse_file(file);
        self.entries.insert(key, parsed.clone());
        parsed
    }

    /// Drops every entry the current program did not ask for, so the cache
    /// does not grow as a file is edited.
    pub fn sweep(&mut self) {
        let live = std::mem::take(&mut self.live);
        self.entries.retain(|k, _| live.contains(k));
    }

    /// [lsp-check-cache] Hands back the last expansion's module for every
    /// file whose expanded module equals it.
    pub fn keep_stamped(&mut self, files: &[SourceFile], modules: &mut [Arc<Module>]) {
        let mut next = HashMap::with_capacity(files.len());
        for (i, file) in files.iter().enumerate() {
            if let Some(prev) = self.stamped.get(&file.name) {
                if !Arc::ptr_eq(prev, &modules[i]) && **prev == *modules[i] {
                    modules[i] = prev.clone();
                }
            }
            next.insert(file.name.clone(), modules[i].clone());
        }
        self.stamped = next;
    }
}

/// A program parsed and expanded, with what that reported.
pub struct Built {
    pub program: Program,
    /// Every parse and expansion diagnostic, in file order.
    pub diagnostics: Vec<FileDiagnostic>,
    pub expansion: Expansion,
}

impl CheckCache {
    /// The modules of `files`, parsed through the cache, with their parse
    /// diagnostics (placed at their files) and their tests.
    pub fn parse_files(&mut self, files: &[SourceFile]) -> (Vec<Arc<Module>>, Vec<FileDiagnostic>, Vec<Vec<FileTest>>) {
        let mut modules = Vec::with_capacity(files.len());
        let mut diagnostics = Vec::new();
        let mut tests = Vec::with_capacity(files.len());
        for (i, file) in files.iter().enumerate() {
            let (module, diags, t) = self.parse.parse(file);
            diagnostics.extend(diags.into_iter().map(|d| FileDiagnostic::from_parse(i, d)));
            modules.push(module);
            tests.push(t);
        }
        (modules, diagnostics, tests)
    }

    /// [lsp-expand-cache] [lsp-scope-cache] The program-wide expansion of
    /// `modules` (each file's, parsed), through the cache: the scopes no
    /// edit can have changed are the last expansion's, and so is every
    /// stamped file whose stamping can read nothing new.
    pub fn expand(&mut self, files: &[SourceFile], modules: &mut [Arc<Module>], tests: Vec<Vec<FileTest>>) -> Expansion {
        let given = self.scopes.plan(files, modules);
        let pre: Vec<Arc<Module>> = modules.to_vec();
        let record = !self.transient;
        let mut expansion = crate::expand::expand_program_cached(files, modules, tests, &given, &mut self.expansion, record);
        // [lsp-scope-cache] The scopes over the stamped program that still
        // stand go with the program, so any resolution of it takes them.
        if let Some(interfaces) = expansion.interfaces.as_mut() {
            let stamped = |f: usize| !Arc::ptr_eq(&pre[f], &modules[f]);
            let given = self.stamped_scopes.plan_with(files, modules, &stamped);
            if let Some(i) = Arc::get_mut(interfaces) {
                i.stamped = given;
            }
        }
        if record {
            if let Some(interfaces) = &expansion.interfaces {
                self.scopes.record(files, &pre, interfaces);
            }
            self.parse.keep_stamped(files, modules);
            self.parse.sweep();
        }
        expansion
    }

    /// `sources` parsed and expanded through the cache.
    pub fn build(&mut self, sources: SourceSet) -> Built {
        let (mut modules, mut diagnostics, tests) = self.parse_files(&sources.files);
        let expansion = self.expand(&sources.files, &mut modules, tests);
        diagnostics.extend(expansion.diagnostics.iter().cloned());
        diagnostics.sort_by_key(|d| (d.file, d.span.expn, d.span.start));
        let interfaces = expansion.interfaces.clone();
        Built { program: Program { files: sources.files, modules, companions: sources.companions, interfaces }, diagnostics, expansion }
    }

    /// [lsp-scope-cache] `program` resolved through the cache: the scopes
    /// over the stamped program no edit can have changed are the last
    /// resolution's. A stamped module holds declarations its text does not
    /// show, so a new one counts as changed.
    pub fn resolve<'p>(&mut self, program: &'p Program) -> Resolution<'p> {
        let pre = self.expansion.pre_modules();
        let stamped = |f: usize| pre.get(f).is_none_or(|m| !Arc::ptr_eq(m, &program.modules[f]));
        let given = self.stamped_scopes.plan_with(&program.files, &program.modules, &stamped);
        let resolution = crate::resolve::resolve_given(program, &given);
        if !self.transient {
            self.stamped_scopes.record_scopes(&program.files, &program.modules, &resolution.scope_ids, &resolution.scope_errors);
        }
        resolution
    }

    /// [lsp-check-cache] `program` checked through the cache.
    pub fn check<'p>(&mut self, program: &'p Program, resolution: &Resolution<'p>, symbols: &Symbols<'p>) -> Checked {
        crate::check::check_program_cached(program, resolution, symbols, self, None)
    }

    /// [test-data] [`Self::check`], judging each checker data annotation
    /// `//~ type[T]` at the `type?` it names: every `type` report there
    /// carries a verdict per annotation (`FileDiagnostic::type_verdicts`).
    /// For a transient cache: the verdicts are not the cache's to keep.
    pub fn check_annotated<'p>(&mut self, program: &'p Program, resolution: &Resolution<'p>, symbols: &Symbols<'p>, annotations: &crate::diag::TypeAnnotations) -> Checked {
        crate::check::check_program_cached(program, resolution, symbols, self, Some(annotations))
    }
}

/// [lsp-check-cache] `program` checked through a clone of `cache` when there is
/// one (a cache warmed with std: only what is new is walked), else in full.
/// Either way the result is what a full check gives.
pub fn check_over<'p>(program: &'p Program, resolution: &Resolution<'p>, symbols: &Symbols<'p>, cache: Option<&CheckCache>) -> Checked {
    match cache {
        Some(cache) => cache.transient().check(program, resolution, symbols),
        None => crate::check_program(program, resolution, symbols),
    }
}
