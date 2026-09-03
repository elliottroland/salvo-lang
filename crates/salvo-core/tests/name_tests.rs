//! Name rules [name-casing] [name-dot]: dot-named structs and qualifiers,
//! the namespace constraints, the collision ban that keeps Rust's flattened
//! spelling unambiguous, and module-path casing.

use std::path::Path;

use salvo_core::{resolve, Program, SourceSet};

/// Parses + resolves a multi-file program (no std) and returns the
/// resolver's diagnostics as rendered messages.
fn resolve_errors(files: &[(&str, &str)]) -> Vec<String> {
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
    resolve(&program)
        .errors
        .iter()
        .map(|d| d.message.clone())
        .collect()
}

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
        ("other.sv", "struct Environment {\n    v: Str\n}\n"),
        ("main.sv", "struct Environment.Id {\n    value: Str\n}\n"),
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
        ("other.sv", "struct EnvironmentId {\n    value: Str\n}\n"),
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
    let types = "struct Environment {\n    id: Environment.Id\n}\n\n\
                 struct Environment.Id {\n    value: Str\n}\n";
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
    let errors = resolve_errors(&[("Utils.sv", "fn helper() -> Str {\n    return \"x\"\n}\n")]);
    assert!(
        errors
            .iter()
            .any(|m| m.contains("must start with a lowercase letter")),
        "got {errors:?}"
    );
}
