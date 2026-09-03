//! Parser integration tests: the standard library and a corpus of LANGUAGE.md
//! examples must parse without errors, with AST snapshots for regressions.

use std::path::{Path, PathBuf};

fn parse_clean(path: &Path) -> salvo_syntax::ast::Module {
    let source = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
    let (module, diagnostics) = salvo_syntax::parse_module(&source);
    let errors: Vec<_> = diagnostics.iter().filter(|d| d.is_error()).collect();
    assert!(
        errors.is_empty(),
        "parse errors in {}:\n{}",
        path.display(),
        errors
            .iter()
            .map(|d| d.render(&path.display().to_string(), &source))
            .collect::<Vec<_>>()
            .join("\n")
    );
    module
}

fn corpus(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/corpus")
        .join(name)
}

fn std_core(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../std/core")
        .join(name)
}

// --- Standard library ---

#[test]
fn std_parses_without_errors() {
    let std_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../std");
    let mut count = 0;
    let mut stack = vec![std_dir];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).unwrap().flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "sv") {
                parse_clean(&path);
                count += 1;
            }
        }
    }
    assert!(count >= 7, "expected at least 7 std files, found {count}");
}

// --- Std snapshots ---

#[test]
fn snapshot_std_basic() {
    insta::assert_debug_snapshot!(parse_clean(&std_core("basic.sv")));
}

#[test]
fn snapshot_std_string() {
    insta::assert_debug_snapshot!(parse_clean(&std_core("string.sv")));
}

#[test]
fn snapshot_std_string_kotlin() {
    insta::assert_debug_snapshot!(parse_clean(&std_core("string.kotlin.sv")));
}

#[test]
fn snapshot_std_list() {
    insta::assert_debug_snapshot!(parse_clean(&std_core("list.sv")));
}

#[test]
fn snapshot_std_list_kotlin() {
    insta::assert_debug_snapshot!(parse_clean(&std_core("list.kotlin.sv")));
}

#[test]
fn snapshot_std_console() {
    insta::assert_debug_snapshot!(parse_clean(&std_core("console.sv")));
}

#[test]
fn snapshot_std_console_kotlin() {
    insta::assert_debug_snapshot!(parse_clean(&std_core("console.kotlin.sv")));
}

// --- LANGUAGE.md example corpus ---

#[test]
fn snapshot_corpus_structs() {
    insta::assert_debug_snapshot!(parse_clean(&corpus("structs.sv")));
}

#[test]
fn snapshot_corpus_control_flow() {
    insta::assert_debug_snapshot!(parse_clean(&corpus("control_flow.sv")));
}

#[test]
fn snapshot_corpus_functions() {
    insta::assert_debug_snapshot!(parse_clean(&corpus("functions.sv")));
}

#[test]
fn snapshot_corpus_qualifiers() {
    insta::assert_debug_snapshot!(parse_clean(&corpus("qualifiers.sv")));
}

#[test]
fn snapshot_corpus_effects() {
    insta::assert_debug_snapshot!(parse_clean(&corpus("effects.sv")));
}

#[test]
fn snapshot_corpus_imports_externals() {
    insta::assert_debug_snapshot!(parse_clean(&corpus("imports_externals.sv")));
}

#[test]
fn snapshot_corpus_defines_kotlin() {
    insta::assert_debug_snapshot!(parse_clean(&corpus("defines.kotlin.sv")));
}

// --- Error reporting ---

#[test]
fn reports_errors_with_spans() {
    let source = "fn broken( {\n}";
    let (_module, diagnostics) = salvo_syntax::parse_module(source);
    assert!(
        diagnostics.iter().any(|d| d.is_error()),
        "expected a parse error"
    );
}

#[test]
fn unterminated_string_is_an_error() {
    let source = "fn f() {\n    let s = \"oops\n}";
    let (_module, diagnostics) = salvo_syntax::parse_module(source);
    assert!(diagnostics
        .iter()
        .any(|d| d.is_error() && d.message.contains("unterminated string")));
}

// --- `canbe` opt-ins [canbe-optin] ---

// [canbe-optin] [struct-mut] [type-canbe-mut] `canbe` opts declarations
// into an auto-qualifier; `with` at those sites is a rename error that
// still parses the declaration.
#[test]
fn canbe_opts_declarations_into_auto_qualifiers() {
    let source = "struct Person canbe Mut {\n    name: Str\n}\n\nexternal type List<T> canbe Mut\n";
    let (module, diagnostics) = salvo_syntax::parse_module(source);
    assert!(
        !diagnostics.iter().any(|d| d.is_error()),
        "unexpected errors: {:?}",
        diagnostics.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
    let quals: Vec<&str> = module
        .items
        .iter()
        .flat_map(|item| match item {
            salvo_syntax::ast::Item::Struct(s) => s.auto_qualifiers.as_slice(),
            salvo_syntax::ast::Item::Type(t) => t.auto_qualifiers.as_slice(),
            _ => &[],
        })
        .map(|q| q.name.name.as_str())
        .collect();
    assert_eq!(quals, vec!["Mut", "Mut"]);
}

// [canbe-optin] [linear-generics] Per-type-parameter opt-in on fns.
#[test]
fn canbe_opts_a_type_parameter_in() {
    let source = "fn hold<T canbe Linear>(value: T) -> T {\n    return value\n}\n";
    let (module, diagnostics) = salvo_syntax::parse_module(source);
    assert!(
        !diagnostics.iter().any(|d| d.is_error()),
        "unexpected errors: {:?}",
        diagnostics.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
    let salvo_syntax::ast::Item::Fn(f) = &module.items[0] else {
        panic!("expected a fn item");
    };
    assert_eq!(f.generic_canbe.len(), 1);
    assert_eq!(f.generic_canbe[0].0.name, "T");
    assert_eq!(f.generic_canbe[0].1.name.name, "Linear");
}

// [canbe-optin] [linear-generics] The clause is fn-only for now.
#[test]
fn canbe_on_a_non_fn_type_parameter_is_an_error() {
    let source = "struct Box<T canbe Linear> {\n    item: T\n}\n";
    let (_module, diagnostics) = salvo_syntax::parse_module(source);
    assert!(
        diagnostics
            .iter()
            .any(|d| d.is_error() && d.message.contains("only supported on functions")),
        "expected a fn-only error, got {:?}",
        diagnostics.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
}

// --- Names: casing and dot-names [name-casing] [name-dot] ---

fn errors_of(source: &str) -> Vec<String> {
    let (_module, diagnostics) = salvo_syntax::parse_module(source);
    diagnostics
        .iter()
        .filter(|d| d.is_error())
        .map(|d| d.message.clone())
        .collect()
}

// [name-dot] A struct or qualifier may be declared `Ns.Name`, and the pair
// is kept as one dotted name.
#[test]
fn dot_names_parse_in_declarations_and_types() {
    let source = "struct Environment {\n    id: Environment.Id\n}\n\n\
                  struct Environment.Id {\n    value: Str\n}\n\n\
                  qualifier Environment.Tag of Str\n\n\
                  fn tag(v: Str) -> Str as Environment.Tag {\n    return v\n}\n";
    let (module, diagnostics) = salvo_syntax::parse_module(source);
    assert!(
        !diagnostics.iter().any(|d| d.is_error()),
        "unexpected errors: {:?}",
        diagnostics.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
    let names: Vec<&str> = module
        .items
        .iter()
        .filter_map(|item| match item {
            salvo_syntax::ast::Item::Struct(s) => Some(s.name.name.as_str()),
            salvo_syntax::ast::Item::Qualifier(q) => Some(q.name.name.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(
        names,
        vec!["Environment", "Environment.Id", "Environment.Tag"]
    );
}

// [name-dot] A dot-name in expression position is a struct literal, while
// a lowercase head stays a field read or dot-call — that is what the
// casing rule buys.
#[test]
fn dot_name_struct_literal_is_not_a_field_read() {
    let source = "fn f() -> None {\n    let a = Environment.Id {value: \"x\"}\n    \
                  let b = a.value\n    let c = a.value.size()\n}\n";
    let (_module, diagnostics) = salvo_syntax::parse_module(source);
    assert!(
        !diagnostics.iter().any(|d| d.is_error()),
        "unexpected errors: {:?}",
        diagnostics.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
}

// [name-dot] Names cap at two segments.
#[test]
fn three_segment_dot_names_are_rejected() {
    let errors = errors_of("struct A.B.C {\n    v: Str\n}\n");
    assert!(
        errors.iter().any(|m| m.contains("exactly two segments")),
        "got {errors:?}"
    );
}

// [name-casing] Types are uppercase, values are not.
#[test]
fn casing_rule_is_enforced_on_declarations() {
    for (source, needle) in [
        ("struct thing {\n    v: Str\n}\n", "struct names must start"),
        ("qualifier tag of Str\n", "qualifier names must start"),
        ("type alias = Str\n", "type names must start"),
        ("effect logger {\n    fn log(m: Str) -> None\n}\n", "effect names must start"),
        ("fn Foo() -> None {\n}\n", "fn names must not start"),
        ("fn f(Bad: Str) -> None {\n}\n", "parameter names must not start"),
        ("struct S {\n    Value: Str\n}\n", "field names must not start"),
        ("fn f() -> None {\n    let Bad = 1\n}\n", "binding names must not start"),
        ("fn f() -> None {\n    for Item in xs {\n    }\n}\n", "binding names must not start"),
        ("fn f() -> None {\n    let g = X -> 1\n}\n", "must not start"),
    ] {
        let errors = errors_of(source);
        assert!(
            errors.iter().any(|m| m.contains(needle)),
            "expected {needle:?} for {source:?}, got {errors:?}"
        );
    }
}

// [name-casing] Generic parameters are type names.
#[test]
fn generic_parameters_must_be_uppercase() {
    let errors = errors_of("fn f<t>(v: t) -> t {\n    return v\n}\n");
    assert!(
        errors
            .iter()
            .any(|m| m.contains("generic parameter names must start")),
        "got {errors:?}"
    );
}

// [qual-subject] `provenance qualifier Q of T` parses with the provenance
// subject; a plain declaration defaults to state.
#[test]
fn provenance_qualifiers_parse() {
    let source = "qualifier Validated of Request\n\
                  provenance qualifier Authenticated of Request\n";
    let (module, diagnostics) = salvo_syntax::parse_module(source);
    assert!(
        !diagnostics.iter().any(|d| d.is_error()),
        "unexpected errors: {:?}",
        diagnostics.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
    let subjects: Vec<salvo_syntax::ast::QualSubject> = module
        .items
        .iter()
        .filter_map(|item| match item {
            salvo_syntax::ast::Item::Qualifier(q) => Some(q.subject),
            _ => None,
        })
        .collect();
    assert_eq!(
        subjects,
        vec![
            salvo_syntax::ast::QualSubject::State,
            salvo_syntax::ast::QualSubject::Provenance
        ]
    );
}

// [qual-subject] `provenance` only modifies a qualifier declaration.
#[test]
fn provenance_must_precede_a_qualifier() {
    let errors = errors_of("provenance struct Thing {\n    v: Str\n}\n");
    assert!(
        errors
            .iter()
            .any(|m| m.contains("expected `qualifier` after `provenance`")),
        "got {errors:?}"
    );
}
