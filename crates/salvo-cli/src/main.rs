//! The `salvo` CLI.
//!
//! ```text
//! salvo compile --backend kotlin --src ./some_dir --target ./some_dir_kotlin
//! ```

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use include_dir::{include_dir, Dir};

use salvo_backend::{BackendError, BackendRegistry};
use salvo_backend_kotlin::KotlinBackend;
use salvo_core::{Program, SourceKind, SourceSet};

/// The standard library, embedded into the binary at build time. Files are
/// filtered per backend at load time, so a Kotlin compile never sees
/// `*.rust.sv` define files.
static STD_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/../../std");

#[derive(Parser)]
#[command(name = "salvo", version, about = "The Salvo language compiler")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Compile Salvo sources to a target language.
    Compile {
        /// Target backend (defaults to `kotlin`).
        #[arg(long, default_value = "kotlin")]
        backend: String,
        /// Directory containing `.sv` source files.
        #[arg(long)]
        src: PathBuf,
        /// Output directory for generated sources.
        #[arg(long)]
        target: PathBuf,
        /// Debug: print the parsed AST and stop before code generation.
        /// Bare `--emit-ast` prints the user modules; `--emit-ast=MODULE`
        /// prints one module (std included), e.g. `--emit-ast=core.list`.
        #[arg(long, num_args = 0..=1, require_equals = true, default_missing_value = "")]
        emit_ast: Option<String>,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::Compile {
            backend,
            src,
            target,
            emit_ast,
        } => compile(&backend, &src, &target, emit_ast.as_deref()),
    }
}

fn compile(
    backend_name: &str,
    src: &PathBuf,
    target: &PathBuf,
    emit_ast: Option<&str>,
) -> ExitCode {
    let mut registry = BackendRegistry::new();
    registry.register(Box::new(KotlinBackend));

    let Some(backend) = registry.get(backend_name) else {
        let available: Vec<_> = registry.names().collect();
        eprintln!(
            "error: unknown backend `{backend_name}` (available: {})",
            available.join(", ")
        );
        return ExitCode::FAILURE;
    };

    // Assemble sources: embedded std first (implicitly imported), then the
    // user's source directory.
    let mut sources = SourceSet::default();
    load_embedded_std(&mut sources, backend.name());

    if !src.is_dir() {
        eprintln!("error: source directory `{}` does not exist", src.display());
        return ExitCode::FAILURE;
    }
    let io_errors = sources.add_dir(src, backend.name(), backend.file_extension(), false);
    for (path, err) in &io_errors {
        eprintln!("error: failed to read `{}`: {err}", path.display());
    }
    if !io_errors.is_empty() {
        return ExitCode::FAILURE;
    }

    // Parse every module and collect diagnostics.
    let mut modules = Vec::with_capacity(sources.files.len());
    let mut error_count = 0usize;
    for file in &sources.files {
        let (module, diagnostics) = salvo_syntax::parse_module(&file.content);
        for diag in &diagnostics {
            eprintln!("{}", diag.render(&file.name, &file.content));
            if diag.is_error() {
                error_count += 1;
            }
        }
        modules.push(module);
    }
    if error_count > 0 {
        eprintln!(
            "error: aborting due to {error_count} parse error{}",
            if error_count == 1 { "" } else { "s" }
        );
        return ExitCode::FAILURE;
    }

    if let Some(filter) = emit_ast {
        let mut printed = 0usize;
        for (file, module) in sources.files.iter().zip(&modules) {
            // Bare `--emit-ast` prints the user modules; a value selects
            // one module by path (std included).
            let selected = if filter.is_empty() {
                !file.is_std
            } else {
                file.module.to_string() == filter
            };
            if !selected {
                continue;
            }
            let kind = match file.kind {
                SourceKind::Language => "module",
                SourceKind::BackendDefine => "backend defines",
            };
            println!("// ===== {} ({kind} `{}`) =====", file.name, file.module);
            println!("{module:#?}");
            printed += 1;
        }
        if printed == 0 {
            eprintln!("error: no module matches `{filter}`");
            return ExitCode::FAILURE;
        }
        return ExitCode::SUCCESS;
    }

    let user_modules = sources.files.iter().filter(|f| !f.is_std).count();
    eprintln!(
        "parsed {} file(s) ({user_modules} user, {} std) with no errors",
        sources.files.len(),
        sources.files.len() - user_modules
    );

    // The JVM entry-point class, if the program has a `main`.
    let entry = sources
        .files
        .iter()
        .zip(&modules)
        .find(|(file, module)| {
            !file.is_std
                && module.items.iter().any(|item| {
                    matches!(item, salvo_syntax::ast::Item::Fn(f)
                        if f.name.name == "main" && f.body.is_some())
                })
        })
        .map(|(file, _)| format!("salvo.{}.MainKt", file.module));

    let program = Program {
        files: sources.files,
        modules,
        companions: sources.companions,
    };
    match backend.emit(&program, target) {
        Ok(written) => {
            for path in &written {
                eprintln!("wrote {}", target.join(path).display());
            }
            // Clean stale output: generated files from previous runs that
            // this compile no longer produces.
            let removed = clean_stale(target, backend.file_extension(), &written);
            for path in &removed {
                eprintln!("removed stale {}", path.display());
            }
            eprintln!(
                "compiled {} module(s) to `{}`",
                written.len(),
                target.display()
            );
            if let Some(entry) = entry {
                eprintln!("entry point: {entry}");
            }
            ExitCode::SUCCESS
        }
        Err(BackendError::Unsupported(msg)) => {
            eprintln!("error: {msg}");
            ExitCode::FAILURE
        }
        Err(err) => {
            eprintln!("error: {err}");
            ExitCode::FAILURE
        }
    }
}

/// Deletes files with the backend's extension under `target` that were not
/// written by this compile (stale output from previous runs). Only
/// backend-extension files are touched; other files are left alone.
fn clean_stale(target: &PathBuf, ext: &str, written: &[PathBuf]) -> Vec<PathBuf> {
    use std::collections::HashSet;
    let written: HashSet<PathBuf> = written.iter().map(|p| target.join(p)).collect();
    let mut removed = Vec::new();
    let mut stack = vec![target.clone()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else { continue };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == ext)
                && !written.contains(&path)
                && std::fs::remove_file(&path).is_ok()
            {
                removed.push(path);
            }
        }
    }
    removed
}

/// Loads the embedded standard library, keeping only language files and the
/// define files of the active backend.
fn load_embedded_std(sources: &mut SourceSet, backend: &str) {
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
