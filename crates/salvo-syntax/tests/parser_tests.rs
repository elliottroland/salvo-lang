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
