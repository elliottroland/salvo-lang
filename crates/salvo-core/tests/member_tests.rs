//! Member-resolution rules [call-resolve] [field-resolve] [index-resolve]
//! [iter-resolve]: Salvo assumes it can see everything, so a call, field
//! read, subscript, or `for` over something the compiler cannot justify is
//! an error. Reaching a target-language feature means *declaring* it
//! (`external fn` and friends) — user decision 2026-09-03, which replaced
//! the interop pass-through that used to make these silent.
//!
//! The narrow leniency that remains is [type-unknown-lenient]: a type the
//! checker could not *infer* (`Ty::Unknown`) still passes through, so one
//! mistake never cascades.

use std::path::Path;

use salvo_core::{check_program, resolve, FileDiagnostic, Program, SourceSet, Symbols};

fn check_errors(src: &str) -> Vec<FileDiagnostic> {
    let mut sources = SourceSet::default();
    let (module, kind) = SourceSet::classify(Path::new("main.sv"), "kotlin").unwrap();
    sources.add("main.sv", module, kind, src.to_string(), false);
    let (ast, diagnostics) = salvo_syntax::parse_module(&sources.files[0].content);
    let parse_errors: Vec<_> = diagnostics.iter().filter(|d| d.is_error()).collect();
    assert!(parse_errors.is_empty(), "parse errors: {parse_errors:?}");
    let program = Program {
        files: sources.files,
        modules: vec![ast],
        companions: Vec::new(),
    };
    let symbols = Symbols::collect(&program);
    let resolution = resolve(&program);
    let mut errors: Vec<FileDiagnostic> = resolution.errors.clone();
    errors.extend(check_program(&program, &resolution, &symbols).errors);
    errors
}

fn messages(src: &str) -> Vec<String> {
    check_errors(src)
        .iter()
        .map(|d| d.message.clone())
        .collect()
}

const PRELUDE: &str = r#"
internal type Int
internal type Str
internal type Bool
internal type Opaque

struct Person {
    name: Str
}

external fn shout(s: Str) [] -> [s] Str
"#;

fn body(src: &str) -> String {
    format!("{PRELUDE}\nfn probe(p: Person) -> [p] Int {{\n{src}\n    return 1\n}}\n")
}

// ===== calls [call-resolve] =====

/// [call-resolve] A name nothing declares is not a call into the target
/// language any more — it is a mistake.
#[test]
fn unresolved_call_is_an_error() {
    let errs = messages(&body("    nowhere()"));
    assert!(
        errs.iter()
            .any(|m| m == "no function named `nowhere` is in scope"),
        "got {errs:?}"
    );
}

/// [call-resolve] [diag-import-suggest] Unresolved calls carry import
/// suggestions, like every other unresolved name.
#[test]
fn unresolved_call_suggests_imports() {
    let lib = "fn helper() -> Int {\n    return 1\n}\n";
    let main = "fn f() -> Int {\n    return helper()\n}\n";
    let mut sources = SourceSet::default();
    for (name, src) in [("lib.sv", lib), ("main.sv", main)] {
        let (module, kind) = SourceSet::classify(Path::new(name), "kotlin").unwrap();
        sources.add(name, module, kind, src.to_string(), false);
    }
    let mut modules = Vec::new();
    for file in &sources.files {
        let (module, diagnostics) = salvo_syntax::parse_module(&file.content);
        assert!(
            !diagnostics.iter().any(|d| d.is_error()),
            "parse errors: {diagnostics:?}"
        );
        modules.push(module);
    }
    let program = Program {
        files: sources.files,
        modules,
        companions: Vec::new(),
    };
    let symbols = Symbols::collect(&program);
    let resolution = resolve(&program);
    let errors = check_program(&program, &resolution, &symbols).errors;
    let diag = errors
        .iter()
        .find(|d| d.message.contains("no function named `helper`"))
        .unwrap_or_else(|| panic!("got {errors:?}"));
    assert_eq!(diag.suggested_imports, vec!["lib.helper".to_string()]);
}

/// [call-resolve] Calling a value whose type is known and is not a
/// function: the old leniency emitted `n()` and let kotlinc/rustc reject
/// it.
#[test]
fn calling_a_non_fn_value_is_an_error() {
    let errs = messages(&body("    let n = 5\n    n()"));
    assert!(
        errs.iter()
            .any(|m| m == "`n` is not callable: its type is `Int`"),
        "got {errs:?}"
    );
}

/// [call-resolve] A generic parameter is opaque: Salvo has no bounds, so
/// nothing makes a `T` callable. The diagnostic says *that*, rather than
/// pointing at a declaration that could not help.
#[test]
fn calling_a_generic_value_is_an_error() {
    let src = format!(
        "{PRELUDE}\nfn probe<T>(f: T) -> [f] Int {{\n    f()\n    return 1\n}}\n"
    );
    let errs = messages(&src);
    let diag = errs
        .iter()
        .find(|m| m.contains("`f` is a type parameter"))
        .unwrap_or_else(|| panic!("got {errs:?}"));
    assert!(
        diag.contains("Declare it with a fn type"),
        "the diagnostic should name the remedy: {diag}"
    );
}

/// [call-resolve] [fn-dot] Dot-notation is sugar for a function call, so an
/// undeclared method name is unresolved — the diagnostic says how to reach
/// a target-language method.
#[test]
fn unresolved_dot_call_is_an_error() {
    let errs = messages(&body("    p.missing_method()"));
    let diag = errs
        .iter()
        .find(|m| m.contains("no function named `missing_method`"))
        .unwrap_or_else(|| panic!("got {errs:?}"));
    assert!(
        diag.contains("`external fn`"),
        "the diagnostic should name the remedy: {diag}"
    );
}

/// [call-resolve] A *declared* function is reachable by dot-notation, which
/// is how target-language members are exposed.
#[test]
fn declared_external_is_callable_by_dot_notation() {
    let errs = messages(&body("    let s = \"x\".shout()"));
    assert!(errs.is_empty(), "got {errs:?}");
}

// ===== fields [field-resolve] =====

/// [field-resolve] Only structs have fields.
#[test]
fn field_on_a_non_struct_is_an_error() {
    let errs = messages(&body("    let n = 5\n    let x = n.whatever"));
    assert!(
        errs.iter().any(|m| m.starts_with("`Int` has no field `whatever`")),
        "got {errs:?}"
    );
}

/// [field-resolve] An opaque (interop) type exposes nothing by itself: its
/// members are reached through declared accessors.
#[test]
fn field_on_an_opaque_type_is_an_error() {
    let src = format!(
        "{PRELUDE}\nfn probe(o: Opaque) -> [o] Int {{\n    let x = o.member\n    return 1\n}}\n"
    );
    let errs = messages(&src);
    assert!(
        errs.iter().any(|m| m.starts_with("`Opaque` has no field `member`")),
        "got {errs:?}"
    );
}

/// [field-resolve] A generic value is opaque too — and its diagnostic must
/// not suggest an `external fn` accessor, which cannot help for a `T`.
#[test]
fn field_on_a_generic_is_an_error() {
    let src = format!(
        "{PRELUDE}\nfn probe<T>(v: T) -> [v] Int {{\n    let x = v.name\n    return 1\n}}\n"
    );
    let errs = messages(&src);
    let diag = errs
        .iter()
        .find(|m| m.contains("`T` is a type parameter"))
        .unwrap_or_else(|| panic!("got {errs:?}"));
    assert!(
        diag.contains("no field `name`") && diag.contains("concrete type"),
        "the diagnostic should name the remedy: {diag}"
    );
    assert!(
        !diag.contains("external fn"),
        "an accessor cannot help for a type parameter: {diag}"
    );
}

// ===== subscripts [index-resolve] =====

/// [index-resolve] `[]` is for arrays; other collections expose element
/// access as functions.
#[test]
fn indexing_a_non_array_is_an_error() {
    let errs = messages(&body("    let n = 5\n    let x = n[0]"));
    assert!(
        errs.iter()
            .any(|m| m.starts_with("`Int` cannot be indexed with `[]`")),
        "got {errs:?}"
    );
}

/// [index-resolve] Arrays still work, of course.
#[test]
fn indexing_an_array_is_fine() {
    let errs = messages(&body("    let xs: Int[] = [1, 2]\n    let x = xs[0]"));
    assert!(errs.is_empty(), "got {errs:?}");
}

// ===== iteration [iter-resolve] =====

/// [iter-resolve] `for` needs an array, an `Iter<T>`, or a value some
/// `iter` function accepts.
#[test]
fn iterating_a_non_iterable_is_an_error() {
    let errs = messages(&body("    let n = 5\n    for x in n {\n        let y = x\n    }"));
    assert!(
        errs.iter().any(|m| m.starts_with("`Int` is not iterable")),
        "got {errs:?}"
    );
}

/// [iter-resolve] Arrays iterate.
#[test]
fn iterating_an_array_is_fine() {
    let errs = messages(&body(
        "    let xs: Int[] = [1, 2]\n    for x in xs {\n        let y = x\n    }",
    ));
    assert!(errs.is_empty(), "got {errs:?}");
}

// ===== the leniency that remains =====

/// [type-unknown-lenient] A type the checker could not *infer* still passes
/// through: one mistake must not cascade. Here the unresolved call is
/// reported once, and reading a member of its `Unknown` result adds
/// nothing.
#[test]
fn uninferred_types_still_pass_through() {
    let errs = messages(&body("    let v = nowhere()\n    let x = v.member"));
    assert_eq!(
        errs.len(),
        1,
        "expected only the unresolved call to be reported: {errs:?}"
    );
    assert!(errs[0].contains("no function named `nowhere`"), "got {errs:?}");
}
