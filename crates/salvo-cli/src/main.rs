//! The `salvo` CLI.
//!
//! ```text
//! salvo compile --backend kotlin --src ./some_dir --target ./some_dir_kotlin
//! salvo analyze --src ./some_dir [--backend kotlin] [--format json]
//! salvo lsp [--backend kotlin]
//! salvo lang tm-grammar [--out vscode/syntaxes/salvo.tmLanguage.json]
//! ```

mod analysis;
mod lang;
mod lsp;

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand, ValueEnum};

use salvo_backend::{BackendError, BackendRegistry};
use salvo_backend_kotlin::KotlinBackend;
use salvo_backend_rust::RustBackend;
use salvo_core::{FileDiagnostic, Program, SourceKind, SourceSet};
use salvo_syntax::diag::Severity;

use analysis::load_embedded_std;

#[derive(Parser)]
#[command(name = "salvo", version, about = "The Salvo language compiler")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Clone, Copy, PartialEq, Eq, ValueEnum)]
enum Format {
    /// Human-readable diagnostics with caret underlines (stderr).
    Text,
    /// A JSON array of diagnostic objects (stdout).
    Json,
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
    /// Parse, resolve, and type-check sources without generating code
    /// [cli-analyze].
    Analyze {
        /// Directory containing `.sv` source files.
        #[arg(long)]
        src: PathBuf,
        /// Also parse this backend's define files (`*.<backend>.sv`).
        /// Without it, analysis is backend-neutral: only language files
        /// are loaded.
        #[arg(long)]
        backend: Option<String>,
        /// Output format for diagnostics.
        #[arg(long, value_enum, default_value_t = Format::Text)]
        format: Format,
    },
    /// Start a language server speaking LSP over stdio [cli-lsp]. The
    /// workspace root comes from the client's `initialize` request.
    Lsp {
        /// Also parse this backend's define files (`*.<backend>.sv`),
        /// like `analyze --backend`.
        #[arg(long)]
        backend: Option<String>,
    },
    /// Emit language metadata for editor tooling [cli-lang].
    Lang {
        #[command(subcommand)]
        command: LangCommand,
    },
}

#[derive(Subcommand)]
enum LangCommand {
    /// Print the TextMate grammar for Salvo (used by the VS Code
    /// extension). Keywords are derived from the lexer's keyword table.
    TmGrammar {
        /// Write the grammar to this file instead of stdout.
        #[arg(long)]
        out: Option<PathBuf>,
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
        Command::Analyze {
            src,
            backend,
            format,
        } => analyze(&src, backend.as_deref(), format),
        Command::Lsp { backend } => match backend_filter(backend.as_deref()) {
            Ok((filter, native_ext)) => lsp::run(filter, native_ext),
            Err(msg) => {
                eprintln!("{msg}");
                ExitCode::FAILURE
            }
        },
        Command::Lang { command } => match command {
            LangCommand::TmGrammar { out } => lang::run_tm_grammar(out.as_ref()),
        },
    }
}

fn registry() -> BackendRegistry {
    let mut registry = BackendRegistry::new();
    registry.register(Box::new(KotlinBackend));
    registry.register(Box::new(RustBackend));
    registry
}

/// Maps an optional `--backend` to the `(filter, native_ext)` pair used
/// when loading sources: a named backend selects its define files; `None`
/// means backend-neutral analysis (language files only) [cli-analyze].
fn backend_filter(backend_name: Option<&str>) -> Result<(String, String), String> {
    match backend_name {
        Some(name) => {
            let registry = registry();
            let Some(backend) = registry.get(name) else {
                let available: Vec<_> = registry.names().collect();
                return Err(format!(
                    "error: unknown backend `{name}` (available: {})",
                    available.join(", ")
                ));
            };
            Ok((
                backend.name().to_string(),
                backend.file_extension().to_string(),
            ))
        }
        None => Ok((String::new(), String::new())),
    }
}

/// `salvo analyze`: the front half of `compile` — parse, resolve, and
/// type-check — reporting every diagnostic instead of emitting code
/// [cli-analyze]. Exits nonzero when any diagnostic is an error.
fn analyze(src: &PathBuf, backend_name: Option<&str>, format: Format) -> ExitCode {
    // `--backend` only selects which define files participate; checking
    // itself is backend-neutral (define files are parsed, not checked).
    let (filter, native_ext) = match backend_filter(backend_name) {
        Ok(pair) => pair,
        Err(msg) => {
            eprintln!("{msg}");
            return ExitCode::FAILURE;
        }
    };

    let analysis =
        match analysis::analyze_sources(src, &filter, &native_ext, &Default::default()) {
            Ok(analysis) => analysis,
            Err(msg) => {
                eprintln!("error: {msg}");
                return ExitCode::FAILURE;
            }
        };
    for (path, err) in &analysis.io_errors {
        eprintln!("error: failed to read `{}`: {err}", path.display());
    }
    if !analysis.io_errors.is_empty() {
        return ExitCode::FAILURE;
    }
    let program = &analysis.program;
    let diagnostics = &analysis.diagnostics;

    let errors = diagnostics.iter().filter(|d| d.is_error()).count();
    let warnings = diagnostics.len() - errors;
    match format {
        Format::Text => {
            for diag in diagnostics {
                eprintln!("{}", diag.render(&program.files));
            }
            let user = program.files.iter().filter(|f| !f.is_std).count();
            let std_count = program.files.len() - user;
            let status = if diagnostics.is_empty() {
                "no errors".to_string()
            } else {
                format!(
                    "{errors} error{}, {warnings} warning{}",
                    if errors == 1 { "" } else { "s" },
                    if warnings == 1 { "" } else { "s" }
                )
            };
            eprintln!(
                "analyzed {} file(s) ({user} user, {std_count} std): {status}",
                program.files.len()
            );
        }
        Format::Json => println!("{}", diagnostics_json(diagnostics, program)),
    }
    if errors > 0 {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

/// Renders diagnostics as a JSON array (one object per diagnostic, with
/// file, 1-based line/col, byte span, severity, and message) [cli-analyze].
fn diagnostics_json(diagnostics: &[FileDiagnostic], program: &Program) -> String {
    let mut out = String::from("[");
    for (i, diag) in diagnostics.iter().enumerate() {
        let file = &program.files[diag.file];
        let (line, col) = salvo_syntax::span::line_col(&file.content, diag.span.start);
        let severity = match diag.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        };
        out.push_str(if i == 0 { "\n" } else { ",\n" });
        out.push_str(&format!(
            "  {{\"file\": {}, \"line\": {line}, \"col\": {col}, \
             \"start\": {}, \"end\": {}, \"severity\": \"{severity}\", \
             \"message\": {}",
            json_str(&file.name),
            diag.span.start,
            diag.span.end,
            json_str(&diag.message)
        ));
        // Import suggestions [diag-import-suggest], present only when
        // there are any.
        if !diag.suggested_imports.is_empty() {
            let imports: Vec<String> =
                diag.suggested_imports.iter().map(|s| json_str(s)).collect();
            out.push_str(&format!(", \"imports\": [{}]", imports.join(", ")));
        }
        out.push('}');
    }
    if !diagnostics.is_empty() {
        out.push('\n');
    }
    out.push(']');
    out
}

/// Minimal JSON string escaping (quotes, backslashes, control chars).
fn json_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn compile(
    backend_name: &str,
    src: &PathBuf,
    target: &PathBuf,
    emit_ast: Option<&str>,
) -> ExitCode {
    let registry = registry();

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

    // The entry point, if the program has a `main` (rendering depends on
    // the backend: JVM class name vs crate-root file).
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
        .map(|(file, _)| match backend.name() {
            "rust" => {
                // The main-declaring module is the crate root (see
                // BACKEND_SPEC.rust.md).
                let mut path = std::path::PathBuf::new();
                for part in &file.module.0 {
                    path.push(part);
                }
                path.set_extension("rs");
                format!(
                    "{} (build with: rustc --edition 2021 {})",
                    target.join(&path).display(),
                    target.join(&path).display()
                )
            }
            _ => format!("salvo.{}.MainKt", file.module),
        });

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
