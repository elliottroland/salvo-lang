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
}

/// Parses, resolves, and type-checks `src` (plus the embedded std).
///
/// `native_ext` is the active backend's companion extension
/// ([backend-companion]); an empty string loads `.sv` files only, which is
/// what backend-neutral analysis wants — companions are copied, never
/// checked. `overlay` maps absolute file paths to in-editor contents that
/// replace (or add to) what is on disk [cli-lsp].
///
/// Errors only when `src` is not a directory.
pub fn analyze_sources(
    src: &Path,
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
    let io_errors = sources.add_dir(&root, native_ext, false);

    // Overlay: open-editor contents win over the disk [cli-lsp]. Files
    // not on disk yet (new unsaved buffers under the root) are added.
    if !overlay.is_empty() {
        for file in sources.files.iter_mut().filter(|f| !f.is_std) {
            if let Some(content) = overlay.get(&root.join(&file.name)) {
                file.content = content.clone();
            }
        }
        for (path, content) in overlay {
            let Ok(rel) = path.strip_prefix(&root) else { continue };
            if sources
                .files
                .iter()
                .any(|f| !f.is_std && Path::new(&f.name) == rel)
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
    // [diag-structured]. The `iter fn` expansion is deferred to a
    // program-level pass, so a subject declared in another file still gets
    // the per-field snapshot [iter-fn].
    let mut diagnostics: Vec<FileDiagnostic> = Vec::new();
    let mut modules = Vec::with_capacity(sources.files.len());
    let mut parse_diags: Vec<Vec<salvo_syntax::Diagnostic>> =
        Vec::with_capacity(sources.files.len());
    for file in &sources.files {
        let (module, diags) = salvo_syntax::parse_module_deferred(&file.content);
        parse_diags.push(diags);
        modules.push(module);
    }
    let all_structs: Vec<salvo_syntax::ast::StructDecl> = modules
        .iter()
        .flat_map(salvo_syntax::desugar::struct_decls)
        .collect();
    for (file_idx, (module, mut diags)) in modules.iter_mut().zip(parse_diags).enumerate() {
        diags.extend(salvo_syntax::desugar::expand_iter_fns_with(
            module,
            &all_structs,
        ));
        diagnostics.extend(diags.into_iter().map(|d| FileDiagnostic {
            file: file_idx,
            severity: d.severity,
            message: d.message,
            span: d.span,
            suggested_imports: Vec::new(),
        }));
    }
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
    let (checked, overloads) = {
        let symbols = Symbols::collect(&program);
        let resolution = salvo_core::resolve(&program);
        let overloads = overload_index(&resolution);
        // `check_program` folds resolution errors into its own.
        let mut checked = salvo_core::check_program(&program, &resolution, &symbols);
        checked.errors.retain(|d| !parse_broken.contains(&d.file));
        diagnostics.append(&mut checked.errors);
        (Some(checked), overloads)
    };

    Ok(Analysis {
        program,
        diagnostics,
        checked,
        io_errors,
        overloads,
    })
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
    for file in files {
        let path = file.path();
        let Some(content) = file.contents_utf8() else {
            continue;
        };
        if path.extension().is_some_and(|e| e == native_ext) {
            if let Some((module, platform)) = SourceSet::classify_companion(path, native_ext)
            {
                sources.add_companion(
                    path.to_path_buf(),
                    module,
                    content.to_string(),
                    platform,
                );
            }
            continue;
        }
        if path.extension().is_none_or(|e| e != "sv") {
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
        let analysis = analyze_sources(&root, "", &HashMap::new()).expect("analysis");
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
