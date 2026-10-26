//! [ir-build] The builder over every inline program of the two backends'
//! codegen tests (several hundred programs covering the language): every one
//! that checks must build with no error and no `Unsupported` node.
//!
//! Each program is a whole check of std plus the program (~0.25s), and the
//! corpus holds ~400 of them. Two things keep that off the default run:
//!
//! - **Stamps** (`salvo-testkit`). A program's outcome is a pure function of
//!   its text, std's files and the compiler — and the compiler is this test
//!   binary (salvo-core and salvo-syntax are linked into it), so the key is
//!   the program, a digest of std, and the binary's fingerprint. A run with
//!   nothing changed checks nothing; a compiler rebuild or a std edit
//!   re-checks every program; a new or edited codegen test re-checks only
//!   that program. `SALVO_E2E_FRESH=1` ignores the stamps. These are not
//!   toolchain tests, so `SALVO_SKIP_E2E` does not skip them.
//! - **A pool of threads** over the programs that missed, and std read and
//!   parsed once: a parse is a pure function of the text, and the parsed
//!   modules are shared the way every driver shares them (`Arc`s an
//!   expansion copies before it rewrites one).

use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock};

use salvo_core::source::{CompanionFile, SourceFile};
use salvo_core::{Program, SourceSet};
use salvo_syntax::ast::Module;

/// std, read and parsed once per test binary.
struct Std {
    files: Vec<SourceFile>,
    companions: Vec<CompanionFile>,
    modules: Vec<Arc<Module>>,
    /// Whether some std file failed to parse (every program is then
    /// skipped, as the per-program parse used to skip it).
    parse_error: bool,
    /// A digest of every file std's `SourceSet` holds, for the stamp keys.
    print: String,
}

fn std_front() -> &'static Std {
    static STD: OnceLock<Std> = OnceLock::new();
    STD.get_or_init(|| {
        let std_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../lib");
        let mut sources = SourceSet::default();
        let errors = sources.add_dir(&std_dir, "kt", true, false);
        assert!(errors.is_empty(), "failed to read std: {errors:?}");
        let mut parse_error = false;
        let mut modules = Vec::new();
        for file in &sources.files {
            let (module, diagnostics) = salvo_syntax::parse_module_deferred(&file.content);
            parse_error |= diagnostics.iter().any(|d| d.is_error());
            modules.push(module);
        }
        let mut parts: Vec<Vec<u8>> = Vec::new();
        for f in &sources.files {
            parts.push(f.name.as_bytes().to_vec());
            parts.push(f.module.0.join(".").into_bytes());
            parts.push(f.content.as_bytes().to_vec());
            parts.push(vec![f.is_std as u8, f.is_shadow as u8, f.is_test as u8]);
        }
        for c in &sources.companions {
            parts.push(c.rel_path.to_string_lossy().as_bytes().to_vec());
            parts.push(c.module.0.join(".").into_bytes());
            parts.push(c.content.as_bytes().to_vec());
        }
        let refs: Vec<&[u8]> = parts.iter().map(|p| p.as_slice()).collect();
        let print = salvo_testkit::fingerprint(&refs);
        Std {
            files: sources.files,
            companions: sources.companions,
            modules: Program::share(modules),
            parse_error,
            print,
        }
    })
}

fn build_program(extra: &[(&str, &str)]) -> Option<Program> {
    let std = std_front();
    if std.parse_error {
        return None;
    }
    let mut sources = SourceSet { files: std.files.clone(), companions: std.companions.clone(), ..Default::default() };
    let mut modules = std.modules.clone();
    for (name, content) in extra {
        let module = SourceSet::classify(Path::new(name)).unwrap();
        sources.add(*name, module, content.to_string(), false);
        let (parsed, diagnostics) = salvo_syntax::parse_module_deferred(content);
        if diagnostics.iter().any(|d| d.is_error()) {
            return None;
        }
        modules.push(Arc::new(parsed));
    }
    let expansion = salvo_core::expand(&sources.files, &mut modules);
    if expansion.diagnostics.iter().any(|d| d.is_error()) {
        return None;
    }
    Some(Program { files: sources.files, modules, companions: sources.companions })
}

/// Every `const NAME: &str = r#"…"#;` in a test file.
fn inline_programs(text: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut at = 0;
    while let Some(i) = text[at..].find("const ") {
        let start = at + i;
        let Some(colon) = text[start..].find(": &str = r#\"") else { break };
        let name = text[start + 6..start + colon].trim().to_string();
        let body_start = start + colon + ": &str = r#\"".len();
        let Some(end_rel) = text[body_start..].find("\"#") else { break };
        let body = text[body_start..body_start + end_rel].to_string();
        out.push((name, body));
        at = body_start + end_rel;
    }
    out
}

/// What one program came to.
enum Outcome {
    /// It did not parse, expand or check: a program its test expects to be
    /// refused.
    Refused,
    /// It built, with these failures (none, for a pass).
    Built(Vec<String>),
}

fn build_one(name: &str, body: &str) -> Outcome {
    let Some(program) = build_program(&[("main.sv", body)]) else { return Outcome::Refused };
    let symbols = salvo_core::Symbols::collect(&program);
    let resolution = salvo_core::resolve(&program);
    let checked = salvo_core::check_program(&program, &resolution, &symbols);
    if checked.errors.iter().any(|d| d.is_error()) {
        return Outcome::Refused;
    }
    let reachable = salvo_core::reachable_modules(&program, &resolution, &checked);
    let wanted: std::collections::HashSet<&salvo_core::ModulePath> =
        program.units().filter(|u| reachable.contains(&u.file.module)).map(|u| &u.file.module).collect();
    let (ir, errors) = salvo_core::ir::build_program(&program, &symbols, &resolution, &checked, Some(&wanted));
    let mut failures: Vec<String> = Vec::new();
    for e in errors {
        failures.push(format!("{name}: {e}"));
    }
    for m in &ir.modules {
        let text = salvo_core::ir::dump::Dumper::module(&ir, m);
        for line in text.lines().filter(|l| l.contains("UNSUPPORTED")) {
            failures.push(format!("{name} ({}): {}", m.path.0.join("."), line.trim()));
        }
    }
    Outcome::Built(failures)
}

/// The stamp for `body` having come to `outcome` ("built" or "refused"),
/// or `None` when one is already on disk.
fn stamp(name: &str, body: &str, outcome: &str) -> Option<salvo_testkit::Stamp> {
    let std = std_front();
    let what = format!("corpus ir-build {outcome}: {name}");
    let compiler = salvo_testkit::self_fingerprint();
    let parts: [&[u8]; 5] = [
        b"corpus-ir-build",
        outcome.as_bytes(),
        compiler.as_bytes(),
        std.print.as_bytes(),
        body.as_bytes(),
    ];
    salvo_testkit::cached(env!("CARGO_TARGET_TMPDIR"), &what, &parts)
}

fn check_corpus(file: &str) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(file);
    let text = std::fs::read_to_string(&path).unwrap();
    let programs = inline_programs(&text);
    assert!(programs.len() > 50, "found only {} programs in {file}", programs.len());
    // A stamp records the outcome in its key, so a stamped program still
    // counts towards `built` below.
    let mut built = 0;
    let mut todo: Vec<(&str, &str, salvo_testkit::Stamp, salvo_testkit::Stamp)> = Vec::new();
    for (name, body) in &programs {
        let Some(built_stamp) = stamp(name, body, "built") else {
            built += 1;
            continue;
        };
        let Some(refused_stamp) = stamp(name, body, "refused") else { continue };
        todo.push((name, body, built_stamp, refused_stamp));
    }
    let workers = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);
    let next = std::sync::atomic::AtomicUsize::new(0);
    let count = todo.len();
    let todo = Mutex::new(todo.into_iter().map(Some).collect::<Vec<_>>());
    let results: Mutex<Vec<(usize, Outcome)>> = Mutex::new(Vec::with_capacity(count));
    std::thread::scope(|scope| {
        for _ in 0..workers.min(count) {
            scope.spawn(|| loop {
                let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                if i >= count {
                    return;
                }
                let (name, body, built_stamp, refused_stamp) = todo.lock().unwrap()[i].take().unwrap();
                let outcome = build_one(name, body);
                // Each outcome is a function of its own key, so it is
                // remembered whatever the other programs came to; a
                // program with failures is not remembered at all.
                match &outcome {
                    Outcome::Refused => refused_stamp.verified(),
                    Outcome::Built(failures) if failures.is_empty() => built_stamp.verified(),
                    Outcome::Built(_) => {}
                }
                results.lock().unwrap().push((i, outcome));
            });
        }
    });
    let mut results = results.into_inner().unwrap();
    results.sort_by_key(|(i, _)| *i);
    let mut failures: Vec<String> = Vec::new();
    for (_, outcome) in results {
        if let Outcome::Built(found) = outcome {
            built += 1;
            failures.extend(found);
        }
    }
    assert!(built > 30, "built only {built} programs from {file}");
    assert!(failures.is_empty(), "{} failures over {built} programs:\n{}", failures.len(), failures.join("\n"));
}

#[test]
fn every_kotlin_codegen_program_builds() {
    check_corpus("../salvo-backend-kotlin/tests/codegen_tests.rs");
}

#[test]
fn every_rust_codegen_program_builds() {
    check_corpus("../salvo-backend-rust/tests/codegen_tests.rs");
}
