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
fn snapshot_std_result() {
    insta::assert_debug_snapshot!(parse_clean(&std_core("result.sv")));
}

#[test]
fn snapshot_std_list() {
    insta::assert_debug_snapshot!(parse_clean(&std_core("list.sv")));
}

#[test]
fn snapshot_std_console() {
    insta::assert_debug_snapshot!(parse_clean(&std_core("console.sv")));
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
fn snapshot_corpus_imports() {
    insta::assert_debug_snapshot!(parse_clean(&corpus("imports.sv")));
}

// --- Tuple indexing [expr-tuple-index] ---

/// Parses one expression statement out of a function body.
fn parse_expr_stmt(src: &str) -> salvo_syntax::ast::Expr {
    let source = format!("fn f() -> None {{\n    let x = {src}\n}}\n");
    let (module, diagnostics) = salvo_syntax::parse_module(&source);
    let errors: Vec<_> = diagnostics.iter().filter(|d| d.is_error()).collect();
    assert!(errors.is_empty(), "parse errors in `{src}`: {errors:?}");
    let salvo_syntax::ast::Item::Fn(f) = &module.items[0] else {
        panic!("expected a fn");
    };
    let Some(salvo_syntax::ast::Stmt::Let { value, .. }) = f.body.as_ref().unwrap().stmts.first()
    else {
        panic!("expected a let");
    };
    value.clone()
}

/// [expr-tuple-index] `t.0` is a tuple-element projection, not a field.
#[test]
fn tuple_index_parses_as_a_projection() {
    let expr = parse_expr_stmt("t.0");
    match expr {
        salvo_syntax::ast::Expr::TupleIndex { base, index, .. } => {
            assert_eq!(index, 0);
            assert!(matches!(*base, salvo_syntax::ast::Expr::Ident(_)));
        }
        other => panic!("expected a tuple index, got {other:?}"),
    }
}

/// [expr-tuple-index] The lexer must not read `0.1` as a fraction here:
/// `t.0.1` is *two* indices. (Numbers own their decimal point everywhere
/// else, so this is the one place the rule is suspended.)
#[test]
fn nested_tuple_index_is_two_projections() {
    let expr = parse_expr_stmt("t.0.1");
    let salvo_syntax::ast::Expr::TupleIndex { base, index, .. } = expr else {
        panic!("expected a tuple index");
    };
    assert_eq!(index, 1, "outermost index");
    match *base {
        salvo_syntax::ast::Expr::TupleIndex { index, .. } => assert_eq!(index, 0),
        other => panic!("expected a nested tuple index, got {other:?}"),
    }
}

/// [expr-tuple-index] Float literals keep their decimal point everywhere a
/// tuple index cannot appear.
#[test]
fn floats_still_lex_as_floats() {
    let expr = parse_expr_stmt("1.5");
    assert!(
        matches!(expr, salvo_syntax::ast::Expr::Float { value, .. } if value == 1.5),
        "expected a float literal, got {expr:?}"
    );
}

/// [expr-tuple-index] A tuple element can be the receiver of a dot-call,
/// so the index must not swallow the following `.name`.
#[test]
fn tuple_index_can_be_a_dot_call_receiver() {
    let expr = parse_expr_stmt("t.1.size()");
    let salvo_syntax::ast::Expr::Call { callee, .. } = expr else {
        panic!("expected a call, got {expr:?}");
    };
    let salvo_syntax::ast::Expr::Field { base, field, .. } = *callee else {
        panic!("expected a field callee");
    };
    assert_eq!(field.name, "size");
    assert!(matches!(
        *base,
        salvo_syntax::ast::Expr::TupleIndex { index: 1, .. }
    ));
}

/// [expr-tuple-index] Numeric suffixes are not tuple indices.
#[test]
fn tuple_index_rejects_numeric_suffixes() {
    let (_, diagnostics) = salvo_syntax::parse_module("fn f() -> None {\n    let x = t.0L\n}\n");
    assert!(
        diagnostics
            .iter()
            .any(|d| d.message.contains("tuple index has no `L` suffix")),
        "unexpected diagnostics: {diagnostics:?}"
    );
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
    let source = "struct Person canbe Mut {\n    name: Str\n}\n\ntype IntList canbe Mut = List<Int>\n";
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

// ===== Doc comments [doc-comment] =====

/// The docs of every top-level fn and struct (with its fields) in `src`.
fn docs_of(src: &str) -> Vec<(String, Vec<String>)> {
    use salvo_syntax::ast::Item;
    let (module, diagnostics) = salvo_syntax::parse_module(src);
    let errors: Vec<_> = diagnostics.iter().filter(|d| d.is_error()).collect();
    assert!(errors.is_empty(), "parse errors: {errors:?}");
    let mut out = Vec::new();
    for item in &module.items {
        match item {
            Item::Fn(f) => out.push((f.name.name.clone(), f.docs.clone())),
            Item::Struct(s) => {
                out.push((s.name.name.clone(), s.docs.clone()));
                for field in &s.fields {
                    out.push((
                        format!("{}.{}", s.name.name, field.name.name),
                        field.docs.clone(),
                    ));
                }
            }
            Item::Qualifier(q) => out.push((q.name.name.clone(), q.docs.clone())),
            Item::Effect(e) => out.push((e.name.name.clone(), e.docs.clone())),
            Item::Handler(h) => out.push((h.name.name.clone(), h.docs.clone())),
            Item::Type(t) => out.push((t.name.name.clone(), t.docs.clone())),
            _ => {}
        }
    }
    out
}

// [doc-comment] A declaration's docs are the run of own-line `//` comments
// directly above it: one blank line ends the run, so an unrelated comment
// earlier in the file stays unrelated. A `//` line inside the run is kept
// (it is a markdown paragraph break, not a separator).
#[test]
fn docs_are_the_comment_block_directly_above() {
    let docs = docs_of(
        "// Unrelated.\n\
         \n\
         // First line.\n\
         //\n\
         // Third line.\n\
         fn documented() -> Int {\n    return 1\n}\n\
         \n\
         // Detached by a blank line.\n\
         \n\
         fn undocumented() -> Int {\n    return 1\n}\n",
    );
    assert_eq!(
        docs,
        vec![
            (
                "documented".to_string(),
                vec![
                    "First line.".to_string(),
                    String::new(),
                    "Third line.".to_string()
                ]
            ),
            ("undocumented".to_string(), Vec::new()),
        ]
    );
}

// [doc-comment] A trailing comment documents nothing: it shares its line
// with code, so it is neither the previous declaration's docs nor the next
// one's.
#[test]
fn trailing_comments_are_not_docs() {
    let docs = docs_of(
        "struct S {\n    \
             a: Int, // the first\n    \
             b: Int\n\
         } // not docs either\n\
         fn f() -> Int {\n    return 1\n}\n",
    );
    assert_eq!(
        docs,
        vec![
            ("S".to_string(), Vec::new()),
            ("S.a".to_string(), Vec::new()),
            ("S.b".to_string(), Vec::new()),
            ("f".to_string(), Vec::new()),
        ]
    );
}

// [doc-struct-fields] Struct docs and field docs are captured separately,
// each from the block above its own declaration.
#[test]
fn struct_and_field_docs_are_separate() {
    let docs = docs_of(
        "// The struct.\n\
         struct S {\n    \
             // The field.\n    \
             a: Int,\n    \
             b: Int\n\
         }\n",
    );
    assert_eq!(
        docs,
        vec![
            ("S".to_string(), vec!["The struct.".to_string()]),
            ("S.a".to_string(), vec!["The field.".to_string()]),
            ("S.b".to_string(), Vec::new()),
        ]
    );
}

// [doc-comment] A backing modifier (`intrinsic`, `provenance`) sits on the
// declaration's own line, so the block above it still counts.
#[test]
fn docs_survive_declaration_modifiers() {
    let docs = docs_of(
        "// An intrinsic fn.\n\
         intrinsic fn e(x: Int) [] -> [] Int\n\
         \n\
         // A provenance qualifier.\n\
         provenance qualifier P of Int\n\
         \n\
         // An intrinsic type.\n\
         intrinsic type T\n",
    );
    assert_eq!(
        docs,
        vec![
            ("e".to_string(), vec!["An intrinsic fn.".to_string()]),
            ("P".to_string(), vec!["A provenance qualifier.".to_string()]),
            ("T".to_string(), vec!["An intrinsic type.".to_string()]),
        ]
    );
}

// [doc-comment] Indentation inside a comment is kept (markdown nesting
// needs it) but the `//` and one following space are not.
#[test]
fn docs_keep_indentation_after_the_marker() {
    let docs = docs_of(
        "// - one\n\
         //   - nested\n\
         //no space\n\
         fn f() -> Int {\n    return 1\n}\n",
    );
    assert_eq!(
        docs[0].1,
        vec![
            "- one".to_string(),
            "  - nested".to_string(),
            "no space".to_string()
        ]
    );
}

// [defer] `defer` takes a block — the language's only body form.
#[test]
fn defer_parses_a_block() {
    let source = "fn f() {\n    defer {\n        close(h)\n    }\n    use_it(h)\n}\n";
    let (module, diagnostics) = salvo_syntax::parse_module(source);
    assert!(
        !diagnostics.iter().any(|d| d.is_error()),
        "unexpected errors: {:?}",
        diagnostics.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
    let salvo_syntax::ast::Item::Fn(f) = &module.items[0] else {
        panic!("expected a fn item");
    };
    let body = f.body.as_ref().expect("fn has a body");
    let salvo_syntax::ast::Stmt::Defer { body: deferred, .. } = &body.stmts[0] else {
        panic!("expected a defer statement, got {:?}", body.stmts[0]);
    };
    assert_eq!(deferred.stmts.len(), 1);
}

// [defer] A statement without braces is a parse error naming the form.
#[test]
fn defer_without_a_block_is_an_error() {
    let source = "fn f() {\n    defer close(h)\n}\n";
    let (_module, diagnostics) = salvo_syntax::parse_module(source);
    assert!(
        diagnostics
            .iter()
            .any(|d| d.is_error() && d.message.contains("`defer` takes a block")),
        "expected a block-required error, got {:?}",
        diagnostics.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
}

// [try] `try` takes a block and is an *expression* — the throw delimiter.
#[test]
fn try_parses_as_a_block_expression() {
    let source = "fn f() {\n    let outcome = try {\n        parse(line)\n    }\n}\n";
    let (module, diagnostics) = salvo_syntax::parse_module(source);
    assert!(
        !diagnostics.iter().any(|d| d.is_error()),
        "unexpected errors: {:?}",
        diagnostics.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
    let salvo_syntax::ast::Item::Fn(f) = &module.items[0] else {
        panic!("expected a fn item");
    };
    let body = f.body.as_ref().expect("fn has a body");
    let salvo_syntax::ast::Stmt::Let { value, .. } = &body.stmts[0] else {
        panic!("expected a let statement, got {:?}", body.stmts[0]);
    };
    let salvo_syntax::ast::Expr::Try { body: inner, .. } = value else {
        panic!("expected a try expression, got {value:?}");
    };
    assert_eq!(inner.stmts.len(), 1);
}

// [try] A bodyless `try` is a parse error naming the form.
#[test]
fn try_without_a_block_is_an_error() {
    let source = "fn f() {\n    let outcome = try parse(line)\n}\n";
    let (_module, diagnostics) = salvo_syntax::parse_module(source);
    assert!(
        diagnostics
            .iter()
            .any(|d| d.is_error() && d.message.contains("`try` takes a block")),
        "expected a block-required error, got {:?}",
        diagnostics.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
}

// ===== The subject-less `when` [when-condition] =====

// [when-condition] `when {` is the subject-less form — a subject is always
// a plain variable, so the brace decides which grammar applies. Branch
// heads are bare boolean expressions; the `else` closes the chain.
#[test]
fn subjectless_when_parses_a_condition_chain() {
    let expr = parse_expr_stmt("when {\n        n < 0 { \"negative\" }\n        n == 0 { \"zero\" }\n        else { \"positive\" }\n    }");
    let salvo_syntax::ast::Expr::WhenCond {
        branches,
        else_block,
        ..
    } = expr
    else {
        panic!("expected a subject-less when, got {expr:?}");
    };
    assert_eq!(branches.len(), 2);
    assert!(matches!(
        branches[0].0,
        salvo_syntax::ast::Expr::Binary { .. }
    ));
    assert_eq!(else_block.stmts.len(), 1);
}

// [when-condition] A subject still parses as the arm-matching form, so the
// two grammars stay separate.
#[test]
fn a_when_with_a_subject_is_still_the_arm_form() {
    let expr = parse_expr_stmt("when result {\n        is Ok { 1 }\n        is Err { 2 }\n    }");
    assert!(
        matches!(expr, salvo_syntax::ast::Expr::When { .. }),
        "expected the subject form, got {expr:?}"
    );
}

// [when-condition] `when` is always exhaustive: with no subject there are
// no arms to be exhaustive over, so the `else` is required.
#[test]
fn subjectless_when_without_an_else_is_an_error() {
    let errors = errors_of("fn f() -> Str {\n    return when {\n        n < 0 { \"neg\" }\n    }\n}\n");
    assert!(
        errors.iter().any(|m| m.contains("must end with an `else`")),
        "got {errors:?}"
    );
}

// [when-condition] An `else` with nothing above it decides nothing.
#[test]
fn when_with_only_an_else_is_an_error() {
    let errors = errors_of("fn f() -> Str {\n    return when {\n        else { \"x\" }\n    }\n}\n");
    assert!(
        errors.iter().any(|m| m.contains("nothing to decide")),
        "got {errors:?}"
    );
}

// [when-condition] The `else` is the fall-back, so nothing follows it.
#[test]
fn a_branch_after_the_else_is_an_error() {
    let errors = errors_of(
        "fn f() -> Str {\n    return when {\n        n < 0 { \"neg\" }\n        \
         else { \"other\" }\n        n > 0 { \"pos\" }\n    }\n}\n",
    );
    assert!(
        errors.iter().any(|m| m.contains("last branch")),
        "got {errors:?}"
    );
}

// [when-union-subject] The subject form takes no `else` — it is exhaustive
// over the union's arms. The diagnostic names the subject-less form instead
// of reporting a missing `is`, which is what the reader actually needs.
#[test]
fn an_else_in_the_subject_form_names_the_other_form() {
    let errors = errors_of(
        "fn f() -> Str {\n    return when result {\n        is Ok { \"ok\" }\n        \
         else { \"err\" }\n    }\n}\n",
    );
    assert!(
        errors.iter().any(|m| m.contains("it takes no `else`")),
        "got {errors:?}"
    );
}

// --- platform effects [platform-effect] ---

/// [platform-effect] `platform effect E { ... }` parses as an effect
/// carrying the flag; the modifier is the only difference from an ordinary
/// declaration.
#[test]
fn platform_effect_parses_with_the_flag() {
    let source = "platform effect Telemetry {\n    \
                  fn record(name: Str, value: Int) [] -> [name, value] None\n}\n";
    let (module, diagnostics) = salvo_syntax::parse_module(source);
    assert!(
        !diagnostics.iter().any(|d| d.is_error()),
        "unexpected errors: {:?}",
        diagnostics.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
    let salvo_syntax::ast::Item::Effect(e) = &module.items[0] else {
        panic!("expected an effect item");
    };
    assert!(e.platform, "expected the platform flag to be set");
    assert_eq!(e.name.name, "Telemetry");
    assert_eq!(e.fns.len(), 1);
}

/// [platform-effect] An ordinary `effect` is not a platform effect — the
/// flag defaults off, so nothing existing changes meaning.
#[test]
fn a_plain_effect_is_not_a_platform_effect() {
    let source = "effect Console {\n    fn print(message: Str) -> [message] None\n}\n";
    let (module, _diagnostics) = salvo_syntax::parse_module(source);
    let salvo_syntax::ast::Item::Effect(e) = &module.items[0] else {
        panic!("expected an effect item");
    };
    assert!(!e.platform);
}

/// [platform-effect] `platform` takes nothing but `effect`: a platform
/// declaration groups the functions the host implements. The diagnostic
/// says so rather than reporting a bare "expected item".
#[test]
fn platform_on_a_non_effect_is_an_error_naming_the_form() {
    for source in ["platform type Handle\n", "platform fn now() [] -> [] Int\n"] {
        let (_module, diagnostics) = salvo_syntax::parse_module(source);
        assert!(
            diagnostics.iter().any(|d| d.is_error()
                && d.message.contains("expected `effect` after `platform`")
                && d.message.contains("always an effect")),
            "expected a platform-form error for {source:?}, got {:?}",
            diagnostics.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }
}

// --- Bodiless declarations are gone [decl-body] ---

/// [decl-body] A top-level `fn` with no body was `external fn`'s shape.
/// With `external` gone it is a parse error naming both forms that remain:
/// write a body, or — for something the target language implements —
/// declare it as a member of a `platform effect`.
#[test]
fn a_bodiless_top_level_fn_is_an_error_naming_platform_effect() {
    let errors = errors_of("fn chars(str: Str) [] -> [str] Char[]\n");
    assert!(
        errors
            .iter()
            .any(|m| m.contains("`fn chars` has no body") && m.contains("`platform effect`")),
        "expected a decl-body error naming `platform effect`, got {errors:?}"
    );
}

/// [decl-body] A `type` with no `= ...` alias (and not `intrinsic`) was only
/// ever meaningful as `external type`; with `external` gone it declares
/// nothing, so it is a parse error.
#[test]
fn a_bodiless_non_alias_type_is_an_error() {
    let errors = errors_of("type LinkedList<T>\n");
    assert!(
        errors
            .iter()
            .any(|m| m.contains("`type LinkedList` declares nothing")
                && m.contains("`platform effect`")),
        "expected a decl-body error for a bodiless type, got {errors:?}"
    );
}

/// [decl-body] A `canbe` clause is not a definition: a `type` opting into an
/// auto-qualifier with no alias still declares nothing.
#[test]
fn a_canbe_type_without_an_alias_is_an_error() {
    let errors = errors_of("type List<T> canbe Mut\n");
    assert!(
        errors
            .iter()
            .any(|m| m.contains("declares nothing")),
        "expected a decl-body error, got {errors:?}"
    );
}

/// [decl-body] An alias `type` and an `intrinsic type` are the two forms
/// that do define something, so neither is a decl-body error.
#[test]
fn alias_and_intrinsic_types_are_not_bodiless_errors() {
    for source in ["type Name = Str\n", "intrinsic type Int\n"] {
        let errors = errors_of(source);
        assert!(errors.is_empty(), "unexpected errors for {source:?}: {errors:?}");
    }
}

// --- `external`/`define` are no longer keywords ---

/// `external` is gone (user decision 2026-09-05): it is an ordinary
/// identifier now, so an `external fn`/`external type` declaration no longer
/// parses — the leading identifier is not a valid item.
#[test]
fn external_is_no_longer_a_keyword() {
    for source in [
        "external fn chars(str: Str) [] -> [str] Char[]\n",
        "external type LinkedList<T>\n",
        "external handler StdOutConsole of Console\n",
    ] {
        let errors = errors_of(source);
        assert!(
            errors.iter().any(|m| m.contains("expected item")),
            "expected `external` to fail to parse for {source:?}, got {errors:?}"
        );
    }
}

/// `define` is gone too: the same identifier-at-item-position parse error.
#[test]
fn define_is_no_longer_a_keyword() {
    for source in [
        "define fn list<T>(...elems: T[]) -> List<T> {\n}\n",
        "define type LinkedList<T> {\n}\n",
        "define handler StdOutConsole of Console {\n}\n",
    ] {
        let errors = errors_of(source);
        assert!(
            errors.iter().any(|m| m.contains("expected item")),
            "expected `define` to fail to parse for {source:?}, got {errors:?}"
        );
    }
}

// ===== [implicit-param] [implicit-group] [implicit-override] =====

/// `?cmp: (T, T) -> Int` is one implicit parameter; `?Field<T>` spreads a
/// group. [name-casing] decides which from the first token after `?`, so the
/// grammar needs no lookahead: values are lowercase, types uppercase.
#[test]
fn implicit_parameters_and_group_spreads_parse() {
    let source = "fn sort<T>(list: List<T>, ?cmp: (T, T) -> Int, ?Field<T>) -> List<T> {\n\
                      return list\n\
                  }\n";
    let (module, diagnostics) = salvo_syntax::parse_module(source);
    assert!(
        !diagnostics.iter().any(|d| d.is_error()),
        "unexpected errors: {:?}",
        diagnostics.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
    let salvo_syntax::ast::Item::Fn(f) = &module.items[0] else {
        panic!("expected a fn item");
    };
    assert_eq!(f.params.len(), 2, "the group spread is not a parameter");
    assert!(!f.params[0].implicit, "`list` is an ordinary parameter");
    assert!(f.params[1].implicit, "`cmp` is implicit");
    assert_eq!(f.implicit_groups.len(), 1);
    assert_eq!(f.implicit_groups[0].name.name, "Field");
}

/// A `params` group is a named bundle of member signatures, shaped like an
/// effect declaration because it is the same thing.
#[test]
fn a_params_group_parses() {
    let source = "params Field<T> {\n    fn add(a: T, b: T) -> T\n    fn zero() -> T\n}\n";
    let (module, diagnostics) = salvo_syntax::parse_module(source);
    assert!(
        !diagnostics.iter().any(|d| d.is_error()),
        "unexpected errors: {:?}",
        diagnostics.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
    let salvo_syntax::ast::Item::Params(g) = &module.items[0] else {
        panic!("expected a params item");
    };
    assert_eq!(g.name.name, "Field");
    assert_eq!(g.generics.len(), 1);
    assert_eq!(g.fns.len(), 2);
    assert!(g.fns.iter().all(|f| f.body.is_none()), "members are signatures");
}

/// `name = value` at a call site supplies one implicit parameter. It is
/// recognised by the `=` after an identifier — unambiguous because
/// assignment is a statement in Salvo, never an expression.
#[test]
fn a_named_argument_parses() {
    let source = "fn f() {\n    let x = total(xs, add = times)\n}\n";
    let (module, diagnostics) = salvo_syntax::parse_module(source);
    assert!(
        !diagnostics.iter().any(|d| d.is_error()),
        "unexpected errors: {:?}",
        diagnostics.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
    let salvo_syntax::ast::Item::Fn(f) = &module.items[0] else {
        panic!("expected a fn item");
    };
    let salvo_syntax::ast::Stmt::Let { value, .. } = &f.body.as_ref().unwrap().stmts[0] else {
        panic!("expected a let");
    };
    let salvo_syntax::ast::Expr::Call { args, named, .. } = value else {
        panic!("expected a call");
    };
    assert_eq!(args.len(), 1, "`xs` is the only positional argument");
    assert_eq!(named.len(), 1);
    assert_eq!(named[0].name.name, "add");
}

/// Implicit parameters trail the ordinary ones: a caller could not pass a
/// positional parameter written after one.
#[test]
fn an_ordinary_parameter_after_an_implicit_is_rejected() {
    let source = "fn f(?cmp: (Int, Int) -> Int, n: Int) -> Int {\n    return n\n}\n";
    let (_, diagnostics) = salvo_syntax::parse_module(source);
    assert!(
        diagnostics
            .iter()
            .any(|d| d.is_error() && d.message.contains("implicit parameters must come last")),
        "expected the ordering error, got {:?}",
        diagnostics.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
}

/// A positional argument after a named one has no meaning.
#[test]
fn a_positional_argument_after_a_named_one_is_rejected() {
    let source = "fn f() {\n    let x = total(add = times, xs)\n}\n";
    let (_, diagnostics) = salvo_syntax::parse_module(source);
    assert!(
        diagnostics.iter().any(|d| {
            d.is_error() && d.message.contains("cannot follow a named one")
        }),
        "expected the ordering error, got {:?}",
        diagnostics.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
}
