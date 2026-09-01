//! The shared front-half pipeline: load embedded std + a source
//! directory, parse, resolve, and type-check. Used by `salvo analyze`
//! [cli-analyze] and the language server [cli-lsp].

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use include_dir::{include_dir, Dir};

use salvo_core::{Checked, FileDiagnostic, Program, SourceSet, Symbols};

/// The standard library, embedded into the binary at build time. Files are
/// filtered per backend at load time, so a Kotlin compile never sees
/// `*.rust.sv` define files.
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
    /// Files that could not be read (does not abort the analysis).
    pub io_errors: Vec<(PathBuf, String)>,
}

/// Parses, resolves, and type-checks `src` (plus the embedded std).
///
/// `filter`/`native_ext` select a backend's define/companion files; empty
/// strings load language files only (backend-neutral analysis, see
/// [cli-analyze]). `overlay` maps absolute file paths to in-editor
/// contents that replace (or add to) what is on disk [cli-lsp].
///
/// Errors only when `src` is not a directory.
pub fn analyze_sources(
    src: &Path,
    filter: &str,
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
    load_embedded_std(&mut sources, filter);
    let io_errors = sources.add_dir(&root, filter, native_ext, false);

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
            let Some((module, kind)) = SourceSet::classify(rel, filter) else {
                continue;
            };
            sources.add(rel.display().to_string(), module, kind, content.clone(), false);
        }
    }

    // Parse every module, attributing diagnostics to files
    // [diag-structured].
    let mut diagnostics: Vec<FileDiagnostic> = Vec::new();
    let mut modules = Vec::with_capacity(sources.files.len());
    for (file_idx, file) in sources.files.iter().enumerate() {
        let (module, diags) = salvo_syntax::parse_module(&file.content);
        diagnostics.extend(diags.into_iter().map(|d| FileDiagnostic {
            file: file_idx,
            severity: d.severity,
            message: d.message,
            span: d.span,
            suggested_imports: Vec::new(),
        }));
        modules.push(module);
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
    let checked = {
        let symbols = Symbols::collect(&program);
        let resolution = salvo_core::resolve(&program);
        // `check_program` folds resolution errors into its own.
        let mut checked = salvo_core::check_program(&program, &resolution, &symbols);
        checked.errors.retain(|d| !parse_broken.contains(&d.file));
        diagnostics.append(&mut checked.errors);
        Some(checked)
    };

    Ok(Analysis {
        program,
        diagnostics,
        checked,
        io_errors,
    })
}

/// Loads the embedded standard library, keeping only language files and the
/// define files of the active backend.
pub fn load_embedded_std(sources: &mut SourceSet, backend: &str) {
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
        if path.extension().is_none_or(|e| e != "sv") {
            continue;
        }
        let Some((module, kind)) = SourceSet::classify(path, backend) else {
            continue;
        };
        let Some(content) = file.contents_utf8() else {
            continue;
        };
        sources.add(
            format!("std/{}", path.display()),
            module,
            kind,
            content.to_string(),
            true,
        );
    }
}
