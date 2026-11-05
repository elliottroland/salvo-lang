//! The shared front-half pipeline: load embedded std + a source
//! directory, parse, resolve, and type-check. Used by `salvo analyze`
//! [cli-analyze] and the language server [cli-lsp].

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use include_dir::{include_dir, Dir};

use salvo_core::{Checked, DefSite, FileDiagnostic, Program, SourceSet, Symbols};
use salvo_syntax::ast::Module;

pub use salvo_core::front::ParseCache;

/// The standard library, embedded into the binary at build time: the tree
/// under `lib/`, whose own manifest (`lib/salvo.toml`) makes `lib/` the
/// source root, so `lib/std/core/list.sv` is module `std.core.list` [std-root]
/// and `platform/std/time.kt` is the host file of `std.time`
/// [platform-root]. The `.sv` files reach their target languages through the
/// backends' `intrinsic` lowerings [backend-intrinsic] and std's platform
/// host files, not through per-backend source files.
static STD_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/../../lib");

/// The result of analyzing a source directory.
pub struct Analysis {
    /// [manifest-discovery] The source root the analysis ran over — the
    /// project's `src`, or the workspace root when no manifest was found —
    /// against which every `SourceFile.name` resolves to a path.
    pub root: PathBuf,
    /// The project the root belongs to, when a manifest was found.
    pub project: Option<salvo_core::Project>,
    pub program: Program,
    /// Parse + resolution + type diagnostics [diag-structured], in file
    /// order (checker diagnostics after parse diagnostics). [diag-std-hidden]
    /// None located in the embedded std: those are counted in `std_errors`.
    pub diagnostics: Vec<FileDiagnostic>,
    /// [diag-std-hidden] How many errors located in the embedded std were
    /// dropped from `diagnostics`: a nonzero count is shown as one line
    /// (`salvo_core::diag::std_errors_line`), and fails the analysis.
    pub std_errors: usize,
    /// Checker side tables (for hover etc.). Checking always runs, even
    /// with parse errors: broken files participate with their recovered
    /// ASTs but contribute no resolution/checker diagnostics of their own
    /// (only parse ones). Its `errors` have been drained into
    /// `diagnostics`.
    pub checked: Option<Checked>,
    /// Files that could not be loaded (does not abort the analysis): one
    /// rendered message each.
    pub io_errors: Vec<String>,
    /// [lsp-hover-overloads] Per file (aligned with `program.files`): every
    /// name that has **more than one** declaration visible there, and where
    /// each of those declarations is written — free fns and effect members
    /// alike, since a call site cannot tell them apart either
    /// [effect-member-overload].
    ///
    /// Kept as owned data rather than the `Resolution` itself, which borrows
    /// the program: this is the slice hover needs, so it is extracted while
    /// the resolution is alive (user request 2026-09-18).
    pub overloads: Vec<HashMap<String, Vec<DefSite>>>,
    /// [comptime-fields] Hover text for the comptime names inside `comptime
    /// fn` bodies, recorded by the expansion — the checker never sees those
    /// bodies, so this is their only source.
    pub comptime_hovers: Vec<salvo_core::CompHover>,
    /// [lsp-completion] The arms each comptime `[when …]` has yet to cover.
    pub comptime_when_arms: Vec<salvo_core::CompWhenArms>,
    /// [lsp-completion] Per file (aligned with `program.files`): every free
    /// fn visible there, for completion.
    pub completions: Vec<Vec<CompletionFn>>,
    /// [lsp-completion] Per file: every type-like name visible there —
    /// structs, effects, handlers, qualifiers, aliases and opaque types —
    /// with what kind of declaration it is.
    pub type_completions: Vec<Vec<(String, TypeKind)>>,
    /// [ir-linear-check] The `(file, span start)` of every diagnostic that
    /// *relies on the IR* — the linear-obligation leak check (`ir::analyze`).
    /// the IR dump dumps the IR anyway and inlines these as `!!! …`, so they
    /// must not block the dump the way a parse/type error does.
    pub ir_reliant: std::collections::HashSet<(usize, salvo_syntax::Span)>,
}

/// [lsp-completion] What a type-like completion names.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum TypeKind {
    Struct,
    Effect,
    Handler,
    Qualifier,
    Type,
}

/// [lsp-completion] One function a file can call, as completion offers it.
#[derive(Clone, Debug)]
pub struct CompletionFn {
    pub name: String,
    /// The key of a free fn, for rendering its signature with deductions.
    /// `None` for an effect member, which is not keyed by its `DeclId`: its signature is
    /// precomputed in `detail` and its availability gated by `effect`.
    pub key: Option<salvo_core::DeclId>,
    /// The base type name of the first parameter, which dot-notation's
    /// receiver fills [fn-uniform]; `None` when it has none or it is one of the
    /// fn's own type parameters.
    pub receiver: Option<String>,
    /// The first parameter is a type parameter: any receiver fits.
    pub generic_receiver: bool,
    /// [effect-member-completion] The name of the owning effect when this is
    /// an effect member — completion offers it only where that effect is
    /// available. `None` for a free fn.
    pub effect: Option<String>,
    /// A precomputed signature detail, used when there is no `key` (effect
    /// members). `None` for a free fn, whose detail is rendered from `key`.
    pub detail: Option<String>,
}

/// Parses, resolves, and type-checks `src` (plus the embedded std).
///
/// `native_ext` is the active backend's companion extension
/// ([backend-companion]); an empty string loads `.sv` files only, which is
/// what backend-neutral analysis wants — companions are copied, never
/// checked. `project` is the manifest the sources belong to, when one was
/// found: it names the dependencies to load [manifest-deps] and whether the
/// tree is std. `overlay` maps absolute file paths to in-editor contents that
/// replace (or add to) what is on disk [cli-lsp].
///
/// Errors only when `src` is not a directory.
pub fn analyze_sources(
    src: &Path,
    project: Option<salvo_core::Project>,
    native_ext: &str,
    overlay: &HashMap<PathBuf, String>,
) -> Result<Analysis, String> {
    analyze_project(src, project, native_ext, overlay, None)
}

/// [lsp-check-cache] A cache warmed with `native_ext`'s embedded std alone:
/// std parsed, expanded, resolved and checked once. The language server
/// starts every project's cache from a clone of it, so std, which sees only
/// std, is never parsed, stamped or checked again.
pub fn warm_std_cache(native_ext: &str) -> salvo_core::CheckCache {
    let mut sources = SourceSet::default();
    load_embedded_std(&mut sources, native_ext);
    let mut cache = salvo_core::CheckCache::new();
    let built = cache.build(sources);
    let symbols = Symbols::collect(&built.program);
    let resolution = cache.resolve(&built.program);
    cache.check(&built.program, &resolution, &symbols);
    cache
}

/// [lsp-std-source] A full `Analysis` of the embedded standard library, as its
/// own program, so the language server can answer hover and go-to-definition
/// *inside* a std file a user navigated into (the read-only copies
/// `embedded_std_copy` writes under the temp dir). Checking std standalone
/// (`check_program`, no cache) records the references within it
/// (`fn_refs`/`refs`), which a project's analysis keeps hidden.
///
/// `root` is the temp directory the std copies live under, so a copy's path
/// (`<root>/std/core/basic.sv`) equals `root.join(file.name)` — the pivot
/// `doc_view` and `site_location` use. Built once and reused (std never
/// changes within a server run).
pub fn analyze_std() -> Analysis {
    let mut sources = SourceSet::default();
    load_embedded_std(&mut sources, "");
    let mut modules = Vec::with_capacity(sources.files.len());
    let mut diagnostics: Vec<FileDiagnostic> = Vec::new();
    for file in &sources.files {
        let (module, diags) = salvo_syntax::parse_module_deferred(&file.content);
        diagnostics.extend(diags.into_iter().map(|d| FileDiagnostic::from_parse(modules.len(), d)));
        modules.push(module);
    }
    let expansion = salvo_core::expand(&sources.files, &mut modules);
    diagnostics.extend(expansion.diagnostics);
    let comptime_hovers = expansion.comptime_hovers;
    let comptime_when_arms = expansion.comptime_when_arms;
    let program = Program {
        files: sources.files,
        modules: Program::share(modules),
        companions: sources.companions,
        interfaces: expansion.interfaces,
    };
    let symbols = Symbols::collect(&program);
    let resolution = salvo_core::resolve(&program);
    // [lsp-std-source] Every std file is a view here.
    let viewable = vec![true; program.files.len()];
    let overloads = overload_index(&resolution, &viewable);
    let completions = completion_index(&resolution, &viewable);
    let type_completions = type_completion_index(&resolution, &viewable);
    let mut checked = salvo_core::check_program(&program, &resolution, &symbols);
    // The std library's own diagnostics are not published (the copies are
    // read-only); drain them so `checked` matches the shape other analyses have.
    checked.errors.clear();
    Analysis {
        root: std_copy_root(),
        project: None,
        program,
        diagnostics: Vec::new(),
        std_errors: 0,
        checked: Some(checked),
        io_errors: Vec::new(),
        overloads,
        comptime_hovers,
        comptime_when_arms,
        completions,
        type_completions,
        ir_reliant: std::collections::HashSet::new(),
    }
}

/// [lsp-std-source] The temp directory the embedded-std copies live under,
/// **per server process**: `<temp>/salvo-std-<version>-<pid>`. A std file
/// named `lib/std/core/basic.sv` is written at `<this>/std/core/basic.sv`, so
/// this is the `Analysis::root` that makes `root.join(name)` resolve a copy's
/// path. The pid keeps concurrent servers (several editor windows, or a test
/// suite's parallel servers) from sharing a directory — so one server's
/// shutdown cleanup cannot delete another's copies.
pub fn std_copy_root() -> PathBuf {
    std::env::temp_dir().join(format!("salvo-std-{}-{}", env!("CARGO_PKG_VERSION"), std::process::id()))
}

/// [lsp-parse-cache] [manifest-discovery] `analyze_sources` for a document,
/// reusing a server-held parse cache so unchanged files skip lexing and
/// parsing. The nearest manifest above `path` decides the source root and
/// whether the tree is std; with none, `fallback` (the workspace root) is
/// analysed and the analysis says so (`manifest_missing`). The cache is
/// advisory — behaviour matches a fresh parse. [lsp-check-cache] So is
/// `check_cache`: the checker rows of the files an edit left alone.
pub fn analyze_for_document_cached(
    path: &Path,
    fallback: &Path,
    overlay: &HashMap<PathBuf, String>,
    cache: &mut salvo_core::CheckCache,
) -> Result<Analysis, String> {
    match salvo_core::Project::find(path)? {
        Some(project) => {
            let src = project.src();
            analyze_project_with(&src, Some(project), "", overlay, Some(cache), Examples::Open)
        }
        None => analyze_project_with(fallback, None, "", overlay, Some(cache), Examples::Open),
    }
}

fn analyze_project(
    src: &Path,
    project: Option<salvo_core::Project>,
    native_ext: &str,
    overlay: &HashMap<PathBuf, String>,
    cache: Option<&mut salvo_core::CheckCache>,
) -> Result<Analysis, String> {
    analyze_project_with(src, project, native_ext, overlay, cache, Examples::All)
}

/// [doc-example] Whose doc examples an analysis checks: every own file's
/// (`salvo analyze`, `salvo docs`), or only those of the documents open in
/// the editor (the language server: a keystroke in one file does not check
/// the examples of every other).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Examples {
    All,
    Open,
}

/// [`analyze_project`], checking the examples `examples` says.
/// [lsp-check-cache] With a `cache`, every file an edit left alone keeps
/// what parsing, stamping, scoping, checking and the IR analyses made of it.
fn analyze_project_with(
    src: &Path,
    project: Option<salvo_core::Project>,
    native_ext: &str,
    overlay: &HashMap<PathBuf, String>,
    mut cache: Option<&mut salvo_core::CheckCache>,
    examples: Examples,
) -> Result<Analysis, String> {
    if !src.is_dir() {
        return Err(format!(
            "source directory `{}` does not exist",
            src.display()
        ));
    }
    let root = src.canonicalize().unwrap_or_else(|_| src.to_path_buf());

    let load = salvo_core::timing::phase("load");
    let mut sources = SourceSet::default();
    let t = salvo_core::timing::phase("  load: embedded std");
    load_embedded_std(&mut sources, native_ext);
    drop(t);
    // [manifest-deps] Dependencies come between std and the project's tree.
    let t = salvo_core::timing::phase("  load: dependencies");
    let mut io_errors = load_dependencies(&mut sources, project.as_ref(), native_ext);
    drop(t);
    // [test-file] `analyze` checks test annexes: a broken test is a broken
    // program, and the language server wants diagnostics in the file being
    // edited (user decision 2026-09-23).
    // [platform-root] The project's platform roots, when it names any.
    if let Some(project) = &project {
        sources.platform = project.platform_roots();
    }
    let t = salvo_core::timing::phase("  load: source tree");
    io_errors.extend(sources.add_dir(&root, native_ext, false, true));
    drop(t);
    // [std-shadow] A tree that declares std's own modules replaces them.
    sources.apply_std_shadow();
    // [manifest] `std = true`: the whole tree is std.
    if project.as_ref().is_some_and(|p| p.is_std()) {
        sources.mark_std_tree();
    }

    // Overlay: open-editor contents win over the disk [cli-lsp]. Files
    // not on disk yet (new unsaved buffers under the root) are added.
    if !overlay.is_empty() {
        // [std-shadow] A shadowing file is `is_std` for the checker but a
        // real on-disk file for the editor: its open buffer replaces its
        // disk content like any other, or the buffer would be *added*
        // beside it and every declaration would report as a duplicate.
        for file in sources.files.iter_mut().filter(|f| !f.is_std || f.is_shadow) {
            if let Some(content) = overlay.get(&root.join(&file.name)) {
                file.content = content.clone();
            }
        }
        for (path, content) in overlay {
            let Ok(rel) = path.strip_prefix(&root) else { continue };
            if sources
                .files
                .iter()
                .any(|f| (!f.is_std || f.is_shadow) && Path::new(&f.name) == rel)
            {
                continue;
            }
            let Ok(module) = SourceSet::classify(rel) else {
                continue;
            };
            sources.add(rel.display().to_string(), module, content.clone(), false);
        }
    }

    drop(load);
    // Parse every module, attributing diagnostics to files
    // [diag-structured]. The per-file expansions run with the parse
    // (`parse_file`); the program-level one (comptime) after.
    // [lsp-parse-cache] With a cache, an unchanged file reuses its parsed
    // module.
    let mut ir_reliant: std::collections::HashSet<(usize, salvo_syntax::Span)> = std::collections::HashSet::new();
    let t = salvo_core::timing::phase("parse");
    let parse = |idx: usize, file: &salvo_core::SourceFile, cache: &mut Option<&mut salvo_core::CheckCache>, diagnostics: &mut Vec<FileDiagnostic>| {
        let (module, diags, tests) = match cache.as_deref_mut() {
            Some(cache) => cache.parse.parse(file),
            None => salvo_core::front::parse_file(file),
        };
        diagnostics.extend(diags.into_iter().map(|d| FileDiagnostic::from_parse(idx, d)));
        (module, tests)
    };
    let mut diagnostics: Vec<FileDiagnostic> = Vec::new();
    let mut modules: Vec<Arc<Module>> = Vec::with_capacity(sources.files.len());
    let mut per_file_tests = Vec::with_capacity(sources.files.len());
    for idx in 0..sources.files.len() {
        let (module, tests) = parse(idx, &sources.files[idx], &mut cache, &mut diagnostics);
        modules.push(module);
        per_file_tests.push(tests);
    }
    // [doc-example] The examples in the project's own doc comments are files
    // of their own, made from the parsed hosts and parsed as any file is
    // (through the cache: an unedited example is a hit).
    {
        let std_project = project.as_ref().is_some_and(|p| p.is_std());
        let open = |f: &salvo_core::SourceFile| examples == Examples::All || overlay.contains_key(&root.join(&f.name));
        let hosts: Vec<usize> =
            (0..sources.files.len()).filter(|&i| own_file(&sources.files[i], std_project) && open(&sources.files[i])).collect();
        for host in hosts {
            let found = salvo_core::doc_examples::extract(&sources.files[host], host, &modules[host]);
            sources.files.extend(found);
        }
        for idx in modules.len()..sources.files.len() {
            let (module, tests) = parse(idx, &sources.files[idx], &mut cache, &mut diagnostics);
            modules.push(module);
            per_file_tests.push(tests);
        }
    }
    drop(t);
    let t = salvo_core::timing::phase("expand");
    let expansion = match cache.as_deref_mut() {
        Some(c) => c.expand(&sources.files, &mut modules, per_file_tests),
        None => salvo_core::expand::expand_program(&sources.files, &mut modules, per_file_tests),
    };
    drop(t);
    diagnostics.extend(expansion.diagnostics);
    let comptime_hovers = expansion.comptime_hovers;
    let comptime_when_arms = expansion.comptime_when_arms;
    diagnostics.sort_by_key(|d| (d.file, d.span.expn, d.span.start));
    let parse_broken: std::collections::HashSet<usize> = diagnostics
        .iter()
        .filter(|d| d.is_error())
        .map(|d| d.file)
        .collect();

    let program = Program {
        files: sources.files,
        modules,
        companions: sources.companions,
        interfaces: expansion.interfaces,
    };

    // Resolve + check always run — a parse error in one file must not
    // suppress diagnostics for the others [cli-analyze]. Parse-broken
    // files participate with their recovered ASTs (so what did parse
    // still resolves for other files), but their own resolution/checker
    // diagnostics are dropped: recovered ASTs cascade nonsense, and the
    // parse errors are the actionable signal there.
    let (checked, overloads, completions, type_completions) = {
        let t = salvo_core::timing::phase("resolve");
        let t2 = salvo_core::timing::phase("  resolve: symbols");
        let symbols = Symbols::collect(&program);
        drop(t2);
        let t2 = salvo_core::timing::phase("  resolve: scopes");
        // [lsp-scope-cache] Through the cache, the scopes no edit can have
        // changed are the last resolution's.
        let resolution = match cache.as_deref_mut() {
            Some(c) => c.resolve(&program),
            None => salvo_core::resolve(&program),
        };
        drop(t2);
        let t2 = salvo_core::timing::phase("  resolve: lsp indexes");
        // [lsp-index-viewable] Only for the files a view of this analysis can
        // show: an embedded std file is opened through `analyze_std`'s own
        // analysis, never this one (`doc_view`), so its entries stay empty.
        // [lsp-index-viewable] The language server asks about open documents
        // only (and the examples made from them), so with `Examples::Open`
        // the indexes cover those: an edit pays for the documents in view,
        // not the whole tree (24ms of a `lib/` keystroke, 2026-10-31).
        let in_view = |f: &salvo_core::SourceFile| examples == Examples::All || f.doc_example.is_some() || overlay.contains_key(&root.join(&f.name));
        let viewable: Vec<bool> = program.files.iter().map(|f| (!f.is_std || f.is_shadow) && in_view(f)).collect();
        let overloads = overload_index(&resolution, &viewable);
        let completions = completion_index(&resolution, &viewable);
        let type_completions = type_completion_index(&resolution, &viewable);
        drop(t2);
        drop(t);
        // `check_program` folds resolution errors into its own.
        let t = salvo_core::timing::phase("check");
        let mut checked = match cache.as_deref_mut() {
            Some(c) => c.check(&program, &resolution, &symbols),
            None => salvo_core::check_program(&program, &resolution, &symbols),
        };
        drop(t);
        checked.errors.retain(|d| !parse_broken.contains(&d.file));
        // [ir-analyze] The IR-level analyses run per declaration: every one
        // whose own checking (and its file's parse) found no error is built
        // and checked, so a type error in one fn does not hide another's
        // ownership errors, and a broken declaration reports its own errors
        // only (`ir::gate`). They are listed after the checker's.
        let ir_diags = {
            let _t = salvo_core::timing::phase("ir analyze");
            // [lsp-ir-cache] With the check cache, a file it did not walk
            // keeps its findings.
            let (ir_diags, decision) = match cache {
                Some(c) => {
                    let walked = c.walked().to_vec();
                    salvo_core::ir::analyze::analyze_cached(&program, &symbols, &resolution, &checked, &diagnostics, &walked, &mut c.ir)
                }
                None => salvo_core::ir::analyze::analyze_full(&program, &symbols, &resolution, &checked, &diagnostics),
            };
            // [fate-link] [fate-move-mode] A binding that took ownership
            // presents no links (the IR decides which did).
            salvo_core::ir::analyze::drop_owned_fate_reads(&mut checked, &decision);
            for d in &ir_diags {
                ir_reliant.insert((d.file, d.span));
            }
            ir_diags
        };
        diagnostics.append(&mut checked.errors);
        diagnostics.extend(ir_diags);
        // [platform-root] Platform declarations need a platform root.
        let backends = project.as_ref().map(salvo_core::required_backends).unwrap_or_else(|| vec!["rust"]);
        diagnostics.extend(salvo_core::platform_root_required(&program, project.as_ref(), &backends));
        (Some(checked), overloads, completions, type_completions)
    };
    let _finish = salvo_core::timing::phase("finish");
    // [doc-example] What the examples found is reported at their comments.
    salvo_core::doc_examples::report(&mut diagnostics, &program.files);
    ir_reliant.retain(|(f, _)| program.files.get(*f).is_none_or(|f| f.doc_example.is_none()));
    // [diag-std-hidden] What is located in the embedded std is not shown.
    let std_errors = salvo_core::diag::hide_std_diagnostics(&mut diagnostics, &program.files);

    Ok(Analysis {
        root,
        project,
        program,
        diagnostics,
        std_errors,
        checked,
        io_errors,
        overloads,
        comptime_hovers,
        comptime_when_arms,
        completions,
        type_completions,
        ir_reliant,
    })
}


/// [doc-example] [cli-docs] Whether `file` is one of the project's own: its
/// doc comments' examples are checked, and `salvo docs` documents it. For a
/// std project the std tree; otherwise every file that is neither std nor a
/// dependency's. Never a test annex, nor a file made from an example.
pub fn own_file(file: &salvo_core::SourceFile, std_project: bool) -> bool {
    let own = if std_project { file.is_std || file.is_shadow } else { !file.is_std && file.dependency.is_none() };
    own && !file.is_test && file.doc_example.is_none()
}

/// [lsp-completion] The per-file index of type-like names, generated ones
/// (`__…`) left out.
fn type_completion_index(resolution: &salvo_core::Resolution<'_>, viewable: &[bool]) -> Vec<Vec<(String, TypeKind)>> {
    resolution
        .scopes
        .iter()
        .enumerate()
        .map(|(file, scope)| {
            if !viewable[file] {
                return Vec::new();
            }
            let mut out: Vec<(String, TypeKind)> = Vec::new();
            let mut add = |name: &str, kind: TypeKind| {
                if !name.starts_with("__") {
                    out.push((name.split('§').next().unwrap_or(name).to_string(), kind));
                }
            };
            for n in scope.structs.keys() {
                add(n, TypeKind::Struct);
            }
            for n in scope.effects.keys() {
                add(n, TypeKind::Effect);
            }
            for n in scope.handlers.keys() {
                add(n, TypeKind::Handler);
            }
            for n in scope.qualifiers.keys() {
                add(n, TypeKind::Qualifier);
            }
            for n in scope.type_aliases.keys().chain(scope.opaque_types.keys()) {
                add(n, TypeKind::Type);
            }
            out.sort();
            out.dedup();
            out
        })
        .collect()
}

/// [lsp-completion] The per-file index of callable fns, read off the
/// resolver's scopes; generated names (`__…`) are left out. Free fns carry
/// their `DeclId`; effect members carry their owning effect's name (so
/// completion offers them only where that effect is available) and a
/// precomputed signature.
fn completion_index(resolution: &salvo_core::Resolution<'_>, viewable: &[bool]) -> Vec<Vec<CompletionFn>> {
    use salvo_syntax::ast::Type;
    // The receiver a first parameter implies: its base type name, or `None`
    // when it is one of the fn's own type parameters (any receiver fits).
    fn receiver_of(decl: &salvo_syntax::ast::FnDecl) -> (Option<String>, bool) {
        let generics: Vec<&str> = decl.generics.iter().map(|g| g.name.as_str()).collect();
        match decl.params.iter().find(|p| !p.implicit) {
            Some(p) => match &p.ty {
                Type::Named { base, .. } if generics.contains(&base.name.name.as_str()) => (None, true),
                Type::Named { base, .. } => (Some(base.name.name.clone()), false),
                _ => (None, false),
            },
            None => (None, false),
        }
    }
    // [effect-member-completion] [effect-alias] [effect-prereq] Every
    // effect member in the program, once: an effect reaches a body through
    // an alias or as a prerequisite without being imported, and its members
    // are callable there all the same. The handler's availability gate keeps
    // the ones whose effect is not available out of the list.
    let mut every_member: Vec<(&str, &salvo_syntax::ast::EffectDecl, &salvo_syntax::ast::FnDecl)> = Vec::new();
    for scope in &resolution.scopes {
        for (name, members) in &scope.effect_members {
            for (owner, decl) in members {
                if !every_member.iter().any(|(_, o, d)| std::ptr::eq(*o, *owner) && std::ptr::eq(*d, *decl)) {
                    every_member.push((name, owner, decl));
                }
            }
        }
    }
    resolution
        .scopes
        .iter()
        .enumerate()
        .map(|(file, scope)| {
            let mut out: Vec<CompletionFn> = Vec::new();
            if !viewable[file] {
                return out;
            }
            for (name, entries) in &scope.fns {
                if name.starts_with("__") {
                    continue;
                }
                for e in entries {
                    let (receiver, generic_receiver) = receiver_of(e.decl);
                    out.push(CompletionFn {
                        name: name.to_string(),
                        key: Some(e.key),
                        receiver,
                        generic_receiver,
                        effect: None,
                        detail: None,
                    });
                }
            }
            // [effect-member-completion] Effect members a call could resolve
            // to — offered only where the owning effect is available, which
            // the completion handler gates on. Their signature is rendered
            // here since a member is not keyed by its `DeclId`.
            let in_scope_set: std::collections::HashSet<(usize, usize)> = scope
                .effect_members
                .values()
                .flatten()
                .map(|(so, sd)| (*so as *const _ as usize, *sd as *const _ as usize))
                .collect();
            let in_scope = |o: &salvo_syntax::ast::EffectDecl, d: &salvo_syntax::ast::FnDecl| {
                in_scope_set.contains(&(o as *const _ as usize, d as *const _ as usize))
            };
            let reached = every_member
                .iter()
                .filter(|(_, o, d)| !in_scope(o, d))
                .map(|(n, o, d)| (*n, vec![(*o, *d)]))
                .collect::<Vec<_>>();
            let scoped = scope.effect_members.iter().map(|(n, m)| (*n, m.clone()));
            for (name, members) in scoped.chain(reached) {
                if name.starts_with("__") {
                    continue;
                }
                for (owner, decl) in &members {
                    let (receiver, generic_receiver) = receiver_of(decl);
                    out.push(CompletionFn {
                        name: name.to_string(),
                        key: None,
                        receiver,
                        generic_receiver,
                        effect: Some(owner.name.name.clone()),
                        detail: Some(crate::lsp::fn_decl_signature(decl, None)),
                    });
                }
            }
            out.sort_by(|a, b| a.name.cmp(&b.name));
            out
        })
        .collect()
}

/// [lsp-hover-overloads] The per-file "names with more than one declaration"
/// index, read off the resolver's scopes: free fns from `scope.fns` and effect
/// members from `scope.effect_members`, each as the span its name is written at
/// so a hover can render the declaration.
///
/// Only names with two or more entries are kept — the common case is one, and
/// storing it would double the index for nothing.
fn overload_index(resolution: &salvo_core::Resolution<'_>, viewable: &[bool]) -> Vec<HashMap<String, Vec<DefSite>>> {
    let mut out = Vec::with_capacity(resolution.scopes.len());
    for (file, scope) in resolution.scopes.iter().enumerate() {
        let mut per_file: HashMap<String, Vec<DefSite>> = HashMap::new();
        if !viewable[file] {
            out.push(per_file);
            continue;
        }
        for (name, entries) in &scope.fns {
            let sites: Vec<DefSite> = entries
                .iter()
                .map(|e| DefSite {
                    file: e.file,
                    span: e.decl.name.span,
                })
                .collect();
            per_file.entry((*name).to_string()).or_default().extend(sites);
        }
        for (name, members) in &scope.effect_members {
            for (owner, decl) in members {
                // The member's file is its effect's, which the scope records
                // by effect name.
                let Some(file) = scope.effect_files.get(owner.name.name.as_str()) else {
                    continue;
                };
                per_file
                    .entry((*name).to_string())
                    .or_default()
                    .push(DefSite {
                        file: *file,
                        span: decl.name.span,
                    });
            }
        }
        // One declaration is not an overload set, and duplicates can arrive
        // from a module visible at two levels.
        for sites in per_file.values_mut() {
            sites.sort_by_key(|s| (s.file, s.span.expn, s.span.start));
            sites.dedup_by_key(|s| (s.file, s.span.expn, s.span.start));
        }
        per_file.retain(|_, sites| sites.len() > 1);
        out.push(per_file);
    }
    out
}

/// [manifest-deps] Loads the project's dependencies into `sources`, after
/// the embedded std and **before** the project's own tree (so the project's
/// [std-shadow] sees a std dependency's modules as std). No project, or a
/// project without `[dependencies]`, loads nothing. Returns one rendered
/// message per problem: a dependency that cannot be found or is at the wrong
/// version, or a file that cannot be loaded.
pub fn load_dependencies(
    sources: &mut SourceSet,
    project: Option<&salvo_core::Project>,
    native_ext: &str,
) -> Vec<String> {
    let Some(project) = project else { return Vec::new() };
    let deps = match project.dependencies() {
        Ok(deps) => deps,
        Err(msg) => return vec![msg],
    };
    let mut errors = Vec::new();
    for dep in &deps {
        errors.extend(sources.add_dependency(dep, native_ext));
    }
    errors
}

/// Loads the embedded standard library: every `.sv` module, plus std's own
/// host companions for the backend in play — `lib/platform/std/**.<native_ext>`,
/// the implementations of std's `platform handler` declarations
/// [platform-handler] [platform-tree]. std ships both languages side by side,
/// exactly as a customer's source tree may, and only the extension of the
/// backend being compiled for is picked up.
pub fn load_embedded_std(sources: &mut SourceSet, native_ext: &str) {
    fn walk<'a>(dir: &Dir<'a>, out: &mut Vec<&'a include_dir::File<'a>>) {
        for file in dir.files() {
            out.push(file);
        }
        for sub in dir.dirs() {
            walk(sub, out);
        }
    }
    let mut files = Vec::new();
    walk(&STD_DIR, &mut files);
    files.sort_by_key(|f| f.path().to_path_buf());
    // [platform-root] std names its platform root in its own manifest, like
    // any project; the embedded paths are relative to the manifest's
    // directory, `lib/`.
    let roots = STD_DIR
        .get_file(salvo_core::MANIFEST_FILE)
        .and_then(|f| f.contents_utf8())
        .and_then(|text| salvo_core::Project::parse(Path::new(salvo_core::MANIFEST_FILE), text).ok())
        .map(|project| project.platform_roots())
        .unwrap_or_default();
    for file in files {
        let path = file.path();
        let Some(content) = file.contents_utf8() else {
            continue;
        };
        if path.extension().is_some_and(|e| e == native_ext) {
            match roots.classify(path) {
                Ok(Some(pf)) if !pf.generated => {
                    let out = SourceSet::platform_output_path(&pf.module, native_ext);
                    sources.add_companion(out, pf.module, content.to_string(), true);
                }
                Ok(Some(_)) | Err(_) => {}
                Ok(None) => {
                    if let Some(module) = SourceSet::classify_companion(path, native_ext) {
                        sources.add_companion(path.to_path_buf(), module, content.to_string(), false);
                    }
                }
            }
            continue;
        }
        if path.extension().is_none_or(|e| e != "sv") {
            continue;
        }
        // [test-file] std's own test annexes are *not* part of the embedded
        // library: they are compiler-repository files, run by
        // `salvo test --src lib` from a checkout (user decision 2026-09-23,
        // N1a), so a shipped binary never carries them — and a program that
        // merely uses std never parses them.
        if SourceSet::is_test_path(path) {
            continue;
        }
        let Ok(module) = SourceSet::classify(path) else {
            continue;
        };
        // Named by its path under `lib/` (`lib/std/core/list.sv`), which is also
        // where `embedded_std_copy` writes it [lsp-std-source].
        sources.add(path.display().to_string(), module, content.to_string(), true);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // [lsp-hover-overloads] The index holds every declaration a name has in a
    // file, which is what a hover lists.
    #[test]
    fn overload_index_records_same_named_declarations() {
        // Repo-local scratch, per AGENTS.md: `tmp/` is gitignored, and a test
        // that writes outside the repository is a test that leaks.
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tmp/overload_index_test");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(
            root.join("main.sv"),
            "fn describe(n: Int) [] -> Str {\n    return \"int\"\n}\n\n\
             fn describe(s: Str) [] -> Str {\n    return s\n}\n",
        )
        .unwrap();
        let analysis = analyze_sources(&root, None, "", &HashMap::new()).expect("analysis");
        let file_idx = analysis
            .program
            .files
            .iter()
            .position(|f| !f.is_std)
            .expect("the user file");
        let sites = analysis.overloads[file_idx]
            .get("describe")
            .unwrap_or_else(|| {
                panic!(
                    "no `describe` entry; keys: {:?}",
                    analysis.overloads[file_idx].keys().take(20).collect::<Vec<_>>()
                )
            });
        assert_eq!(sites.len(), 2, "expected both overloads: {sites:?}");
    }

    // [lsp-parse-cache] The cache is advisory: analysing with it must give
    // the same diagnostics as analysing without, including after an edit —
    // the edited file re-parses while the rest hit the cache, and a stale
    // entry for the pre-edit text is dropped rather than reused.
    #[test]
    fn parse_cache_matches_a_fresh_analysis_across_an_edit() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tmp/parse_cache_test");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        // Two files, so one is edited and one stays put (a cache hit).
        std::fs::write(
            root.join("main.sv"),
            "fn main() [] {\n    let x = helper()\n}\n",
        )
        .unwrap();
        std::fs::write(
            root.join("lib.sv"),
            "export fn helper() [] -> Int {\n    return 1\n}\n",
        )
        .unwrap();

        let messages = |a: &Analysis| -> Vec<(usize, u32, String)> {
            let mut v: Vec<_> = a
                .diagnostics
                .iter()
                .map(|d| (d.file, d.span.start, d.message.clone()))
                .collect();
            v.sort();
            v
        };

        let mut cache = salvo_core::CheckCache::new();
        // First analysis: everything a cache miss.
        let cached1 =
            analyze_project(&root, None, "", &HashMap::new(), Some(&mut cache)).expect("a1");
        let fresh1 = analyze_sources(&root, None, "", &HashMap::new()).expect("f1");
        assert_eq!(
            messages(&cached1),
            messages(&fresh1),
            "cold cache diverged from a fresh analysis"
        );

        // Edit main.sv to introduce an error (calling a name that is not in
        // scope), then re-analyse. lib.sv is unchanged, so it hits the cache;
        // main.sv's new text misses and re-parses.
        let edited = "fn main() [] {\n    let x = nonexistent_fn()\n}\n";
        let mut overlay = HashMap::new();
        overlay.insert(root.join("main.sv"), edited.to_string());
        let cached2 =
            analyze_project(&root, None, "", &overlay, Some(&mut cache)).expect("a2");
        let fresh2 = analyze_sources(&root, None, "", &overlay).expect("f2");
        assert_eq!(
            messages(&cached2),
            messages(&fresh2),
            "post-edit cache diverged from a fresh analysis"
        );
        assert!(
            cached2.diagnostics.iter().any(|d| d.is_error()),
            "the edit should have produced an error, so the test actually \
             exercises a changed file: {:?}",
            cached2.diagnostics.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }

    /// [lsp-check-cache] The embedded std, warmed once for the tests.
    fn warm() -> &'static salvo_core::CheckCache {
        static WARM: std::sync::OnceLock<salvo_core::CheckCache> = std::sync::OnceLock::new();
        WARM.get_or_init(|| warm_std_cache(""))
    }

    /// How many files the embedded std has: the first files of every
    /// program over it.
    fn std_count() -> usize {
        let mut sources = SourceSet::default();
        load_embedded_std(&mut sources, "");
        sources.files.len()
    }

    // [lsp-check-cache] A cache warmed with std alone stands in for std: the
    // project's files are the only ones walked, and they report what a full
    // check reports.
    #[test]
    fn a_warm_std_cache_matches_a_full_check() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tmp/std_cache_test");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(
            root.join("main.sv"),
            "fn main() [use] {\n    use StdConsole()\n    let xs = List(1, 2, 3)\n    \
             let n = size(xs)\n    println(\"${n}\")\n}\n",
        )
        .unwrap();

        let project_messages = |a: &Analysis| -> Vec<(String, u32, String)> {
            let mut v: Vec<_> = a
                .diagnostics
                .iter()
                .filter(|d| !a.program.files[d.file].is_std)
                .map(|d| (a.program.files[d.file].name.clone(), d.span.start, d.message.clone()))
                .collect();
            v.sort();
            v
        };

        let full = analyze_sources(&root, None, "", &HashMap::new()).expect("full");
        let mut cache = warm().clone();
        let cached = analyze_project(&root, None, "", &HashMap::new(), Some(&mut cache)).expect("cached");
        let walked: Vec<&str> = cache.walked().iter().map(|&f| cached.program.files[f].name.as_str()).collect();
        assert_eq!(walked, ["main.sv"], "std was walked again");
        // [lsp-expand-cache] and std's stamped files are not stamped again.
        assert!(cache.expansion.kept() > 0, "std was stamped again");
        assert_eq!(
            project_messages(&cached),
            project_messages(&full),
            "the warm cache diverged from a full check"
        );
        let errors: Vec<_> = cached.diagnostics.iter().filter(|d| d.is_error()).map(|d| &d.message).collect();
        assert!(errors.is_empty(), "the cached analysis reported errors: {errors:?}");
    }

    /// [lsp-warm-std] The programs the warm cache's soundness tests run over:
    /// every example, and a few written here for what the examples lack — a
    /// project of several modules with its own `by auto` and comptime fns and
    /// errors in both; a project module `auto` and a project `comptime fn
    /// auto`, which std's `by auto` must not see; and a project module under
    /// `std.core`, the one way a project could have been implicitly visible to
    /// std — refused as a module under the reserved root, and not visible
    /// [std-root]. The `bool` says whether the program is free of that error.
    /// `test` names the caller's own scratch directory: nextest runs the two
    /// callers in parallel processes, and a shared one raced (one deleting
    /// what the other was writing; EINVAL, 2026-10-14).
    fn front_programs(test: &str) -> Vec<(PathBuf, bool)> {
        let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut out: Vec<(PathBuf, bool)> = Vec::new();
        let mut examples: Vec<PathBuf> = std::fs::read_dir(repo.join("examples"))
            .expect("examples/")
            .filter_map(|e| e.ok())
            .map(|e| e.path().join("salvo"))
            .filter(|p| p.is_dir())
            .collect();
        examples.sort();
        assert!(examples.len() >= 10, "expected the examples: {examples:?}");
        out.extend(examples.into_iter().map(|p| (p, true)));
        let scratch = |name: &str, files: &[(&str, &str)]| -> PathBuf {
            let root = repo.join("tmp/std_front_test").join(test).join(name);
            let _ = std::fs::remove_dir_all(&root);
            std::fs::create_dir_all(&root).unwrap();
            for (file, text) in files {
                let path = root.join(file);
                std::fs::create_dir_all(path.parent().unwrap()).unwrap();
                std::fs::write(path, text).unwrap();
            }
            root
        };
        out.push((
            scratch(
                "modules",
                &[
                    (
                        "geo.sv",
                        "export struct Pt : Ordered<self> by auto, Hashed<self> by auto {\n    x: Int,\n    y: Int\n}\n\n\
                         export struct Bad : Hashed<self> by auto {\n    f: Float\n}\n\n\
                         export comptime fn total(p: Pt) [] -> Int => p {\n    let n = 0\n    [for field in Pt.fields] {\n        n = n + p.[field]\n    }\n    return n\n}\n",
                    ),
                    (
                        "main.sv",
                        "import geo\n\nfn main() [use] {\n    use StdConsole()\n    let p = Pt { x: 1, y: 2 }\n    \
                         let q = Pt { x: 1, y: 3 }\n    let n = total(p)\n    let same = p == q\n    \
                         println(\"${n} ${same}\")\n    let m = nonexistent(p)\n}\n",
                    ),
                ],
            ),
            true,
        ));
        // [obligation-by] std sees only std: a project module `auto`, and a
        // project `comptime fn auto`, are no second reading of std's `by
        // auto` (ROADMAP 0s), so std expands alike and the warm cache stands in.
        out.push((
            scratch(
                "auto_module",
                &[
                    ("auto.sv", "export fn helper() [] -> Int {\n    return 1\n}\n"),
                    ("main.sv", "import auto\n\nfn main() [] {\n    let x = helper()\n}\n"),
                ],
            ),
            true,
        ));
        out.push((
            scratch(
                "auto_fn",
                &[
                    (
                        "derive.sv",
                        "export comptime fn auto<T is Struct>(value: T) [] -> Int => value {\n    return 0\n}\n",
                    ),
                    ("main.sv", "import derive\n\nfn main() [] {\n    let x = 1\n}\n"),
                ],
            ),
            true,
        ));
        // [std-root] The one way a project could have reached std: a module
        // under `std.core`, which every file sees implicitly. Its path is
        // under the reserved root, so it is an error.
        out.push((
            scratch(
                "core_module",
                &[("std/core/extra/auto.sv", "export fn helper() [] -> Int {\n    return 1\n}\n"), ("main.sv", "fn main() [] {\n    let x = helper()\n}\n")],
            ),
            false,
        ));
        out
    }

    /// The analysis of a program directory, as the language server makes it
    /// (`analyze_for_document_cached`), through a clone of the warm std
    /// cache, or with no cache at all.
    fn front_analysis(dir: &Path, warm: bool) -> (Analysis, Option<salvo_core::CheckCache>) {
        let overlay = HashMap::new();
        let (src, project) = match salvo_core::Project::find(dir).expect("manifest") {
            Some(project) => (project.src(), Some(project)),
            None => (dir.to_path_buf(), None),
        };
        let mut cache = warm.then(|| self::warm().clone());
        let analysis = analyze_project_with(&src, project, "", &overlay, cache.as_mut(), Examples::All).expect("analysis");
        (analysis, cache)
    }

    // [lsp-check-cache] std's expansion is a function of std alone: expanding
    // std inside each program gives, module for module, what expanding it on
    // its own gave — which is what lets one cache warmed with std serve every
    // program.
    #[test]
    fn std_expansion_is_project_independent() {
        let mut sources = SourceSet::default();
        load_embedded_std(&mut sources, "");
        let std_files: Vec<String> = sources.files.iter().map(|f| f.name.clone()).collect();
        let n = std_files.len();
        let alone: Vec<String> = salvo_core::CheckCache::new().build(sources).program.modules.iter().map(|m| format!("{m:?}")).collect();
        for (dir, clean) in front_programs("project_independent") {
            let (full, _) = front_analysis(&dir, false);
            for i in 0..n {
                assert_eq!(full.program.files[i].name, std_files[i], "{}: std did not load first", dir.display());
                assert!(
                    format!("{:?}", full.program.modules[i]) == alone[i],
                    "{}: std's `{}` expanded differently inside the program",
                    dir.display(),
                    std_files[i]
                );
            }
            if !clean {
                // [std-root] A project module under `std` (here
                // `std.core.extra.auto`) is refused where it is written.
                assert!(
                    full.diagnostics.iter().any(|d| {
                        d.message.contains("module `std.core.extra.auto` is under `std`")
                            && d.id == Some("std-root:reserved-module")
                            && full.program.files[d.file].name.ends_with("auto.sv")
                    }),
                    "{}: a project module under `std` was not refused: {:#?}",
                    dir.display(),
                    full.diagnostics.iter().map(|d| &d.message).collect::<Vec<_>>()
                );
            }
        }
    }

    /// The checker rows of the project's files (from file `n` on).
    fn rows<K: Copy, V: std::fmt::Debug>(
        table: &salvo_core::check::KeyMap<V>,
        n: usize,
        _k: std::marker::PhantomData<K>,
    ) -> Vec<String> {
        let mut out: Vec<String> = table
            .iter()
            .filter(|(k, _)| k.file() >= n)
            .map(|(k, v)| format!("{} {}..{} {v:?}", k.file(), k.span.start, k.span.end))
            .collect();
        out.sort();
        out
    }
    /// What the language server reads of an analysis, for the project's
    /// files (from file `n` on): diagnostics, the rows its views read, the
    /// IR's diagnostics, hovers, overloads and completions.
    fn view(a: &Analysis, n: usize) -> Vec<(&'static str, Vec<String>)> {
        let c = a.checked.as_ref().expect("checked");
        let p = std::marker::PhantomData::<()>;
        let mut diags: Vec<String> = a
            .diagnostics
            .iter()
            .map(|d| format!("{} {}..{} {:?} {} {:?}", d.file, d.span.start, d.span.end, d.severity, d.message, d.suggested_imports))
            .collect();
        diags.sort();
        let mut locals: Vec<String> =
            c.local_decls.iter().filter(|(f, _)| **f >= n).map(|(f, v)| format!("{f} {v:?}")).collect();
        locals.sort();
        let mut ir: Vec<String> = a.ir_reliant.iter().map(|(f, s)| format!("{f} {}..{}", s.start, s.end)).collect();
        ir.sort();
        let mut errors: Vec<String> = c.errors.iter().map(|d| format!("{} {}..{} {}", d.file, d.span.start, d.span.end, d.message)).collect();
        errors.sort();
        let hovers = a.comptime_hovers.iter().map(|h| format!("{} {}..{} {} {:?}", h.file, h.span.start, h.span.end, h.text, h.module)).collect();
        let arms = a.comptime_when_arms.iter().map(|w| format!("{w:?}")).collect();
        let mut overloads: Vec<String> = a.overloads.iter().enumerate().skip(n).flat_map(|(f, m)| m.iter().map(move |(k, v)| format!("{f} {k} {v:?}"))).collect();
        overloads.sort();
        let completions = a.completions.iter().skip(n).map(|c| format!("{c:?}")).collect();
        vec![
            ("diagnostics", diags),
            ("expr_ty", rows(&c.expr_ty, n, p)),
            ("fn_refs", rows(&c.fn_refs, n, p)),
            ("refs", rows(&c.refs, n, p)),
            ("calls", rows(&c.calls, n, p)),
            // [lsp-ir-cache] What the IR decided shows in the fate links kept.
            ("fate_reads", rows(&c.fate_reads, n, p)),
            ("local_decls", locals),
            ("errors", errors),
            ("ir diagnostics", ir),
            ("comptime hovers", hovers),
            ("comptime when arms", arms),
            ("overloads", overloads),
            ("completions", completions),
        ]
    }
    // [lsp-check-cache] An analysis through the warm std cache reports what one
    // without any cache reports, and the checker rows the language server
    // reads for the project's files are the same, as are the IR's
    // diagnostics; std is neither stamped nor walked again.
    #[test]
    fn a_warm_std_cache_matches_an_analysis_without_it() {
        let n = std_count();
        for (dir, _) in front_programs("matches_without") {
            let (without, _) = front_analysis(&dir, false);
            let (with, cache) = front_analysis(&dir, true);
            let cache = cache.expect("a cache");
            assert!(cache.walked().iter().all(|&f| f >= n), "{}: std was walked: {:?}", dir.display(), cache.walked());
            for ((what, a), (_, b)) in view(&with, n).into_iter().zip(view(&without, n)) {
                assert!(a == b, "{}: {what} differ through the warm cache:\n{a:#?}\nwithout it:\n{b:#?}", dir.display());
            }
            // The written program's mistakes are there to be compared: a
            // checker error, and a comptime one in a project file.
            if dir.ends_with("modules") {
                let messages: Vec<&str> = with.diagnostics.iter().filter(|d| d.is_error()).map(|d| d.message.as_str()).collect();
                assert!(messages.iter().any(|m| m.contains("nonexistent")), "{messages:#?}");
                assert!(messages.iter().any(|m| m.contains("Float")), "{messages:#?}");
                assert!(!with.comptime_hovers.iter().all(|h| h.file < n), "no project hover");
            }
        }
    }

    /// [lsp-check-cache] Programs of more than one file for the check cache's
    /// test: two written ones whose files reach into each
    /// other's declarations — types, fields, a qualifier with a value slot, a
    /// module-level rename, a generic struct read through a substitution, an
    /// effect and its handler, a test annex, and an error in a body.
    fn check_cache_programs() -> Vec<PathBuf> {
        let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let scratch = |name: &str, files: &[(&str, &str)]| -> PathBuf {
            let root = repo.join("tmp/check_cache_test").join(name);
            let _ = std::fs::remove_dir_all(&root);
            std::fs::create_dir_all(&root).unwrap();
            for (file, text) in files {
                std::fs::write(root.join(file), text).unwrap();
            }
            root
        };
        vec![
            scratch(
                "shapes",
                &[
                    (
                        "geo.sv",
                        "export struct Pt {\n    x: Int,\n    y: Int\n}\n\n\
                         export fn origin() [] -> Pt {\n    let p = Pt { x: 0, y: 0 }\n    return p\n}\n\n\
                         export struct Box<T> {\n    item: T,\n    count: Int\n}\n\n\
                         export fn boxed<T>(item: T) [] -> Box<T> => !item {\n    return Box { item: item, count: 1 }\n}\n\n\
                         export qualifier Below(limit: Int) for Int {\n    fn qualifies(n: Int, limit: Int) -> Bool {\n        return n < limit\n    }\n}\n\n\
                         export fn norm(p: Pt) [] -> Int => p {\n    return p.x * p.x + p.y * p.y\n}\n\n\
                         rename fn sum2 = add2(a: Int, b: Int)\n\n\
                         export fn add2(a: Int, b: Int) [] -> Int {\n    return a + b\n}\n",
                    ),
                    (
                        "geo.test.sv",
                        "test \"the origin is at zero\" {\n    let p = origin()\n    let n = norm(p)\n    let s = sum2(n, 1)\n}\n",
                    ),
                    (
                        "count.sv",
                        "export effect Counter {\n    fn bump(by: Int) -> Int\n}\n\n\
                         export handler Tally(start: Int) for Counter {\n    state { total: Int = start }\n    fn bump(by: Int) -> Int {\n        total = total + by\n        return total\n    }\n}\n",
                    ),
                    (
                        "main.sv",
                        "import geo\nimport count\n\n\
                         fn run() [Counter] -> Int {\n    let p = origin()\n    let b = boxed(p)\n    let n = norm(b.item) + b.count\n    let lim = 100\n    \
                         if n is Below(lim) {\n        return bump(n)\n    }\n    return bump(1)\n}\n\n\
                         fn main() [use] {\n    use StdConsole()\n    use Tally(1)\n    let n = run()\n    let m = missing_fn(n)\n    println(\"${n}\")\n}\n",
                    ),
                ],
            ),
            scratch(
                "chain",
                &[
                    (
                        "a.sv",
                        "export fn first() [] -> Int {\n    return 1\n}\n\n\
                         export struct Wrap {\n    value: Int\n}\n\n\
                         export fn wrap(n: Int) [] -> Wrap {\n    return Wrap { value: n }\n}\n",
                    ),
                    (
                        "b.sv",
                        "import a\n\nexport fn second() [] -> Int {\n    return first() + 1\n}\n\n\
                         export fn unwrap(w: Wrap) [] -> Int => w {\n    return w.value\n}\n",
                    ),
                    (
                        "c.sv",
                        "import a\nimport b\n\nfn third() [] -> Int {\n    let w = wrap(second())\n    return unwrap(w) + first()\n}\n",
                    ),
                ],
            ),
        ]
    }

    /// [lsp-check-cache] `text` with a statement added at the start of its
    /// first fn body, so every later declaration in the file moves; `None`
    /// for a file without one.
    fn edit_first_body(text: &str) -> Option<String> {
        let mut at = 0;
        let mut after_fn = false;
        for line in text.split_inclusive('\n') {
            let t = line.trim();
            let is_fn = ["fn ", "export fn ", "send fn "].iter().any(|k| t.starts_with(k));
            // A body's brace ends the fn's line, or a deduction clause's on
            // the next one.
            if (is_fn || (after_fn && t.starts_with("=>"))) && t.ends_with('{') {
                let brace = at + line.trim_end().len();
                let mut out = text.to_string();
                out.insert_str(brace, "\n    let edited_here = 12345");
                return Some(out);
            }
            after_fn = is_fn;
            at += line.len();
        }
        None
    }

    // [doc-example] [lsp-check-cache] An edit inside a doc comment's example
    // walks the host and that example only, and the example's report moves
    // with the comment: through the cache the analysis says what one without
    // it says.
    #[test]
    fn an_edit_inside_a_doc_example_walks_its_host_and_the_example() {
        let warm = warm();
        let n = std_count();
        let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let src = repo.join("tmp/check_cache_test/doc_examples");
        let _ = std::fs::remove_dir_all(&src);
        std::fs::create_dir_all(&src).unwrap();
        let shapes = "// Squares.\n//\n// ```\n// let a = area(Square { side: 2 })\n// ```\n\n\
                      // A square.\n//\n// ```\n// struct Pair { a: Square, b: Square }\n// ```\n\
                      export struct Square {\n    side: Int\n}\n\n\
                      // Its area.\n//\n// ```\n// let a = area(Square { side: 3 })\n// ```\n\
                      export fn area(s: Square) -> Int {\n    return s.side * s.side\n}\n";
        std::fs::write(src.join("shapes.sv"), shapes).unwrap();
        std::fs::write(src.join("main.sv"), "import shapes\n\nfn main() [use] {\n    let _a = area(Square { side: 1 })\n}\n").unwrap();
        let root = src.canonicalize().unwrap();
        let mut checks = warm.clone();
        let analyze = |overlay: &HashMap<PathBuf, String>, checks: Option<&mut salvo_core::CheckCache>| {
            analyze_project_with(&src, None, "", overlay, checks, Examples::All).expect("analysis")
        };
        let mut overlay: HashMap<PathBuf, String> = HashMap::new();
        let first = analyze(&overlay, Some(&mut checks));
        let examples: Vec<usize> = (n..first.program.files.len()).filter(|&i| first.program.files[i].doc_example.is_some()).collect();
        assert_eq!(examples.len(), 3, "{:?}", first.program.files[n..].iter().map(|f| &f.name).collect::<Vec<_>>());
        let host = (n..first.program.files.len()).find(|&i| first.program.files[i].name == "shapes.sv").unwrap();
        let area = examples.iter().copied().find(|&i| first.program.files[i].module.to_string() == "shapes.#area.0").unwrap();
        analyze(&overlay, Some(&mut checks));
        // A mistake inside the last example: the host and it are walked.
        let edited = shapes.replace("area(Square { side: 3 })", "area(Square { side: \"three\" })");
        overlay.insert(root.join("shapes.sv"), edited);
        let cached = analyze(&overlay, Some(&mut checks));
        assert_eq!(checks.walked(), [host, area].as_slice());
        let uncached = analyze(&overlay, Some(&mut warm.clone()));
        for ((what, a), (_, b)) in view(&cached, n).into_iter().zip(view(&uncached, n)) {
            assert!(a == b, "{what} differ:\n{a:#?}\nwithout the cache:\n{b:#?}");
        }
        let warned: Vec<(usize, u32, bool)> = cached.diagnostics.iter().map(|d| (d.file, d.span.start, d.doc_example)).collect();
        let at = cached.program.files[host].content.find("\"three\"").unwrap() as u32;
        assert_eq!(warned, [(host, at, true)], "{:#?}", cached.diagnostics);
    }

    // [lsp-check-cache] In a `std = true` tree (std is what is edited, so no
    // std cache stands in) the check cache covers every file: a body edit, a
    // doc-comment edit, and a body edit in a file comptime stamps into (`net`,
    // whose templates are `auto`'s) each walk the edited file only; a
    // signature edit walks the file and what can see it (its annex and
    // examples), and a body edit in a file holding templates (`args`) the
    // files whose copies of one moved with it (its examples deriving
    // `FromArgs`); and each analysis reads what one without the cache does.
    #[test]
    fn the_check_cache_covers_a_std_tree() {
        let lib = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../lib").canonicalize().unwrap();
        let project = salvo_core::Project::find(&lib.join("salvo.toml")).expect("manifest").expect("lib/ has a manifest");
        let src = project.src();
        let root = src.canonicalize().unwrap();
        let mut checks = salvo_core::CheckCache::new();
        let analyze = |overlay: &HashMap<PathBuf, String>, checks: Option<&mut salvo_core::CheckCache>| {
            analyze_project_with(&src, Some(project.clone()), "", overlay, checks, Examples::Open).expect("analysis")
        };
        let read = |name: &str| std::fs::read_to_string(root.join(name)).unwrap();
        let heap = read("std/heap.sv");
        let args = read("std/args.sv");
        let net = read("std/net.sv");
        // The documents an editor has open, unedited: their examples are in
        // the program from the first analysis on.
        let open: HashMap<PathBuf, String> = [("std/heap.sv", &heap), ("std/args.sv", &args), ("std/net.sv", &net)]
            .into_iter()
            .map(|(n, t)| (root.join(n), t.clone()))
            .collect();
        let mut overlay = open.clone();
        let first = analyze(&overlay, Some(&mut checks));
        let all = first.program.files.len();
        assert_eq!(checks.walked().len(), all, "the first analysis walks every file");
        analyze(&overlay, Some(&mut checks));
        assert!(checks.walked().is_empty(), "nothing changed, and a stamped file's equal module is kept: {:?}", checks.walked());
        let file = |name: &str| (0..all).find(|&i| first.program.files[i].name == name).unwrap();
        let edits: [(&str, String, bool); 5] = [
            ("std/heap.sv", heap.replacen("    return Mut List()", "    return  Mut List()", 1), false),
            ("std/heap.sv", heap.replacen("// A binary heap, a priority queue", "// A binary heap,  a priority queue", 1), false),
            ("std/net.sv", edit_first_body(&net).expect("a body in net.sv"), false),
            ("std/args.sv", edit_first_body(&args).expect("a body in args.sv"), true),
            ("std/heap.sv", heap.replacen("export fn heapify<T>(list: Mut List<T>,", "export fn heapify<T>(list:  Mut List<T>,", 1), true),
        ];
        for (name, text, everything) in edits {
            assert_ne!(text, read(name), "the edit of {name} changed nothing");
            // Back to the files as written, then the one edit.
            analyze(&open, Some(&mut checks));
            overlay = open.clone();
            overlay.insert(root.join(name), text);
            let cached = analyze(&overlay, Some(&mut checks));
            if everything {
                // The file, and what can see it: its annex and its examples
                // (nothing else in std imports `std.heap`), or for `args` the
                // examples a template copy moved in.
                let stem = name.trim_end_matches(".sv");
                let walked: Vec<&str> = checks.walked().iter().map(|&f| cached.program.files[f].name.as_str()).collect();
                assert!(walked.contains(&name) || name == "std/args.sv", "editing {name}: {walked:?}");
                assert!(walked.len() > 1 && walked.iter().all(|w| w.starts_with(stem)), "editing {name}: {walked:?}");
            } else {
                let walked: Vec<&str> = checks.walked().iter().map(|&f| cached.program.files[f].name.as_str()).collect();
                assert_eq!(walked, [name], "editing {name}");
                assert_eq!(checks.walked(), [file(name)]);
                // [lsp-ir-cache] The IR is built for what was walked only.
                assert_eq!(checks.ir.built(), [file(name)], "editing {name}");
                // [lsp-scope-cache] and every scope is the last expansion's.
                assert_eq!(checks.scopes.given(), all, "editing {name}");
            }
            let uncached = analyze(&overlay, None);
            for ((what, a), (_, b)) in view(&cached, 0).into_iter().zip(view(&uncached, 0)) {
                assert!(a == b, "{what} differ after editing {name}:\n{a:#?}\nwithout the cache:\n{b:#?}");
            }
        }
    }

    // [lsp-check-cache] Rows are a file's, not a position's: opening a
    // document in `lib/` (which adds its examples to the program, moving the
    // files after its own) walks those examples only, and closing it walks
    // nothing; each analysis reads what one without the cache does.
    #[test]
    fn opening_and_closing_a_document_walks_its_examples_only() {
        let lib = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../lib").canonicalize().unwrap();
        let project = salvo_core::Project::find(&lib.join("salvo.toml")).expect("manifest").expect("lib/ has a manifest");
        let src = project.src();
        let root = src.canonicalize().unwrap();
        let mut checks = salvo_core::CheckCache::new();
        let analyze = |overlay: &HashMap<PathBuf, String>, checks: Option<&mut salvo_core::CheckCache>| {
            analyze_project_with(&src, Some(project.clone()), "", overlay, checks, Examples::Open).expect("analysis")
        };
        let read = |name: &str| (root.join(name), std::fs::read_to_string(root.join(name)).unwrap());
        // `heap` comes after `args`, so opening `args` moves `heap`'s examples.
        let heap: HashMap<PathBuf, String> = [read("std/heap.sv")].into_iter().collect();
        let both: HashMap<PathBuf, String> = [read("std/heap.sv"), read("std/args.sv")].into_iter().collect();
        analyze(&heap, Some(&mut checks));
        let opened = analyze(&both, Some(&mut checks));
        let walked: Vec<&str> = checks.walked().iter().map(|&f| opened.program.files[f].name.as_str()).collect();
        assert!(!walked.is_empty() && walked.iter().all(|w| w.starts_with("std/args.sv#")), "{walked:?}");
        let uncached = analyze(&both, None);
        for ((what, a), (_, b)) in view(&opened, 0).into_iter().zip(view(&uncached, 0)) {
            assert!(a == b, "{what} differ after opening:\n{a:#?}\nwithout the cache:\n{b:#?}");
        }
        let closed = analyze(&heap, Some(&mut checks));
        assert!(checks.walked().is_empty(), "{:?}", checks.walked());
        let uncached = analyze(&heap, None);
        for ((what, a), (_, b)) in view(&closed, 0).into_iter().zip(view(&uncached, 0)) {
            assert!(a == b, "{what} differ after closing:\n{a:#?}\nwithout the cache:\n{b:#?}");
        }
    }

    // [lsp-check-cache] An interface edit walks the edited file and what can
    // see it, through imports and on through what those see: a leaf's
    // signature its own file; a type `b` reads through `a`'s signature,
    // `a`'s readers and theirs, and not the file that sees neither.
    #[test]
    fn an_interface_edit_walks_what_can_see_it() {
        let warm = warm();
        let n = std_count();
        let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let src = repo.join("tmp/check_cache_test/dependents");
        let _ = std::fs::remove_dir_all(&src);
        std::fs::create_dir_all(&src).unwrap();
        let files = [
            ("t.sv", "export struct Token {\n    v: Int\n}\n"),
            ("a.sv", "import t\n\nexport fn make() [] -> Token {\n    return Token { v: 1 }\n}\n"),
            ("b.sv", "import a\n\nfn use_it() [] -> Int {\n    return make().v\n}\n"),
            ("c.sv", "fn alone() [] -> Int {\n    return 2\n}\n"),
        ];
        for (name, text) in files {
            std::fs::write(src.join(name), text).unwrap();
        }
        let root = src.canonicalize().unwrap();
        let mut checks = warm.clone();
        let analyze = |overlay: &HashMap<PathBuf, String>, checks: Option<&mut salvo_core::CheckCache>| {
            analyze_project_with(&src, None, "", overlay, checks, Examples::All).expect("analysis")
        };
        let first = analyze(&HashMap::new(), Some(&mut checks));
        let walked_names = |a: &Analysis, w: &[usize]| -> Vec<String> { w.iter().map(|&f| a.program.files[f].name.clone()).collect() };
        assert_eq!(checks.walked().len(), first.program.files.len() - n);
        let edits: [(&str, String, &[&str]); 2] = [
            ("c.sv", files[3].1.replace("fn alone() [] -> Int", "fn alone(x: Int) [] -> Int"), &["c.sv"]),
            ("t.sv", files[0].1.replace("    v: Int\n", "    v: Int,\n    w: Int = 0\n"), &["a.sv", "b.sv", "t.sv"]),
        ];
        for (name, text, want) in edits {
            analyze(&HashMap::new(), Some(&mut checks));
            let overlay: HashMap<PathBuf, String> = [(root.join(name), text)].into_iter().collect();
            let cached = analyze(&overlay, Some(&mut checks));
            let mut walked = walked_names(&cached, checks.walked());
            walked.sort();
            assert_eq!(walked, want, "editing {name}");
            let uncached = analyze(&overlay, Some(&mut warm.clone()));
            for ((what, a), (_, b)) in view(&cached, n).into_iter().zip(view(&uncached, n)) {
                assert!(a == b, "{what} differ after editing {name}:\n{a:#?}\nwithout the cache:\n{b:#?}");
            }
        }
    }

    // [lsp-ir-cache] A file the check did not walk keeps its IR findings (a
    // use after a move in `u`) while another file's body is edited; an edit
    // that changes the program's gate (an error in a body) builds everything
    // again; and each analysis reports what one without the caches does.
    #[test]
    fn a_file_not_walked_keeps_its_ir_findings() {
        let warm = warm();
        let n = std_count();
        let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let src = repo.join("tmp/check_cache_test/ir_cache");
        let _ = std::fs::remove_dir_all(&src);
        std::fs::create_dir_all(&src).unwrap();
        let t = "export struct Token {\n    v: Int\n}\n\n\
                 export fn spend(t: Token) [] -> Int => !t {\n    return t.v\n}\n\n\
                 export fn other() [] -> Int {\n    return 1\n}\n";
        let u = "import t\n\nfn twice() [] -> Int {\n    let k = Token { v: 1 }\n    let a = spend(k)\n    let b = spend(k)\n    return a + b + other()\n}\n";
        std::fs::write(src.join("t.sv"), t).unwrap();
        std::fs::write(src.join("u.sv"), u).unwrap();
        let root = src.canonicalize().unwrap();
        let mut checks = warm.clone();
        let analyze = |overlay: &HashMap<PathBuf, String>, checks: Option<&mut salvo_core::CheckCache>| {
            analyze_project_with(&src, None, "", overlay, checks, Examples::All).expect("analysis")
        };
        let first = analyze(&HashMap::new(), Some(&mut checks));
        let file = |name: &str| (n..first.program.files.len()).find(|&i| first.program.files[i].name == name).unwrap();
        let moved = |a: &Analysis| a.diagnostics.iter().any(|d| d.file == file("u.sv") && d.message.contains("consumed (moved)"));
        assert!(moved(&first), "{:#?}", first.diagnostics);
        let edits: [(String, Vec<usize>); 2] = [
            (t.replace("    return 1\n", "    return 2\n"), vec![file("t.sv")]),
            (t.replace("    return 1\n", "    return \"one\"\n"), vec![file("t.sv"), file("u.sv")]),
        ];
        for (text, built) in edits {
            analyze(&HashMap::new(), Some(&mut checks));
            let overlay: HashMap<PathBuf, String> = [(root.join("t.sv"), text)].into_iter().collect();
            let cached = analyze(&overlay, Some(&mut checks));
            assert_eq!(checks.walked(), [file("t.sv")]);
            assert_eq!(checks.ir.built(), built.as_slice());
            assert!(moved(&cached), "{:#?}", cached.diagnostics);
            let uncached = analyze(&overlay, Some(&mut warm.clone()));
            for ((what, a), (_, b)) in view(&cached, n).into_iter().zip(view(&uncached, n)) {
                assert!(a == b, "{what} differ:\n{a:#?}\nwithout the cache:\n{b:#?}");
            }
        }
    }

    // [lsp-check-cache] A body edit, file after file (edits accumulating),
    // then all reverted: through the check cache the analysis reports and
    // reads what an analysis without it does, and only the edited file is
    // walked (with any file comptime stamps, which every analysis makes new).
    #[test]
    fn check_cache_matches_an_uncached_analysis_across_edits() {
        let warm = warm();
        let n = std_count();
        for dir in check_cache_programs() {
            let (src, project) = match salvo_core::Project::find(&dir.join("x.sv")).expect("manifest") {
                Some(project) => (project.src(), Some(project)),
                None => (dir.clone(), None),
            };
            let root = src.canonicalize().unwrap();
            let mut checks = warm.clone();
            let analyze = |overlay: &HashMap<PathBuf, String>, checks: Option<&mut salvo_core::CheckCache>| {
                analyze_project_with(&src, project.clone(), "", overlay, checks, Examples::All).expect("analysis")
            };
            let mut overlay: HashMap<PathBuf, String> = HashMap::new();
            let first = analyze(&overlay, Some(&mut checks));
            let files: Vec<usize> = (n..first.program.files.len()).collect();
            assert!(files.len() > 1, "{}: one file", dir.display());
            assert_eq!(checks.walked(), files.as_slice(), "{}: the first analysis walks every file", dir.display());
            // What every analysis walks whatever changed: the files comptime
            // stamps (a new module each time).
            analyze(&overlay, Some(&mut checks));
            let always: Vec<usize> = checks.walked().to_vec();
            assert!(always.len() < files.len(), "{}: nothing was cached: {always:?}", dir.display());
            let mut edited: Vec<usize> = Vec::new();
            for &f in &files {
                let path = root.join(&first.program.files[f].name);
                let Some(text) = edit_first_body(&first.program.files[f].content) else { continue };
                overlay.insert(path, text);
                let cached = analyze(&overlay, Some(&mut checks));
                let mut want: Vec<usize> = always.iter().copied().chain([f]).collect();
                want.sort();
                want.dedup();
                assert_eq!(checks.walked(), want.as_slice(), "{}: editing {}", dir.display(), first.program.files[f].name);
                let uncached = analyze(&overlay, Some(&mut warm.clone()));
                for ((what, a), (_, b)) in view(&cached, n).into_iter().zip(view(&uncached, n)) {
                    assert!(a == b, "{}: {what} differ after editing {}:\n{a:#?}\nwithout the cache:\n{b:#?}", dir.display(), first.program.files[f].name);
                }
                edited.push(f);
            }
            assert!(edited.len() > 1, "{}: edited {edited:?}", dir.display());
            overlay.clear();
            let cached = analyze(&overlay, Some(&mut checks));
            let mut want: Vec<usize> = always.iter().copied().chain(edited.iter().copied()).collect();
            want.sort();
            want.dedup();
            assert_eq!(checks.walked(), want.as_slice(), "{}: reverting", dir.display());
            let uncached = analyze(&overlay, Some(&mut warm.clone()));
            for ((what, a), (_, b)) in view(&cached, n).into_iter().zip(view(&uncached, n)) {
                assert!(a == b, "{}: {what} differ after reverting:\n{a:#?}\nwithout the cache:\n{b:#?}", dir.display());
            }
            if dir.ends_with("shapes") {
                let messages: Vec<&str> = cached.diagnostics.iter().filter(|d| d.is_error()).map(|d| d.message.as_str()).collect();
                assert!(messages.iter().any(|m| m.contains("missing_fn")), "{messages:#?}");
            }
        }
    }
}
