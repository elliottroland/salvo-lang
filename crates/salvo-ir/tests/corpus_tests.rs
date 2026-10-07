//! [ir-build] The builder over every inline program of the two backends'
//! codegen tests (several hundred programs covering the language): every one
//! that checks must build with no error and no `Unsupported` node.

use std::path::Path;

use salvo_core::{Program, SourceSet};

fn build_program(extra: &[(&str, &str)]) -> Option<Program> {
    let std_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../std");
    let mut sources = SourceSet::default();
    let errors = sources.add_dir(&std_dir, "kt", true, false);
    assert!(errors.is_empty(), "failed to read std: {errors:?}");
    for (name, content) in extra {
        let module = SourceSet::classify(Path::new(name)).unwrap();
        sources.add(*name, module, content.to_string(), false);
    }
    let mut modules = Vec::new();
    for file in &sources.files {
        let (module, diagnostics) = salvo_syntax::parse_module_deferred(&file.content);
        if diagnostics.iter().any(|d| d.is_error()) {
            return None;
        }
        modules.push(module);
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

fn check_corpus(file: &str) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(file);
    let text = std::fs::read_to_string(&path).unwrap();
    let programs = inline_programs(&text);
    assert!(programs.len() > 50, "found only {} programs in {file}", programs.len());
    let mut failures: Vec<String> = Vec::new();
    let mut built = 0;
    for (name, body) in &programs {
        let Some(program) = build_program(&[("main.sv", body)]) else { continue };
        let symbols = salvo_core::Symbols::collect(&program);
        let resolution = salvo_core::resolve(&program);
        let checked = salvo_core::check_program(&program, &resolution, &symbols);
        if checked.errors.iter().any(|d| d.is_error()) {
            continue; // a program the test expects to be refused
        }
        let reachable = salvo_core::reachable_modules(&program, &resolution, &checked);
        let wanted: std::collections::HashSet<&salvo_core::ModulePath> =
            program.units().filter(|u| reachable.contains(&u.file.module)).map(|u| &u.file.module).collect();
        let (ir, errors) = salvo_ir::build_program(&program, &symbols, &resolution, &checked, Some(&wanted));
        built += 1;
        for e in errors {
            failures.push(format!("{name}: {e}"));
        }
        for m in &ir.modules {
            let text = salvo_ir::dump::Dumper::module(&ir, m);
            for line in text.lines().filter(|l| l.contains("UNSUPPORTED")) {
                failures.push(format!("{name} ({}): {}", m.path.0.join("."), line.trim()));
            }
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
