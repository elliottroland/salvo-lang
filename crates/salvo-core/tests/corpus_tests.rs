//! [ir-build] The builder over every inline program of the two backends'
//! codegen tests, every program of the compile-and-run data
//! ([test-run-data], `salvo-cli/tests/run/`) and of the backends' emit data
//! ([test-emit-data], `salvo-backend-*/tests/emit/`) — several hundred programs
//! covering the language: every one that checks must build with no error
//! and no `Unsupported` node.
//!
//! The corpus holds ~400 programs. Three things keep them cheap:
//!
//! - **Stamps** (`salvo-testkit`). A program's outcome is a pure function of
//!   its text, std's files and the compiler — and the compiler is this test
//!   binary (salvo-core and salvo-syntax are linked into it), so the key is
//!   the program, a digest of std, and the binary's fingerprint. A run with
//!   nothing changed checks nothing; a compiler rebuild or a std edit
//!   re-checks every program; a new or edited codegen test re-checks only
//!   that program. `SALVO_E2E_FRESH=1` ignores the stamps. These are not
//!   toolchain tests, so `SALVO_SKIP_E2E` does not skip them.
//! - **A pool of threads** over the programs that missed.
//! - [std-cache] **std checked once**: every program is built over the
//!   process's std front (`salvo_testkit::front`, TESTS.md C2), so only the
//!   program itself is parsed, expanded and checked.

use std::path::Path;
use std::sync::{Mutex, OnceLock};

use salvo_core::Program;

/// What the stamps know of std: whether it parsed, and a digest of every
/// file its front was built from.
struct Std {
    /// Whether some std file failed to parse or expand (every program is
    /// then skipped, as the per-program parse used to skip it).
    parse_error: bool,
    /// A digest of every file std's `SourceSet` holds, for the stamp keys.
    print: String,
}

fn front() -> &'static salvo_core::StdFront {
    salvo_testkit::front::std_front("kt")
}

fn std_front() -> &'static Std {
    static STD: OnceLock<Std> = OnceLock::new();
    STD.get_or_init(|| {
        let front = front();
        let parse_error = front.parse_diagnostics().iter().any(|d| d.is_error())
            || front.program(Vec::new(), Vec::new()).diagnostics.iter().any(|d| d.is_error());
        let mut parts: Vec<Vec<u8>> = Vec::new();
        for f in front.files() {
            parts.push(f.name.as_bytes().to_vec());
            parts.push(f.module.0.join(".").into_bytes());
            parts.push(f.content.as_bytes().to_vec());
            parts.push(vec![f.is_std as u8, f.is_shadow as u8, f.is_test as u8]);
        }
        for c in front.companions() {
            parts.push(c.rel_path.to_string_lossy().as_bytes().to_vec());
            parts.push(c.module.0.join(".").into_bytes());
            parts.push(c.content.as_bytes().to_vec());
        }
        let refs: Vec<&[u8]> = parts.iter().map(|p| p.as_slice()).collect();
        let print = salvo_testkit::fingerprint(&refs);
        Std { parse_error, print }
    })
}

fn build_program(extra: &[(&str, &str)]) -> Option<Program> {
    if std_front().parse_error {
        return None;
    }
    let built = salvo_testkit::front::program("kt", extra);
    if built.diagnostics.iter().any(|d| d.is_error()) {
        return None;
    }
    Some(built.program)
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

fn build_one(name: &str, files: &[(&str, &str)]) -> Outcome {
    let Some(program) = build_program(files) else { return Outcome::Refused };
    let symbols = salvo_core::Symbols::collect(&program);
    let resolution = salvo_core::resolve(&program);
    let checked = front().check(&program, &resolution, &symbols);
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
fn stamp(name: &str, files: &[(&str, &str)], outcome: &str) -> Option<salvo_testkit::Stamp> {
    let std = std_front();
    let what = format!("corpus ir-build {outcome}: {name}");
    let compiler = salvo_testkit::self_fingerprint();
    let mut parts: Vec<&[u8]> = vec![b"corpus-ir-build", outcome.as_bytes(), compiler.as_bytes(), std.print.as_bytes()];
    // One file named `main.sv` keys as it did before files had names.
    if let [("main.sv", body)] = files {
        parts.push(body.as_bytes());
    } else {
        for (n, c) in files {
            parts.push(n.as_bytes());
            parts.push(c.as_bytes());
        }
    }
    salvo_testkit::cached(env!("CARGO_TARGET_TMPDIR"), &what, &parts)
}

fn check_corpus(file: &str) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(file);
    let text = std::fs::read_to_string(&path).unwrap();
    let programs: Vec<(String, Vec<(String, String)>)> =
        inline_programs(&text).into_iter().map(|(name, body)| (name, vec![("main.sv".to_string(), body)])).collect();
    // Most programs now live in the run and emit data; the codegen files
    // keep the few whose tests read emitted text a data file cannot.
    check_programs(file, programs, 5);
}

/// [test-run-data] Every program of the compile-and-run data
/// (`salvo-cli/tests/run/<area>/*.sv`), its `.sv` files alone.
fn run_data_programs() -> Vec<(String, Vec<(String, String)>)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../salvo-cli/tests/run");
    let mut out = Vec::new();
    let mut areas: Vec<_> = std::fs::read_dir(&root).unwrap().filter_map(|e| e.ok()).map(|e| e.path()).filter(|p| p.is_dir()).collect();
    areas.sort();
    for area in areas {
        let mut files: Vec<_> = std::fs::read_dir(&area).unwrap().filter_map(|e| e.ok()).map(|e| e.path()).filter(|p| p.extension().is_some_and(|e| e == "sv")).collect();
        files.sort();
        for file in files {
            let text = std::fs::read_to_string(&file).unwrap();
            let cases = salvo_testkit::run::parse(&text).unwrap_or_else(|e| panic!("{}: {e}", file.display()));
            for case in cases {
                let name = format!("{}#{}", file.display(), case.name);
                let sources = case.sources().into_iter().map(|(n, c)| (n.to_string(), c.to_string())).collect();
                out.push((name, sources));
            }
        }
    }
    out
}

/// [test-emit-data] Every program of a backend's emit data
/// (`<backend crate>/tests/emit/<area>/*.sv`) that is not refused.
fn emit_data_programs(backend: &str) -> Vec<(String, Vec<(String, String)>)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../salvo-backend-{backend}/tests/emit"));
    let mut out = Vec::new();
    let mut areas: Vec<_> = std::fs::read_dir(&root).unwrap().filter_map(|e| e.ok()).map(|e| e.path()).filter(|p| p.is_dir()).collect();
    areas.sort();
    for area in areas {
        let mut files: Vec<_> = std::fs::read_dir(&area).unwrap().filter_map(|e| e.ok()).map(|e| e.path()).filter(|p| p.extension().is_some_and(|e| e == "sv")).collect();
        files.sort();
        for file in files {
            let text = std::fs::read_to_string(&file).unwrap();
            let cases = salvo_testkit::data::parse(&text).unwrap_or_else(|e| panic!("{}: {e}", file.display()));
            for case in cases.into_iter().filter(|c| c.refused.is_empty()) {
                out.push((format!("{}#{}", file.display(), case.name), case.files));
            }
        }
    }
    out
}

fn check_programs(file: &str, programs: Vec<(String, Vec<(String, String)>)>, at_least: usize) {
    assert!(programs.len() > at_least, "found only {} programs in {file}", programs.len());
    // A stamp records the outcome in its key, so a stamped program still
    // counts towards `built` below.
    let mut built = 0;
    let programs: Vec<(&str, Vec<(&str, &str)>)> =
        programs.iter().map(|(n, fs)| (n.as_str(), fs.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect())).collect();
    let mut todo: Vec<(&str, &[(&str, &str)], salvo_testkit::Stamp, salvo_testkit::Stamp)> = Vec::new();
    for (name, files) in &programs {
        let Some(built_stamp) = stamp(name, files, "built") else {
            built += 1;
            continue;
        };
        let Some(refused_stamp) = stamp(name, files, "refused") else { continue };
        todo.push((name, files, built_stamp, refused_stamp));
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
                let (name, files, built_stamp, refused_stamp) = todo.lock().unwrap()[i].take().unwrap();
                let outcome = build_one(name, files);
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
    assert!(built > at_least.min(30), "built only {built} programs from {file}");
    assert!(failures.is_empty(), "{} failures over {built} programs:\n{}", failures.len(), failures.join("\n"));
}

#[test]
fn every_kotlin_codegen_program_builds() {
    check_corpus("../salvo-backend-kotlin/tests/codegen_tests.rs");
}

#[test]
fn every_emit_data_program_builds() {
    let mut programs = emit_data_programs("rust");
    programs.extend(emit_data_programs("kotlin"));
    check_programs("salvo-backend-*/tests/emit", programs, 0);
}

#[test]
fn every_run_data_program_builds() {
    check_programs("salvo-cli/tests/run", run_data_programs(), 0);
}

#[test]
fn every_rust_codegen_program_builds() {
    check_corpus("../salvo-backend-rust/tests/codegen_tests.rs");
}
