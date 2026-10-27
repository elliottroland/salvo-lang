//! The shared front-half pipeline: load embedded std + a source
//! directory, parse, resolve, and type-check. Used by `salvo analyze`
//! [cli-analyze] and the language server [cli-lsp].

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use include_dir::{include_dir, Dir};

use salvo_core::{Checked, DefSite, FileDiagnostic, Program, SourceSet, Symbols};
use salvo_syntax::ast::Module;
use salvo_syntax::diag::Diagnostic;

/// [lsp-parse-cache] A per-server cache of parsed modules, keyed by the
/// **content** of the file that produced them: parsing is per-file and
/// local, so a file whose text has not changed since it was last parsed
/// reuses its module (and its parse diagnostics) rather than being lexed and
/// parsed again. Only the edited file misses on a keystroke; std and every
/// untouched project file hit.
///
/// The cached module is the parse **with its per-file expansions**
/// (`salvo_core::expand::expand_file`: the struct-body hoist, `iter fn`s,
/// `test` blocks), which depend on the file's text, its module path and
/// whether it is an annex — the cache key. The program-wide expansion
/// (comptime) runs fresh each analysis; a hit hands out the shared module,
/// and the expansion copies it only when it has something to stamp in it
/// [lsp-std-front]. Diagnostics here carry no file index — that is
/// assigned at use, so a cached entry is valid wherever the file lands.
///
/// Keyed by content hash. A collision would reuse the wrong module; with a
/// 64-bit hash over editor-sized files that is not a practical risk, and the
/// cache is advisory (a wrong hit degrades to stale diagnostics for one
/// keystroke, never miscompilation — the backends never see this).
#[derive(Default)]
pub struct ParseCache {
    entries: HashMap<u64, Parsed>,
}

/// One file parsed and expanded on its own.
type Parsed = (Arc<Module>, Vec<Diagnostic>, Vec<salvo_core::expand::FileTest>);

/// A file parsed and through its per-file expansions.
fn parse_file(file: &salvo_core::source::SourceFile) -> Parsed {
    let (mut module, mut diags) = salvo_syntax::parse_module_deferred(&file.content);
    let (tests, more) = salvo_core::expand::expand_file(file, &mut module);
    diags.extend(more);
    (Arc::new(module), diags, tests)
}

/// What a cached parse is keyed by: the text, and the two facts about the
/// file its per-file expansions read.
fn parse_key(file: &salvo_core::source::SourceFile) -> u64 {
    content_hash(&format!("{}\u{0}{}\u{0}{}", file.is_test, file.module, file.content))
}

impl ParseCache {
    pub fn new() -> Self {
        Self::default()
    }

    /// The parsed module for `file`: the cached one when its key is known,
    /// otherwise freshly parsed and remembered.
    fn parse(&mut self, file: &salvo_core::source::SourceFile) -> Parsed {
        let key = parse_key(file);
        if let Some(hit) = self.entries.get(&key) {
            return hit.clone();
        }
        let parsed = parse_file(file);
        self.entries.insert(key, parsed.clone());
        parsed
    }

    /// Drops every entry no file in `live` still produces, so the cache does
    /// not grow without bound as a file is edited (each distinct text leaves
    /// an entry otherwise). Called once per analysis with that analysis's
    /// file contents.
    fn retain_live(&mut self, live: &std::collections::HashSet<u64>) {
        self.entries.retain(|k, _| live.contains(k));
    }
}

/// FNV-1a over the file text — the same stable 64-bit hash the Kotlin std
/// cache uses for its keys ([kt-std-library]); it need only be stable within
/// one server run.
fn content_hash(content: &str) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in content.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

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
    /// `salvo ir` dumps the IR anyway and inlines these as `!!! …`, so they
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
    /// receiver fills [fn-dot]; `None` when it has none or it is one of the
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
    analyze_project(src, project, native_ext, overlay, None, None, None)
}

/// [lsp-std-front] [std-cache] The embedded std's front half, built once by
/// the language server and shared by every analysis it then makes: std parsed
/// and expanded as its own program, its modules kept (every analysis's
/// `Program` holds these very `Arc`s, so std is neither parsed, cloned nor
/// expanded again), what the comptime expansion needs to expand a project
/// over them, std's own expansion output, and the `StdCache` — checked over a
/// program sharing the same modules, so its address-keyed rows
/// (`Checked::written_types`, `type_ref_keys`) are keyed by the addresses an
/// analysis sees, and stay valid while the front lives.
pub struct StdFront {
    /// std's files, as `load_embedded_std` loads them: what an analysis's
    /// leading files must be for the front to stand in for them.
    files: Vec<salvo_core::SourceFile>,
    /// std's modules after every expansion.
    modules: Vec<Arc<Module>>,
    /// std's parse and per-file expansion diagnostics (file indices `0..N`).
    parse_diagnostics: Vec<FileDiagnostic>,
    /// std's comptime expansion output (diagnostics, hovers, `[when]` arms).
    expansion: salvo_core::Expansion,
    comptime: salvo_core::comptime::StdComptime,
    cache: salvo_core::StdCache,
}

/// [lsp-std-front] Builds the [`StdFront`] for `native_ext`'s std.
pub fn build_std_front(native_ext: &str) -> StdFront {
    let mut sources = SourceSet::default();
    load_embedded_std(&mut sources, native_ext);
    let std_files = sources.files.len();
    let mut modules = Vec::with_capacity(std_files);
    let mut parse_diagnostics: Vec<FileDiagnostic> = Vec::new();
    for file in &sources.files {
        let (module, diags, _tests) = parse_file(file);
        parse_diagnostics.extend(diags.into_iter().map(|d| FileDiagnostic {
            file: modules.len(),
            severity: d.severity,
            message: d.message,
            span: d.span,
            suggested_imports: Vec::new(),
            decl: None,
        }));
        modules.push(module);
    }
    let (comptime_out, comptime) = salvo_core::comptime::expand_comptime_prefix(&sources.files, &mut modules);
    let expansion = salvo_core::expand::program_expansion(Vec::new(), comptime_out);
    let program = Program {
        files: sources.files.clone(),
        modules: modules.clone(),
        companions: sources.companions,
    };
    let symbols = Symbols::collect(&program);
    let resolution = salvo_core::resolve(&program);
    let checked = salvo_core::check_program(&program, &resolution, &symbols);
    let cache = salvo_core::StdCache::new(&program, &resolution, &symbols, checked);
    StdFront {
        files: sources.files,
        modules,
        parse_diagnostics,
        expansion,
        comptime,
        cache,
    }
}

/// [lsp-std-front] Whether `front` may stand in for the leading files of
/// `files`: they are exactly the embedded std it was built from (names,
/// modules and contents), unshadowed, and nothing after them is std except
/// a shadowing file — so not a `std = true` project, whose files are all
/// std. Nothing a project declares changes std's expansion: std sees only
/// std [obligation-by], and no project module is under `std` [std-root].
fn std_front_applies(files: &[salvo_core::SourceFile], front: &StdFront) -> bool {
    let n = front.files.len();
    n <= files.len()
        && files[..n].iter().zip(&front.files).all(|(f, g)| {
            f.is_std && !f.is_shadow && f.name == g.name && f.module == g.module && f.is_test == g.is_test && f.content == g.content
        })
        && files[n..].iter().all(|f| !f.is_std || f.is_shadow)
}

/// [lsp-std-source] A full `Analysis` of the embedded standard library, as its
/// own program, so the language server can answer hover and go-to-definition
/// *inside* a std file a user navigated into (the read-only copies
/// `embedded_std_copy` writes under the temp dir). Checking std standalone
/// (`check_program`, no cache) records the references within it
/// (`fn_refs`/`def_refs`/`field_refs`), which the `StdCache` keeps hidden.
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
        diagnostics.extend(diags.into_iter().map(|d| FileDiagnostic {
            file: modules.len(),
            severity: d.severity,
            message: d.message,
            span: d.span,
            suggested_imports: Vec::new(),
            decl: None,
        }));
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
    cache: &mut ParseCache,
    std_front: &StdFront,
    check_cache: &mut salvo_core::CheckCache,
) -> Result<Analysis, String> {
    match salvo_core::Project::find(path)? {
        Some(project) => {
            let src = project.src();
            analyze_project(&src, Some(project), "", overlay, Some(cache), Some(std_front), Some(check_cache))
        }
        None => analyze_project(fallback, None, "", overlay, Some(cache), Some(std_front), Some(check_cache)),
    }
}

fn analyze_project(
    src: &Path,
    project: Option<salvo_core::Project>,
    native_ext: &str,
    overlay: &HashMap<PathBuf, String>,
    cache: Option<&mut ParseCache>,
    std_front: Option<&StdFront>,
    check_cache: Option<&mut salvo_core::CheckCache>,
) -> Result<Analysis, String> {
    analyze_project_with(src, project, native_ext, overlay, cache, std_front, true, check_cache)
}

/// [`analyze_project`], with `front_half` saying whether the std front may
/// stand in for std's parse and expansion (its `StdCache` is used either
/// way): off, the analysis is what it was before [lsp-std-front], which is
/// what its soundness tests compare against. [lsp-check-cache] With a
/// `check_cache`, a check over the std cache reuses its rows
/// (`check_project_cached`).
#[allow(clippy::too_many_arguments)]
fn analyze_project_with(
    src: &Path,
    project: Option<salvo_core::Project>,
    native_ext: &str,
    overlay: &HashMap<PathBuf, String>,
    mut cache: Option<&mut ParseCache>,
    std_front: Option<&StdFront>,
    front_half: bool,
    check_cache: Option<&mut salvo_core::CheckCache>,
) -> Result<Analysis, String> {
    if !src.is_dir() {
        return Err(format!(
            "source directory `{}` does not exist",
            src.display()
        ));
    }
    let root = src.canonicalize().unwrap_or_else(|_| src.to_path_buf());

    let mut sources = SourceSet::default();
    load_embedded_std(&mut sources, native_ext);
    // [manifest-deps] Dependencies come between std and the project's tree.
    let mut io_errors = load_dependencies(&mut sources, project.as_ref(), native_ext);
    // [test-file] `analyze` checks test annexes: a broken test is a broken
    // program, and the language server wants diagnostics in the file being
    // edited (user decision 2026-09-23).
    // [platform-root] The project's platform roots, when it names any.
    if let Some(project) = &project {
        sources.platform = project.platform_roots();
    }
    io_errors.extend(sources.add_dir(&root, native_ext, false, true));
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

    // Parse every module, attributing diagnostics to files
    // [diag-structured]. The per-file expansions run with the parse
    // (`parse_file`); the program-level one (comptime) after.
    //
    // [lsp-parse-cache] With a cache, an unchanged file reuses its parsed
    // module; `live` collects this analysis's file hashes so the cache can
    // drop everything else and not grow with each edit.
    //
    // [lsp-std-front] With a std front that stands in for the leading files,
    // std is not parsed at all: its expanded modules are the front's, and
    // only the files after it are parsed and stamped.
    let mut diagnostics: Vec<FileDiagnostic> = Vec::new();
    let mut ir_reliant: std::collections::HashSet<(usize, salvo_syntax::Span)> = std::collections::HashSet::new();
    let mut live: std::collections::HashSet<u64> = std::collections::HashSet::new();
    let front = std_front.filter(|front| front_half && std_front_applies(&sources.files, front));
    let t = salvo_core::timing::phase("parse");
    let mut parse = |idx: usize, cache: &mut Option<&mut ParseCache>, diagnostics: &mut Vec<FileDiagnostic>| {
        let file = &sources.files[idx];
        let (module, diags, tests) = match cache.as_deref_mut() {
            Some(cache) => {
                live.insert(parse_key(file));
                cache.parse(file)
            }
            None => parse_file(file),
        };
        diagnostics.extend(diags.into_iter().map(|d| FileDiagnostic {
            file: idx,
            severity: d.severity,
            message: d.message,
            span: d.span,
            suggested_imports: Vec::new(),
            decl: None,
        }));
        (module, tests)
    };
    let first = front.map_or(0, |f| f.files.len());
    let mut project_diagnostics: Vec<FileDiagnostic> = Vec::new();
    let mut modules: Vec<Arc<Module>> = Vec::with_capacity(sources.files.len());
    let mut per_file_tests = Vec::with_capacity(sources.files.len());
    if let Some(front) = front {
        modules.extend(front.modules.iter().cloned());
        per_file_tests.extend((0..first).map(|_| Vec::new()));
    }
    for idx in first..sources.files.len() {
        let (module, tests) = parse(idx, &mut cache, &mut project_diagnostics);
        modules.push(module);
        per_file_tests.push(tests);
    }
    diagnostics.append(&mut project_diagnostics);
    if let Some(cache) = cache {
        cache.retain_live(&live);
    }
    drop(t);
    let t = salvo_core::timing::phase("expand");
    let expansion = match front {
        Some(front) => {
            let mut std_diagnostics = front.parse_diagnostics.clone();
            std_diagnostics.append(&mut diagnostics);
            diagnostics = std_diagnostics;
            let rest = salvo_core::expand::expand_program_after(&sources.files, &mut modules, per_file_tests, &front.comptime);
            let mut expansion = salvo_core::Expansion {
                tests: rest.tests,
                diagnostics: front.expansion.diagnostics.clone(),
                comptime_hovers: front.expansion.comptime_hovers.clone(),
                comptime_when_arms: front.expansion.comptime_when_arms.clone(),
            };
            expansion.diagnostics.extend(rest.diagnostics);
            expansion.comptime_hovers.extend(rest.comptime_hovers);
            expansion.comptime_when_arms.extend(rest.comptime_when_arms);
            expansion
        }
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
        let resolution = salvo_core::resolve(&program);
        drop(t2);
        let t2 = salvo_core::timing::phase("  resolve: lsp indexes");
        // [lsp-index-viewable] Only for the files a view of this analysis can
        // show: an embedded std file is opened through `analyze_std`'s own
        // analysis, never this one (`doc_view`), so its entries stay empty.
        let viewable: Vec<bool> = program.files.iter().map(|f| !f.is_std || f.is_shadow).collect();
        let overloads = overload_index(&resolution, &viewable);
        let completions = completion_index(&resolution, &viewable);
        let type_completions = type_completion_index(&resolution, &viewable);
        drop(t2);
        drop(t);
        // `check_program` folds resolution errors into its own.
        let t = salvo_core::timing::phase("check");
        let mut checked = match (std_front.map(|f| &f.cache), check_cache) {
            (Some(cache), Some(check_cache)) if std_cache_applies(&program, cache) => {
                salvo_core::check_project_cached(&program, &resolution, &symbols, cache, check_cache)
            }
            (Some(cache), _) if std_cache_applies(&program, cache) => {
                salvo_core::check_project(&program, &resolution, &symbols, cache)
            }
            _ => salvo_core::check_program(&program, &resolution, &symbols),
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
            let (ir_diags, decision) = salvo_core::ir::analyze::analyze_full(&program, &symbols, &resolution, &checked, &diagnostics);
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

/// [std-cache] Whether `cache` may stand in for checking `program`'s std: its
/// leading `cache.std_files` files are std and unshadowed, and nothing after
/// them is std except a shadowing file. A std shadow or a `std = true` project
/// (whose files are all `is_std`) fails this and gets a full check (A4).
fn std_cache_applies(program: &Program, cache: &salvo_core::StdCache) -> bool {
    cache.std_files <= program.files.len()
        && program.files[..cache.std_files].iter().all(|f| f.is_std && !f.is_shadow)
        && program.files[cache.std_files..].iter().all(|f| !f.is_std || f.is_shadow)
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

        let mut cache = ParseCache::new();
        // First analysis: everything a cache miss.
        let cached1 =
            analyze_project(&root, None, "", &HashMap::new(), Some(&mut cache), None, None).expect("a1");
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
            analyze_project(&root, None, "", &overlay, Some(&mut cache), None, None).expect("a2");
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

    // [std-cache] Reusing a std-only check must give the project files the
    // same diagnostics a full check gives: the seeded std rows stand in for
    // the ones a full check would compute.
    #[test]
    fn std_cache_matches_a_full_check() {
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
        let std_front = build_std_front("");
        let cached = analyze_project(&root, None, "", &HashMap::new(), None, Some(&std_front), None)
            .expect("cached");
        assert!(
            std_cache_applies(&cached.program, &std_front.cache),
            "the gate fell back to a full check, so this test compared a full \
             check with itself"
        );
        assert_eq!(
            project_messages(&cached),
            project_messages(&full),
            "the std cache diverged from a full check"
        );
        let errors: Vec<_> = cached.diagnostics.iter().filter(|d| d.is_error()).map(|d| &d.message).collect();
        assert!(errors.is_empty(), "the cached analysis reported errors: {errors:?}");
    }

    /// [lsp-std-front] The programs the front's soundness tests run over:
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
                         export struct Bad : Ordered<self> by auto {\n    f: Float\n}\n\n\
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
        // auto` (ROADMAP 0s), so std expands alike and the front stands in.
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
    /// (`analyze_for_document_cached`, without the check cache), with or
    /// without the front half.
    fn front_analysis(dir: &Path, front: Option<&StdFront>, front_half: bool) -> Analysis {
        let overlay = HashMap::new();
        let mut cache = ParseCache::new();
        let (src, project) = match salvo_core::Project::find(dir).expect("manifest") {
            Some(project) => (project.src(), Some(project)),
            None => (dir.to_path_buf(), None),
        };
        analyze_project_with(&src, project, "", &overlay, Some(&mut cache), front, front_half, None).expect("analysis")
    }

    // [lsp-std-front] std's expansion is a function of std alone: expanding
    // std inside each program gives, module for module, what expanding it on
    // its own gave — which is what lets one expansion serve every analysis.
    #[test]
    fn std_front_expansion_is_project_independent() {
        let front = build_std_front("");
        let n = front.files.len();
        let alone: Vec<String> = front.modules.iter().map(|m| format!("{m:?}")).collect();
        for (dir, clean) in front_programs("project_independent") {
            let full = front_analysis(&dir, None, false);
            assert!(std_front_applies(&full.program.files, &front), "{}: std did not load as the front's", dir.display());
            let same: Vec<bool> = (0..n).map(|i| format!("{:?}", full.program.modules[i]) == alone[i]).collect();
            for (i, same) in same.iter().enumerate() {
                assert!(
                    same,
                    "{}: std's `{}` expanded differently inside the program",
                    dir.display(),
                    front.files[i].name
                );
            }
            if !clean {
                // [std-root] A project module under `std` (here
                // `std.core.extra.auto`) is refused where it is written.
                assert!(
                    full.diagnostics.iter().any(|d| {
                        d.message.contains("module `std.core.extra.auto` is under `std`")
                            && d.message.contains("[std-root]")
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
        table: &HashMap<(usize, salvo_syntax::Span), V>,
        n: usize,
        _k: std::marker::PhantomData<K>,
    ) -> Vec<String> {
        let mut out: Vec<String> = table
            .iter()
            .filter(|((file, _), _)| *file >= n)
            .map(|((file, span), v)| format!("{file} {}..{} {v:?}", span.start, span.end))
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
            ("def_refs", rows(&c.def_refs, n, p)),
            ("field_refs", rows(&c.field_refs, n, p)),
            ("call_fn", rows(&c.call_fn, n, p)),
            ("local_decls", locals),
            ("errors", errors),
            ("ir diagnostics", ir),
            ("comptime hovers", hovers),
            ("comptime when arms", arms),
            ("overloads", overloads),
            ("completions", completions),
        ]
    }
    // [lsp-std-front] An analysis through the front reports what one without
    // it reports, and the checker rows the language server reads for the
    // project's files are the same, as are the IR's diagnostics.
    #[test]
    fn std_front_matches_an_analysis_without_it() {
        let front = build_std_front("");
        let n = front.files.len();
        for (dir, _) in front_programs("matches_without") {
            let without = front_analysis(&dir, Some(&front), false);
            let with = front_analysis(&dir, Some(&front), true);
            assert!(std_cache_applies(&with.program, &front.cache), "{}: the std cache did not apply", dir.display());
            // The front stands in for every program, a refused one included
            // (there is no gate since [std-root]): std's modules are the
            // front's own `Arc`s.
            let shared = Arc::ptr_eq(&with.program.modules[0], &front.modules[0]);
            assert!(shared, "{}: the front did not stand in", dir.display());
            assert!(!Arc::ptr_eq(&without.program.modules[0], &front.modules[0]));
            for ((what, a), (_, b)) in view(&with, n).into_iter().zip(view(&without, n)) {
                assert!(a == b, "{}: {what} differ through the front:\n{a:#?}\nwithout it:\n{b:#?}", dir.display());
            }
            // [lsp-std-front] std's written keys came from the cache, the
            // project's were computed: together, what computing them all over
            // this program gives.
            if shared {
                let c = with.checked.as_ref().expect("checked");
                let symbols = Symbols::collect(&with.program);
                let resolution = salvo_core::resolve(&with.program);
                let keys = salvo_core::resolve::written_keys(&with.program, &resolution, &symbols);
                let by_span = salvo_core::resolve::written_keys_by_span(&with.program, &resolution, &symbols);
                assert!(c.type_ref_keys == keys, "{}: written keys differ", dir.display());
                assert!(c.type_ref_span_keys == by_span, "{}: written keys by span differ", dir.display());
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
    /// test: `modules/aws`, and two written ones whose files reach into each
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
            repo.join("modules/aws"),
            scratch(
                "shapes",
                &[
                    (
                        "geo.sv",
                        "export struct Pt {\n    x: Int,\n    y: Int\n}\n\n\
                         export fn origin() [] -> Pt {\n    let p = Pt { x: 0, y: 0 }\n    return p\n}\n\n\
                         export struct Box<T> {\n    item: T,\n    count: Int\n}\n\n\
                         export fn boxed<T>(item: T) [] -> Box<T> => !item {\n    return Box { item: item, count: 1 }\n}\n\n\
                         export qualifier Below(limit: Int) of Int {\n    fn qualifies(n: Int, limit: Int) -> Bool {\n        return n < limit\n    }\n}\n\n\
                         export fn norm(p: Pt) [] -> Int => p {\n    return p.x * p.x + p.y * p.y\n}\n\n\
                         rename fn plus = add2(a: Int, b: Int)\n\n\
                         export fn add2(a: Int, b: Int) [] -> Int {\n    return a + b\n}\n",
                    ),
                    (
                        "geo.test.sv",
                        "test \"the origin is at zero\" {\n    let p = origin()\n    let n = norm(p)\n    let s = plus(n, 1)\n}\n",
                    ),
                    (
                        "count.sv",
                        "export effect Counter {\n    fn bump(by: Int) -> Int\n}\n\n\
                         export handler Tally(start: Int) of Counter {\n    state { total: Int = start }\n    fn bump(by: Int) -> Int {\n        total = total + by\n        return total\n    }\n}\n",
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

    // [lsp-check-cache] A body edit, file after file (edits accumulating),
    // then all reverted: through the check cache the analysis reports and
    // reads what an analysis without it does, and only the edited file is
    // walked (with any file comptime stamps, which every analysis makes new).
    #[test]
    fn check_cache_matches_an_uncached_analysis_across_edits() {
        let front = build_std_front("");
        let n = front.files.len();
        for dir in check_cache_programs() {
            let (src, project) = match salvo_core::Project::find(&dir.join("x.sv")).expect("manifest") {
                Some(project) => (project.src(), Some(project)),
                None => (dir.clone(), None),
            };
            let root = src.canonicalize().unwrap();
            let mut parse = ParseCache::new();
            let mut checks = salvo_core::CheckCache::new();
            let mut analyze = |overlay: &HashMap<PathBuf, String>, checks: Option<&mut salvo_core::CheckCache>| {
                analyze_project_with(&src, project.clone(), "", overlay, Some(&mut parse), Some(&front), true, checks).expect("analysis")
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
                let uncached = analyze(&overlay, None);
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
            let uncached = analyze(&overlay, None);
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
