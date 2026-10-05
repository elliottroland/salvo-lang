//! The `salvo` CLI.
//!
//! ```text
//! salvo compile --backend kotlin --src ./some_dir --target ./some_dir_kotlin
//! salvo run [--backend rust] --main ./some_dir/main.sv
//! salvo run --backend rust --src ./some_dir --main ./some_dir/bin/tool.sv
//! salvo run --backend rust --src ./some_dir --target ./out --clean-target before
//! salvo analyze --src ./some_dir [--format json]
//! salvo test --src ./some_dir [--backend rust] [FILTER] [--list] [--clean-target both]
//! salvo platform generate --backend kotlin --src ./some_dir
//! salvo lsp
//! salvo lang tm-grammar [--out vscode/syntaxes/salvo.tmLanguage.json]
//! ```

mod analysis;
mod docs;
mod lang;
mod lsp;

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand, ValueEnum};

use salvo_backend::{BackendError, BackendRegistry};
use salvo_backend_kotlin::KotlinBackend;
use salvo_backend_rust::RustBackend;
use salvo_core::{FileDiagnostic, Program, SourceSet};
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
        /// Target backend: `rust`, `kotlin`, or `*` for every backend
        /// (default: the manifest's `[build] backend`, else `kotlin`).
        #[arg(long)]
        backend: Option<String>,
        /// Directory containing `.sv` source files (default: the manifest's
        /// `[build] src`) [manifest].
        #[arg(long)]
        src: Option<PathBuf>,
        /// Output directory for generated sources (default: the manifest's
        /// `[<backend>] target`, else `[build] target`).
        #[arg(long)]
        target: Option<PathBuf>,
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
        /// Target backend: `rust`, `kotlin`, or `*` for every backend
        /// (default: the manifest's `[build] backend`, else `rust`, whose
        /// toolchain is the cheapest to start).
        #[arg(long)]
        backend: Option<String>,
        /// Directory containing `.sv` source files (default: the manifest's
        /// `[build] src`). Without `--main`, the unique `main` it declares is
        /// the entry point.
        #[arg(long)]
        src: Option<PathBuf>,
        /// The `.sv` file declaring `main` — how you choose between several
        /// entry points (default: the manifest's `[build] main`). Without
        /// `--src` it also implies `--src $(dirname <file>)`; with it, the
        /// file must be somewhere inside that directory.
        #[arg(long = "main")]
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
    /// Run the tests a source tree declares [test-run]: every
    /// `test "name" { … }` block in a `<module>.test.sv` annex.
    Test {
        /// Target backend: `rust`, `kotlin`, or `*` (default: the manifest's
        /// `[build] backend`, else `rust`).
        #[arg(long)]
        backend: Option<String>,
        /// Directory containing `.sv` source files, annexes included
        /// (default: the manifest's `[build] src`).
        #[arg(long)]
        src: Option<PathBuf>,
        /// Run only tests whose id contains this text — `module :: name`,
        /// so one word selects a module, a test, or a family [test-filter].
        filter: Option<String>,
        /// List the tests that would run, and run nothing.
        #[arg(long)]
        list: bool,
        /// Output directory for the generated sources (default:
        /// `.salvo_tmp_test`).
        #[arg(long)]
        target: Option<PathBuf>,
        /// Whether and when the target may be deleted. The default keeps the
        /// generated harness *after* the run, so a failure can be read —
        /// which is the one place it differs from `run`'s default.
        #[arg(long, value_enum, default_value_t = CleanTarget::Before)]
        clean_target: CleanTarget,
    },
    /// Parse, resolve, and type-check sources without generating code
    /// [cli-analyze].
    Analyze {
        /// Directory containing `.sv` source files (default: the manifest's
        /// `[build] src`).
        #[arg(long)]
        src: Option<PathBuf>,
        /// Output format for diagnostics.
        #[arg(long, value_enum, default_value_t = Format::Text)]
        format: Format,
    },
    /// Start a language server speaking LSP over stdio [cli-lsp]. The
    /// workspace root comes from the client's `initialize` request.
    Lsp,
    /// Emit language metadata for editor tooling [cli-lang].
    Lang {
        #[command(subcommand)]
        command: LangCommand,
    },
    /// Work with the host side of `platform handler` declarations and
    /// `platform fn`s [cli-platform].
    Platform {
        #[command(subcommand)]
        command: PlatformCommand,
    },
}

#[derive(Subcommand)]
enum PlatformCommand {
    /// Write the implementation skeleton for every `platform handler` and
    /// `platform fn` into each backend's platform root [platform-root]
    /// [cli-platform].
    ///
    /// Existing files are never touched: the skeleton is generated once and
    /// belongs to you afterwards, and every later divergence from the
    /// generated interface is a target-language compile error rather than
    /// something the compiler has to merge.
    Generate {
        /// Target backend, which decides the language of the skeleton
        /// (default: the manifest's `[build] backend`; `*` writes both).
        #[arg(long)]
        backend: Option<String>,
        /// Directory containing `.sv` source files — also where the
        /// `platform/` tree is written (default: the manifest's `[build] src`).
        #[arg(long)]
        src: Option<PathBuf>,
        /// The `.sv` file declaring `main`, as for `salvo run`: it picks
        /// between several entry points, and on its own implies its own
        /// directory as the source directory.
        #[arg(long = "main")]
        main_file: Option<PathBuf>,
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
        } => compile(backend, src, target, emit_ast.as_deref()),
        Command::Run {
            backend,
            src,
            main_file,
            target,
            clean_target,
        } => run(backend, src, main_file, target, clean_target),
        Command::Analyze { src, format } => analyze(src, format),
        Command::Test {
            backend,
            src,
            filter,
            list,
            target,
            clean_target,
        } => test(backend, src, filter.as_deref(), list, target, clean_target),
        Command::Lsp => lsp::run(),
        Command::Lang { command } => match command {
            LangCommand::TmGrammar { out } => lang::run_tm_grammar(out.as_ref()),
        },
        Command::Platform { command } => match command {
            PlatformCommand::Generate {
                backend,
                src,
                main_file,
            } => platform_generate(backend, src, main_file),
        },
    }
}

fn registry() -> BackendRegistry {
    let mut registry = BackendRegistry::new();
    registry.register(Box::new(KotlinBackend));
    registry.register(Box::new(RustBackend));
    registry
}

/// `salvo analyze`: the front half of `compile` — parse, resolve, and
/// type-check — reporting every diagnostic instead of emitting code
/// [cli-analyze]. Exits nonzero when any diagnostic is an error.
///
/// There is no `--backend`: checking is backend-neutral, and nothing a
/// backend selects participates in it (companions are copied, never
/// checked).
fn analyze(src: Option<PathBuf>, format: Format) -> ExitCode {
    // [manifest] `--src` or the manifest's source root.
    let inputs = match resolve_inputs(src, None, None, "rust", false) {
        Ok(i) => i,
        Err(msg) => {
            eprintln!("error: {msg}");
            return ExitCode::FAILURE;
        }
    };
    let src = &inputs.layout.src;
    // [manifest-deps] The project is what names the dependencies to load.
    let analysis = match analysis::analyze_sources(src, inputs.project.clone(), "", &Default::default()) {
        Ok(analysis) => analysis,
        Err(msg) => {
            eprintln!("error: {msg}");
            return ExitCode::FAILURE;
        }
    };
    for err in &analysis.io_errors {
        eprintln!("error: {err}");
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
    /// [platform-host-deps] The host libraries this program actually needs.
    host: salvo_core::HostDeps,
}

/// [platform-host-deps] The host libraries of a program: the project's own, and
/// each dependency's only when that dependency's platform code is reached — a
/// program using only the aws module's fakes builds without either SDK.
fn effective_host(
    program: &Program,
    project: Option<&salvo_core::Project>,
    all: &salvo_core::HostDeps,
) -> Result<salvo_core::HostDeps, String> {
    let Some(project) = project else {
        return Ok(all.clone());
    };
    let reached = salvo_core::dependencies_with_reached_platform(
        &program.files,
        &program.modules,
        &program.companions,
    );
    project.host_deps_for(&|name| reached.contains(name))
}

/// The source layout of a build: the directory to scan, plus (with
/// `--main`) the file inside it that must declare the entry point
/// [cli-run].
struct Layout {
    src: PathBuf,
    /// The entry file's path relative to `src`. `None` means "whatever
    /// unique `main` the directory declares".
    main_file: Option<String>,
    /// [test-file] Whether `<name>.test.sv` annexes are loaded. Only
    /// `salvo test` sets it: a production build does not walk them, which is
    /// the whole of how tests stay out of a shipped program.
    tests: bool,
}

/// A parsed, entry-resolved program: what `compile`, `run` and
/// `platform generate` all need before they diverge.
struct Assembled {
    program: Program,
    main_module: Option<salvo_core::ModulePath>,
    /// The names of every file declaring `main`, when the choice was left
    /// open and there was more than one.
    ambiguous: Vec<String>,
    /// [test-run] Every test the sources declare, in file order. Empty
    /// unless `layout.tests`.
    tests: Vec<salvo_core::TestCase>,
}

/// Loads, parses and entry-resolves `layout`'s sources for `backend`.
///
/// The front half shared by `compile`, `run` and `platform generate`
/// [cli-run] [cli-platform]: `Err` carries the exit code to return
/// (diagnostics already reported), `Ok(None)` means the caller asked to stop
/// early (`--emit-ast`).
fn assemble(
    backend: &dyn salvo_backend::Backend,
    layout: &Layout,
    emit_ast: Option<&str>,
    verbose: bool,
    project: Option<&salvo_core::Project>,
) -> Result<Option<Assembled>, ExitCode> {
    // Assemble sources: embedded std first (implicitly imported), then the
    // user's source directory.
    let mut sources = SourceSet::default();
    load_embedded_std(&mut sources, backend.file_extension());

    if !layout.src.is_dir() {
        eprintln!(
            "error: source directory `{}` does not exist",
            layout.src.display()
        );
        return Err(ExitCode::FAILURE);
    }
    // [manifest-deps] Dependencies load between std and the project's own
    // tree, companions included: a dependency's platform handlers need their
    // host files as much as the project's do.
    let mut io_errors =
        analysis::load_dependencies(&mut sources, project, backend.file_extension());
    // [platform-root] The project's platform roots, when it names any.
    if let Some(project) = project {
        sources.platform = project.platform_roots();
    }
    io_errors.extend(sources.add_dir(&layout.src, backend.file_extension(), false, layout.tests));
    for err in &io_errors {
        eprintln!("error: {err}");
    }
    if !io_errors.is_empty() {
        return Err(ExitCode::FAILURE);
    }
    // [std-shadow] A source tree declaring std's own modules replaces them,
    // which is what `salvo test --src std` rests on.
    for note in sources.apply_std_shadow() {
        if verbose {
            eprintln!("note: {note}");
        }
    }
    // [manifest] `[project] std = true`: the tree *is* the standard library,
    // so a module the embedded copy lacks (a new file) is std too.
    if project.is_some_and(|p| p.is_std()) {
        sources.mark_std_tree();
    }

    // Parse every module and collect diagnostics. The `iter fn` and `test`
    // expansions are deferred to a program-level pass: the first needs the
    // rest of the program's structs [iter-fn], the second needs to know
    // which files are test annexes [test-file].
    let mut modules = Vec::with_capacity(sources.files.len());
    let mut error_count = 0usize;
    for file in &sources.files {
        let (module, diagnostics) = salvo_syntax::parse_module_deferred(&file.content);
        for diag in &diagnostics {
            eprintln!("{}", diag.render(&file.name, &file.content));
            if diag.is_error() {
                error_count += 1;
            }
        }
        modules.push(module);
    }
    let expansion = salvo_core::expand(&sources.files, &mut modules);
    for diag in &expansion.diagnostics {
        eprintln!("{}", diag.render(&sources.files));
        if diag.is_error() {
            error_count += 1;
        }
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
            println!("// ===== {} (module `{}`) =====", file.name, file.module);
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
            // [manifest-deps] A dependency's `main` is its own business.
            !file.is_std
                && !file.is_test
                && file.dependency.is_none()
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
    // [platform-root] Platform declarations need a platform root for every
    // backend in play — the one being built, and every one the manifest
    // builds.
    {
        let mut backends: Vec<&str> = project.map(salvo_core::required_backends).unwrap_or_default();
        if !backends.contains(&backend.name()) {
            backends.push(backend.name());
        }
        let missing = salvo_core::platform_root_required(&program, project, &backends);
        for diag in &missing {
            eprintln!("{}", diag.render(&program.files));
        }
        if !missing.is_empty() {
            return Err(ExitCode::FAILURE);
        }
    }
    Ok(Some(Assembled {
        program,
        main_module,
        ambiguous,
        tests: expansion.tests,
    }))
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
    project: Option<&salvo_core::Project>,
    host: &salvo_core::HostDeps,
    for_run: bool,
) -> Result<Option<Built>, ExitCode> {
    let Some(Assembled {
        program,
        main_module,
        ambiguous,
        tests: _,
    }) = assemble(backend, layout, emit_ast, verbose, project)?
    else {
        return Ok(None);
    };
    // [protocol-lock] Before emission: a protocol that changed under a
    // deployed version is refused whether or not the code compiles.
    if let Some(p) = project {
        if let Err(msg) = reconcile_lock(p, &program) {
            eprintln!("error: {msg}");
            return Err(ExitCode::FAILURE);
        }
    }
    let host = match effective_host(&program, project, host) {
        Ok(h) => h,
        Err(msg) => {
            eprintln!("error: {msg}");
            return Err(ExitCode::FAILURE);
        }
    };
    let host = &host;
    write_platform_abi(backend, &program, main_module.as_ref(), project, host)?;
    let emitted = match backend.emit(&program, target, main_module.as_ref()) {
        Ok(emitted) => emitted,
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
    // Non-fatal diagnostics reach the builder here [diag-structured]: the
    // program compiled, so these are reported and nothing else happens.
    // Printed unconditionally (not only under `--verbose`), since a warning
    // nobody sees is the same as no warning [qual-refn-ambiguous].
    for msg in &emitted.warnings {
        eprintln!("{msg}");
    }
    check_dependency_stamps(backend, &program, project, &emitted.files)?;
    // [kt-std-library] A build about to run readies what it links.
    if for_run {
        if let Err(err) = backend.prepare_run(&program, target) {
            eprintln!("error: {err}");
            return Err(ExitCode::FAILURE);
        }
    }
    let mut written = emitted.files;
    // [platform-host-deps] What the host build needs to know about the
    // declared libraries — a `Cargo.toml` for Rust — beside the sources.
    // Only a program with an entry point is buildable at all, so only then.
    if let Some(module) = &main_module {
        match backend.write_host_manifest(target, module, host) {
            Ok(more) => written.extend(more),
            Err(err) => {
                eprintln!("error: {err}");
                return Err(ExitCode::FAILURE);
            }
        }
    }
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
            eprintln!(
                "entry point: {}",
                backend.entry_hint(target, module, &written)
            );
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
        host: host.clone(),
    }))
}

fn compile(
    backend_name: Option<String>,
    src: Option<PathBuf>,
    target: Option<PathBuf>,
    emit_ast: Option<&str>,
) -> ExitCode {
    let registry = registry();
    let inputs = match resolve_inputs(src, None, backend_name, "kotlin", false) {
        Ok(i) => i,
        Err(msg) => {
            eprintln!("error: {msg}");
            return ExitCode::FAILURE;
        }
    };
    for name in &inputs.backends {
        let Some(backend) = registry.get(name) else {
            eprintln!("{}", unknown_backend(&registry, name));
            return ExitCode::FAILURE;
        };
        let target = match target_for(&inputs, name, target.as_ref(), None) {
            Ok(t) => t,
            Err(msg) => {
                eprintln!("error: {msg}");
                return ExitCode::FAILURE;
            }
        };
        if let Err(code) = build(backend, &inputs.layout, &target, emit_ast, true, inputs.project.as_ref(), &inputs.host, false) {
            return code;
        }
    }
    ExitCode::SUCCESS
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
    backend_name: Option<String>,
    src: Option<PathBuf>,
    main_file: Option<PathBuf>,
    target: Option<PathBuf>,
    clean: CleanTarget,
) -> ExitCode {
    let registry = registry();
    // `--src` and `--main` each supply a default for the other (user
    // decision 2026-09-05), and the manifest supplies both [manifest].
    let inputs = match resolve_inputs(src, main_file, backend_name, "rust", false) {
        Ok(i) => i,
        Err(msg) => {
            eprintln!("error: {msg}");
            return ExitCode::FAILURE;
        }
    };
    let mut worst = ExitCode::SUCCESS;
    for name in &inputs.backends {
        let Some(backend) = registry.get(name) else {
            eprintln!("{}", unknown_backend(&registry, name));
            return ExitCode::FAILURE;
        };
        let target = match target_for(&inputs, name, target.as_ref(), Some(DEFAULT_RUN_TARGET)) {
            Ok(t) => t,
            Err(msg) => {
                eprintln!("error: {msg}");
                return ExitCode::FAILURE;
            }
        };
        let code = run_one(backend, &inputs, &target, clean);
        if code != ExitCode::SUCCESS {
            worst = code;
        }
    }
    worst
}

fn run_one(
    backend: &dyn salvo_backend::Backend,
    inputs: &Inputs,
    target: &PathBuf,
    clean: CleanTarget,
) -> ExitCode {
    let layout = &inputs.layout;
    if let Err(msg) = check_target_overlap(&layout.src, target) {
        eprintln!("error: {msg}");
        return ExitCode::FAILURE;
    }
    // Both `--clean-target` modes clear the target first: a run must not
    // pick up a previous run's output.
    if let Err(msg) = clear_target(target) {
        eprintln!("error: {msg}");
        return ExitCode::FAILURE;
    }

    let built = match build(backend, layout, target, None, false, inputs.project.as_ref(), &inputs.host, true) {
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

    let outcome = backend.run(target, &main_module, &built.written, &built.host);

    // The target is deleted after the run only with `--clean-target both`,
    // and never before the program's output has been produced.
    if clean == CleanTarget::Both {
        if let Err(msg) = clear_target(target) {
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

/// The default `--target` for `salvo test` [test-run]: dot-prefixed like
/// `salvo run`'s, so it is skipped by source discovery [mod-ignore].
const DEFAULT_TEST_TARGET: &str = ".salvo_tmp_test";

/// `salvo test`: run the tests a source tree declares [test-run].
///
/// The whole of it: assemble the sources *with* their `<name>.test.sv`
/// annexes, synthesize a harness module that delimits each test with a `try`,
/// compile and run it exactly as `salvo run` would, and render its protocol as
/// the report [test-report]. Nothing here is backend-specific — the harness is
/// a Salvo program.
fn test(
    backend_name: Option<String>,
    src: Option<PathBuf>,
    filter: Option<&str>,
    list: bool,
    target: Option<PathBuf>,
    clean: CleanTarget,
) -> ExitCode {
    let registry = registry();
    let inputs = match resolve_inputs(src, None, backend_name, "rust", true) {
        Ok(i) => i,
        Err(msg) => {
            eprintln!("error: {msg}");
            return ExitCode::FAILURE;
        }
    };
    let mut worst = ExitCode::SUCCESS;
    for name in &inputs.backends {
        let Some(backend) = registry.get(name) else {
            eprintln!("{}", unknown_backend(&registry, name));
            return ExitCode::FAILURE;
        };
        let target = match target_for(&inputs, name, target.as_ref(), Some(DEFAULT_TEST_TARGET)) {
            Ok(t) => t,
            Err(msg) => {
                eprintln!("error: {msg}");
                return ExitCode::FAILURE;
            }
        };
        // [test-report] Under `backend = "*"` the suite runs once per
        // backend, and each report says whose it is.
        if inputs.backends.len() > 1 {
            if name != &inputs.backends[0] {
                println!();
            }
            println!("{name}:");
        }
        let code = test_one(backend, &inputs, filter, list, target, clean);
        if code != ExitCode::SUCCESS {
            worst = code;
        }
        if list {
            break;
        }
    }
    worst
}

fn test_one(
    backend: &dyn salvo_backend::Backend,
    inputs: &Inputs,
    filter: Option<&str>,
    list: bool,
    target: PathBuf,
    clean: CleanTarget,
) -> ExitCode {
    let layout = &inputs.layout;
    let Some(mut assembled) = (match assemble(backend, layout, None, false, inputs.project.as_ref()) {
        Ok(assembled) => assembled,
        Err(code) => return code,
    }) else {
        return ExitCode::SUCCESS;
    };

    let host = match effective_host(&assembled.program, inputs.project.as_ref(), &inputs.host) {
        Ok(h) => h,
        Err(msg) => {
            eprintln!("error: {msg}");
            return ExitCode::FAILURE;
        }
    };
    if let Err(code) = write_platform_abi(
        backend,
        &assembled.program,
        assembled.main_module.as_ref(),
        inputs.project.as_ref(),
        &host,
    )
    {
        return code;
    }
    let selected = salvo_test::select(&assembled.tests, filter);
    if list {
        for test in &selected {
            println!("{}", test.id());
        }
        return ExitCode::SUCCESS;
    }
    if selected.is_empty() {
        // Not a failure: a tree with no tests, or a filter that matched
        // none, is a run with nothing to do — and the count says which.
        eprintln!(
            "no tests {}in `{}`: a test is a `test \"name\" {{ … }}` block in a \
             `<module>.test.sv` file beside the module it tests [test-file]",
            match filter {
                Some(f) => format!("match `{f}` "),
                None => String::new(),
            },
            layout.src.display()
        );
        return ExitCode::SUCCESS;
    }

    // The harness: a synthesized module, added to the program rather than
    // written into the source tree [test-run].
    let harness_module = salvo_core::ModulePath(vec![salvo_test::HARNESS_MODULE.to_string()]);
    if assembled
        .program
        .files
        .iter()
        .any(|f| f.module == harness_module)
    {
        eprintln!(
            "error: `{}` declares a module called `{}`, which is the name the test \
             harness is generated under — rename it",
            layout.src.display(),
            harness_module
        );
        return ExitCode::FAILURE;
    }
    if let Err(msg) = check_target_overlap(&layout.src, &target) {
        eprintln!("error: {msg}");
        return ExitCode::FAILURE;
    }
    if let Err(msg) = clear_target(&target) {
        eprintln!("error: {msg}");
        return ExitCode::FAILURE;
    }
    let color = if std::io::IsTerminal::is_terminal(&std::io::stdout()) {
        salvo_test::Color::Always
    } else {
        salvo_test::Color::Never
    };

    // [test-recover] A run is one pass per process, and a test that *dies* —
    // a failed `assert!` [assert-trap], an intrinsic's own trap, a killed
    // program — takes the process with it. So the runner attributes the trap to
    // the test that was in flight, and re-runs what was left in a fresh
    // process: a death costs one extra build, not the rest of the suite (user
    // decision 2026-09-23, A-5).
    let mut total = salvo_test::Summary::default();
    // [test-kind] One program per runtime (D11): the plain tests on the
    // threaded runtime, then the actor tests on the virtual one — a process
    // cannot leave the threaded runtime once its workers have started.
    let (actor_tests, plain_tests): (Vec<salvo_core::TestCase>, Vec<salvo_core::TestCase>) =
        selected
            .iter()
            .cloned()
            .partition(|t| matches!(t.kind, salvo_syntax::ast::TestKind::Actor { .. }));
    for group in [plain_tests, actor_tests] {
        if let Err(code) = run_test_group(
            backend,
            &mut assembled.program,
            &harness_module,
            group,
            &target,
            color,
            &host,
            inputs.project.as_ref(),
            &mut total,
        ) {
            return code;
        }
    }
    let mut out = std::io::stdout().lock();
    let _ = salvo_test::print_summary(&total, &mut out, color);
    drop(out);
    // [cli-run] `--clean-target both` deletes the generated harness after the
    // report, exactly as `run` deletes a program's sources. The default is
    // `before`, because a failing harness is worth reading.
    if clean == CleanTarget::Both {
        if let Err(msg) = clear_target(&target) {
            eprintln!("warning: {msg}");
        }
    }
    if total.ok() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// [test-recover] [test-kind] Runs one group of tests — one runtime's — in as
/// many passes as deaths require, adding to [total].
#[allow(clippy::too_many_arguments)]
fn run_test_group(
    backend: &dyn salvo_backend::Backend,
    program: &mut Program,
    harness_module: &salvo_core::ModulePath,
    group: Vec<salvo_core::TestCase>,
    target: &Path,
    color: salvo_test::Color,
    host: &salvo_core::HostDeps,
    project: Option<&salvo_core::Project>,
    total: &mut salvo_test::Summary,
) -> Result<(), ExitCode> {
    let mut remaining: Vec<salvo_core::TestCase> = group;
    // One pass per death, plus the first: a pass always either finishes or
    // removes one test from `remaining`, so this cannot spin.
    let mut passes_left = remaining.len() + 1;
    while !remaining.is_empty() && passes_left > 0 {
        passes_left -= 1;
        let pass = match run_test_pass(
            backend,
            program,
            harness_module,
            &remaining,
            target,
            color,
            host,
            project,
        ) {
            Ok(pass) => pass,
            Err(code) => return Err(code),
        };
        let died = pass.summary.unfinished.last().cloned();
        let ok_so_far = pass.summary.ok();
        total.merge(pass.summary);
        match died {
            Some(id) => {
                // The trap message goes under the test that died, which is the
                // whole point: a failed assertion reads as that test's failure
                // rather than as a dead run.
                let mut out = std::io::stdout().lock();
                for line in salvo_test::died_detail(&pass.stderr) {
                    let _ = writeln!(out, "    {line}");
                }
                drop(out);
                let at = remaining.iter().position(|t| t.id() == id);
                remaining = match at {
                    Some(at) => remaining.split_off(at + 1),
                    // The runner and the harness disagree about what ran, which
                    // is a compiler bug rather than a test failure.
                    None => Vec::new(),
                };
                if !remaining.is_empty() {
                    eprintln!(
                        "note: `{id}` took the process with it; re-running the \
                         remaining {} test(s)",
                        remaining.len()
                    );
                }
            }
            None => {
                if !pass.stderr.trim().is_empty() {
                    eprint!("{}", pass.stderr);
                }
                if !pass.exit_ok && ok_so_far {
                    // Every test passed and the process still failed: something
                    // outside a test went wrong, and silence would report a
                    // green run for a broken one.
                    eprintln!(
                        "error: the test harness exited with a failure although \
                         every test passed"
                    );
                    return Err(ExitCode::FAILURE);
                }
                remaining.clear();
            }
        }
    }

    Ok(())
}

/// [test-recover] One pass of the harness: synthesize a module for `tests`,
/// emit the program, run it, and render its protocol. The harness module is
/// added to the program for the pass and taken off again, so a later pass
/// synthesizes a fresh one.
struct TestPass {
    summary: salvo_test::Summary,
    /// What the program wrote to stderr — where a trap message lands.
    stderr: String,
    exit_ok: bool,
}

fn run_test_pass(
    backend: &dyn salvo_backend::Backend,
    program: &mut Program,
    harness_module: &salvo_core::ModulePath,
    tests: &[salvo_core::TestCase],
    target: &Path,
    color: salvo_test::Color,
    host: &salvo_core::HostDeps,
    project: Option<&salvo_core::Project>,
) -> Result<TestPass, ExitCode> {
    let source = salvo_test::harness_source(tests);
    let (ast, diagnostics) = salvo_syntax::parse_module_deferred(&source);
    if diagnostics.iter().any(|d| d.is_error()) {
        // A generated program that does not parse is a compiler bug, and the
        // source is printed with it so the bug is one read away.
        for diag in &diagnostics {
            eprintln!("{}", diag.render("<generated harness>", &source));
        }
        eprintln!("error: internal: the generated test harness does not parse\n{source}");
        return Err(ExitCode::FAILURE);
    }
    program.files.push(salvo_core::SourceFile {
        name: format!("{}.sv", salvo_test::HARNESS_MODULE),
        module: harness_module.clone(),
        content: source,
        is_std: false,
        is_shadow: false,
        // [test-implicit-import] The harness *is* test code: marking it so gives
        // it `std.test` without an import line, which is also what keeps it
        // clear of an `import <module>.test` for an annex of `test` itself.
        is_test: true,
        dependency: None,
    });
    program.modules.push(ast);
    let emitted = backend.emit(program, target, Some(harness_module));
    program.files.pop();
    program.modules.pop();
    let emitted = match emitted {
        Ok(emitted) => emitted,
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
    for msg in &emitted.warnings {
        eprintln!("{msg}");
    }
    check_dependency_stamps(backend, program, project, &emitted.files)?;
    if let Err(err) = backend.prepare_run(program, target) {
        eprintln!("error: {err}");
        return Err(ExitCode::FAILURE);
    }
    // [platform-host-deps] The harness is a program like any other.
    let mut files = emitted.files;
    match backend.write_host_manifest(target, harness_module, host) {
        Ok(more) => files.extend(more),
        Err(err) => {
            eprintln!("error: {err}");
            return Err(ExitCode::FAILURE);
        }
    }

    let mut command = match backend.program_command(target, harness_module, &files, host) {
        Ok(command) => command,
        Err(err) => {
            eprintln!("error: {err}");
            return Err(ExitCode::FAILURE);
        }
    };
    // The harness's stdout is the protocol the report is rendered from
    // [test-report]; its stderr is captured so a trap can be attributed to the
    // test that was running [test-recover].
    command.stdout(std::process::Stdio::piped());
    command.stderr(std::process::Stdio::piped());
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(err) => {
            eprintln!(
                "error: failed to start the test harness (`{}`): {err}",
                command.get_program().to_string_lossy()
            );
            return Err(ExitCode::FAILURE);
        }
    };
    let stdout = child.stdout.take().expect("piped");
    let mut err_pipe = child.stderr.take().expect("piped");
    // Drained on a thread: a program that writes more to stderr than the pipe
    // holds would otherwise block while the runner reads stdout.
    let err_reader = std::thread::spawn(move || {
        let mut buf = String::new();
        let _ = std::io::Read::read_to_string(&mut err_pipe, &mut buf);
        buf
    });
    let mut out = std::io::stdout().lock();
    let summary = salvo_test::render_stream(std::io::BufReader::new(stdout), &mut out, color);
    drop(out);
    let stderr = err_reader.join().unwrap_or_default();
    let status = child.wait();
    let summary = match summary {
        Ok(summary) => summary,
        Err(err) => {
            eprintln!("error: failed to read the test harness's output: {err}");
            return Err(ExitCode::FAILURE);
        }
    };
    let exit_ok = match status {
        Ok(status) => status.success(),
        Err(err) => {
            eprintln!("error: failed to wait for the test harness: {err}");
            return Err(ExitCode::FAILURE);
        }
    };
    Ok(TestPass {
        summary,
        stderr,
        exit_ok,
    })
}

/// `salvo platform generate` [cli-platform]: writes the implementation
/// skeletons into each backend's platform root [platform-root].
///
/// **Never overwrites.** With an interface between Salvo and the host, the
/// file only has to be right once: afterwards every kind of drift — a member
/// added, removed, or re-signed — is an error from the
/// *target* compiler, so there is nothing for this command to merge and no
/// reason for it to touch code a human has edited.
fn platform_generate(
    backend_name: Option<String>,
    src: Option<PathBuf>,
    main_file: Option<PathBuf>,
) -> ExitCode {
    let registry = registry();
    let inputs = match resolve_inputs(src, main_file, backend_name, "kotlin", false) {
        Ok(i) => i,
        Err(msg) => {
            eprintln!("error: {msg}");
            return ExitCode::FAILURE;
        }
    };
    for name in &inputs.backends {
        let Some(backend) = registry.get(name) else {
            eprintln!("{}", unknown_backend(&registry, name));
            return ExitCode::FAILURE;
        };
        let code = platform_generate_one(backend, &inputs);
        if code != ExitCode::SUCCESS {
            return code;
        }
    }
    ExitCode::SUCCESS
}

/// [platform-stamp] A dependency checks in its own host project (ABI D9), and
/// its implementation files were written against it. Before building them,
/// its stamps are compared with what this compiler computes for its platform
/// signatures: a different ABI revision, a changed signature, or no generated
/// files at all is reported naming the dependency and the command that fixes
/// it — rather than as a host compile error inside code the consumer did not
/// write.
fn check_dependency_stamps(
    backend: &dyn salvo_backend::Backend,
    program: &Program,
    project: Option<&salvo_core::Project>,
    emitted: &[PathBuf],
) -> Result<(), ExitCode> {
    let Some(project) = project else { return Ok(()) };
    let Ok(deps) = project.dependencies() else { return Ok(()) };
    let symbols = salvo_core::Symbols::collect(program);
    let ext = backend.file_extension();
    for dep in deps {
        // Only a dependency whose implementation files this build compiles.
        let reached = program.companions.iter().any(|c| {
            c.platform
                && emitted.contains(&c.rel_path)
                && program
                    .units()
                    .any(|u| u.file.module == c.module && u.file.dependency.as_deref() == Some(dep.name.as_str()))
        });
        if !reached {
            continue;
        }
        let Some(expected) = salvo_core::abi::abi_stamp(program, &symbols, Some(&dep.name)) else {
            continue;
        };
        let Some(root) = dep.project.platform_root(backend.name()) else { continue };
        let regenerate = format!(
            "run `salvo platform generate --backend {}` in `{}` and check in what it writes",
            backend.name(),
            dep.project.dir.display()
        );
        let mut found: Option<(PathBuf, String)> = None;
        let mut any = false;
        let mut stack = vec![root.clone()];
        while let Some(dir) = stack.pop() {
            let Ok(entries) = std::fs::read_dir(&dir) else { continue };
            for entry in entries.flatten() {
                let path = entry.path();
                let name = entry.file_name().to_string_lossy().to_string();
                if path.is_dir() {
                    if !name.starts_with('.') && name != "target" {
                        stack.push(path);
                    }
                    continue;
                }
                if !name.ends_with(&format!(".sv.{ext}")) {
                    continue;
                }
                any = true;
                let text = std::fs::read_to_string(&path).unwrap_or_default();
                let stamp = text
                    .lines()
                    .take(6)
                    .find_map(|l| l.strip_prefix("// salvo-abi ").map(|r| format!("salvo-abi {r}")))
                    .unwrap_or_default();
                if stamp != expected && found.is_none() {
                    found = Some((path, stamp));
                }
            }
        }
        let missing = !any;
        if missing {
            eprintln!(
                "error: dependency `{}` has platform code but no generated host project under `{}`: \
                 {regenerate} [platform-stamp]",
                dep.name,
                root.display()
            );
            return Err(ExitCode::FAILURE);
        }
        if let Some((path, stamp)) = found {
            let parse = |s: &str| s.split_whitespace().nth(1).map(str::to_string);
            let why = match (parse(&stamp), parse(&expected)) {
                (None, _) => "carries no ABI stamp".to_string(),
                (Some(had), Some(now)) if had != now => {
                    format!("was generated for ABI {had}, and this compiler's is {now}")
                }
                _ => "was generated from platform signatures that have changed since".to_string(),
            };
            eprintln!(
                "error: dependency `{}`'s host project {why} (`{}`): {regenerate} [platform-stamp]",
                dep.name,
                path.display()
            );
            return Err(ExitCode::FAILURE);
        }
    }
    Ok(())
}

/// [platform-abi] Rewrites the generated files of this backend's platform root
/// — the host project the root's implementation files compile in (ABI D2,
/// D3) — and removes generated files the program no longer produces. Only the
/// project's own root: a dependency's generated files are its own (D9), and
/// std's are written when std itself is built. Nothing when the project has
/// no root for this backend.
fn write_platform_abi(
    backend: &dyn salvo_backend::Backend,
    program: &Program,
    entry: Option<&salvo_core::ModulePath>,
    project: Option<&salvo_core::Project>,
    host: &salvo_core::HostDeps,
) -> Result<(), ExitCode> {
    let Some(root) = project.and_then(|p| p.platform_root(backend.name())) else {
        return Ok(());
    };
    let files = match backend.platform_abi(program, entry, host) {
        Ok(files) => files,
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
    let ext = backend.file_extension();
    let written: std::collections::HashSet<PathBuf> = files.iter().map(|(p, _)| root.join(p)).collect();
    // Stale ones first: generated (by their header), of this backend's
    // kind, and not produced this time.
    let mut stack = vec![root.clone()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else { continue };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if path.is_dir() {
                if !name.starts_with('.') && name != "build" && name != "target" {
                    stack.push(path);
                }
                continue;
            }
            let ours = name.ends_with(&format!(".sv.{ext}"))
                || (ext == "kt" && (name == "build.gradle.kts" || name == "settings.gradle.kts"))
                || (ext == "rs" && name == "Cargo.toml");
            if ours
                && !written.contains(&path)
                && std::fs::read_to_string(&path)
                    .is_ok_and(|t| t.starts_with("// GENERATED by salvo") || t.starts_with("# GENERATED by salvo"))
            {
                let _ = std::fs::remove_file(&path);
            }
        }
    }
    for (rel, content) in files {
        let path = root.join(rel);
        if std::fs::read_to_string(&path).is_ok_and(|t| t == content) {
            continue;
        }
        if let Some(parent) = path.parent() {
            if let Err(err) = std::fs::create_dir_all(parent) {
                eprintln!("error: failed to create `{}`: {err}", parent.display());
                return Err(ExitCode::FAILURE);
            }
        }
        if let Err(err) = std::fs::write(&path, content) {
            eprintln!("error: failed to write `{}`: {err}", path.display());
            return Err(ExitCode::FAILURE);
        }
    }
    Ok(())
}

/// Writes `content` at `path` unless a file is there already; `Ok(true)` when
/// written [cli-platform].
fn write_once(path: &Path, content: &str) -> Result<bool, ExitCode> {
    if path.exists() {
        eprintln!("kept {} (already exists)", path.display());
        return Ok(false);
    }
    if let Some(parent) = path.parent() {
        if let Err(err) = std::fs::create_dir_all(parent) {
            eprintln!("error: failed to create `{}`: {err}", parent.display());
            return Err(ExitCode::FAILURE);
        }
    }
    if let Err(err) = std::fs::write(path, content) {
        eprintln!("error: failed to write `{}`: {err}", path.display());
        return Err(ExitCode::FAILURE);
    }
    eprintln!("wrote {}", path.display());
    Ok(true)
}

/// [cli-platform] [platform-root] One backend's skeletons, into that
/// backend's platform root: an **implementation file** (`<m>.<ext>`) per
/// module with platform handlers or `platform fn`s, every member and fn
/// stubbed. A file that exists is never touched.
fn platform_generate_one(backend: &dyn salvo_backend::Backend, inputs: &Inputs) -> ExitCode {
    let layout = &inputs.layout;
    let project = inputs.project.as_ref();
    let Some(assembled) = (match assemble(backend, layout, None, false, project) {
        Ok(assembled) => assembled,
        Err(code) => return code,
    }) else {
        return ExitCode::SUCCESS;
    };
    let host = match effective_host(&assembled.program, project, &inputs.host) {
        Ok(h) => h,
        Err(msg) => {
            eprintln!("error: {msg}");
            return ExitCode::FAILURE;
        }
    };
    if let Err(code) =
        write_platform_abi(backend, &assembled.program, assembled.main_module.as_ref(), project, &host)
    {
        return code;
    }
    // `assemble` refuses platform declarations without a root, so a program
    // that has any has one here.
    let Some(root) = project.and_then(|p| p.platform_root(backend.name())) else {
        eprintln!(
            "no `platform handler` or `platform fn` in `{}`: nothing to generate",
            layout.src.display()
        );
        return ExitCode::SUCCESS;
    };
    let mut written = 0usize;
    let mut kept = 0usize;
    let mut any = false;
    let skeletons = match backend
        .platform_skeletons(&assembled.program, assembled.main_module.as_ref())
    {
        Ok(files) => files,
        Err(BackendError::Codegen(msgs)) => {
            for msg in &msgs {
                eprintln!("{msg}");
            }
            return ExitCode::FAILURE;
        }
        Err(err) => {
            eprintln!("error: {err}");
            return ExitCode::FAILURE;
        }
    };
    for (rel_path, content) in skeletons {
        any = true;
        // The backend names the file as it is emitted, under `platform/`;
        // on disk it goes under this backend's root.
        let rel = rel_path.strip_prefix(salvo_core::PLATFORM_DIR).unwrap_or(&rel_path);
        match write_once(&root.join(rel), &content) {
            Ok(true) => written += 1,
            Ok(false) => kept += 1,
            Err(code) => return code,
        }
    }
    if !any {
        eprintln!(
            "no `platform handler` or `platform fn` in `{}`: nothing to generate",
            layout.src.display()
        );
        return ExitCode::SUCCESS;
    }
    eprintln!(
        "generated {written} host file(s){}; implement the stubbed members, then \
         `salvo run --backend {}`",
        if kept == 0 {
            String::new()
        } else {
            format!(", left {kept} existing file(s) alone")
        },
        backend.name()
    );
    ExitCode::SUCCESS
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
/// [manifest] What a command resolved its inputs to, flag over manifest over
/// default: the source layout, the project the sources belong to (if any),
/// and the backends to build for — one, or both under `backend = "*"`.
struct Inputs {
    layout: Layout,
    project: Option<salvo_core::Project>,
    backends: Vec<String>,
    /// [platform-host-deps] What the platform companions need of each target
    /// language: the project's declarations merged with its dependencies'.
    host: salvo_core::HostDeps,
}

/// [manifest-discovery] Finds the project for a command: the nearest
/// `salvo.toml` above `--src`, else above `--main`, else above the working
/// directory. `--src`/`--main` given on the command line win over what the
/// manifest states; the manifest fills whatever was not.
fn resolve_inputs(
    src: Option<PathBuf>,
    main_file: Option<PathBuf>,
    backend: Option<String>,
    default_backend: &str,
    tests: bool,
) -> Result<Inputs, String> {
    let start = src
        .clone()
        .or_else(|| main_file.as_ref().and_then(|m| m.parent().map(Path::to_path_buf)))
        .unwrap_or_else(|| PathBuf::from("."));
    let project = salvo_core::Project::find(&start)?;
    let (src, main_file) = match (&project, src, main_file) {
        // Neither flag: the manifest supplies both (or `main` stays open).
        (Some(p), None, None) => (Some(p.src()), p.main()),
        // `--src` alone: the manifest's `main` applies only inside that src.
        (Some(p), Some(s), None) => {
            let m = p.main().filter(|m| absolute(m).starts_with(absolute(&s)));
            (Some(s), m)
        }
        (_, s, m) => (s, m),
    };
    if src.is_none() && main_file.is_none() {
        return Err(format!(
            "no `{}` found above `{}`: pass `--src` (or `--main`), or write a manifest \
             [manifest-discovery]",
            salvo_core::MANIFEST_FILE,
            start.display()
        ));
    }
    let mut layout = layout_of(src, main_file)?;
    layout.tests = tests;
    let backends = match backend {
        Some(b) if b == "*" => vec!["rust".to_string(), "kotlin".to_string()],
        Some(b) => vec![b],
        None => project
            .as_ref()
            .and_then(|p| p.backends())
            .unwrap_or_else(|| vec![default_backend.to_string()]),
    };
    // [platform-host-deps] Merged here, so a conflict is reported before
    // anything is built and every command sees one answer.
    let host = match &project {
        Some(p) => p.host_deps()?,
        None => salvo_core::HostDeps::default(),
    };
    Ok(Inputs {
        layout,
        project,
        backends,
        host,
    })
}

/// The output directory for one backend: the flag, else — for `compile`,
/// whose output is the deliverable — the manifest's per-backend or shared
/// `target`, else `default` (the scratch directory `run`/`test` have). A
/// single `--target` cannot serve two backends.
///
/// `run` and `test` never read the manifest's `target`: they clean their
/// directory before and after, and the manifest's is where `compile` keeps
/// the checked-in output (found the hard way, 2026-09-29: a `salvo run` in an
/// example deleted its generated tree).
fn target_for(
    inputs: &Inputs,
    backend: &str,
    flag: Option<&PathBuf>,
    default: Option<&str>,
) -> Result<PathBuf, String> {
    if let Some(t) = flag {
        if inputs.backends.len() > 1 {
            // A scratch directory (`run`, `test`) is per backend, so one
            // flag names a family; a `compile` output is the deliverable
            // and cannot be two things at once.
            if default.is_some() {
                let mut per = t.clone().into_os_string();
                per.push(format!("_{backend}"));
                return Ok(PathBuf::from(per));
            }
            return Err(format!(
                "`--target {}` cannot serve both backends: drop it and give each a \
                 `target` in the manifest (`[rust]`, `[kotlin]`), or pass `--backend` \
                 [manifest]",
                t.display()
            ));
        }
        return Ok(t.clone());
    }
    if default.is_none() {
        if let Some(t) = inputs.project.as_ref().and_then(|p| p.target(backend)) {
            return Ok(t);
        }
    }
    match default {
        Some(d) if inputs.backends.len() == 1 => Ok(PathBuf::from(d)),
        Some(d) => Ok(PathBuf::from(format!("{d}_{backend}"))),
        None => Err(format!(
            "no output directory for `{backend}`: pass `--target`, or state one in the \
             manifest (`[build] target`, or `[{backend}] target`) [manifest]"
        )),
    }
}

/// [protocol-lock] Reconciles the project's lock file with the protocols this
/// build declares. Std's protocols are not the project's; a protocol with no
/// wire form has no hash and is not listed.
fn reconcile_lock(project: &salvo_core::Project, program: &Program) -> Result<(), String> {
    let symbols = salvo_core::Symbols::collect(program);
    let mut current = std::collections::BTreeMap::new();
    for unit in program.units() {
        // [manifest-deps] A dependency's protocols are locked by its own file.
        if unit.file.is_std || unit.file.dependency.is_some() {
            continue;
        }
        for item in &unit.ast.items {
            let salvo_syntax::ast::Item::Effect(e) = item else { continue };
            if !e.is_actor || !salvo_core::effect_has_wire_form(&symbols, e) {
                continue;
            }
            let canonical = salvo_core::protocol_canonical(&symbols, e);
            current.insert(e.name.name.clone(), salvo_core::protocol_hash(&canonical));
        }
    }
    match salvo_core::reconcile_lock(
        &project.lock_path(),
        project.manifest.project.version.as_deref(),
        &current,
    )? {
        salvo_core::LockOutcome::Written => {
            eprintln!("wrote {}", project.lock_path().display());
        }
        _ => {}
    }
    Ok(())
}

fn layout_of(src: Option<PathBuf>, main_file: Option<PathBuf>) -> Result<Layout, String> {
    let Some(main) = main_file else {
        // clap guarantees one of the two is present.
        let src = src.ok_or("pass `--src`, `--main`, or both")?;
        return Ok(Layout {
            src,
            main_file: None,
            tests: false,
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
                tests: false,
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
                tests: false,
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
    // [mod-file-name] A dotted stem cannot be a module at all, so it can
    // hardly declare `main`; source discovery reports it too, but saying so
    // here names the file the user actually passed.
    if name.matches('.').count() > 1 {
        return Err(format!(
            "`{name}` cannot be a module: a source file name may not contain a \
             dot — module paths come from the directory layout"
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
                // A hidden directory is a toolchain's (`.salvo_bin`, where
                // cargo keeps its own generated sources [rs-cargo]), not ours.
                if path.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with('.')) {
                    continue;
                }
                // So is a cache directory — cargo's `target/`, which the
                // `compile` hint's `cargo build --manifest-path` puts inside
                // the output tree, full of build scripts' `.rs` output — by
                // the marker source discovery already skips [mod-ignore].
                if path.join("CACHEDIR.TAG").is_file() {
                    continue;
                }
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
