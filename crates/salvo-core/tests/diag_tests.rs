//! Structured-diagnostic tests [diag-structured]: resolver and checker
//! errors carry a file index, span, and severity instead of pre-rendered
//! strings, and render at the boundary. Which diagnostics a program
//! reports is test data, `tests/check/diagnostics/` (TESTS.md T1); this
//! reads what annotations cannot: the span itself and the rendering.

// [diag-structured] Checker errors are attributed to the declaring file
// with a nonempty span, and render with file:line:col + caret.
#[test]
fn checker_errors_carry_file_and_span() {
    let src = "fn broken() -> Int {\n    let x: Int = \"hello\"\n    return x\n}\n";
    let out = salvo_testkit::front::check_main(src);
    let main = out.program.files.iter().position(|f| f.name == "main.sv").expect("main.sv");
    let diags: Vec<_> = out.diagnostics().collect();
    assert_eq!(diags.len(), 1, "diagnostics: {diags:?}");
    let diag = diags[0];
    assert_eq!(diag.file, main);
    assert!(diag.is_error());
    assert!(diag.message.contains("expected `Int`, found `Str`"));
    // The span points at the string literal on line 2.
    assert_eq!(&src[diag.span.start as usize..diag.span.end as usize], "\"hello\"");
    let rendered = diag.render(&out.program.files);
    assert!(rendered.contains("main.sv:2:18"), "unexpected rendering: {rendered}");
    assert!(rendered.contains('^'), "no caret line: {rendered}");
}
