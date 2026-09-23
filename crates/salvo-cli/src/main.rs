//! The `salvo` CLI.
//!
//! ```text
//! salvo compile --backend kotlin --src ./some_dir --target ./some_dir_kotlin
//! salvo run --backend kotlin --main ./some_dir/main.sv
//! salvo run --backend rust --src ./some_dir --main ./some_dir/bin/tool.sv
//! salvo run --backend rust --src ./some_dir --target ./out --clean-target before
//! salvo analyze --src ./some_dir [--format json]
//! salvo test --src ./some_dir [--backend rust] [FILTER] [--list]
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
    /// Run the tests a source tree declares [test-run]: every
    /// `test "name" { … }` block in a `<module>.test.sv` annex.
    Test {
        /// Target backend (defaults to `rust`, whose toolchain is the
        /// cheapest to start).
        #[arg(long, default_value = "rust")]
        backend: String,
        /// Directory containing `.sv` source files, annexes included.
        #[arg(long)]
        src: PathBuf,
        /// Run only tests whose id contains this text — `module :: name`,
        /// so one word selects a module, a test, or a family [test-filter].
        filter: Option<String>,
        /// List the tests that would run, and run nothing.
        #[arg(long)]
        list: bool,
        /// Output directory for the generated sources (default:
        /// `.salvo_tmp_test`). Cleared before the run and left in place
        /// afterwards, so a failing harness can be read.
        #[arg(long)]
        target: Option<PathBuf>,
    },
    /// Parse, resolve, and type-check sources without generating code
    /// [cli-analyze].
    Analyze {
        /// Directory containing `.sv` source files.
        #[arg(long)]
        src: PathBuf,
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
    /// Work with the host side of `platform effect` and `platform handler`
    /// declarations [cli-platform].
    Platform {
        #[command(subcommand)]
        command: PlatformCommand,
    },
}

#[derive(Subcommand)]
enum PlatformCommand {
    /// Write the host implementation skeleton for every `platform effect`
    /// and `platform handler` into the source root's `platform/` tree
    /// [platform-tree] [cli-platform].
    ///
    /// Existing files are never touched: the skeleton is generated once and
    /// belongs to you afterwards, and every later divergence from the
    /// generated interface is a target-language compile error rather than
    /// something the compiler has to merge.
    Generate {
        /// Target backend, which decides the language of the skeleton.
        #[arg(long)]
        backend: String,
        /// Directory containing `.sv` source files — also where the
        /// `platform/` tree is written.
        #[arg(long, required_unless_present = "main_file")]
        src: Option<PathBuf>,
        /// The `.sv` file declaring `main`, as for `salvo run`: it picks
        /// between several entry points, and on its own implies its own
        /// directory as the source directory.
        #[arg(long = "main", required_unless_present = "src")]
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
        } => compile(&backend, &src, &target, emit_ast.as_deref()),
        Command::Run {
            backend,
            src,
            main_file,
            target,
            clean_target,
        } => run(&backend, src, main_file, target, clean_target),
        Command::Analyze { src, format } => analyze(&src, format),
        Command::Test {
            backend,
            src,
            filter,
            list,
            target,
        } => test(&backend, &src, filter.as_deref(), list, target),
        Command::Lsp => lsp::run(),
        Command::Lang { command } => match command {
            LangCommand::TmGrammar { out } => lang::run_tm_grammar(out.as_ref()),
        },
        Command::Platform { command } => match command {
            PlatformCommand::Generate {
                backend,
                src,
                main_file,
            } => platform_generate(&backend, src, main_file),
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
fn analyze(src: &PathBuf, format: Format) -> ExitCode {
    let analysis = match analysis::analyze_sources(src, "", &Default::default()) {
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
    let io_errors = sources.add_dir(&layout.src, backend.file_extension(), false, layout.tests);
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
            !file.is_std
                && !file.is_test
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
) -> Result<Option<Built>, ExitCode> {
    let Some(Assembled {
        program,
        main_module,
        ambiguous,
        tests: _,
    }) = assemble(backend, layout, emit_ast, verbose)?
    else {
        return Ok(None);
    };
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
    // nobody sees is the same as no warning [qual-refn-conflict].
    for msg in &emitted.warnings {
        eprintln!("{msg}");
    }
    let written = emitted.files;
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
        tests: false,
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
    backend_name: &str,
    src: &Path,
    filter: Option<&str>,
    list: bool,
    target: Option<PathBuf>,
) -> ExitCode {
    let registry = registry();
    let Some(backend) = registry.get(backend_name) else {
        eprintln!("{}", unknown_backend(&registry, backend_name));
        return ExitCode::FAILURE;
    };
    let layout = Layout {
        src: src.to_path_buf(),
        main_file: None,
        tests: true,
    };
    let Some(mut assembled) = (match assemble(backend, &layout, None, false) {
        Ok(assembled) => assembled,
        Err(code) => return code,
    }) else {
        return ExitCode::SUCCESS;
    };

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
            src.display()
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
            src.display(),
            harness_module
        );
        return ExitCode::FAILURE;
    }
    let target = target.unwrap_or_else(|| PathBuf::from(DEFAULT_TEST_TARGET));
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
    let mut remaining: Vec<salvo_core::TestCase> = selected.clone();
    let mut total = salvo_test::Summary::default();
    // One pass per death, plus the first: a pass always either finishes or
    // removes one test from `remaining`, so this cannot spin.
    let mut passes_left = selected.len() + 1;
    while !remaining.is_empty() && passes_left > 0 {
        passes_left -= 1;
        let pass = match run_test_pass(
            backend,
            &mut assembled.program,
            &harness_module,
            &remaining,
            &target,
            color,
        ) {
            Ok(pass) => pass,
            Err(code) => return code,
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
                    return ExitCode::FAILURE;
                }
                remaining.clear();
            }
        }
    }

    let mut out = std::io::stdout().lock();
    let _ = salvo_test::print_summary(&total, &mut out, color);
    drop(out);
    if total.ok() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
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
        // [test-implicit-import] The harness *is* test code: marking it so gives
        // it `std.test` without an import line, which is also what keeps it
        // clear of an `import <module>.test` for an annex of `test` itself.
        is_test: true,
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

    let mut command = match backend.program_command(target, harness_module, &emitted.files) {
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

/// `salvo platform generate` [cli-platform]: writes the host implementation
/// skeleton for every `platform effect` into `<src>/platform/`
/// [platform-tree].
///
/// **Never overwrites.** With an interface between Salvo and the host, the
/// file only has to be right once: afterwards every kind of drift — a member
/// added, removed, or re-signed, a new platform effect — is an error from the
/// *target* compiler, so there is nothing for this command to merge and no
/// reason for it to touch code a human has edited.
fn platform_generate(
    backend_name: &str,
    src: Option<PathBuf>,
    main_file: Option<PathBuf>,
) -> ExitCode {
    let registry = registry();
    let Some(backend) = registry.get(backend_name) else {
        eprintln!("{}", unknown_backend(&registry, backend_name));
        return ExitCode::FAILURE;
    };
    let layout = match layout_of(src, main_file) {
        Ok(layout) => layout,
        Err(msg) => {
            eprintln!("error: {msg}");
            return ExitCode::FAILURE;
        }
    };
    let Some(assembled) = (match assemble(backend, &layout, None, false) {
        Ok(assembled) => assembled,
        Err(code) => return code,
    }) else {
        return ExitCode::SUCCESS;
    };

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
    if skeletons.is_empty() {
        eprintln!(
            "no `platform effect` or `platform handler` declarations in `{}`: \
             nothing to generate",
            layout.src.display()
        );
        return ExitCode::SUCCESS;
    }

    let mut written = 0usize;
    let mut kept = 0usize;
    for (rel_path, content) in skeletons {
        let path = layout.src.join(&rel_path);
        if path.exists() {
            eprintln!("kept {} (already exists)", path.display());
            kept += 1;
            continue;
        }
        if let Some(parent) = path.parent() {
            if let Err(err) = std::fs::create_dir_all(parent) {
                eprintln!("error: failed to create `{}`: {err}", parent.display());
                return ExitCode::FAILURE;
            }
        }
        if let Err(err) = std::fs::write(&path, &content) {
            eprintln!("error: failed to write `{}`: {err}", path.display());
            return ExitCode::FAILURE;
        }
        eprintln!("wrote {}", path.display());
        written += 1;
    }
    eprintln!(
        "generated {written} host file(s){}; implement the stubbed members, then \
         `salvo run --backend {backend_name}`",
        if kept == 0 {
            String::new()
        } else {
            format!(", left {kept} existing file(s) alone")
        }
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
