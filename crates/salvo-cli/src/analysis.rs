//! The shared front-half pipeline: load embedded std + a source
//! directory, parse, resolve, and type-check. Used by `salvo analyze`
//! [cli-analyze] and the language server [cli-lsp].

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use include_dir::{include_dir, Dir};

use salvo_core::{Checked, DefSite, FileDiagnostic, Program, SourceSet, Symbols};

/// The standard library, embedded into the binary at build time. Every file
/// in it is a language file: std reaches its target languages through the
/// backends' `intrinsic` lowerings [backend-intrinsic], not through
/// per-backend source files.
static STD_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/../../std");

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
    /// order (checker diagnostics after parse diagnostics).
    pub diagnostics: Vec<FileDiagnostic>,
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
    /// [lsp-completion] Per file (aligned with `program.files`): every free
    /// fn visible there, for completion.
    pub completions: Vec<Vec<CompletionFn>>,
    /// [lsp-completion] Per file: every type-like name visible there —
    /// structs, effects, handlers, qualifiers, aliases and opaque types —
    /// with what kind of declaration it is.
    pub type_completions: Vec<Vec<(String, TypeKind)>>,
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
    pub key: salvo_core::FnKey,
    /// The base type name of the first parameter, which dot-notation's
    /// receiver fills [fn-dot]; `None` when it has none or it is one of the
    /// fn's own type parameters.
    pub receiver: Option<String>,
    /// The first parameter is a type parameter: any receiver fits.
    pub generic_receiver: bool,
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
    analyze_project(src, project, native_ext, overlay)
}

/// [manifest-discovery] `analyze_sources` for a document: the nearest
/// manifest above `path` decides the source root and whether the tree is
/// std; with none, `fallback` (the workspace root) is analysed as before and
/// the analysis says so (`manifest_missing`).
pub fn analyze_for_document(
    path: &Path,
    fallback: &Path,
    overlay: &HashMap<PathBuf, String>,
) -> Result<Analysis, String> {
    match salvo_core::Project::find(path)? {
        Some(project) => {
            let src = project.src();
            analyze_project(&src, Some(project), "", overlay)
        }
        None => analyze_project(fallback, None, "", overlay),
    }
}

fn analyze_project(
    src: &Path,
    project: Option<salvo_core::Project>,
    native_ext: &str,
    overlay: &HashMap<PathBuf, String>,
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
    // [diag-structured]. The `iter fn` and `test` expansions are deferred to
    // a program-level pass: the first needs the rest of the program's structs
    // [iter-fn], the second needs to know which files are annexes
    // [test-file].
    let mut diagnostics: Vec<FileDiagnostic> = Vec::new();
    let mut modules = Vec::with_capacity(sources.files.len());
    for file in &sources.files {
        let (module, diags) = salvo_syntax::parse_module_deferred(&file.content);
        diagnostics.extend(diags.into_iter().map(|d| FileDiagnostic {
            file: modules.len(),
            severity: d.severity,
            message: d.message,
            span: d.span,
            suggested_imports: Vec::new(),
        }));
        modules.push(module);
    }
    let expansion = salvo_core::expand(&sources.files, &mut modules);
    diagnostics.extend(expansion.diagnostics);
    let comptime_hovers = expansion.comptime_hovers;
    diagnostics.sort_by_key(|d| (d.file, d.span.start));
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
        let symbols = Symbols::collect(&program);
        let resolution = salvo_core::resolve(&program);
        let overloads = overload_index(&resolution);
        let completions = completion_index(&resolution);
        let type_completions = type_completion_index(&resolution);
        // `check_program` folds resolution errors into its own.
        let mut checked = salvo_core::check_program(&program, &resolution, &symbols);
        checked.errors.retain(|d| !parse_broken.contains(&d.file));
        diagnostics.append(&mut checked.errors);
        // [platform-root] Platform declarations need a platform root.
        let backends = project.as_ref().map(salvo_core::required_backends).unwrap_or_else(|| vec!["rust"]);
        diagnostics.extend(salvo_core::platform_root_required(&program, project.as_ref(), &backends));
        (Some(checked), overloads, completions, type_completions)
    };

    Ok(Analysis {
        root,
        project,
        program,
        diagnostics,
        checked,
        io_errors,
        overloads,
        comptime_hovers,
        completions,
        type_completions,
    })
}

/// [lsp-completion] The per-file index of type-like names, generated ones
/// (`__…`) left out.
fn type_completion_index(resolution: &salvo_core::Resolution<'_>) -> Vec<Vec<(String, TypeKind)>> {
    resolution
        .scopes
        .iter()
        .map(|scope| {
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
/// resolver's scopes; generated names (`__…`) are left out.
fn completion_index(resolution: &salvo_core::Resolution<'_>) -> Vec<Vec<CompletionFn>> {
    use salvo_syntax::ast::Type;
    resolution
        .scopes
        .iter()
        .map(|scope| {
            let mut out: Vec<CompletionFn> = Vec::new();
            for (name, entries) in &scope.fns {
                if name.starts_with("__") {
                    continue;
                }
                for e in entries {
                    let generics: Vec<&str> = e.decl.generics.iter().map(|g| g.name.as_str()).collect();
                    let (receiver, generic_receiver) = match e.decl.params.iter().find(|p| !p.implicit) {
                        Some(p) => match &p.ty {
                            Type::Named { base, .. } if generics.contains(&base.name.name.as_str()) => (None, true),
                            Type::Named { base, .. } => (Some(base.name.name.clone()), false),
                            _ => (None, false),
                        },
                        None => (None, false),
                    };
                    out.push(CompletionFn { name: name.to_string(), key: e.key, receiver, generic_receiver });
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
fn overload_index(resolution: &salvo_core::Resolution<'_>) -> Vec<HashMap<String, Vec<DefSite>>> {
    let mut out = Vec::with_capacity(resolution.scopes.len());
    for scope in &resolution.scopes {
        let mut per_file: HashMap<String, Vec<DefSite>> = HashMap::new();
        for (name, entries) in &scope.fns {
            let sites: Vec<DefSite> = entries
                .iter()
                .map(|e| DefSite {
                    file: e.key.file,
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
            sites.sort_by_key(|s| (s.file, s.span.start));
            sites.dedup_by_key(|s| (s.file, s.span.start));
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
/// host companions for the backend in play — `std/platform/**.<native_ext>`,
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
    // any project; the embedded paths are relative to std's directory.
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
        // `salvo test --src std` from a checkout (user decision 2026-09-23,
        // N1a), so a shipped binary never carries them — and a program that
        // merely uses std never parses them.
        if SourceSet::is_test_path(path) {
            continue;
        }
        let Ok(module) = SourceSet::classify(path) else {
            continue;
        };
        sources.add(
            format!("std/{}", path.display()),
            module,
            content.to_string(),
            true,
        );
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
}
