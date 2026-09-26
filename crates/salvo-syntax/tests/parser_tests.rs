//! Parser integration tests: the standard library and a corpus of docs/language/
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
    assert!(count >= 9, "expected at least 9 std files, found {count}");
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

/// [bytes-type] The byte buffer's surface: an `intrinsic type` that opts into
/// `Mut`, its two constructors, and the one pass declared over it.
#[test]
fn snapshot_std_bytes() {
    insta::assert_debug_snapshot!(parse_clean(&std_core("bytes.sv")));
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

/// [implicit-group] [iter-protocol] The `params` group `for` and every
/// combinator read: with `compare.sv` one of the two places std declares
/// groups, and — with `seq.sv` — the two places a fn signature carries
/// `?Group<...>`.
#[test]
fn snapshot_std_iterator() {
    insta::assert_debug_snapshot!(parse_clean(&std_core("iterator.sv")));
}

/// [cmp-groups] The three capability groups and the canonical implementations
/// for the intrinsic types: `params` declarations beside `intrinsic fn`
/// overloads of one name, which is the shape the whole ordering round is built
/// on.
#[test]
fn snapshot_std_compare() {
    insta::assert_debug_snapshot!(parse_clean(&std_core("compare.sv")));
}

#[test]
fn snapshot_std_seq() {
    insta::assert_debug_snapshot!(parse_clean(&std_core("seq.sv")));
}

// --- docs/language/ example corpus ---

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

// --- `with`: implicits filled together [implicit-with] ---

/// [implicit-with] `=> eq with hash` on a function and on a `params` group, and
/// a **chain** (`a with b with c`), which is one entry naming the rest
/// (user decision 2026-09-26).
#[test]
fn with_entries_relate_implicit_parameters() {
    use salvo_syntax::ast::{DeductionKind, DeductionTarget, Item};
    let source = "params Hashed<T> => eq with hash {\n    \
                  fn hash(value: T) -> Long\n    \
                  fn eq(a: T, b: T) -> Bool\n\
                  }\n\n\
                  fn f<T>(?a: (T) -> Bool, ?b: (T) -> Bool, ?c: (T) -> Bool) -> None \
                  => a with b with c {\n}\n";
    let (module, diagnostics) = salvo_syntax::parse_module(source);
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let Item::Params(group) = &module.items[0] else { panic!("params") };
    assert_eq!(group.deductions.len(), 1, "one entry");
    let DeductionTarget::Param { name, .. } = &group.deductions[0].target else {
        panic!("param target")
    };
    assert_eq!(name.name, "eq");
    let DeductionKind::With { others } = &group.deductions[0].kind else { panic!("with") };
    assert_eq!(others.len(), 1);
    assert_eq!(others[0].name, "hash");

    // A chain is one entry naming every later member.
    let Item::Fn(f) = &module.items[1] else { panic!("fn") };
    let clause = f.deductions.as_ref().expect("clause");
    let chain = clause
        .iter()
        .find_map(|d| match &d.kind {
            DeductionKind::With { others } => Some(others),
            _ => None,
        })
        .expect("a with entry");
    assert_eq!(
        chain.iter().map(|i| i.name.as_str()).collect::<Vec<_>>(),
        vec!["b", "c"]
    );
}

/// A group has no body and no parameters of its own, so its clause states only
/// which members are filled together.
#[test]
fn a_params_groups_clause_takes_only_with_entries() {
    let source = "params Hashed<T> => !hash {\n    fn hash(value: T) -> Long\n}\n";
    let (_, diagnostics) = salvo_syntax::parse_module(source);
    assert!(
        diagnostics
            .iter()
            .any(|d| d.message.contains("states only which of its members are filled together")),
        "{diagnostics:?}"
    );
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

// [group-obligation] `: Group<Args>` on a struct — obligations before the
// `canbe` clause, comma-separated, with type arguments; `Self` in a `params`
// group member parses as an ordinary type name [group-self].
#[test]
fn a_struct_declares_obligations_before_canbe() {
    let source = "struct Lines : Linear, Yield<Str> canbe Mut {\n    name: Str\n}\n\n\
                  params Yield<It, T> {\n    fn next(it: Mut It) -> Emitted T | Finished => it: Mut\n}\n";
    let (module, diagnostics) = salvo_syntax::parse_module(source);
    assert!(
        !diagnostics.iter().any(|d| d.is_error()),
        "unexpected errors: {:?}",
        diagnostics.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
    let salvo_syntax::ast::Item::Struct(s) = &module.items[0] else {
        panic!("expected a struct item");
    };
    let names: Vec<&str> = s
        .obligations
        .iter()
        .map(|o| o.group.name.name.as_str())
        .collect();
    assert_eq!(names, vec!["Linear", "Yield"]);
    assert!(s.obligations[0].group.args.is_empty());
    assert_eq!(s.obligations[1].group.args.len(), 1);
    // [cmp-auto] Neither entry wrote `auto`.
    assert!(s.obligations.iter().all(|o| !o.auto));
    let quals: Vec<&str> = s.auto_qualifiers.iter().map(|q| q.name.name.as_str()).collect();
    assert_eq!(quals, vec!["Mut"]);
    // The group member's `Self` is a plain named type to the parser.
    let salvo_syntax::ast::Item::Params(g) = &module.items[1] else {
        panic!("expected a params item");
    };
    assert_eq!(g.fns.len(), 1);
}

// [group-obligation] The clause is optional in both halves: obligations
// without `canbe`, and neither.
#[test]
fn obligations_parse_without_canbe() {
    let source = "struct Counter : Yield<self, Int> {\n    start: Int\n}\n";
    let (module, diagnostics) = salvo_syntax::parse_module(source);
    assert!(
        !diagnostics.iter().any(|d| d.is_error()),
        "unexpected errors: {:?}",
        diagnostics.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
    let salvo_syntax::ast::Item::Struct(s) = &module.items[0] else {
        panic!("expected a struct item");
    };
    assert_eq!(s.obligations.len(), 1);
    assert!(s.auto_qualifiers.is_empty());
}

// [canbe-optin] [linear-generics] Per-type-parameter opt-in on fns.
#[test]
fn canbe_opts_a_type_parameter_in() {
    let source = "fn hold<T canbe linear>(value: T) -> T {\n    return value\n}\n";
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
    assert_eq!(f.generic_canbe[0].1.name.name, "linear");
}

// [canbe-optin] [linear-generics] The clause is fn-only for now.
#[test]
fn canbe_linear_parses_on_a_struct_type_parameter() {
    // [linear-generics] The conditional-container declaration (user
    // decision 2026-09-12) — until then the clause was fn-only.
    let source = "struct Box<T canbe linear> {\n    item: T\n}\n";
    let (module, diagnostics) = salvo_syntax::parse_module(source);
    let errors: Vec<_> = diagnostics.iter().filter(|d| d.is_error()).collect();
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
    let salvo_syntax::ast::Item::Struct(s) = &module.items[0] else {
        panic!("expected a struct item");
    };
    assert_eq!(s.generic_canbe.len(), 1);
    assert_eq!(s.generic_canbe[0].0.name, "T");
    assert_eq!(s.generic_canbe[0].1.name.name, "linear");
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
                  fn tag(v: Str) -> +Environment.Tag Str {\n    return v\n}\n";
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
         intrinsic fn e(x: Int) [] -> Int\n\
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
                  fn record(name: Str, value: Int) [] -> None => name, value\n}\n";
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
    let source = "effect Console {\n    fn print(message: Str) -> None => message\n}\n";
    let (module, _diagnostics) = salvo_syntax::parse_module(source);
    let salvo_syntax::ast::Item::Effect(e) = &module.items[0] else {
        panic!("expected an effect item");
    };
    assert!(!e.platform);
}

/// [platform-effect] [platform-handler] `platform` takes `effect` or
/// `handler`, and nothing else. The diagnostic names both forms rather than
/// reporting a bare "expected item".
#[test]
fn platform_on_a_non_effect_is_an_error_naming_the_form() {
    for source in ["platform type Handle\n", "platform fn now() [] -> Int\n"] {
        let (_module, diagnostics) = salvo_syntax::parse_module(source);
        assert!(
            diagnostics.iter().any(|d| d.is_error()
                && d.message
                    .contains("expected `effect` or `handler` after `platform`")
                && d.message.contains("`platform handler`")),
            "expected a platform-form error for {source:?}, got {:?}",
            diagnostics.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }
}

// --- platform handlers [platform-handler] ---

/// [platform-handler] `platform handler HostRawFs of RawFs` parses as a
/// handler carrying the flag: bodyless, and otherwise the same grammar as
/// any handler — the modifier says only who supplies the members.
#[test]
fn platform_handler_parses_with_the_flag() {
    let source = "platform handler HostRawFs of RawFs\n";
    let (module, diagnostics) = salvo_syntax::parse_module(source);
    assert!(
        !diagnostics.iter().any(|d| d.is_error()),
        "unexpected errors: {:?}",
        diagnostics.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
    let salvo_syntax::ast::Item::Handler(h) = &module.items[0] else {
        panic!("expected a handler item");
    };
    assert!(h.platform, "expected the platform flag to be set");
    assert!(!h.intrinsic, "`platform` is not `intrinsic`");
    assert_eq!(h.name.name, "HostRawFs");
    assert!(h.fns.is_empty() && h.state.is_empty());
}

/// [platform-handler] Constructor parameters are the host class's, so the
/// grammar keeps them: `use HostS3("bucket")` passes them through.
#[test]
fn platform_handler_takes_constructor_parameters() {
    let source = "platform handler HostS3(bucket: Str) of Fs\n";
    let (module, diagnostics) = salvo_syntax::parse_module(source);
    assert!(
        !diagnostics.iter().any(|d| d.is_error()),
        "unexpected errors: {:?}",
        diagnostics.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
    let salvo_syntax::ast::Item::Handler(h) = &module.items[0] else {
        panic!("expected a handler item");
    };
    assert!(h.platform);
    assert_eq!(h.params.len(), 1);
    assert_eq!(h.params[0].name.name, "bucket");
}

/// [platform-handler] A plain or `intrinsic` handler is not a platform
/// handler — the flag defaults off, so nothing existing changes meaning.
#[test]
fn a_plain_handler_is_not_a_platform_handler() {
    let source = "handler Loud of Console {\n    \
                  fn print(message: Str) -> None => message {\n    }\n}\n\
                  intrinsic handler StdOutConsole of Console\n";
    let (module, _diagnostics) = salvo_syntax::parse_module(source);
    for item in &module.items {
        let salvo_syntax::ast::Item::Handler(h) = item else {
            panic!("expected handler items");
        };
        assert!(!h.platform, "`{}` should not be platform", h.name.name);
    }
}

// --- Bodiless declarations are gone [decl-body] ---

/// [decl-body] A top-level `fn` with no body was `external fn`'s shape.
/// With `external` gone it is a parse error naming both forms that remain:
/// write a body, or — for something the target language implements —
/// declare it as a member of a `platform effect`.
#[test]
fn a_bodiless_top_level_fn_is_an_error_naming_platform_effect() {
    let errors = errors_of("fn chars(str: Str) [] -> Char[] => str\n");
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
        "external fn chars(str: Str) [] -> Char[] => str\n",
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
        "define fn list_of<T>(...elems: T[]) -> List<T> {\n}\n",
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

// --- Qualifier refinements [qual-refn] ---

/// [qual-refn] A refinement in a qualifier body parses into the qualifier's
/// own `refns` list, carrying its parameters (which pick the overload
/// [qual-refn-match]) and a deduction list of additions and removals.
#[test]
fn a_refinement_parses_in_a_qualifier_body() {
    let source = "qualifier NonEmpty<T> of List<T> {\n    \
                  fn qualifies(list: List<T>) -> Bool {\n        \
                  return true\n    }\n\n    \
                  // Adding an element makes the list non-empty.\n    \
                  refn add(list: Mut List<T>, elem: T) => list: +NonEmpty -Sorted\n}\n";
    let (module, diagnostics) = salvo_syntax::parse_module(source);
    assert!(
        !diagnostics.iter().any(|d| d.is_error()),
        "unexpected errors: {:?}",
        diagnostics.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
    let salvo_syntax::ast::Item::Qualifier(q) = &module.items[0] else {
        panic!("expected a qualifier item");
    };
    assert_eq!(q.fns.len(), 1, "the `qualifies` fn is still a fn");
    assert_eq!(q.refns.len(), 1);
    let refn = &q.refns[0];
    assert_eq!(refn.name.name, "add");
    assert_eq!(refn.params.len(), 2);
    assert_eq!(refn.params[0].name.name, "list");
    // [doc-comment] The block above it is its documentation, which hover
    // merges into the refined fn's [qual-refn-docs].
    assert_eq!(
        refn.docs,
        vec!["Adding an element makes the list non-empty.".to_string()]
    );
    assert_eq!(refn.deductions.len(), 1);
    let entry = &refn.deductions[0];
    assert_eq!(entry.param.name, "list");
    assert_eq!(
        entry.add.iter().map(|r| r.name.name.as_str()).collect::<Vec<_>>(),
        vec!["NonEmpty"]
    );
    assert_eq!(
        entry.remove.iter().map(|r| r.name.name.as_str()).collect::<Vec<_>>(),
        vec!["Sorted"]
    );
}

/// [qual-refn-scope] A top-level `refn` is an item of its own — the form
/// that reconciles two qualifiers' conflicting refinements in one's own
/// [qual-refn-scope] A refinement belongs to the qualifier whose claim it is
/// about, so the **top-level** form is refused, naming the move (user decision
/// 2026-09-26). The declaration still parses — the diagnostic names it — so one
/// misplaced refinement produces one error rather than a cascade.
#[test]
fn a_top_level_refinement_is_refused() {
    let source = "refn add<T>(list: Mut List<T>, elem: T) => list: +NonEmpty\n";
    let (module, diagnostics) = salvo_syntax::parse_module(source);
    let msgs: Vec<String> = diagnostics
        .iter()
        .filter(|d| d.is_error())
        .map(|d| d.message.clone())
        .collect();
    assert_eq!(msgs.len(), 1, "one misplaced refinement, one error: {msgs:?}");
    assert!(
        msgs[0].contains("belongs to the qualifier") && msgs[0].contains("refn add"),
        "{}",
        msgs[0]
    );
    let salvo_syntax::ast::Item::Refn(r) = &module.items[0] else {
        panic!("expected a refn item");
    };
    assert_eq!(r.name.name, "add");
}

/// [qual-refn] The three things a refinement may not say are diagnostics
/// naming the reason, not bare parse errors: it changes what is *known*
/// after a call, never what the call does.
#[test]
fn a_refinement_may_not_declare_effects_a_return_type_or_an_unsigned_qualifier() {
    let cases = [
        (
            "refn add<T>(list: Mut List<T>) [Console] => list: +NonEmpty\n",
            "cannot declare effects",
        ),
        (
            "refn add<T>(list: Mut List<T>) -> Int => list: +NonEmpty\n",
            "cannot declare a return type",
        ),
        (
            "refn add<T>(list: Mut List<T>) => list: NonEmpty\n",
            "need a sign",
        ),
        (
            "refn add<T>(list: Mut List<T>)\n",
            "expected a deduction clause",
        ),
    ];
    for (source, expected) in cases {
        let (_module, diagnostics) = salvo_syntax::parse_module(source);
        assert!(
            diagnostics
                .iter()
                .any(|d| d.is_error() && d.message.contains(expected)),
            "expected {expected:?} for {source:?}, got {:?}",
            diagnostics.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }
}

// ===== [fn-overload-at] the scope selector, [fn-rename] the rename =====

/// [fn-overload-at] `f@core.list(x)` parses as a call whose callee names the
/// module, in plain and dot form, and as a *value*.
#[test]
fn a_scope_selector_parses_on_names_and_dot_calls() {
    for source in [
        "fn probe() -> None {\n    let n = size@core.list(xs)\n}\n",
        "fn probe() -> None {\n    let n = xs.size@core.list()\n}\n",
        "fn probe() -> None {\n    let f = describe@main\n}\n",
        "fn probe() -> None {\n    let n = size@a.b.c(xs)\n}\n",
    ] {
        let errors = errors_of(source);
        assert!(errors.is_empty(), "unexpected errors for {source:?}: {errors:?}");
    }
}

/// The selector belongs to a *name*, so writing it after anything else is a
/// parse error naming both places it may go.
#[test]
fn a_scope_selector_needs_a_name() {
    let errors = errors_of("fn probe() -> None {\n    let n = (1 + 2)@core.list\n}\n");
    assert!(
        errors
            .iter()
            .any(|m| m.contains("`@` selects which module's overload")),
        "expected the `@` placement error, got {errors:?}"
    );
}

/// [effect-at] A *capitalized* name after `@` is an effect selector:
/// `close@Fs(s)` parses as a call whose callee names the effect, in plain
/// and dot form, and with the call's type arguments pinning a generic
/// instance (`next_random@Random<Int>()`).
#[test]
fn an_effect_selector_parses_on_names_and_dot_calls() {
    for source in [
        "fn probe() -> None {\n    let s = close@Fs(h)\n}\n",
        "fn probe() -> None {\n    let s = h.close@Fs()\n}\n",
        "fn probe() -> None {\n    let n = next_random@Random<Int>()\n}\n",
    ] {
        let errors = errors_of(source);
        assert!(errors.is_empty(), "unexpected errors for {source:?}: {errors:?}");
    }
    // The AST shape: an `EffectScoped` callee carrying the effect's name.
    let (module, diagnostics) =
        salvo_syntax::parse_module("fn probe() -> None {\n    let s = close@Fs(h)\n}\n");
    assert!(diagnostics.iter().all(|d| !d.is_error()));
    let printed = format!("{module:?}");
    assert!(
        printed.contains("EffectScoped"),
        "expected an EffectScoped callee: {printed}"
    );
}

/// [effect-at] The effect selector follows a name too.
#[test]
fn an_effect_selector_needs_a_name() {
    let errors = errors_of("fn probe() -> None {\n    let n = (1 + 2)@Fs\n}\n");
    assert!(
        errors
            .iter()
            .any(|m| m.contains("`@` selects which effect's member")),
        "expected the `@` placement error, got {errors:?}"
    );
}

/// [fn-rename] `rename fn add2 = add(a: Int, b: Int)`, at module level and as
/// a statement, with type parameters where the overload has them.
#[test]
fn a_rename_parses_at_module_and_statement_level() {
    for source in [
        "rename fn add2 = add(a: Int, b: Int)\n",
        "rename fn list_size = size<T>(list: List<T>)\n",
        "fn probe() -> None {\n    rename fn tag2 = tag(v: Str)\n    let s = tag2(\"x\")\n}\n",
        "fn probe() -> None {\n    for i in xs {\n        rename fn tag2 = tag(v: Int)\n    }\n}\n",
    ] {
        let errors = errors_of(source);
        assert!(errors.is_empty(), "unexpected errors for {source:?}: {errors:?}");
    }
}

/// A rename names an overload by its parameters, so anything that takes no
/// part in *choosing* one is refused by name: an effect list, a deduction
/// list, a return type, and an implicit-group spread.
#[test]
fn a_rename_takes_no_effects_deductions_or_return_type() {
    for (source, needle) in [
        (
            "rename fn add2 = add(a: Int) [Console]\n",
            "takes no effect or deduction clause",
        ),
        (
            // The `=>` is reported: a deduction clause is not part of a rename.
            "rename fn add2 = add(a: Int) => a\n",
            "takes no effect or deduction",
        ),
        ("rename fn add2 = add(a: Int) -> Int\n", "takes no return type"),
        (
            "rename fn total2 = total(xs: List<Int>, ?Field<Int>)\n",
            "implicit parameters take no part",
        ),
    ] {
        let errors = errors_of(source);
        assert!(
            errors.iter().any(|m| m.contains(needle)),
            "expected {needle:?} for {source:?}, got {errors:?}"
        );
    }
}

/// [iter-fn] The `iter fn` expansion, as the rest of the compiler sees it: a
/// hidden `__Pass_Countdown` carrying the subject and the `state` fields, an
/// `iter` that mints one (copying the subject, so a second drive starts over),
/// and the author's body as an ordinary `next` with every field written out.
///
/// Snapshotted rather than asserted piecemeal because the *whole* shape is the
/// contract: this is the only place the generated declarations are visible.
#[test]
fn snapshot_iter_fn_expansion() {
    let source = "\
struct Countdown {
    from: Int
}

iter fn next(c: Countdown) -> Emitted Int | Finished {
    state {
        at: Int = c.from
    }
    if at <= 0 {
        return finished()
    }
    at = at - 1
    return emitted(at + 1)
}
";
    let (module, diagnostics) = salvo_syntax::parse_module(source);
    let errors: Vec<_> = diagnostics.iter().filter(|d| d.is_error()).collect();
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
    insta::assert_debug_snapshot!(module);
}

/// [iter-fn] The per-field snapshot across files: with the program-level
/// expansion (`parse_module_deferred` + `expand_iter_fns_with`), a subject
/// declared in *another* file still gets one snapshot field per field read,
/// instead of falling back to holding the whole value — the roadmap's
/// "mutable origins, option (e)" residue. The same source expanded with
/// this file's declarations alone keeps the whole-subject fallback.
#[test]
fn a_foreign_subject_gets_the_per_field_snapshot() {
    let subject_file = "\
struct Countdown {
    from: Int,
    label: Str
}
";
    let iter_file = "\
iter fn next(c: Countdown) -> Emitted Int | Finished {
    state {
        at: Int = 0
    }
    if at >= c.from {
        return finished()
    }
    at = at + 1
    return emitted(at)
}
";
    let pass_fields = |module: &salvo_syntax::ast::Module| -> Vec<String> {
        module
            .items
            .iter()
            .find_map(|item| match item {
                salvo_syntax::ast::Item::Struct(s) if s.name.name.starts_with("__Pass_") => {
                    Some(s.fields.iter().map(|f| f.name.name.clone()).collect())
                }
                _ => None,
            })
            .expect("the generated pass struct")
    };

    // Program-level expansion: the foreign declaration is visible, so the
    // pass holds only the one field the body reads (plus the state field).
    let (subject_module, diags) = salvo_syntax::parse_module_deferred(subject_file);
    assert!(diags.iter().all(|d| !d.is_error()), "{diags:?}");
    let (mut iter_module, diags) = salvo_syntax::parse_module_deferred(iter_file);
    assert!(diags.iter().all(|d| !d.is_error()), "{diags:?}");
    let externs = salvo_syntax::desugar::struct_decls(&subject_module);
    let diags = salvo_syntax::desugar::expand_iter_fns_with(&mut iter_module, &externs);
    assert!(diags.iter().all(|d| !d.is_error()), "{diags:?}");
    assert_eq!(
        pass_fields(&iter_module),
        vec!["from".to_string(), "at".to_string()],
        "expected the per-field snapshot for a foreign subject"
    );

    // Single-file expansion of the same source: the declaration is not
    // visible, so the pass keeps the whole subject.
    let (fallback_module, diags) = salvo_syntax::parse_module(iter_file);
    assert!(diags.iter().all(|d| !d.is_error()), "{diags:?}");
    assert_eq!(
        pass_fields(&fallback_module),
        vec!["__subject".to_string(), "at".to_string()],
        "a single-file parse cannot see the declaration, so the whole value rides"
    );
}

/// [deduce-syntax] The `=>` clause: every entry form, a group scoped to a
/// fn-typed parameter, and a clause continued on the next line. The bracket
/// form after `->` is a plain parse error (no compatibility [decision
/// 2026-09-03]). The opaque lends are the return-type annotation
/// `-> T holds proj(a)` [proj-infer] (user decision 2026-09-24), which the
/// parser synthesizes into the clause as a `DeductionTarget::Opaque` entry.
#[test]
fn the_deduction_clause_parses_every_entry_form() {
    use salvo_syntax::ast::{DeductionKind, DeductionTarget, Item, Type};
    let src = "fn f<T, U>(a: List<T>, b: List<U>, c: Mut View, keep: (t: T) -> Bool, d: Q Int, e: Int) -> Pair<T, U> holds proj(a)\n\
               =>[keep] !t\n\
               => !a, b: None, c: Mut, d: -Q, .first: proj(a), .second: proj(a, b), c.items: proj(b) {\n\
               }\n";
    let (module, diags) = salvo_syntax::parse_module(src);
    let errs: Vec<_> = diags.iter().filter(|d| d.is_error()).collect();
    assert!(errs.is_empty(), "{errs:?}");
    let Item::Fn(f) = &module.items[0] else { panic!("expected a fn") };
    let list = f.deductions.as_ref().expect("own clause");
    let kinds: Vec<String> = list
        .iter()
        .map(|d| match (&d.target, &d.kind) {
            (DeductionTarget::Param { name, path }, k) if path.is_empty() => {
                format!("{}:{}", name.name, kind_name(k))
            }
            (DeductionTarget::Param { name, path }, DeductionKind::Proj(srcs)) => format!(
                "{}.{}<-{}",
                name.name,
                path.iter().map(|p| p.name.as_str()).collect::<Vec<_>>().join("."),
                srcs.iter().map(|s| s.name.as_str()).collect::<Vec<_>>().join(",")
            ),
            (DeductionTarget::Result { path }, DeductionKind::Proj(srcs)) => format!(
                ".{}<-{}",
                path.iter().map(|p| p.name.as_str()).collect::<Vec<_>>().join("."),
                srcs.iter().map(|s| s.name.as_str()).collect::<Vec<_>>().join(",")
            ),
            (DeductionTarget::Opaque, DeductionKind::Proj(srcs)) => format!(
                "<-{}",
                srcs.iter().map(|s| s.name.as_str()).collect::<Vec<_>>().join(",")
            ),
            other => panic!("unexpected entry {other:?}"),
        })
        .collect();
    assert_eq!(
        kinds,
        vec![
            "a:Moved",
            "b:Exhaustive[]",
            "c:Exhaustive[Mut]",
            "d:Remove[Q]",
            ".first<-a",
            ".second<-a,b",
            "c.items<-b",
            "<-a",
        ]
    );
    // The `=>[keep]` group landed on the fn type.
    let keep = f.params.iter().find(|p| p.name.name == "keep").unwrap();
    let Type::Fn { deductions: Some(group), .. } = &keep.ty else { panic!("group") };
    assert_eq!(group.len(), 1);
    assert!(matches!(group[0].kind, DeductionKind::Moved));

    fn kind_name(k: &DeductionKind) -> String {
        match k {
            DeductionKind::KeepAll => "KeepAll".into(),
            DeductionKind::Moved => "Moved".into(),
            DeductionKind::Deferred => "Deferred".into(),
            // [deduce-reapply] A re-applied claim shows its `+`: it is a
            // different statement from one that merely survived.
            DeductionKind::Exhaustive { quals, reapplied } => {
                let mut shown: Vec<String> =
                    quals.iter().map(|r| r.name.name.clone()).collect();
                shown.extend(reapplied.iter().map(|r| format!("+{}", r.name.name)));
                format!("Exhaustive[{}]", shown.join(" "))
            }
            DeductionKind::Remove(q) => {
                format!("Remove[{}]", q.iter().map(|r| r.name.name.as_str()).collect::<Vec<_>>().join(" "))
            }
            DeductionKind::Proj(_) => "proj".into(),
            DeductionKind::Preserve(q) => {
                format!("Preserve[{}]", q.iter().map(|r| r.name.name.as_str()).collect::<Vec<_>>().join(" "))
            }
            // [canbe-entry] The alias-group relation.
            DeductionKind::CanBe { others, anchored } => {
                let shown: Vec<String> = others
                    .iter()
                    .map(|p| p.iter().map(|i| i.name.clone()).collect::<Vec<_>>().join("."))
                    .collect();
                let head = if *anchored { "CanBeIn" } else { "CanBe" };
                format!("{head}[{}]", shown.join("|"))
            }
            // [implicit-with] The fill-together relation.
            DeductionKind::With { others } => format!(
                "With[{}]",
                others.iter().map(|i| i.name.clone()).collect::<Vec<_>>().join(" ")
            ),
        }
    }
}

/// [deduce-syntax] The old bracket list is refused with a pointer to `=>`;
/// a fn type may not carry an inline list; a group must name a fn-typed
/// parameter; a result path can only project.
#[test]
fn the_deduction_clause_rejects_the_old_and_ill_formed_shapes() {
    let errs = errors_of("fn f(a: Int) -> [a] Int {\n}\n");
    assert!(errs.iter().any(|e| e.contains("behind `=>`")), "{errs:?}");
    let errs = errors_of("fn f(g: (v: Int) -> [v] Int) -> Int {\n}\n");
    assert!(errs.iter().any(|e| e.contains("`=>[name] …`")), "{errs:?}");
    let errs = errors_of("fn f(a: Int) -> Int =>[b] !a {\n}\n");
    assert!(errs.iter().any(|e| e.contains("names no parameter")), "{errs:?}");
    let errs = errors_of("fn f(a: Int) -> Int =>[a] !a {\n}\n");
    assert!(errs.iter().any(|e| e.contains("is not one")), "{errs:?}");
    let errs = errors_of("fn f(a: Int) -> Int => .x: Mut {\n}\n");
    assert!(errs.iter().any(|e| e.contains("can only state a projection")), "{errs:?}");
    let errs = errors_of("fn f(a: Int) -> Int => a: {\n}\n");
    assert!(errs.iter().any(|e| e.contains("`None` to strip every")), "{errs:?}");
}

// [obligation-spelling] The obligation keywords parse in qualifier
// position, carry their lowercase spelling as the qualifier name, and
// `proj` still takes its `[from: …]` bracket.
#[test]
fn obligation_keywords_parse_in_type_positions() {
    let source = "fn f(step: Emitted (proj Str) | Finished, g: once (Int) -> Str) \
                  -> proj(step) Str => step, g {\n    return \"x\"\n}\n";
    let (module, diagnostics) = salvo_syntax::parse_module(source);
    let errors: Vec<_> = diagnostics.iter().filter(|d| d.is_error()).collect();
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
    let salvo_syntax::ast::Item::Fn(f) = &module.items[0] else {
        panic!("expected a fn item");
    };
    // `once (Int) -> Str`: the keyword qualifies the fn type.
    let salvo_syntax::ast::Type::QualifiedGroup { qualifiers, .. } = &f.params[1].ty else {
        panic!("expected a qualified fn type, got {:?}", f.params[1].ty);
    };
    assert_eq!(qualifiers[0].name.name, "once");
    // The return type is `proj(step) Str`.
    let Some(salvo_syntax::ast::Type::Named { qualifiers, .. }) = &f.return_type else {
        panic!("expected a named return type");
    };
    assert_eq!(qualifiers[0].name.name, "proj");
    assert_eq!(qualifiers[0].from[0].name, "step");
}

// [obligation-spelling] `is once …` narrows through the keyword like any
// qualifier, and the keywords stay usable nowhere else: a value named
// `proj` is a parse error, which is what "reserved" means.
#[test]
fn obligation_keywords_are_reserved() {
    let source = "fn f() {\n    let proj = 1\n}\n";
    let (_module, diagnostics) = salvo_syntax::parse_module(source);
    assert!(
        diagnostics.iter().any(|d| d.is_error()),
        "expected `let proj` to be a parse error"
    );
}

/// [actor-send-fn] [actor-spawn-effect] The asynchronous surface's
/// declaration forms: `send fn` members of an effect and of a handler, and
/// the `[spawn]` capability in an effect list. Both new words are
/// **contextual** — the test below also declares a *state field* named
/// `send` and an ordinary `fn` named `spawn` to prove nothing was reserved.
#[test]
fn send_members_and_the_spawn_capability_parse() {
    let source = "\
actor effect Counter {
    send fn bump(n: Int)
    send fn report(out: Reply<Int>)
}

handler Counting() of Counter {
    sum: Int = 0

    send fn bump(n: Int) {
        sum = sum + n
    }

    send fn report(out: Reply<Int>) {
        send(out, sum)
    }
}

fn main() [use, spawn] {
    let c = 0
}
";
    let (module, diagnostics) = salvo_syntax::parse_module(source);
    let errors: Vec<_> = diagnostics.iter().filter(|d| d.is_error()).collect();
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");

    let mut send_members = Vec::new();
    let mut plain_members = Vec::new();
    for item in &module.items {
        match item {
            salvo_syntax::ast::Item::Effect(e) => {
                for f in &e.fns {
                    if f.is_send {
                        send_members.push(format!("effect {}.{}", e.name.name, f.name.name));
                    } else {
                        plain_members.push(format!("effect {}.{}", e.name.name, f.name.name));
                    }
                }
            }
            salvo_syntax::ast::Item::Handler(h) => {
                for f in &h.fns {
                    if f.is_send {
                        send_members.push(format!("handler {}.{}", h.name.name, f.name.name));
                    } else {
                        plain_members.push(format!("handler {}.{}", h.name.name, f.name.name));
                    }
                }
                // The state field is still a field, not swallowed by the
                // member loop's new `send` case.
                assert_eq!(h.state.len(), 1, "the handler's state field is lost");
                assert_eq!(h.state[0].name.name, "sum");
            }
            salvo_syntax::ast::Item::Fn(f) if f.name.name == "main" => {
                let effects = f.effects.as_ref().expect("main declares effects");
                assert!(
                    effects
                        .iter()
                        .any(|e| matches!(e, salvo_syntax::ast::EffectRef::Use(_))),
                    "the `use` effect is lost"
                );
                assert!(
                    effects
                        .iter()
                        .any(|e| matches!(e, salvo_syntax::ast::EffectRef::Spawn(_))),
                    "the `spawn` capability is lost"
                );
            }
            _ => {}
        }
    }
    assert_eq!(
        send_members,
        vec![
            "effect Counter.bump",
            "effect Counter.report",
            "handler Counting.bump",
            "handler Counting.report",
        ]
    );
    assert!(
        plain_members.is_empty(),
        "these members lost their `send`: {plain_members:?}"
    );
}

/// [actor-send-fn] Neither new word is reserved: `send` remains usable as a
/// field and a function name, and `spawn` as a function name — which is what
/// makes `r.send(v)` (a reply's discharge) and a user's own `spawn` legal.
#[test]
fn send_and_spawn_are_not_reserved_words() {
    let source = "\
struct Mailer {
    send: Int
}

fn send(to: Int, what: Str) -> Int {
    return to
}

fn spawn(n: Int) -> Int {
    return n
}

fn use_them(m: Mailer) -> Int {
    let a = send(1, \"hi\")
    let b = spawn(2)
    return m.send + a + b
}
";
    let (_module, diagnostics) = salvo_syntax::parse_module(source);
    let errors: Vec<_> = diagnostics.iter().filter(|d| d.is_error()).collect();
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
}

/// [actor-spawn-expr] [actor-replyto] [actor-waitfor] The asynchronous
/// surface's *expression* forms, in one program that uses every clause:
/// `spawn` with a spawn's `with` clause and an `on` pool (the mailbox is the
/// handler's slot [actor-mailbox]); `replyto` and its gated `replyto!`; and
/// `waitfor`, `main`'s bridge.
#[test]
fn the_asynchronous_expression_forms_parse() {
    use salvo_syntax::ast::{Expr, Item, Stmt};

    let source = "\
actor effect Counter {
    send fn bump(n: Int)
    send fn total(out: Reply<Int>)
    send fn totalled(n: Int)
}

handler Counting() of Counter {
    sum: Int = 0

    send fn bump(n: Int) {
        sum = sum + n
    }

    send fn total(out: Reply<Int>) {
        out.send(sum)
    }

    send fn totalled(n: Int) {
        total(replyto totalled())
        total(replyto! totalled(1, \"tag\"))
    }
}

fn main() [use, spawn] {
    let counter = spawn Counting() on pool(2)
    let audited = spawn Counting() with counter, Counting() on pool(1)
    use counter
    let sum = waitfor out: Reply<Int> {
        counter.total(out)
    }
}
";
    let (module, diagnostics) = salvo_syntax::parse_module(source);
    let errors: Vec<_> = diagnostics.iter().filter(|d| d.is_error()).collect();
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");

    // The two `replyto` forms, inside the handler member that mints them.
    let mut replytos = Vec::new();
    for item in &module.items {
        if let Item::Handler(h) = item {
            for f in &h.fns {
                collect_replyto(f.body.as_ref(), &mut replytos);
            }
        }
    }
    assert_eq!(
        replytos,
        vec![
            ("totalled".to_string(), false, 0),
            ("totalled".to_string(), true, 2),
        ],
        "the `replyto` mints, as (member, gated, captures)"
    );

    // `main`'s three statements: two spawns and the bridge.
    let main = module
        .items
        .iter()
        .find_map(|i| match i {
            Item::Fn(f) if f.name.name == "main" => Some(f),
            _ => None,
        })
        .expect("main is declared");
    let body = main.body.as_ref().expect("main has a body");
    let values: Vec<&Expr> = body
        .stmts
        .iter()
        .filter_map(|s| match s {
            Stmt::Let { value, .. } => Some(value),
            _ => None,
        })
        .collect();
    assert_eq!(values.len(), 3, "main binds three values");

    // `use addr` is first-pass surface, not sugar: it needs no new syntax —
    // the `use` statement already takes an expression, and a bare name here
    // is an `Addr` rather than a handler construction. Distinguishing them is
    // the checker's job.
    let bound: Vec<&str> = body
        .stmts
        .iter()
        .filter_map(|s| match s {
            Stmt::Use { handler, .. } => match handler {
                Expr::Ident(id) => Some(id.name.as_str()),
                _ => None,
            },
            _ => None,
        })
        .collect();
    assert_eq!(bound, vec!["counter"], "`use counter` binds the Addr");

    // A spawn with no `use` clause still carries both required clauses.
    match values[0] {
        Expr::Spawn {
            handler,
            with_items,
            pool,
            ..
        } => {
            assert!(
                matches!(handler.as_ref(), Expr::Call { .. }),
                "the handler construction is a call: {handler:?}"
            );
            assert!(with_items.is_empty(), "no `with` clause was written");
            assert!(
                matches!(pool.as_deref(), Some(Expr::Call { .. })),
                "`on pool(2)` is an ordinary call: {pool:?}"
            );
        }
        other => panic!("expected a spawn, got {other:?}"),
    }

    // The `with` clause takes both an `Addr` value and a handler construction.
    match values[1] {
        Expr::Spawn { with_items, .. } => {
            assert_eq!(
                with_items.len(),
                2,
                "two dependencies were supplied: {with_items:?}"
            );
            assert!(
                matches!(&with_items[0], Expr::Ident(id) if id.name == "counter"),
                "the first is an Addr value: {:?}",
                with_items[0]
            );
            assert!(
                matches!(&with_items[1], Expr::Call { .. }),
                "the second is a handler construction: {:?}",
                with_items[1]
            );
        }
        other => panic!("expected a spawn, got {other:?}"),
    }

    match values[2] {
        Expr::WaitFor { binding, ty, .. } => {
            assert_eq!(binding.name, "out");
            // [waitfor-infer] The type is optional; written here.
            let ty = ty.as_ref().expect("a written type");
            assert_eq!(format!("{ty}"), "Reply<Int>");
        }
        other => panic!("expected a waitfor, got {other:?}"),
    }
}

/// Collects every `replyto` in a body as (member, gated, capture count).
fn collect_replyto(
    body: Option<&salvo_syntax::ast::Block>,
    out: &mut Vec<(String, bool, usize)>,
) {
    use salvo_syntax::ast::{Expr, Stmt};
    let Some(body) = body else { return };
    fn walk(expr: &Expr, out: &mut Vec<(String, bool, usize)>) {
        match expr {
            Expr::ReplyTo {
                member,
                captures,
                gated,
                ..
            } => out.push((member.name.clone(), *gated, captures.len())),
            Expr::Call { args, .. } => {
                for a in args {
                    walk(a, out);
                }
            }
            _ => {}
        }
    }
    for stmt in &body.stmts {
        if let Stmt::Expr(e) = stmt {
            walk(e, out);
        }
    }
}

/// [free-send-fn] [task-pool-inherit] The task kernel's two bits of grammar: a
/// top-level `send fn`, and a mint's optional `on POOL` clause. Both words stay
/// **contextual** — `send` is a function and a method name, `on` an ordinary
/// identifier — which is what the last assertion checks.
#[test]
fn a_free_send_fn_and_a_placed_mint_parse() {
    use salvo_syntax::ast::{Expr, Item, Stmt};

    let src = "\
send fn finish(label: Str, out: Reply<Str>, total: Int) => !label, !out, !total {
    out.send(\"${label}=${total}\")
}

fn fetch(out: Reply<Str>) [Counter] -> None => !out {
    total(replyto finish(\"count\", out) on pool(2))
}

fn plain(out: Reply<Str>) [Counter] -> None => !out {
    total(replyto finish(\"count\", out))
}

fn on(n: Int) -> Int {
    let send = n
    return send
}
";
    let (module, diagnostics) = salvo_syntax::parse_module(src);
    let errors: Vec<_> = diagnostics.iter().filter(|d| d.is_error()).collect();
    assert!(errors.is_empty(), "the kernel's grammar parses: {errors:?}");

    let fns: Vec<(&str, bool)> = module
        .items
        .iter()
        .filter_map(|i| match i {
            Item::Fn(f) => Some((f.name.name.as_str(), f.is_send)),
            _ => None,
        })
        .collect();
    assert_eq!(
        fns,
        vec![("finish", true), ("fetch", false), ("plain", false), ("on", false)],
        "a top-level `send fn` carries the kind on the declaration"
    );

    // The mint's placement: written in `fetch`, omitted in `plain`.
    let mut pools: Vec<bool> = Vec::new();
    for item in &module.items {
        let Item::Fn(f) = item else { continue };
        let Some(body) = &f.body else { continue };
        for stmt in &body.stmts {
            let Stmt::Expr(Expr::Call { args, .. }) = stmt else {
                continue;
            };
            for a in args {
                if let Expr::ReplyTo { pool, .. } = a {
                    pools.push(pool.is_some());
                }
            }
        }
    }
    assert_eq!(
        pools,
        vec![true, false],
        "`on POOL` is optional at a mint: {pools:?}"
    );
}

/// [actor-spawn-expr] [actor-mailbox] [main-pool] The clause words are
/// **contextual**, and both clauses are optional: an omitted `on` parses to no
/// pool expression at all, which means the pool current where the spawn was
/// written (user decision 2026-09-17, FC-4(a) — that is how a spawn names the
/// main pool). The mailbox bound is not a clause either — it is the handler's
/// `mailbox` slot (user decision 2026-09-16), which is why `capacity` is an
/// ordinary word again everywhere except inside that block.
#[test]
fn a_spawn_states_its_pool() {
    let no_pool = "\
fn main() [use, spawn] {
    let h = spawn H()
}
";
    let (module, diagnostics) = salvo_syntax::parse_module(no_pool);
    let errors: Vec<_> = diagnostics.iter().filter(|d| d.is_error()).collect();
    assert!(errors.is_empty(), "`on` is optional: {errors:?}");
    let main = module
        .items
        .iter()
        .find_map(|i| match i {
            salvo_syntax::ast::Item::Fn(f) if f.name.name == "main" => Some(f),
            _ => None,
        })
        .expect("main is declared");
    let bound = main
        .body
        .as_ref()
        .and_then(|b| {
            b.stmts.iter().find_map(|s| match s {
                salvo_syntax::ast::Stmt::Let { value, .. } => Some(value),
                _ => None,
            })
        })
        .expect("the spawn is bound to a local");
    match bound {
        salvo_syntax::ast::Expr::Spawn { pool, .. } => assert!(
            pool.is_none(),
            "an omitted `on` carries no pool expression: {pool:?}"
        ),
        other => panic!("expected a spawn, got {other:?}"),
    }

    // `capacity` outside the slot is an ordinary identifier.
    let ordinary = "\
fn main() [use] -> Int {
    let capacity = 8
    return capacity
}
";
    let (_m, diagnostics) = salvo_syntax::parse_module(ordinary);
    let errors: Vec<_> = diagnostics.iter().filter(|d| d.is_error()).collect();
    assert!(errors.is_empty(), "`capacity` is not reserved: {errors:?}");
}

/// A spawn's `with` clause is same-line, like its `on` clause: without the
/// guard, a bare spawn followed by a `use` **statement** swallowed the next
/// line as its dependency clause (defect found and closed 2026-09-19 — the
/// pair is the monitor spawn's natural shape [monitor-handler]).
#[test]
fn a_spawn_does_not_swallow_a_use_statement_on_the_next_line() {
    let source = "\
fn main() [use, spawn] {
    let rng = spawn CyclicRandom(1)
    use rng
}
";
    let (module, diagnostics) = salvo_syntax::parse_module(source);
    let errors: Vec<_> = diagnostics.iter().filter(|d| d.is_error()).collect();
    assert!(errors.is_empty(), "both statements parse: {errors:?}");
    let main = module
        .items
        .iter()
        .find_map(|i| match i {
            salvo_syntax::ast::Item::Fn(f) if f.name.name == "main" => Some(f),
            _ => None,
        })
        .expect("main is declared");
    let stmts = &main.body.as_ref().expect("main has a body").stmts;
    assert_eq!(stmts.len(), 2, "the `use` is its own statement: {stmts:?}");
    match &stmts[0] {
        salvo_syntax::ast::Stmt::Let { value, .. } => match value {
            salvo_syntax::ast::Expr::Spawn { with_items, .. } => {
                assert!(
                    with_items.is_empty(),
                    "the spawn has no clause: {with_items:?}"
                )
            }
            other => panic!("expected a spawn, got {other:?}"),
        },
        other => panic!("expected the let, got {other:?}"),
    }
    assert!(
        matches!(&stmts[1], salvo_syntax::ast::Stmt::Use { .. }),
        "the second statement is the `use`: {:?}",
        stmts[1]
    );

    // The clause itself still parses when written where it belongs.
    let with_clause = "\
fn main() [use, spawn] {
    let child = spawn Child() with rng,
        Printing()
}
";
    let (_m, diagnostics) = salvo_syntax::parse_module(with_clause);
    let errors: Vec<_> = diagnostics.iter().filter(|d| d.is_error()).collect();
    assert!(
        errors.is_empty(),
        "a same-line clause may continue past a comma: {errors:?}"
    );
}

/// [actor-mailbox] The slot itself: a struct literal with its type elided, and
/// `mailbox` contextual — a state field may still be called `mailbox`.
#[test]
fn a_handler_declares_its_mailbox() {
    use salvo_syntax::ast::{Expr, Item};

    let source = "\
actor effect E {
    send fn ping()
}

handler H(room: Int) of E {
    mailbox { capacity: room }
    seen: Int = 0
    send fn ping() {}
}

handler Odd() of E {
    mailbox { capacity: 1 }
    mailbox: Int = 3
    send fn ping() {}
}
";
    let (module, diagnostics) = salvo_syntax::parse_module(source);
    let errors: Vec<_> = diagnostics.iter().filter(|d| d.is_error()).collect();
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
    let handlers: Vec<_> = module
        .items
        .iter()
        .filter_map(|i| match i {
            Item::Handler(h) => Some(h),
            _ => None,
        })
        .collect();
    let slot = handlers[0]
        .mailbox
        .as_ref()
        .expect("the slot parsed into the handler");
    match slot {
        Expr::StructLit { ty, fields, .. } => {
            assert!(
                format!("{ty:?}").contains("Mailbox"),
                "the slot's type is elided-but-known: {ty:?}"
            );
            assert_eq!(fields.len(), 1, "one field: {fields:?}");
        }
        other => panic!("the slot is a struct literal, got {other:?}"),
    }
    // A *field* called `mailbox` is still a field: only a brace makes the slot.
    assert!(
        handlers[1].mailbox.is_some() && handlers[1].state.len() == 1,
        "a state field named `mailbox` must survive: {:?}",
        handlers[1].state
    );
}

/// [effect-handler-multi] `of E1, E2, …`: one handler, one face per effect. The
/// list is the whole of the syntax the step adds, and one face keeps parsing
/// exactly as it did.
#[test]
fn a_handler_may_declare_several_effects() {
    use salvo_syntax::ast::Item;

    let source = "\
actor effect Timer {
    send fn after(millis: Int) => !millis
}

actor effect TimerCtl {
    send fn advance(millis: Int) => !millis
}

handler ManualTime() of Timer, TimerCtl {
    mailbox { capacity: 8 }
    send fn after(millis: Int) {}
    send fn advance(millis: Int) {}
}

handler Ticking() of Timer {
    mailbox { capacity: 1 }
    send fn after(millis: Int) {}
}
";
    let (module, diagnostics) = salvo_syntax::parse_module(source);
    let errors: Vec<_> = diagnostics.iter().filter(|d| d.is_error()).collect();
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
    let handlers: Vec<_> = module
        .items
        .iter()
        .filter_map(|i| match i {
            Item::Handler(h) => Some(h),
            _ => None,
        })
        .collect();
    let faces: Vec<String> = handlers[0].of.iter().map(|of| of.to_string()).collect();
    assert_eq!(faces, vec!["Timer", "TimerCtl"], "both faces, in order");
    assert_eq!(handlers[1].of.len(), 1, "one face stays one face");
}

/// None of the three new words is reserved: each is recognised only in the
/// shape its form takes (`spawn` before a *name*, `replyto` before a member
/// name, `waitfor` before `name:`), so ordinary code that uses them as
/// identifiers keeps parsing.
#[test]
fn the_asynchronous_words_are_not_reserved() {
    let source = "\
fn spawn(n: Int) -> Int {
    return n
}

fn replyto(n: Int) -> Int {
    return n
}

fn waitfor(n: Int) -> Int {
    return n
}

fn capacity(n: Int) -> Int {
    return n
}

fn on(n: Int) -> Int {
    return n
}

fn use_them() -> Int {
    let spawn = 1
    let replyto = 2
    let waitfor = 3
    return spawn(spawn) + replyto(replyto) + waitfor(waitfor) + capacity(1) + on(2)
}
";
    let (_module, diagnostics) = salvo_syntax::parse_module(source);
    let errors: Vec<_> = diagnostics.iter().filter(|d| d.is_error()).collect();
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
}

/// [actor-self-send] `k@self(args)` — the self-send, parsed as one more
/// member of the selector family (`k@E`, `k@module`) rather than as a
/// receiver. `self` is contextual: only special immediately after `@`, so it
/// remains an ordinary name — and the *old* `self.k(…)` spelling is a plain
/// parse error naming the new one.
#[test]
fn the_self_selector_parses_and_the_receiver_form_does_not() {
    use salvo_syntax::ast::{Expr, Item, Stmt};

    let source = "\
actor effect Work {
    send fn start(n: Int) => !n
    send fn step(n: Int) => !n
}

handler Working() of Work {
    mailbox { capacity: 1 }

    send fn start(n: Int) {
        step@self(n)
    }

    send fn step(n: Int) {
        let self = n
    }
}
";
    let (module, diagnostics) = salvo_syntax::parse_module(source);
    let errors: Vec<_> = diagnostics.iter().filter(|d| d.is_error()).collect();
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");

    let mut selectors = Vec::new();
    for item in &module.items {
        if let Item::Handler(h) = item {
            for f in &h.fns {
                for stmt in f.body.iter().flat_map(|b| &b.stmts) {
                    if let Stmt::Expr(Expr::Call { callee, .. }) = stmt {
                        if let Expr::SelfScoped { name, .. } = callee.as_ref() {
                            selectors.push(name.name.clone());
                        }
                    }
                }
            }
        }
    }
    assert_eq!(selectors, vec!["step"], "the self-selector call is lost");

    let old_form = "\
handler Working() of Work {
    mailbox { capacity: 1 }

    send fn start(n: Int) {
        self.step(n)
    }
}
";
    let (_m, diagnostics) = salvo_syntax::parse_module(old_form);
    assert!(
        diagnostics.iter().any(|d| d.is_error()
            && d.message.contains("is not the self-send form")
            && d.message.contains("`k@self(…)`")),
        "expected the retired spelling to be a parse error: {diagnostics:?}"
    );
}

/// [defer-deduction] `=> !d, defer out` — the deferral entry (SH-10, user
/// decision 2026-09-19): consumed like `!out`, and the obligation may
/// outlive the frame. Contextual: a parameter named `defer` stays writable
/// as a bare keep.
#[test]
fn a_defer_deduction_parses() {
    let source = "\
handler H() of E {
    mailbox { capacity: 2 }
    send fn after(d: Int, out: Reply<Int>) => !d, defer out {
    }
}
";
    let (module, diagnostics) = salvo_syntax::parse_module(source);
    let errors: Vec<_> = diagnostics.iter().filter(|d| d.is_error()).collect();
    assert!(errors.is_empty(), "the clause parses: {errors:?}");
    let handler = module
        .items
        .iter()
        .find_map(|i| match i {
            salvo_syntax::ast::Item::Handler(h) => Some(h),
            _ => None,
        })
        .expect("the handler is declared");
    let member = &handler.fns[0];
    let kinds: Vec<String> = member
        .deductions
        .iter()
        .flatten()
        .map(|d| {
            format!(
                "{}:{:?}",
                d.param_name().map(|n| n.name.as_str()).unwrap_or("?"),
                matches!(d.kind, salvo_syntax::ast::DeductionKind::Deferred)
            )
        })
        .collect();
    assert_eq!(
        kinds,
        vec!["d:false".to_string(), "out:true".to_string()],
        "the second entry is the deferral"
    );
}

/// [waitfor-infer] The type may be **omitted**: `waitfor out { … }` parses, and
/// the binder is still bound (user decision 2026-09-21). A name followed by `{`
/// is the tell — a call of a fn named `waitfor` would open with `(`.
#[test]
fn a_waitfor_without_a_written_type_parses() {
    let (module, diagnostics) = salvo_syntax::parse_module(
        "fn main() [use] {\n    let sum = waitfor out { total(out) }\n}\n",
    );
    let errors: Vec<_> = diagnostics.iter().filter(|d| d.is_error()).collect();
    assert!(errors.is_empty(), "unexpected parse errors: {errors:?}");
    let mut found = false;
    for item in &module.items {
        if let salvo_syntax::ast::Item::Fn(f) = item {
            for stmt in &f.body.as_ref().expect("a body").stmts {
                if let salvo_syntax::ast::Stmt::Let {
                    value: salvo_syntax::ast::Expr::WaitFor { binding, ty, .. },
                    ..
                } = stmt
                {
                    assert_eq!(binding.name, "out");
                    assert!(ty.is_none(), "expected an inferred type");
                    found = true;
                }
            }
        }
    }
    assert!(found, "expected a waitfor");
}

// ===== [is-not] `x !is Q` =====

/// Sugar for `!(x is Q)` (user decision 2026-09-22): one `Expr::Is` under a
/// `Not`, so everything downstream — narrowing, `when` heads, the guard rule —
/// reaches it without knowing the spelling exists.
#[test]
fn not_is_parses_as_a_negated_check() {
    let expr = parse_expr_stmt("value !is Str");
    match expr {
        salvo_syntax::ast::Expr::Unary { op, operand, .. } => {
            assert!(matches!(op, salvo_syntax::ast::UnaryOp::Not), "got {op:?}");
            assert!(
                matches!(*operand, salvo_syntax::ast::Expr::Is { .. }),
                "expected an `is` under the `Not`, got {operand:?}"
            );
        }
        other => panic!("expected a negated check, got {other:?}"),
    }
}

/// The reason this needed a parser rule rather than a lexer token: `!` is also
/// the **assert** postfix, so `s !is Str` used to parse as `(s!) is Str` — which
/// type-checked, and meant the *opposite* of what it reads like. The assert is
/// still there for everything that is not an `is`.
#[test]
fn the_assert_postfix_survives_beside_not_is() {
    let expr = parse_expr_stmt("first(xs)! + 1");
    assert!(
        matches!(expr, salvo_syntax::ast::Expr::Binary { .. }),
        "expected the assert to still bind as a postfix, got {expr:?}"
    );
    // …and with parentheses the old reading is still writable.
    let expr = parse_expr_stmt("(s!) is Str");
    assert!(
        matches!(expr, salvo_syntax::ast::Expr::Is { .. }),
        "expected a positive `is` over an asserted value, got {expr:?}"
    );
}

/// A negated test narrows nothing on the branch it guards, so there is no
/// value for a binding to name.
#[test]
fn not_is_refuses_a_binding() {
    let errors = errors_of("fn f(s: Str?) -> Str {\n    if s !is Str text {\n        return \"no\"\n    }\n    return \"yes\"\n}\n");
    assert!(
        errors.iter().any(|m| m.contains("`!is` binds nothing")),
        "got {errors:?}"
    );
}

/// …and `^` has nothing to widen either [qual-lift].
#[test]
fn not_is_refuses_a_widening() {
    let errors = errors_of("fn f(o: Ok Int | Err Str) -> Int {\n    if o !is ^Ok {\n        return 0\n    }\n    return 1\n}\n");
    assert!(
        errors.iter().any(|m| m.contains("`!is ^Q` is not a check")),
        "got {errors:?}"
    );
}

// ===== [op-compound] `x += e` =====

/// Pure desugar (user decision 2026-09-22): the AST holds an ordinary
/// assignment whose value is the binary operation, so nothing downstream knows
/// the spelling exists. Duplicating the target is safe because a place is an
/// identifier or a field path — there is nothing in one to evaluate twice.
#[test]
fn compound_assignment_desugars_to_assignment() {
    let source = "fn f() -> None {\n    let n = 1\n    n += 2\n}\n";
    let (module, diagnostics) = salvo_syntax::parse_module(source);
    let errors: Vec<_> = diagnostics.iter().filter(|d| d.is_error()).collect();
    assert!(errors.is_empty(), "parse errors: {errors:?}");
    let salvo_syntax::ast::Item::Fn(f) = &module.items[0] else {
        panic!("expected a fn");
    };
    let stmts = &f.body.as_ref().unwrap().stmts;
    match &stmts[1] {
        salvo_syntax::ast::Stmt::Assign { target, value, .. } => {
            assert!(matches!(target, salvo_syntax::ast::Expr::Ident(_)));
            match value {
                salvo_syntax::ast::Expr::Binary { op, lhs, .. } => {
                    assert!(matches!(op, salvo_syntax::ast::BinaryOp::Add), "got {op:?}");
                    assert!(
                        matches!(**lhs, salvo_syntax::ast::Expr::Ident(_)),
                        "the target is read on the left, got {lhs:?}"
                    );
                }
                other => panic!("expected a binary value, got {other:?}"),
            }
        }
        other => panic!("expected an assignment, got {other:?}"),
    }
}

/// `++` keeps its own node: the two forms mean the same thing for `+= 1` in
/// statement position, but `++` is also an *expression* [inc-dec], so it could
/// not be desugared the same way.
#[test]
fn the_step_operators_are_untouched() {
    let expr = parse_expr_stmt("i++");
    assert!(
        matches!(expr, salvo_syntax::ast::Expr::IncDec { .. }),
        "got {expr:?}"
    );
}

/// A compound assignment is a **statement**, so the operator does not chain and
/// does not cross a line break.
#[test]
fn compound_assignment_does_not_cross_a_line() {
    let errors = errors_of("fn f() -> None {\n    let n = 1\n    n\n    += 2\n}\n");
    assert!(
        errors.iter().any(|m| m.contains("expected expression, found `+=`")),
        "got {errors:?}"
    );
}

// ===== [test-decl] tests =====

/// [test-decl] A test is a declaration with a **string-literal** name and a
/// block body, and `test` stays an ordinary identifier elsewhere.
#[test]
fn a_test_declaration_carries_its_name_as_a_string() {
    let (module, diagnostics) = salvo_syntax::parse_module(
        "test \"an empty heap pops nothing\" {\n    expect(true, \"fine\")\n}\n",
    );
    let errors: Vec<_> = diagnostics.iter().filter(|d| d.is_error()).collect();
    assert!(errors.is_empty(), "{errors:?}");
    match &module.items[..] {
        [salvo_syntax::ast::Item::Test(t)] => {
            assert_eq!(t.name, "an empty heap pops nothing");
            assert_eq!(t.body.stmts.len(), 1);
        }
        other => panic!("expected one test declaration, got {other:?}"),
    }
}

/// [test-decl] The name must be knowable without running anything — `--list`
/// and the filter depend on it — so interpolation is refused.
#[test]
fn a_test_name_may_not_interpolate() {
    let errors = errors_of("test \"case ${n}\" {\n    expect(true, \"x\")\n}\n");
    assert!(
        errors.iter().any(|m| m.contains("plain string literal")),
        "got {errors:?}"
    );
}

/// [test-decl] A test is run, never referenced, so there is nothing to export.
#[test]
fn a_test_cannot_be_exported() {
    let errors = errors_of("export test \"nope\" {\n    expect(true, \"x\")\n}\n");
    assert!(
        errors.iter().any(|m| m.contains("`export` cannot precede a `test`")),
        "got {errors:?}"
    );
}

/// [test-decl] `test` is contextual: a variable, a parameter and a module may
/// still be called `test`.
#[test]
fn test_stays_an_ordinary_name() {
    let (_module, diagnostics) = salvo_syntax::parse_module(
        "fn f(test: Int) -> Int {\n    let test2 = test\n    return test2\n}\n",
    );
    let errors: Vec<_> = diagnostics.iter().filter(|d| d.is_error()).collect();
    assert!(errors.is_empty(), "{errors:?}");
}

/// [proj-infer] The opaque projection annotation `-> T holds proj(a, b)`
/// (user decision 2026-09-25, respelling the prefix `proj(a, b) in (T)` of
/// the day before, which read as though the result *were* the projection):
/// parses on fn declarations and on fn types, synthesizing the
/// `DeductionTarget::Opaque` entry, reads the type first, needs no
/// parentheses of its own (`proj(…)` carries them), and both older
/// spellings are plain parse errors.
#[test]
fn the_opaque_projection_is_a_return_annotation() {
    use salvo_syntax::ast::{DeductionKind, DeductionTarget, Item, Type};
    // On a fn declaration: sources synthesize into the clause, and the
    // parenthesized type — a union here — is the return type.
    let src = "fn view(a: List<Int>, b: List<Int>) -> Mut View | None holds proj(a, b)\n=> a, b {\n}\n";
    let (module, diags) = salvo_syntax::parse_module(src);
    assert!(diags.iter().all(|d| !d.is_error()), "{diags:?}");
    let Item::Fn(f) = &module.items[0] else { panic!("expected a fn") };
    let list = f.deductions.as_ref().expect("clause");
    let opaque: Vec<_> = list
        .iter()
        .filter(|d| matches!(d.target, DeductionTarget::Opaque))
        .collect();
    assert_eq!(opaque.len(), 1, "{list:?}");
    let DeductionKind::Proj(srcs) = &opaque[0].kind else { panic!("expected proj sources") };
    assert_eq!(
        srcs.iter().map(|s| s.name.as_str()).collect::<Vec<_>>(),
        vec!["a", "b"]
    );
    assert!(
        matches!(f.return_type, Some(Type::Union { .. })),
        "the inner type is the return type: {:?}",
        f.return_type
    );

    // On a fn type: the entry lands in the fn type's own contract list,
    // where the old remote `=>[iter] proj(c)` group used to put it.
    let src = "fn total<C, It>(c: C, ?iter: (c: C) -> Mut It holds proj(c)) -> Int => c {\n    return 0\n}\n";
    let (module, diags) = salvo_syntax::parse_module(src);
    assert!(diags.iter().all(|d| !d.is_error()), "{diags:?}");
    let Item::Fn(f) = &module.items[0] else { panic!("expected a fn") };
    let iter = f.params.iter().find(|p| p.name.name == "iter").unwrap();
    let Type::Fn { deductions: Some(group), ret, .. } = &iter.ty else {
        panic!("expected a fn type with a contract: {:?}", iter.ty)
    };
    assert!(group.iter().any(|d| matches!(d.target, DeductionTarget::Opaque)));
    assert!(matches!(**ret, Type::Named { .. }), "inner type is the ret: {ret:?}");

    // `holds` needs its sources, and they live inside `proj(…)`.
    let src = "fn view(a: List<Int>) -> Mut View holds proj() => a {\n}\n";
    let (_, diags) = salvo_syntax::parse_module(src);
    assert!(
        diags
            .iter()
            .any(|d| d.is_error() && d.message.contains("holds` needs the sources")),
        "{diags:?}"
    );

    // Both older spellings stopped parsing (plain errors, no shim): the
    // prefix annotation of 2026-09-24 and the clause entry before it.
    for src in [
        "fn view(a: List<Int>) -> proj(a) in (Mut View) => a {\n}\n",
        "fn view(a: List<Int>) -> Mut View => a, proj(a) {\n}\n",
    ] {
        let (_, diags) = salvo_syntax::parse_module(src);
        assert!(diags.iter().any(|d| d.is_error()), "{src}: {diags:?}");
    }

    // `holds` is reserved, which is what keeps a type's qualifier chain from
    // eating it — so it is not an identifier any more.
    let src = "fn holds(a: Int) -> Int => a {\n    return a\n}\n";
    let (_, diags) = salvo_syntax::parse_module(src);
    assert!(diags.iter().any(|d| d.is_error()), "{diags:?}");
}

/// [canbe-entry] The alias-group entry's decided grammar (GB-1(s), user
/// decisions 2026-09-24): the core form, a `|` hub on the right, a plural
/// subject on the left, and the anchored `canbe in` with a path list.
#[test]
fn canbe_entries_parse_in_every_decided_form() {
    let src = "\
        fn f(a: Mut Int, b: Mut Int, c: Mut Int) -> None => a canbe b {}\n\
        fn g(a: Mut Int, b: Mut Int, c: Mut Int) -> None => a canbe b|c {}\n\
        fn h(a: Mut Int, b: Mut Int, c: Mut Int, es: Mut Int) -> None\n\
        => a|b|c canbe in es {}\n\
        fn k(t: Mut Int, lib: Mut Int, pool: Mut Int) -> None\n\
        => t canbe in lib|pool {}\n";
    let (module, diagnostics) = salvo_syntax::parse_module(src);
    let errors: Vec<_> = diagnostics.iter().filter(|d| d.is_error()).collect();
    assert!(errors.is_empty(), "{errors:?}");
    assert_eq!(module.items.len(), 4);
}

/// [canbe-entry] A `canbe` entry names parameters: a result path is an
/// error, so the entry cannot be confused with a projection one.
#[test]
fn a_canbe_entry_refuses_a_result_path() {
    let src = "fn f(a: Mut Int) -> None => .x canbe a {}\n";
    let (_, diagnostics) = salvo_syntax::parse_module(src);
    assert!(
        diagnostics
            .iter()
            .any(|d| d.is_error() && d.message.contains("names parameters")),
        "{diagnostics:?}"
    );
}

/// [deduce-field] The field-granular entries parse: a field-level `Mut`
/// (the call mutates only there) and a field-level `!` (the field is
/// replaced). A field path stating anything else is refused.
#[test]
fn field_granular_mutation_entries_parse() {
    let src = "\
        fn a(h: Mut Int, n: Int) -> None => h.tags: Mut, n {}\n\
        fn b(h: Mut Int) -> None => !h.tags {}\n\
        fn c(h: Mut Int) -> None => h.tags: Mut, h.more: Mut {}\n";
    let (module, diagnostics) = salvo_syntax::parse_module(src);
    let errors: Vec<_> = diagnostics.iter().filter(|d| d.is_error()).collect();
    assert!(errors.is_empty(), "{errors:?}");
    assert_eq!(module.items.len(), 3);

    let (_, bad) = salvo_syntax::parse_module("fn d(h: Mut Int) -> None => h.tags: preserve Idx {}\n");
    assert!(
        bad.iter().any(|d| d.is_error() && d.message.contains("field path can state")),
        "{bad:?}"
    );
}

/// [interp-nested-str] A string literal inside `${...}` is part of the
/// fragment, not the end of the enclosing literal. Before this, the
/// interpolation scanner stopped at the inner quote and the file lexed as an
/// unterminated string — which made `?:` with a text fallback unwritable.
#[test]
fn an_interpolation_may_hold_a_string_literal() {
    let errors = errors_of(
        "fn f(name: Str?) -> Str {\n    return \"hello ${name ?: \"unknown\"}\"\n}\n",
    );
    assert!(errors.is_empty(), "got {errors:?}");
}

/// [interp-nested-str] The nested literal may interpolate in turn, and its
/// braces belong to it: the skip recurses rather than counting braces flat.
#[test]
fn a_nested_string_may_interpolate_in_turn() {
    let errors = errors_of(
        "fn f(a: Str, b: Str) -> Str {\n    return \"x ${size(\"${a} and ${b}\")} y\"\n}\n",
    );
    assert!(errors.is_empty(), "got {errors:?}");
}

/// [interp-nested-str] An escaped quote inside the nested literal does not end
/// it, so the scanner honours escapes while skipping.
#[test]
fn a_nested_string_honours_escapes() {
    let errors = errors_of(
        "fn f() -> Str {\n    return \"q ${size(\"a\\\"b\")} z\"\n}\n",
    );
    assert!(errors.is_empty(), "got {errors:?}");
}

/// [interp-nested-str] Unterminated is still unterminated: the fragment's
/// literal has to close, and the diagnostic is the interpolation's.
#[test]
fn an_unterminated_nested_string_is_reported() {
    let errors = errors_of("fn f() -> Str {\n    return \"q ${size(\"a)} z\"\n}\n");
    assert!(
        errors.iter().any(|m| m.contains("unterminated")),
        "got {errors:?}"
    );
}
