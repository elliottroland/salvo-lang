//! The `salvo` CLI.
//!
//! ```text
//! salvo compile --backend kotlin --src ./some_dir --target ./some_dir_kotlin
//! salvo run --backend kotlin --main ./some_dir/main.sv
//! salvo run --backend rust --src ./some_dir --main ./some_dir/bin/tool.sv
//! salvo run --backend rust --src ./some_dir --target ./out --clean-target before
//! salvo analyze --src ./some_dir [--backend kotlin] [--format json]
//! salvo lsp [--backend kotlin]
//! salvo lang tm-grammar [--out vscode/syntaxes/salvo.tmLanguage.json]
//! ```

mod analysis;
mod docs;
mod lang;
mod lsp;

use std::path::{Path, PathBuf};
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

/// Whether and when `salvo run` may delete its `--target` [cli-run]. Both
/// modes clear it *before* the build — a run must never pick up a previous
/// run's output — and differ only in what is left behind.
#[derive(Clone, Copy, PartialEq, Eq, ValueEnum)]
enum CleanTarget {
    /// Clear the target before compiling; leave the output in place after
    /// the run (so it can be inspected).
    Before,
    /// Clear it before compiling and delete it again after the run.
    Both,
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
    /// Compile Salvo sources and run the resulting program with the
    /// backend's toolchain [cli-run]. The command's exit code is the
    /// program's.
    Run {
        /// Target backend.
        #[arg(long)]
        backend: String,
        /// Directory containing `.sv` source files. Without `--main`, the
        /// unique `main` it declares is the entry point.
        #[arg(long, required_unless_present = "main_file")]
        src: Option<PathBuf>,
        /// The `.sv` file declaring `main` — how you choose between several
        /// entry points. Without `--src` it also implies
        /// `--src $(dirname <file>)`; with it, the file must be somewhere
        /// inside that directory.
        #[arg(long = "main", required_unless_present = "src")]
        main_file: Option<PathBuf>,
        /// Output directory for generated sources (default:
        /// `.salvo_tmp_run` in the working directory). It may not overlap
        /// the sources.
        #[arg(long)]
        target: Option<PathBuf>,
        /// Whether and when the target may be deleted.
        #[arg(long, value_enum, default_value_t = CleanTarget::Both)]
        clean_target: CleanTarget,
    },
    /// Parse, resolve, and type-check sources without generating code
    /// [cli-analyze].
    Analyze {        /// Directory containing `.sv` source files.
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
        Command::Run {
            backend,
            src,
            main_file,
            target,
            clean_target,
        } => run(&backend, src, main_file, target, clean_target),
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

/// What a build produced: the files written (relative to the target) and
/// the module declaring `main`, when the program has one.
struct Built {
    written: Vec<PathBuf>,
    main_module: Option<salvo_core::ModulePath>,
}

/// The source layout of a build: the directory to scan, plus (with
/// `--main`) the file inside it that must declare the entry point
/// [cli-run].
struct Layout {
    src: PathBuf,
    /// The entry file's path relative to `src`. `None` means "whatever
    /// unique `main` the directory declares".
    main_file: Option<String>,
}

/// Parses, checks and emits `layout`'s sources with `backend`.
///
/// The shared half of `compile` and `run` [cli-run]: `Err` carries the exit
/// code to return (diagnostics already reported), `Ok(None)` means the build
/// stopped early on purpose (`--emit-ast`).
fn build(
    backend: &dyn salvo_backend::Backend,
    layout: &Layout,
    target: &PathBuf,
    emit_ast: Option<&str>,
    verbose: bool,
) -> Result<Option<Built>, ExitCode> {
    // Assemble sources: embedded std first (implicitly imported), then the
    // user's source directory.
    let mut sources = SourceSet::default();
    load_embedded_std(&mut sources, backend.name());

    if !layout.src.is_dir() {
        eprintln!(
            "error: source directory `{}` does not exist",
            layout.src.display()
        );
        return Err(ExitCode::FAILURE);
    }
    let io_errors =
        sources.add_dir(&layout.src, backend.name(), backend.file_extension(), false);
    for (path, err) in &io_errors {
        eprintln!("error: failed to read `{}`: {err}", path.display());
    }
    if !io_errors.is_empty() {
        return Err(ExitCode::FAILURE);
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
        return Err(ExitCode::FAILURE);
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
            return Err(ExitCode::FAILURE);
        }
        return Ok(None);
    }

    if verbose {
        let user_modules = sources.files.iter().filter(|f| !f.is_std).count();
        eprintln!(
            "parsed {} file(s) ({user_modules} user, {} std) with no errors",
            sources.files.len(),
            sources.files.len() - user_modules
        );
    }

    // The modules declaring an entry point, in source order.
    let entries: Vec<(&salvo_core::SourceFile, &salvo_syntax::ast::Module)> = sources
        .files
        .iter()
        .zip(&modules)
        .filter(|(file, module)| {
            !file.is_std
                && module.items.iter().any(|item| {
                    matches!(item, salvo_syntax::ast::Item::Fn(f)
                        if f.name.name == "main" && f.body.is_some())
                })
        })
        .collect();
    let main_module = match &layout.main_file {
        // `--main` names the entry file: the `main` must be *there*, so a
        // directory with several is unambiguous [cli-run].
        Some(name) => {
            let found = entries.iter().find(|(file, _)| file.name == *name);
            match found {
                Some((file, _)) => Some(file.module.clone()),
                None => {
                    eprintln!(
                        "error: `{}` declares no `main` function with a body",
                        layout.src.join(name).display()
                    );
                    return Err(ExitCode::FAILURE);
                }
            }
        }
        None => entries.first().map(|(file, _)| file.module.clone()),
    };
    let ambiguous: Vec<String> = if layout.main_file.is_none() && entries.len() > 1 {
        entries.iter().map(|(f, _)| f.name.clone()).collect()
    } else {
        Vec::new()
    };

    let program = Program {
        files: sources.files,
        modules,
        companions: sources.companions,
    };
    let written = match backend.emit(&program, target, main_module.as_ref()) {
        Ok(written) => written,
        // Codegen messages are rendered diagnostics: they carry their own
        // `error:` prefix and one line each.
        Err(BackendError::Codegen(msgs)) => {
            for msg in &msgs {
                eprintln!("{msg}");
            }
            return Err(ExitCode::FAILURE);
        }
        Err(err) => {
            eprintln!("error: {err}");
            return Err(ExitCode::FAILURE);
        }
    };
    if verbose {
        for path in &written {
            eprintln!("wrote {}", target.join(path).display());
        }
        // Clean stale output: generated files from previous runs that this
        // compile no longer produces.
        for path in clean_stale(target, backend.file_extension(), &written) {
            eprintln!("removed stale {}", path.display());
        }
        eprintln!(
            "compiled {} module(s) to `{}`",
            written.len(),
            target.display()
        );
        if let Some(module) = &main_module {
            eprintln!("entry point: {}", backend.entry_hint(target, module));
        }
    } else {
        clean_stale(target, backend.file_extension(), &written);
    }
    if !ambiguous.is_empty() {
        eprintln!(
            "warning: {} files declare `main` ({}); `{}` was used — pass \
             `--main` to choose",
            ambiguous.len(),
            ambiguous.join(", "),
            ambiguous[0]
        );
    }
    Ok(Some(Built {
        written,
        main_module,
    }))
}

fn compile(
    backend_name: &str,
    src: &PathBuf,
    target: &PathBuf,
    emit_ast: Option<&str>,
) -> ExitCode {
    let registry = registry();
    let Some(backend) = registry.get(backend_name) else {
        eprintln!("{}", unknown_backend(&registry, backend_name));
        return ExitCode::FAILURE;
    };
    let layout = Layout {
        src: src.clone(),
        main_file: None,
    };
    match build(backend, &layout, target, emit_ast, true) {
        Ok(_) => ExitCode::SUCCESS,
        Err(code) => code,
    }
}

/// The default `--target` for `salvo run` [cli-run]: dot-prefixed, so it is
/// skipped by source discovery [mod-ignore] even when it lands inside the
/// source tree — which is the normal case, since it is created in the
/// working directory.
const DEFAULT_RUN_TARGET: &str = ".salvo_tmp_run";

/// `salvo run`: compile with a backend and run the result through that
/// backend's toolchain [cli-run]. The command's exit code is the
/// *program's*, so `salvo run` is a drop-in for running the binary.
fn run(
    backend_name: &str,
    src: Option<PathBuf>,
    main_file: Option<PathBuf>,
    target: Option<PathBuf>,
    clean: CleanTarget,
) -> ExitCode {
    let registry = registry();
    let Some(backend) = registry.get(backend_name) else {
        eprintln!("{}", unknown_backend(&registry, backend_name));
        return ExitCode::FAILURE;
    };

    // `--src` and `--main` each supply a default for the other (user
    // decision 2026-09-05).
    let layout = match layout_of(src, main_file) {
        Ok(layout) => layout,
        Err(msg) => {
            eprintln!("error: {msg}");
            return ExitCode::FAILURE;
        }
    };

    let target = target.unwrap_or_else(|| PathBuf::from(DEFAULT_RUN_TARGET));
    if let Err(msg) = check_target_overlap(&layout.src, &target) {
        eprintln!("error: {msg}");
        return ExitCode::FAILURE;
    }
    // Both `--clean-target` modes clear the target first: a run must not
    // pick up a previous run's output.
    if let Err(msg) = clear_target(&target) {
        eprintln!("error: {msg}");
        return ExitCode::FAILURE;
    }

    let built = match build(backend, &layout, &target, None, false) {
        Ok(Some(built)) => built,
        Ok(None) => return ExitCode::SUCCESS,
        Err(code) => return code,
    };
    let Some(main_module) = built.main_module else {
        eprintln!(
            "error: no `main` function found in `{}`: there is nothing to run",
            layout.src.display()
        );
        return ExitCode::FAILURE;
    };

    let outcome = backend.run(&target, &main_module, &built.written);

    // The target is deleted after the run only with `--clean-target both`,
    // and never before the program's output has been produced.
    if clean == CleanTarget::Both {
        if let Err(msg) = clear_target(&target) {
            eprintln!("warning: {msg}");
        }
    }

    match outcome {
        Ok(0) => ExitCode::SUCCESS,
        // The program's own exit code, so `salvo run` can stand in for it.
        Ok(code) => ExitCode::from(code.clamp(1, 255) as u8),
        Err(err) => {
            eprintln!("error: {err}");
            ExitCode::FAILURE
        }
    }
}

fn unknown_backend(registry: &BackendRegistry, name: &str) -> String {
    let available: Vec<_> = registry.names().collect();
    format!(
        "error: unknown backend `{name}` (available: {})",
        available.join(", ")
    )
}

/// Resolves `--src` / `--main` into a build layout [cli-run].
///
/// Neither option is subordinate: each supplies a reasonable default for the
/// other (user decision 2026-09-05). `--main` alone takes the entry file's
/// own directory as the source directory; `--src` alone looks for the unique
/// `main` the directory declares. Given both, the entry file must live inside
/// the source directory — at any depth, which is the one thing `--main` alone
/// cannot express.
fn layout_of(src: Option<PathBuf>, main_file: Option<PathBuf>) -> Result<Layout, String> {
    let Some(main) = main_file else {
        // clap guarantees one of the two is present.
        let src = src.ok_or("pass `--src`, `--main`, or both")?;
        return Ok(Layout {
            src,
            main_file: None,
        });
    };
    check_entry_file(&main)?;

    match src {
        // The entry file names its own directory as the source root.
        None => {
            let name = main
                .file_name()
                .and_then(|n| n.to_str())
                .ok_or_else(|| format!("`{}` has no file name", main.display()))?;
            let dir = main.parent().filter(|p| !p.as_os_str().is_empty());
            Ok(Layout {
                src: dir.map(Path::to_path_buf).unwrap_or_else(|| PathBuf::from(".")),
                main_file: Some(name.to_string()),
            })
        }
        // Both given: the entry is identified by its path *relative to the
        // source directory*, which is how the loaded sources are named.
        Some(src) => {
            let rel = absolute(&main)
                .strip_prefix(absolute(&src))
                .map(Path::to_path_buf)
                .map_err(|_| {
                    format!(
                        "`--main {}` is not inside `--src {}`: the entry file must be \
                         one of the compiled sources",
                        main.display(),
                        src.display()
                    )
                })?;
            Ok(Layout {
                src,
                main_file: Some(rel.display().to_string()),
            })
        }
    }
}

/// Rejects a `--main` that cannot be an entry point [cli-run], before any
/// source is loaded so the message can say *why*.
fn check_entry_file(main: &Path) -> Result<(), String> {
    if !main.is_file() {
        return Err(format!("`{}` is not a file", main.display()));
    }
    if main.extension().is_none_or(|e| e != "sv") {
        return Err(format!("`{}` is not a `.sv` source file", main.display()));
    }
    let name = main
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| format!("`{}` has no file name", main.display()))?;
    // A define file carries native templates, not code, so it declares no
    // `main` [backend-define-inline].
    if name.matches('.').count() > 1 {
        return Err(format!(
            "`{name}` is a backend define file, which declares no `main`: pass \
             the language file instead"
        ));
    }
    Ok(())
}

/// [cli-run] Rejects a `--target` that overlaps the sources. Two distinct
/// hazards, both fatal rather than surprising:
///
/// - The target *containing* the sources: `--clean-target` deletes the
///   target, so this would delete the program.
/// - The target sitting where source discovery would read it back: emitted
///   files with the backend's native extension are indistinguishable from
///   hand-written companion files [backend-companion], so the next build
///   would compile its own output. A dot-prefixed directory is skipped by
///   discovery [mod-ignore], which is why nesting is allowed only there.
fn check_target_overlap(src: &Path, target: &Path) -> Result<(), String> {
    let src_abs = absolute(src);
    let target_abs = absolute(target);
    if src_abs == target_abs {
        return Err(format!(
            "`--target {}` is the source directory itself",
            target.display()
        ));
    }
    if src_abs.starts_with(&target_abs) {
        return Err(format!(
            "`--target {}` contains the sources `{}`, and the target is deleted \
             before the build",
            target.display(),
            src.display()
        ));
    }
    if let Ok(rel) = target_abs.strip_prefix(&src_abs) {
        let hidden = rel
            .components()
            .next()
            .and_then(|c| c.as_os_str().to_str())
            .is_some_and(|c| c.starts_with('.'));
        if !hidden {
            return Err(format!(
                "`--target {}` is inside the sources `{}`, where the next build \
                 would read the emitted files back as sources: use a \
                 dot-prefixed directory (like the default `{DEFAULT_RUN_TARGET}`) \
                 or a target outside the source tree",
                target.display(),
                src.display()
            ));
        }
    }
    Ok(())
}

/// Lexically absolutizes a path against the working directory: `.` and `..`
/// are folded away without touching the filesystem, so a target that does
/// not exist yet still compares correctly.
fn absolute(path: &Path) -> PathBuf {
    use std::path::Component;
    let mut out = if path.is_absolute() {
        PathBuf::new()
    } else {
        std::env::current_dir().unwrap_or_default()
    };
    for comp in path.components() {
        match comp {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// [cli-run] Deletes the target directory. Refuses when it holds `.sv`
/// files: that means it is a source tree, and deleting it would be
/// destroying the user's work — the overlap check should have caught it, so
/// this is the belt to its braces.
fn clear_target(target: &Path) -> Result<(), String> {
    if !target.exists() {
        return Ok(());
    }
    if !target.is_dir() {
        return Err(format!(
            "`--target {}` exists and is not a directory",
            target.display()
        ));
    }
    if let Some(found) = find_source_file(target) {
        return Err(format!(
            "refusing to delete `--target {}`: it contains the Salvo source \
             `{}`",
            target.display(),
            found.display()
        ));
    }
    std::fs::remove_dir_all(target)
        .map_err(|err| format!("failed to clear `{}`: {err}", target.display()))
}

/// The first `.sv` file anywhere under `dir`, if any.
fn find_source_file(dir: &Path) -> Option<PathBuf> {
    let mut stack = vec![dir.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "sv") {
                return Some(path);
            }
        }
    }
    None
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
