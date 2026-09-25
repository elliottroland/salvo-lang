//! Structured-diagnostic tests [diag-structured]: resolver and checker
//! errors carry a file index, span, and severity instead of pre-rendered
//! strings, and render at the boundary.

use std::path::Path;

use salvo_core::{check_program, resolve, Checked, Program, SourceSet, Symbols};

/// [intrinsic-std-only] Base types the checker needs, loaded as a std file
/// (module `core.prelude`, implicitly imported) rather than pasted into the
/// sources under test. Added *after* the user files so their file indices —
/// which these tests assert on — stay stable.
const STD_PRELUDE: &str = "export intrinsic type Int\nexport intrinsic type Str\nexport intrinsic type Bool\n";

/// Parses + resolves + checks a multi-file program. Each entry is
/// `(file_name, source)`; a std prelude supplies the base types.
fn check_files(files: &[(&str, &str)]) -> (Program, Checked) {
    let mut sources = SourceSet::default();
    for (name, src) in files {
        let module = SourceSet::classify(Path::new(name)).unwrap();
        sources.add(*name, module, src.to_string(), false);
    }
    sources.add(
        "std/core/prelude.sv",
        SourceSet::classify(Path::new("core/prelude.sv")).unwrap(),
        STD_PRELUDE.to_string(),
        true,
    );
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
    // The declarations are supplied by the std prelude, so the asserted
    // spans in `main.sv` stay put.
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
    let ok = "export fn fine() -> Int {\n    return 1\n}\n";
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

// ===== [mod-collision] name collisions error instead of last-win =====

/// Resolves a multi-file program and returns the resolution errors.
fn resolve_errors(files: &[(&str, &str)]) -> Vec<String> {
    let mut sources = SourceSet::default();
    for (name, src) in files {
        let module = SourceSet::classify(Path::new(name)).unwrap();
        sources.add(*name, module, src.to_string(), false);
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
    let resolution = resolve(&program);
    resolution
        .errors
        .iter()
        .map(|d| d.render(&program.files))
        .collect()
}

// [mod-collision] Two imports bringing the same visible name collide.
#[test]
fn conflicting_imports_are_an_error() {
    let errors = resolve_errors(&[
        ("a.sv", "export struct Person {\n    name: Str\n}\n"),
        ("b.sv", "export struct Person {\n    age: Int\n}\n"),
        (
            "main.sv",
            "import a.Person\nimport b.Person\n\nfn main() -> None {\n}\n",
        ),
    ]);
    assert!(
        errors.iter().any(|e| e.contains("conflicts with the import from `a`")),
        "unexpected errors: {errors:?}"
    );
}

// [mod-collision] An import colliding with an own-module declaration.
#[test]
fn import_shadowing_own_declaration_is_an_error() {
    let errors = resolve_errors(&[
        ("a.sv", "export struct Person {\n    name: Str\n}\n"),
        (
            "main.sv",
            "import a.Person\n\nstruct Person {\n    age: Int\n}\n\nfn main() -> None {\n}\n",
        ),
    ]);
    assert!(
        errors
            .iter()
            .any(|e| e.contains("conflicts with a declaration in this module")),
        "unexpected errors: {errors:?}"
    );
}

// [mod-collision] Aliasing one of the imports resolves the conflict.
#[test]
fn aliased_import_avoids_the_collision() {
    let errors = resolve_errors(&[
        ("a.sv", "export struct Person {\n    name: Str\n}\n"),
        ("b.sv", "export struct Person {\n    age: Int\n}\n"),
        (
            "main.sv",
            "import a.Person\nimport b.Person as AgedPerson\n\nfn main() -> None {\n}\n",
        ),
    ]);
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
}

// [mod-collision] A same-kind duplicate within one module is an error.
#[test]
fn duplicate_declaration_in_module_is_an_error() {
    let errors = resolve_errors(&[(
        "main.sv",
        "struct Person {\n    name: Str\n}\n\nstruct Person {\n    age: Int\n}\n",
    )]);
    assert!(
        errors
            .iter()
            .any(|e| e.contains("duplicate struct `Person` in module `main`")),
        "unexpected errors: {errors:?}"
    );
}

// [mod-collision] [fn-overload] Same-name fns are overloads, never
// collisions.
#[test]
fn same_name_fns_do_not_collide() {
    let errors = resolve_errors(&[
        ("a.sv", "export fn describe(x: Int) -> Str {\n    return \"int\"\n}\n"),
        (
            "main.sv",
            "import a.describe\n\nfn describe(x: Str) -> Str {\n    return \"str\"\n}\n",
        ),
    ]);
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
}

// ===== generic binding widening in unify =====

// A later argument may reveal the more general type: `pick(1, x)` with
// `x: Int?` binds `T = Int?` regardless of argument order — first-binding
// -wins would let the checker believe the result is plain `Int`.
#[test]
fn generic_binding_widens_to_the_general_type() {
    let src = "fn pick<T>(a: T, b: T) -> T {\n    return a\n}\n\n\
               fn main() -> None {\n    let x: Int? = None\n    let z: Int = pick(1, x)\n}\n";
    let (_, checked) = check_files(&[("main.sv", src)]);
    assert!(
        checked
            .errors
            .iter()
            .any(|e| e.message.contains("expected `Int`, found `Int?`")),
        "errors: {:?}",
        checked.errors
    );
}

#[test]
fn generic_binding_widens_in_either_order() {
    let src = "fn pick<T>(a: T, b: T) -> T {\n    return a\n}\n\n\
               fn main() -> None {\n    let x: Int? = None\n    let z: Int = pick(x, 1)\n}\n";
    let (_, checked) = check_files(&[("main.sv", src)]);
    assert!(
        checked
            .errors
            .iter()
            .any(|e| e.message.contains("expected `Int`, found `Int?`")),
        "errors: {:?}",
        checked.errors
    );
}

// Unrelated bindings still fail the overload (no accidental widening
// across incompatible types).
#[test]
fn incompatible_generic_bindings_still_reject() {
    let src = "fn pick<T>(a: T, b: T) -> T {\n    return a\n}\n\n\
               fn main() -> None {\n    let z = pick(1, \"s\")\n}\n";
    let (_, checked) = check_files(&[("main.sv", src)]);
    assert!(
        !checked.errors.is_empty(),
        "expected an overload-resolution error, got none"
    );
}

// ===== [op-no-none] [interp-no-none] optionals at operators and in
// interpolation are errors (user decisions 2026-09-02) =====

/// Runs the checker on a source *with std loaded* (the new rules only
/// fire on known types) and returns the rendered errors.
fn check_errors(src: &str) -> Vec<String> {
    let std_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../std");
    let mut sources = SourceSet::default();
    let io_errors = sources.add_dir(&std_dir, "kt", true, false);
    assert!(io_errors.is_empty(), "failed to read std: {io_errors:?}");
    let module = SourceSet::classify(Path::new("main.sv")).unwrap();
    sources.add("main.sv", module, src.to_string(), false);
    let mut modules = Vec::new();
    for file in &sources.files {
        let (module, diagnostics) = salvo_syntax::parse_module(&file.content);
        let errors: Vec<_> = diagnostics.iter().filter(|d| d.is_error()).collect();
        assert!(errors.is_empty(), "parse errors in {}: {errors:?}", file.name);
        modules.push(module);
    }
    let program = Program {
        files: sources.files,
        modules,
        companions: sources.companions,
    };
    let symbols = Symbols::collect(&program);
    let resolution = resolve(&program);
    let checked = check_program(&program, &resolution, &symbols);
    // Errors only: an unused-variable *warning* [unused-var] is a different
    // severity, and these tests are about which programs are rejected.
    checked
        .errors
        .iter()
        .filter(|d| d.is_error())
        .map(|d| d.render(&program.files))
        .collect()
}

// [op-no-none] Arithmetic, ordering, and equality all reject a
// possibly-`None` operand; `None` itself is rejected outright.
#[test]
fn optional_operands_are_rejected() {
    let errors = check_errors(
        "fn f() -> None {\n\
        \x20   let maybe: Int? = None\n\
        \x20   let a = maybe + 1\n\
        \x20   let b = maybe < 2\n\
        \x20   let c = maybe == 3\n\
        \x20   let d = 1 - maybe\n\
        \x20   let e = None * 2\n\
         }\n",
    );
    for (needle, count) in [
        ("operand of `+` may be `None`", 1),
        ("operand of `<` may be `None`", 1),
        ("operand of `==` may be `None`", 1),
        ("operand of `-` may be `None`", 1),
        ("`None` is not a valid operand of `*`", 1),
    ] {
        assert_eq!(
            errors.iter().filter(|e| e.contains(needle)).count(),
            count,
            "expected `{needle}` in: {errors:?}"
        );
    }
}

// [op-no-none] Narrowing or asserting makes the operand legal.
#[test]
fn narrowed_operands_are_accepted() {
    let errors = check_errors(
        "fn f() -> None {\n\
        \x20   let maybe: Int? = None\n\
        \x20   if maybe is Int m {\n\
        \x20       let a = m + 1\n\
        \x20       let b = m < 2\n\
        \x20   }\n\
        \x20   let c = maybe! + 1\n\
         }\n",
    );
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
}

// [interp-no-none] Interpolating a possibly-`None` value is an error —
// including a struct field, which does not flow-narrow [is-narrowing].
#[test]
fn interpolating_an_optional_is_rejected() {
    let errors = check_errors(
        "struct Person {\n\
        \x20   name: Str,\n\
        \x20   surname: Str? = None\n\
         }\n\
         \n\
         fn f() -> None {\n\
        \x20   let maybe: Int? = None\n\
        \x20   let p = Person {name: \"Ada\"}\n\
        \x20   let a = \"${maybe}\"\n\
        \x20   let b = \"${p.surname}\"\n\
        \x20   let c = \"${None}\"\n\
         }\n",
    );
    assert_eq!(
        errors
            .iter()
            .filter(|e| e.contains("cannot interpolate a value that may be `None`"))
            .count(),
        2,
        "errors: {errors:?}"
    );
    assert!(
        errors
            .iter()
            .any(|e| e.contains("cannot interpolate `None` into a string")),
        "errors: {errors:?}"
    );
}

// [interp-no-none] The `is`-binding and `!` forms interpolate fine — and
// this is exactly the docs/language/ nullability idiom.
#[test]
fn interpolating_a_narrowed_optional_is_accepted() {
    let errors = check_errors(
        "struct Person {\n\
        \x20   name: Str,\n\
        \x20   surname: Str? = None\n\
         }\n\
         \n\
         fn f() -> None {\n\
        \x20   let p = Person {name: \"Ada\", surname: \"L\"}\n\
        \x20   if p.surname is Str surname {\n\
        \x20       let a = \"${p.name} ${surname}\"\n\
        \x20   }\n\
        \x20   let b = \"${p.surname!}\"\n\
         }\n",
    );
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
}
