//! Name rules [name-casing] [name-dot] [name-resolve]: dot-named structs
//! and qualifiers, the namespace constraints, the collision ban that keeps
//! Rust's flattened spelling unambiguous, module-path casing, and written
//! names in type positions resolving to declarations.

use std::path::Path;

use salvo_core::{check_program, resolve, FileDiagnostic, Program, SourceSet, Symbols};

/// Parses + resolves a multi-file program (no std) and returns the
/// resolver's diagnostics as rendered messages.
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
    resolve(&program)
        .errors
        .iter()
        .map(|d| d.message.clone())
        .collect()
}

/// Parses + resolves + checks a multi-file program and returns the
/// checker's structured diagnostics. A std prelude ([intrinsic-std-only])
/// supplies the base types the checker needs, loaded as a std file rather
/// than pasted into the sources under test.
fn check_errors(files: &[(&str, &str)]) -> Vec<FileDiagnostic> {
    let mut sources = SourceSet::default();
    sources.add(
        "std/core/prelude.sv",
        SourceSet::classify(Path::new("core/prelude.sv")).unwrap(),
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
    let program = Program {
        files: sources.files,
        modules,
        companions: Vec::new(),
    };
    let symbols = Symbols::collect(&program);
    let resolution = resolve(&program);
    check_program(&program, &resolution, &symbols).errors
}

/// The checker's error messages for a single-file program. The std prelude
/// loaded by [check_errors] supplies the base types, so a source under test
/// names `Int`/`Str`/`Bool` without declaring them [name-resolve].
fn messages(src: &str) -> Vec<String> {
    check_errors(&[("main.sv", src)])
        .iter()
        .map(|d| d.message.clone())
        .collect()
}

/// [intrinsic-std-only] The base-type declarations the checker needs.
/// `intrinsic` is the compiler's modifier and only std may write it, so this
/// is loaded as a *std* file (module `core.prelude`, implicitly imported)
/// rather than pasted into the source under test.
const STD_PRELUDE: &str = "export intrinsic type Int\nexport intrinsic type Str\nexport intrinsic type Bool\nexport intrinsic type Store<T> canbe Mut\n";

// [name-dot] A dot-name resolves as one dotted name, and the namespace
// struct in the same file satisfies the rule.
#[test]
fn dot_names_resolve() {
    let src = "struct Environment {\n    id: Environment.Id\n}\n\n\
               struct Environment.Id {\n    value: Str\n}\n\n\
               qualifier Environment.Tag of Str\n";
    assert!(resolve_errors(&[("main.sv", src)]).is_empty());
}

// [name-dot] The namespace must be a struct declared in the same file.
#[test]
fn namespace_must_be_a_struct_in_the_same_file() {
    let errors = resolve_errors(&[
        ("other.sv", "export struct Environment {\n    v: Str\n}\n"),
        ("main.sv", "export struct Environment.Id {\n    value: Str\n}\n"),
    ]);
    assert!(
        errors
            .iter()
            .any(|m| m.contains("must be a struct declared in this file")),
        "got {errors:?}"
    );
}

// [name-dot] A generic namespace cannot nest a member: Kotlin's nested
// class cannot reference the outer type parameters.
#[test]
fn generic_namespace_is_rejected() {
    let src = "struct Pair<S, T> {\n    first: S,\n    second: T\n}\n\n\
               struct Pair.Entry {\n    value: Str\n}\n";
    let errors = resolve_errors(&[("main.sv", src)]);
    assert!(
        errors.iter().any(|m| m.contains("is generic, so it cannot namespace")),
        "got {errors:?}"
    );
}

// [name-dot] Nothing visible may carry the concatenated spelling — Rust
// flattens dot-names and overload mangling uses the same form.
#[test]
fn concatenated_name_collision_is_an_error() {
    let src = "struct Environment {\n    v: Str\n}\n\n\
               struct Environment.Id {\n    value: Str\n}\n\n\
               struct EnvironmentId {\n    value: Str\n}\n";
    let errors = resolve_errors(&[("main.sv", src)]);
    assert!(
        errors.iter().any(|m| m.contains("collides with `EnvironmentId`")),
        "got {errors:?}"
    );
}

// [name-dot] The ban reaches *imported* names too, which a file-scoped
// check would miss.
#[test]
fn concatenated_collision_covers_imports() {
    let errors = resolve_errors(&[
        ("other.sv", "export struct EnvironmentId {\n    value: Str\n}\n"),
        (
            "main.sv",
            "import other.EnvironmentId\n\nstruct Environment {\n    v: Str\n}\n\n\
             struct Environment.Id {\n    value: Str\n}\n",
        ),
    ]);
    assert!(
        errors.iter().any(|m| m.contains("collides with `EnvironmentId`")),
        "got {errors:?}"
    );
}

// [name-dot-import] Importing a namespace struct brings its dot-named
// members; importing a member directly works too.
#[test]
fn importing_a_namespace_brings_its_members() {
    let types = "export struct Environment {\n    id: Environment.Id\n}\n\n\
                 export struct Environment.Id {\n    value: Str\n}\n";
    let user = "import types.Environment\n\n\
                fn read(id: Environment.Id) -> Str {\n    return id.value\n}\n";
    assert!(resolve_errors(&[("types.sv", types), ("main.sv", user)]).is_empty());

    let member_only = "import types.Environment.Id\n\n\
                       fn read(id: Environment.Id) -> Str {\n    return id.value\n}\n";
    assert!(resolve_errors(&[("types.sv", types), ("main.sv", member_only)]).is_empty());
}

// [name-casing] Module paths are file paths [mod-file], so an uppercase
// file or directory name is an error naming the file.
#[test]
fn uppercase_module_path_is_an_error() {
    let errors = resolve_errors(&[("Utils.sv", "export fn helper() -> Str {\n    return \"x\"\n}\n")]);
    assert!(
        errors
            .iter()
            .any(|m| m.contains("must start with a lowercase letter")),
        "got {errors:?}"
    );
}

// ===== [name-resolve] written names in type positions =====

// [name-resolve] An undeclared qualifier in an `is` check is reported as
// the unresolved name it is — not as the arm mismatch it causes. Before
// this rule the same program said "this check can never succeed" (the
// name was silently treated as a base type, which no arm matched) and
// then cascaded a bogus consumed-value error on the subject.
#[test]
fn unknown_name_in_an_is_check_is_reported_once() {
    let src = format!(
        "qualifier Ok<T> of T\n\n\
         fn ok<T>(value: T) [] -> +Ok T => !value {{\n    return value\n}}\n\n\
         fn f() -> Ok Int | Err Str {{\n    return ok(1)\n}}\n\n\
         fn g() -> Int {{\n    let v = f()\n    if v is Err {{\n    }}\n    return 1\n}}\n"
    );
    let messages = messages(&src);
    assert!(
        messages
            .iter()
            .any(|m| m == "unknown type or qualifier `Err`"),
        "got {messages:?}"
    );
    assert!(
        !messages.iter().any(|m| m.contains("can never succeed")),
        "the arm mismatch must not be reported on top: {messages:?}"
    );
    // Exactly one error: the unresolved name in the `is` check. The
    // return type mentions `Err` too, and that site reports it as an
    // unknown qualifier.
    assert_eq!(
        messages
            .iter()
            .filter(|m| m.contains("can never") || m.contains("consumed"))
            .count(),
        0,
        "got {messages:?}"
    );
}

// [name-resolve] A `when` branch naming something unresolved reports the
// name; the arms it fails to match must not also surface as a
// non-exhaustive `when`.
#[test]
fn unknown_name_in_a_when_branch_does_not_cascade() {
    let src = format!(
        "qualifier Ok<T> of T\nqualifier Err<T> of T\n\n\
         fn ok<T>(value: T) [] -> +Ok T => !value {{\n    return value\n}}\n\n\
         fn f() -> Ok Int | Err Str {{\n    return ok(1)\n}}\n\n\
         fn g() -> Int {{\n    let v = f()\n    when v {{\n        \
         is Ok {{\n            1\n        }}\n        \
         is Nope {{\n            2\n        }}\n    }}\n    return 1\n}}\n"
    );
    let messages = messages(&src);
    assert!(
        messages
            .iter()
            .any(|m| m == "unknown type or qualifier `Nope`"),
        "got {messages:?}"
    );
    assert!(
        !messages.iter().any(|m| m.contains("non-exhaustive")),
        "got {messages:?}"
    );
    assert!(
        !messages.iter().any(|m| m.contains("matches no remaining")),
        "got {messages:?}"
    );
}

// [name-resolve] Undeclared base types and qualifiers are reported at
// every declaration site: fn signatures, struct fields, `let`
// annotations, type aliases, effect members.
#[test]
fn unknown_names_are_reported_at_declaration_sites() {
    let cases = [
        ("fn f(x: Nope) -> Int {\n    return 1\n}\n", "unknown type `Nope`"),
        ("fn f() -> Nope {\n    return 1\n}\n", "unknown type `Nope`"),
        ("fn f(x: Bogus Int) -> Int {\n    return 1\n}\n", "unknown qualifier `Bogus`"),
        ("struct S {\n    field: Nope\n}\n", "unknown type `Nope`"),
        ("struct S {\n    field: Bogus Int\n}\n", "unknown qualifier `Bogus`"),
        (
            "fn f() -> Int {\n    let x: Nope = 1\n    return 1\n}\n",
            "unknown type `Nope`",
        ),
        ("type Alias = Nope | Int\n", "unknown type `Nope`"),
        (
            "effect E {\n    fn member(x: Nope) -> Int\n}\n",
            "unknown type `Nope`",
        ),
        (
            "qualifier Q of Nope\n",
            "unknown type `Nope`",
        ),
        (
            "qualifier Q of Int with Nope\n",
            "unknown qualifier `Nope`",
        ),
    ];
    for (body, want) in cases {
        let src = format!("{body}");
        let messages = messages(&src);
        assert!(
            messages.iter().any(|m| m == want),
            "`{body}` should report `{want}`, got {messages:?}"
        );
    }
}

// [name-resolve] A name that exists in the *other* namespace is a
// position mistake: say so rather than suggesting an import that cannot
// help.
#[test]
fn a_name_in_the_wrong_namespace_says_so() {
    let src = format!("qualifier Tag of Int\n\nfn f(x: Tag) -> Int {{\n    return 1\n}}\n");
    let as_type = messages(&src);
    assert!(
        as_type
            .iter()
            .any(|m| m == "unknown type `Tag` (`Tag` is a qualifier, not a type)"),
        "got {as_type:?}"
    );

    let src = format!("struct S {{\n    v: Int\n}}\n\nfn f(x: S Int) -> Int {{\n    return 1\n}}\n");
    let as_qual = messages(&src);
    assert!(
        as_qual
            .iter()
            .any(|m| m == "unknown qualifier `S` (`S` is a type, not a qualifier)"),
        "got {as_qual:?}"
    );
}

// [name-resolve] [diag-import-suggest] Unresolved type names carry the
// modules that declare them, like every other unresolved-name
// diagnostic.
#[test]
fn unknown_type_names_suggest_imports() {
    let lib = "export struct Widget {\n    v: Str\n}\n";
    let main = "fn f(w: Widget) -> Str {\n    return w.v\n}\n";
    let errors = check_errors(&[("lib.sv", lib), ("main.sv", main)]);
    let diag = errors
        .iter()
        .find(|d| d.message == "unknown type `Widget`")
        .unwrap_or_else(|| panic!("got {errors:?}"));
    assert_eq!(diag.suggested_imports, vec!["lib.Widget".to_string()]);
}

// [name-resolve] The compiler's intrinsic qualifiers have no declaration
// to find, so they must not be reported as unknown; their own rules
// still apply.
#[test]
fn intrinsic_qualifiers_are_known_names() {
    let src = "fn f(s: Mut Store<Int>) -> Int {\n    return 1\n}\n";
    assert!(messages(src).is_empty(), "got {:?}", messages(src));
}

// [canbe-optin] `canbe` grants one of the compiler's own qualifiers; a
// user qualifier there is a confusion with `with` [qual-with].
#[test]
fn canbe_only_accepts_the_intrinsic_qualifiers() {
    let src = format!("qualifier Tag of Int\n\nstruct S canbe Tag {{\n    v: Int\n}}\n");
    let messages = messages(&src);
    assert!(
        messages
            .iter()
            .any(|m| m
                .contains("only `Mut`, `once`, `hashed` and `ordered` can be opted into with `canbe`")),
        "got {messages:?}"
    );
}
