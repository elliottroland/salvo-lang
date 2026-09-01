//! Structured-diagnostic tests [diag-structured]: resolver and checker
//! errors carry a file index, span, and severity instead of pre-rendered
//! strings, and render at the boundary.

use std::path::Path;

use salvo_core::{check_program, resolve, Checked, Program, SourceSet, Symbols};

/// Parses + resolves + checks a multi-file program (no std). Each entry is
/// `(file_name, source)`.
fn check_files(files: &[(&str, &str)]) -> (Program, Checked) {
    let mut sources = SourceSet::default();
    for (name, src) in files {
        let (module, kind) = SourceSet::classify(Path::new(name), "kotlin").unwrap();
        sources.add(*name, module, kind, src.to_string(), false);
    }
    let mut modules = Vec::new();
    for file in &sources.files {
        let (module, diagnostics) = salvo_syntax::parse_module(&file.content);
        let errors: Vec<_> = diagnostics.iter().filter(|d| d.is_error()).collect();
        assert!(errors.is_empty(), "parse errors: {errors:?}");
        modules.push(module);
    }
    let program = Program {
        files: sources.files,
        modules,
        companions: Vec::new(),
    };
    let symbols = Symbols::collect(&program);
    let resolution = resolve(&program);
    let checked = check_program(&program, &resolution, &symbols);
    (program, checked)
}

// [diag-structured] Checker errors are attributed to the declaring file
// with a nonempty span, and render with file:line:col + caret.
#[test]
fn checker_errors_carry_file_and_span() {
    let src = "fn broken() -> Int {\n    let x: Int = \"hello\"\n    return x\n}\n";
    let (program, checked) = check_files(&[("main.sv", src)]);
    assert_eq!(checked.errors.len(), 1, "errors: {:?}", checked.errors);
    let diag = &checked.errors[0];
    assert_eq!(diag.file, 0);
    assert!(diag.is_error());
    assert!(diag.message.contains("expected `Int`, found `Str`"));
    // The span points at the string literal on line 2.
    assert_eq!(
        &src[diag.span.start as usize..diag.span.end as usize],
        "\"hello\""
    );
    let rendered = diag.render(&program.files);
    assert!(
        rendered.contains("main.sv:2:18"),
        "unexpected rendering: {rendered}"
    );
    assert!(rendered.contains('^'), "no caret line: {rendered}");
}

// [diag-structured] Errors from different files of one program index the
// right file; resolution errors flow into `Checked::errors` structured.
#[test]
fn errors_index_the_declaring_file() {
    let ok = "fn fine() -> Int {\n    return 1\n}\n";
    let bad = "import nope.thing\n";
    let (program, checked) = check_files(&[("a.sv", ok), ("b.sv", bad)]);
    assert_eq!(checked.errors.len(), 1, "errors: {:?}", checked.errors);
    let diag = &checked.errors[0];
    assert_eq!(diag.file, 1);
    assert!(diag.message.contains("unresolved import"));
    assert!(
        diag.render(&program.files).contains("b.sv:1:1"),
        "unexpected rendering: {}",
        diag.render(&program.files)
    );
}
