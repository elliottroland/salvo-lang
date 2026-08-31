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
        /// Target backend (e.g. `kotlin`).
        #[arg(long)]
        backend: String,
        /// Directory containing `.sv` source files.
        #[arg(long)]
        src: PathBuf,
        /// Output directory for generated sources.
        #[arg(long)]
        target: PathBuf,
        /// Debug: print the parsed AST of every module and stop before
        /// code generation.
        #[arg(long)]
        emit_ast: bool,
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
        } => compile(&backend, &src, &target, emit_ast),
    }
}

fn compile(backend_name: &str, src: &PathBuf, target: &PathBuf, emit_ast: bool) -> ExitCode {
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
    let io_errors = sources.add_dir(src, backend.name(), false);
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

    if emit_ast {
        for (file, module) in sources.files.iter().zip(&modules) {
            let kind = match file.kind {
                SourceKind::Language => "module",
                SourceKind::BackendDefine => "backend defines",
            };
            println!("// ===== {} ({kind} `{}`) =====", file.name, file.module);
            println!("{module:#?}");
        }
        return ExitCode::SUCCESS;
    }

    let user_modules = sources.files.iter().filter(|f| !f.is_std).count();
    eprintln!(
        "parsed {} file(s) ({user_modules} user, {} std) with no errors",
        sources.files.len(),
        sources.files.len() - user_modules
    );

    let program = Program {
        files: sources.files,
        modules,
    };
    match backend.emit(&program, target) {
        Ok(written) => {
            for path in &written {
                eprintln!("wrote {}", target.join(path).display());
            }
            eprintln!(
                "compiled {} module(s) to `{}`",
                written.len(),
                target.display()
            );
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
