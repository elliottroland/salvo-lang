//! [round-trip] The round-trip check (the pipeline record B4 and D4, user decisions
//! P5 (b) and P24: the corpus and the examples). Every example and every
//! corpus program that checks clean is written back with its choices spelled
//! out (`salvo_core::round_trip`: callees, type arguments, implicits,
//! operators and interpolated parts as calls, inferred types), checked again,
//! and must check clean, make the same choices and infer the same types.

use std::path::Path;

use salvo_core::source::SourceFile;
use salvo_core::{FrontProgram, Program, Symbols};

/// Checks `built`, writes its user files out, re-checks them, and answers
/// what differed (empty when the round trip holds) and how many calls were
/// written out, and how many types. `None` when the program does not check
/// clean to begin with.
fn round_trip(built: FrontProgram) -> Option<(Vec<String>, usize, usize)> {
    let front = salvo_testkit::front::std_front("kt");
    if built.diagnostics.iter().any(|d| d.is_error()) {
        return None;
    }
    let program: Program = built.program;
    let symbols = Symbols::collect(&program);
    let resolution = salvo_core::resolve(&program);
    let checked = front.check(&program, &resolution, &symbols);
    if checked.errors.iter().any(|d| d.is_error()) {
        return None;
    }
    let mut files: Vec<SourceFile> = Vec::new();
    let mut rewrites = Vec::new();
    for (i, f) in program.files.iter().enumerate() {
        if f.is_std {
            continue;
        }
        let rewrite = salvo_core::round_trip::write_out(&program, &symbols, &checked, i);
        let mut file = f.clone();
        file.content = rewrite.text.clone();
        files.push(file);
        rewrites.push((f.name.clone(), rewrite));
    }
    let again = front.program(files, program.companions.clone());
    let mut out = Vec::new();
    for d in again.diagnostics.iter().filter(|d| d.is_error()) {
        out.push(format!("written out, it does not parse: {}", d.render(&again.program.files)));
    }
    let calls = rewrites.iter().map(|(_, r)| r.calls.values().map(Vec::len).sum::<usize>()).sum();
    let types = rewrites.iter().map(|(_, r)| r.types.len()).sum();
    if !out.is_empty() {
        return Some((out, calls, types));
    }
    let program2 = again.program;
    let symbols2 = Symbols::collect(&program2);
    let resolution2 = salvo_core::resolve(&program2);
    let checked2 = front.check(&program2, &resolution2, &symbols2);
    for d in checked2.errors.iter().filter(|d| d.is_error()) {
        out.push(format!("written out, it does not check: {}", d.render(&program2.files)));
    }
    for (name, rewrite) in &rewrites {
        let Some(i) = program2.files.iter().position(|f| !f.is_std && f.name == *name) else { continue };
        for diff in salvo_core::round_trip::compare(&program2, &checked2, i, rewrite) {
            out.push(format!("{name}: {diff}"));
        }
    }
    if !out.is_empty() {
        let texts: Vec<String> = rewrites.iter().map(|(n, r)| format!("--- {n}\n{}", r.text)).collect();
        out.push(format!("the written-out sources:\n{}", texts.join("\n")));
    }
    Some((out, calls, types))
}

#[test]
fn every_example_round_trips() {
    let examples = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
    let mut dirs: Vec<_> = std::fs::read_dir(&examples)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        // Examples with dependencies need the CLI's loader.
        .filter(|p| p.join("salvo.toml").exists() && !std::fs::read_to_string(p.join("salvo.toml")).unwrap_or_default().contains("[dependencies]"))
        .collect();
    dirs.sort();
    let mut failures = Vec::new();
    let mut written = 0;
    let mut typed = 0;
    for dir in dirs {
        let name = dir.file_name().unwrap().to_string_lossy().to_string();
        match round_trip(salvo_testkit::front::project("kt", &dir)) {
            None => failures.push(format!("{name}: does not check clean")),
            Some((diffs, _, _)) if !diffs.is_empty() => failures.push(format!("{name}:\n  {}", diffs.join("\n  "))),
            Some((_, calls, types)) => {
                written += calls;
                typed += types;
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
    eprintln!("[round-trip] {written} example calls and {typed} types round-tripped");
    // Floors well under today's counts (calls 1,152 with operators and
    // interpolation, types 165, 2026-10-30), so a writer that silently
    // stops spelling things out cannot pass vacuously.
    assert!(written >= FLOOR, "only {written} calls were written out");
    assert!(typed >= TYPE_FLOOR, "only {typed} types were written out");
}

const FLOOR: usize = 900;
const TYPE_FLOOR: usize = 100;

#[test]
fn every_corpus_program_round_trips() {
    let corpus = Path::new(env!("CARGO_MANIFEST_DIR")).join("../salvo-syntax/tests/corpus");
    let mut paths: Vec<_> = std::fs::read_dir(&corpus).unwrap().filter_map(|e| e.ok()).map(|e| e.path()).collect();
    paths.sort();
    let mut failures = Vec::new();
    let mut checked = 0;
    for path in paths {
        let text = std::fs::read_to_string(&path).unwrap();
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        match round_trip(salvo_testkit::front::program("kt", &[("main.sv", &text)])) {
            None => {}
            Some((diffs, _, _)) if !diffs.is_empty() => failures.push(format!("{name}:\n  {}", diffs.join("\n  "))),
            Some(_) => checked += 1,
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
    eprintln!("[round-trip] {checked} corpus programs round-tripped");
}
