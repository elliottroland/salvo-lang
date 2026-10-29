//! [lsp-std-front] [std-cache] std's front half, built once and shared by
//! every program checked over it: std parsed and expanded as its own
//! program, its modules kept (every program built from the front holds these
//! very `Arc`s, so std is neither parsed, cloned nor expanded again), what the
//! comptime expansion needs to expand a project over them, std's own
//! expansion output, and the `StdCache` — checked over a program sharing the
//! same modules, so its address-keyed rows (`Checked::written_types`) are
//! keyed by the addresses a program built from the front sees, and stay valid
//! while the front lives.
//!
//! The language server holds one for its whole run; the repository's tests
//! hold one per process (TESTS.md T3). Where std comes from is the caller's:
//! the CLI embeds it, the tests read `lib/`.

use std::sync::Arc;

use salvo_syntax::ast::Module;
use salvo_syntax::diag::Diagnostic;

use crate::expand::FileTest;
use crate::source::{CompanionFile, SourceFile, SourceSet};
use crate::{Checked, Expansion, FileDiagnostic, Program, Resolution, StdCache, Symbols};

/// One file parsed and through its per-file expansions.
pub type Parsed = (Arc<Module>, Vec<Diagnostic>, Vec<FileTest>);

/// A file parsed and through its per-file expansions
/// (`expand::expand_file`): what a program's module for it is before the
/// program-wide expansion.
pub fn parse_file(file: &SourceFile) -> Parsed {
    let (mut module, mut diags) = salvo_syntax::parse_module_deferred(&file.content);
    let (tests, more) = crate::expand::expand_file(file, &mut module);
    diags.extend(more);
    (Arc::new(module), diags, tests)
}

/// A parse diagnostic, attributed to file `file`.
pub fn file_diagnostic(file: usize, d: Diagnostic) -> FileDiagnostic {
    FileDiagnostic::from_parse(file, d)
}

pub struct StdFront {
    /// std's files, as loaded: what a program's leading files must be for
    /// the front to stand in for them.
    files: Vec<SourceFile>,
    /// std's companions (its platform host files for one backend), handed
    /// to every program built from the front.
    companions: Vec<CompanionFile>,
    /// std's modules after every expansion.
    modules: Vec<Arc<Module>>,
    /// std's parse and per-file expansion diagnostics (file indices `0..N`).
    parse_diagnostics: Vec<FileDiagnostic>,
    /// std's comptime expansion output (diagnostics, hovers, `[when]` arms).
    expansion: Expansion,
    comptime: crate::comptime::StdComptime,
    cache: StdCache,
}

/// A program built over a front: std's modules are the front's own, the
/// rest parsed and expanded after them.
pub struct FrontProgram {
    pub program: Program,
    /// Every parse and expansion diagnostic, std's first, in file order.
    pub diagnostics: Vec<FileDiagnostic>,
    /// The program's expansion: std's output followed by the rest's.
    pub expansion: Expansion,
}

impl StdFront {
    /// Builds the front over `std`, a source set holding std's files (all
    /// `is_std`) and, when a backend is in view, its companions.
    pub fn build(std: SourceSet) -> StdFront {
        let mut modules = Vec::with_capacity(std.files.len());
        let mut parse_diagnostics: Vec<FileDiagnostic> = Vec::new();
        for file in &std.files {
            let (module, diags, _tests) = parse_file(file);
            parse_diagnostics.extend(diags.into_iter().map(|d| file_diagnostic(modules.len(), d)));
            modules.push(module);
        }
        let (comptime_out, comptime) = crate::comptime::expand_comptime_prefix(&std.files, &mut modules);
        let expansion = crate::expand::program_expansion(Vec::new(), comptime_out);
        let program = Program {
            files: std.files.clone(),
            modules: modules.clone(),
            companions: std.companions.clone(),
        };
        let symbols = Symbols::collect(&program);
        let resolution = crate::resolve(&program);
        let checked = crate::check_program(&program, &resolution, &symbols);
        let cache = StdCache::new(&program, &resolution, &symbols, checked);
        StdFront {
            files: std.files,
            companions: std.companions,
            modules,
            parse_diagnostics,
            expansion,
            comptime,
            cache,
        }
    }

    pub fn files(&self) -> &[SourceFile] {
        &self.files
    }

    pub fn companions(&self) -> &[CompanionFile] {
        &self.companions
    }

    pub fn modules(&self) -> &[Arc<Module>] {
        &self.modules
    }

    pub fn cache(&self) -> &StdCache {
        &self.cache
    }

    /// Whether the front may stand in for the leading files of `files`:
    /// they are exactly the std it was built from (names, modules and
    /// contents), unshadowed, and nothing after them is std except a
    /// shadowing file — so not a `std = true` project, whose files are all
    /// std. Nothing a project declares changes std's expansion: std sees
    /// only std [obligation-by], and no project module is under `std`
    /// [std-root].
    pub fn applies(&self, files: &[SourceFile]) -> bool {
        let n = self.files.len();
        n <= files.len()
            && files[..n].iter().zip(&self.files).all(|(f, g)| {
                f.is_std && !f.is_shadow && f.name == g.name && f.module == g.module && f.is_test == g.is_test && f.content == g.content
            })
            && files[n..].iter().all(|f| !f.is_std || f.is_shadow)
    }

    /// The program-wide expansion of `modules` — the front's own followed
    /// by the rest's, each through [`parse_file`] (their tests in
    /// `per_file`, aligned with `files`) — merged with std's expansion
    /// output, which is not recomputed.
    pub fn expand_after(&self, files: &[SourceFile], modules: &mut [Arc<Module>], per_file: Vec<Vec<FileTest>>) -> Expansion {
        let rest = crate::expand::expand_program_after(files, modules, per_file, &self.comptime);
        let mut expansion = Expansion {
            tests: rest.tests,
            diagnostics: self.expansion.diagnostics.clone(),
            comptime_hovers: self.expansion.comptime_hovers.clone(),
            comptime_when_arms: self.expansion.comptime_when_arms.clone(),
        };
        expansion.diagnostics.extend(rest.diagnostics);
        expansion.comptime_hovers.extend(rest.comptime_hovers);
        expansion.comptime_when_arms.extend(rest.comptime_when_arms);
        expansion
    }

    /// std's parse diagnostics, file indices `0..N`.
    pub fn parse_diagnostics(&self) -> &[FileDiagnostic] {
        &self.parse_diagnostics
    }

    /// A program of std and `extra` (none of them std), with std's modules
    /// the front's own and the rest parsed and expanded after them.
    pub fn program(&self, extra: Vec<SourceFile>, extra_companions: Vec<CompanionFile>) -> FrontProgram {
        let mut files = self.files.clone();
        let mut modules = self.modules.clone();
        let mut per_file: Vec<Vec<FileTest>> = (0..files.len()).map(|_| Vec::new()).collect();
        let mut diagnostics = self.parse_diagnostics.clone();
        for file in extra {
            let (module, diags, tests) = parse_file(&file);
            diagnostics.extend(diags.into_iter().map(|d| file_diagnostic(files.len(), d)));
            files.push(file);
            modules.push(module);
            per_file.push(tests);
        }
        let expansion = self.expand_after(&files, &mut modules, per_file);
        diagnostics.extend(expansion.diagnostics.iter().cloned());
        diagnostics.sort_by_key(|d| (d.file, d.span.expn, d.span.start));
        let mut companions = self.companions.clone();
        companions.extend(extra_companions);
        FrontProgram { program: Program { files, modules, companions }, diagnostics, expansion }
    }

    /// Checks `program` over the front's std: [`crate::check_project`] when
    /// the program is one of the front's ([`check_over`]), else a full
    /// [`crate::check_program`] (a std shadow, a `std = true` project, a std
    /// parsed again).
    pub fn check<'p>(&self, program: &'p Program, resolution: &Resolution<'p>, symbols: &Symbols<'p>) -> Checked {
        check_over(program, resolution, symbols, Some(&self.cache))
    }
}

/// [std-cache] Whether `cache` may stand in for checking `program`'s std: its
/// leading `cache.std_files` files are std and unshadowed, and nothing after
/// them is std except a shadowing file. A std shadow or a `std = true` project
/// (whose files are all `is_std`) fails this and gets a full check (A4).
pub fn cache_applies(program: &Program, cache: &StdCache) -> bool {
    cache.std_files <= program.files.len()
        && program.files[..cache.std_files].iter().all(|f| f.is_std && !f.is_shadow)
        && program.files[cache.std_files..].iter().all(|f| !f.is_std || f.is_shadow)
}

/// [std-cache] Checks `program`, over `cache` when there is one, it applies
/// ([`cache_applies`]) and `program` holds the very std modules it was
/// checked over (`StdCache::shares_modules`, as a program from
/// [`StdFront::program`] does); else in full. The second gate is what makes
/// the result good for **every** row, std's included, and so for emission: a
/// std parsed again has other nodes, and std's address-keyed rows
/// (`written_types`) would not be theirs.
pub fn check_over<'p>(program: &'p Program, resolution: &Resolution<'p>, symbols: &Symbols<'p>, cache: Option<&StdCache>) -> Checked {
    match cache {
        Some(cache) if cache_applies(program, cache) && cache.shares_modules(program) => {
            crate::check_project(program, resolution, symbols, cache)
        }
        _ => crate::check_program(program, resolution, symbols),
    }
}

// A front is shared across a test process's threads (`OnceLock`).
const _: fn() = || {
    fn send_sync<T: Send + Sync>() {}
    send_sync::<StdFront>();
};
