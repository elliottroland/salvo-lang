//! Name rules [name-casing] [name-dot] [name-resolve] [mod-namespace]: what
//! annotations cannot say, and what is kept until a defect is fixed. Which
//! diagnostics the name rules report is test data, `tests/check/names/`
//! (TESTS.md T1).

use std::path::Path;

use salvo_core::{check_program, resolve, FileDiagnostic, Program, SourceSet, Symbols};

/// Parses + resolves + checks a multi-file program and returns the
/// checker's structured diagnostics. A std prelude ([platform-intrinsic-std-only])
/// supplies the base types the checker needs, loaded as a std file rather
/// than pasted into the sources under test.
fn check_errors(files: &[(&str, &str)]) -> Vec<FileDiagnostic> {
    let mut sources = SourceSet::default();
    sources.add(
        "std/core/prelude.sv",
        SourceSet::classify(Path::new("std/core/prelude.sv")).unwrap(),
        STD_PRELUDE.to_string(),
        true,
    );
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
    // [comptime-instantiate] The `by` sites stamp before resolution.
    let _expansion = salvo_core::expand(&sources.files, &mut modules);
    let program = Program {
        files: sources.files,
        modules: Program::share(modules),
        companions: Vec::new(),
    };
    let symbols = Symbols::collect(&program);
    let resolution = resolve(&program);
    check_program(&program, &resolution, &symbols).errors
}

/// [diag-id] The id of every checker diagnostic for a single-file program.
/// The std prelude loaded by [check_errors] supplies the base types, so a
/// source under test names `Int`/`Str`/`Bool` without declaring them
/// [name-resolve].
fn ids(src: &str) -> Vec<&'static str> {
    check_errors(&[("main.sv", src)]).iter().map(|d| d.id.unwrap_or("?")).collect()
}

/// [platform-intrinsic-std-only] The base-type declarations the checker needs.
/// `intrinsic` is the compiler's modifier and only std may write it, so this
/// is loaded as a *std* file (module `std.core.prelude`, implicitly imported)
/// rather than pasted into the source under test.
const STD_PRELUDE: &str = concat!(
    "export intrinsic type Int\nexport intrinsic type Str\nexport intrinsic type Bool\nexport intrinsic type Store<T> canbe Mut\n",
    // [op-equality] [op-order] What the comparison operators call.
    "export intrinsic fn eq(a: Int, b: Int) [] -> Bool => a, b\n",
    "export intrinsic fn eq(a: Bool, b: Bool) [] -> Bool => a, b\n",
    "export intrinsic fn eq(a: Str, b: Str) [] -> Bool => a, b\n",
    "export intrinsic fn cmp(a: Int, b: Int) [] -> Int => a, b\n",
);

// [name-casing] Module paths are file paths [mod-file], so an uppercase
// file or directory name is an error naming the file. Rust, not data: the
// diagnostic is the file's (span 0..0, line 1), and no annotation can stand
// above line 1.
#[test]
fn uppercase_module_path_is_an_error() {
    let out = salvo_testkit::front::check(&[("Utils.sv", "export fn helper() -> Str {\n    return \"x\"\n}\n")]);
    let found: Vec<(&str, &str)> = out
        .diagnostics()
        .map(|d| (out.program.files[d.file].name.as_str(), d.id.unwrap_or("?")))
        .collect();
    assert_eq!(found, [("Utils.sv", "name-casing:module-segment")], "{:?}", out.errors());
}

// [name-resolve] Undeclared base types and qualifiers are reported at
// every declaration site: fn signatures, struct fields, `let`
// annotations, type aliases, effect members.
// Kept over the prelude until a defect is fixed (ROADMAP §0ar): over the
// real std, `fn f() -> Nope { return 1 }` also reports
// `fn-syntax:return-mismatch` and `let x: Nope = 1` also reports
// `var-let-infer:annotation-mismatch` beside the unknown type.
#[test]
fn unknown_names_are_reported_at_declaration_sites() {
    let cases = [
        ("fn f(x: Nope) -> Int {\n    return 1\n}\n", "name-resolve:unknown-type"),
        ("fn f() -> Nope {\n    return 1\n}\n", "name-resolve:unknown-type"),
        ("fn f(x: Bogus Int) -> Int {\n    return 1\n}\n", "name-resolve:unknown-type"),
        ("struct S {\n    field: Nope\n}\n", "name-resolve:unknown-type"),
        ("struct S {\n    field: Bogus Int\n}\n", "name-resolve:unknown-type"),
        (
            "fn f() -> Int {\n    let x: Nope = 1\n    return 1\n}\n",
            "name-resolve:unknown-type",
        ),
        ("type Alias = Nope | Int\n", "name-resolve:unknown-type"),
        (
            "effect E {\n    fn member(x: Nope) -> Int\n}\n",
            "name-resolve:unknown-type",
        ),
        (
            "qualifier Q of Nope\n",
            "name-resolve:unknown-type",
        ),
    ];
    for (body, want) in cases {
        let src = format!("{body}");
        let ids = ids(&src);
        assert!(ids.contains(&want), "`{body}` should report `{want}`, got {ids:?}");
    }
}

// [name-resolve] [diag-import-suggest] Unresolved type names carry the
// modules that declare them, like every other unresolved-name
// diagnostic. Rust, not data: an annotation names the id, not the
// suggestions.
#[test]
fn unknown_type_names_suggest_imports() {
    let lib = "export struct Widget {\n    v: Str\n}\n";
    let main = "fn f(w: Widget) -> Str {\n    return w.v\n}\n";
    let out = salvo_testkit::front::check(&[("lib.sv", lib), ("main.sv", main)]);
    let diag = out
        .diagnostics()
        .find(|d| d.id == Some("name-resolve:unknown-type"))
        .unwrap_or_else(|| panic!("got {:?}", out.errors()));
    assert_eq!(diag.suggested_imports, vec!["lib.Widget".to_string()]);
}

// [mod-namespace] An arm the shorthand cannot resolve inside the alias's own
// body — no such sibling in its namespace — is still a clear error naming
// the namespace searched (same error `resolve_alias_arm_dot_ref` reports
// for every other arm position).
// Kept over the prelude until a defect is fixed (ROADMAP §0ar): over the
// real std, `mod-namespace:no-alias-member` is reported twice for the one
// arm.
#[test]
fn leading_dot_shorthand_reports_an_unresolvable_sibling() {
    let src = "namespace Arg {\n    \
                   struct Int {}\n    \
                   type Type = .Int | .Missing\n\
               }\n";
    let ids = ids(src);
    assert!(ids.contains(&"mod-namespace:no-alias-member"), "{ids:?}");
}
