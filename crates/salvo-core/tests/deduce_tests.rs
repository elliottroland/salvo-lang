//! Deduction inference and validation tests [deduce-syntax] [deduce-infer].

use std::path::Path;

use salvo_core::{check_program, resolve, Checked, FnKey, Program, SourceSet, Symbols};
use salvo_syntax::ast::Item;

/// Parses + resolves + checks a single-file program (no std).
fn check_src(src: &str) -> (Program, Checked) {
    let mut sources = SourceSet::default();
    let (module, kind) = SourceSet::classify(Path::new("main.sv"), "kotlin").unwrap();
    sources.add("main.sv", module, kind, src.to_string(), false);
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

/// Finds the `FnKey` of a top-level fn by name.
fn fn_key(program: &Program, name: &str) -> FnKey {
    for (file, ast) in program.modules.iter().enumerate() {
        for (item, it) in ast.items.iter().enumerate() {
            if let Item::Fn(f) = it {
                if f.name.name == name {
                    return FnKey { file, item };
                }
            }
        }
    }
    panic!("no fn `{name}`");
}

/// The `(kept, quals)` facts for one parameter of one fn.
fn facts(program: &Program, checked: &Checked, fn_name: &str, param: &str) -> (bool, Vec<String>) {
    let key = fn_key(program, fn_name);
    let ded = checked.deductions[&key]
        .iter()
        .find(|d| d.param == param)
        .unwrap_or_else(|| panic!("no deduction entry for `{param}` in `{fn_name}`"));
    (ded.kept, ded.quals.clone())
}

const QUALIFIED_LISTS: &str = r#"
internal type List<T>

qualifier A<T> of List<T>
qualifier B<T> of List<T> with A<T>
qualifier C<T> of List<T> with A<T>, B<T>

external fn drop_a<T>(list: A B List<T>) -> [list: B] None
external fn consume<T>(list: List<T>) -> [] None
"#;

// [deduce-infer] A call removes exactly the callee's removal set
// (declared − kept) from the parameter's inferred qualifiers.
#[test]
fn inference_subtracts_the_callee_removal_set() {
    let src = format!(
        "{QUALIFIED_LISTS}
fn caller<T>(list: A B List<T>) -> None {{
    drop_a(list)
}}
"
    );
    let (program, checked) = check_src(&src);
    assert!(checked.errors.is_empty(), "errors: {:?}", checked.errors);
    let (kept, quals) = facts(&program, &checked, "caller", "list");
    assert!(kept);
    assert_eq!(quals, vec!["B".to_string()]);
}

// [deduce-syntax] Deductions are interpreted relative to the *declared*
// parameter qualifiers: a qualifier the callee never declared (here `C`)
// is unaffected by the call.
#[test]
fn undeclared_qualifiers_pass_through_calls() {
    let src = format!(
        "{QUALIFIED_LISTS}
fn caller<T>(list: A B C List<T>) -> None {{
    drop_a(list)
}}
"
    );
    let (program, checked) = check_src(&src);
    assert!(checked.errors.is_empty(), "errors: {:?}", checked.errors);
    let (kept, quals) = facts(&program, &checked, "caller", "list");
    assert!(kept);
    assert_eq!(quals, vec!["B".to_string(), "C".to_string()]);
}

// [deduce-infer] Calling a fn that omits the parameter from its written
// list moves the argument; returning the bare parameter also moves it.
#[test]
fn moves_are_inferred() {
    let src = format!(
        "{QUALIFIED_LISTS}
fn caller<T>(list: List<T>) -> None {{
    consume(list)
}}

fn identity<T>(list: List<T>) -> List<T> {{
    return list
}}
"
    );
    let (program, checked) = check_src(&src);
    assert!(checked.errors.is_empty(), "errors: {:?}", checked.errors);
    assert_eq!(facts(&program, &checked, "caller", "list").0, false);
    assert_eq!(facts(&program, &checked, "identity", "list").0, false);
}

// [deduce-infer] Inference reaches a fixpoint across the call graph:
// `top` loses `A` through `mid`'s *inferred* deduction.
#[test]
fn inference_is_transitive() {
    let src = format!(
        "{QUALIFIED_LISTS}
fn top<T>(list: A B List<T>) -> None {{
    mid(list)
}}

fn mid<T>(list: A B List<T>) -> None {{
    drop_a(list)
}}
"
    );
    let (program, checked) = check_src(&src);
    assert!(checked.errors.is_empty(), "errors: {:?}", checked.errors);
    assert_eq!(
        facts(&program, &checked, "top", "list"),
        (true, vec!["B".to_string()])
    );
}

// [deduce-infer] Unresolved calls (backend interop) borrow leniently and
// preserve all qualifiers; plain reads never move.
#[test]
fn reads_and_unresolved_calls_borrow() {
    let src = format!(
        r#"{QUALIFIED_LISTS}
fn caller<T>(list: A B List<T>) -> Int {{
    let s = "${{list}}"
    unknown_interop(list)
    return 1
}}
"#
    );
    let (program, checked) = check_src(&src);
    assert!(checked.errors.is_empty(), "errors: {:?}", checked.errors);
    assert_eq!(
        facts(&program, &checked, "caller", "list"),
        (true, vec!["A".to_string(), "B".to_string()])
    );
}

// [deduce-syntax] A written list promising a parameter back that the body
// moves is an error; so is promising a qualifier the body may remove.
#[test]
fn written_lists_are_validated_against_the_body() {
    let src = format!(
        "{QUALIFIED_LISTS}
fn bad_move<T>(list: List<T>) -> [list] None {{
    consume(list)
}}

fn bad_qual<T>(list: A B List<T>) -> [list: A B] None {{
    drop_a(list)
}}
"
    );
    let (_, checked) = check_src(&src);
    assert!(
        checked
            .errors
            .iter()
            .any(|e| e.message.contains("promises `list` back to the caller, but the body moves it")),
        "unexpected errors: {:?}",
        checked.errors
    );
    assert!(
        checked
            .errors
            .iter()
            .any(|e| e.message.contains("promises qualifier `A` on `list`, but the body may remove it")),
        "unexpected errors: {:?}",
        checked.errors
    );
}

// [deduce-syntax] Written-list shape errors: unknown parameter, duplicate
// entry, and keeping a qualifier the parameter does not declare.
#[test]
fn written_list_shape_is_validated() {
    let src = format!(
        "{QUALIFIED_LISTS}
fn unknown_param(x: Int) -> [y] None {{
}}

fn duplicate(x: Int) -> [x, x] None {{
}}

fn undeclared_qual<T>(list: List<T>) -> [list: A] None {{
}}
"
    );
    let (_, checked) = check_src(&src);
    assert!(checked
        .errors
        .iter()
        .any(|e| e.message.contains("deduction names unknown parameter `y`")));
    assert!(checked
        .errors
        .iter()
        .any(|e| e.message.contains("duplicate deduction for parameter `x`")));
    assert!(
        checked.errors.iter().any(|e| e
            .message
            .contains("deduction keeps qualifier `A`, which is not declared on parameter `list`")),
        "unexpected errors: {:?}",
        checked.errors
    );
}

// [deduce-infer] A stricter-than-inferred written list is fine: the
// contract may drop qualifiers or move parameters the body gives back.
#[test]
fn written_lists_may_be_stricter_than_the_body() {
    let src = format!(
        "{QUALIFIED_LISTS}
fn stricter<T>(list: A B List<T>) -> [list: B] None {{
}}

fn moves_anyway<T>(list: List<T>) -> [] None {{
}}
"
    );
    let (program, checked) = check_src(&src);
    assert!(checked.errors.is_empty(), "errors: {:?}", checked.errors);
    assert_eq!(
        facts(&program, &checked, "stricter", "list"),
        (true, vec!["B".to_string()])
    );
    assert_eq!(facts(&program, &checked, "moves_anyway", "list").0, false);
}

// [deduce-syntax] A bare `[list]` entry keeps *all* declared qualifiers;
// the explicit-empty `[list:]` keeps none.
#[test]
fn bare_entries_keep_all_qualifiers_and_explicit_empty_keeps_none() {
    let src = format!(
        "{QUALIFIED_LISTS}
external fn keep_all<T>(list: A B List<T>) -> [list] None
external fn strip_all<T>(list: A B List<T>) -> [list:] None
"
    );
    let (program, checked) = check_src(&src);
    assert!(checked.errors.is_empty(), "errors: {:?}", checked.errors);
    assert_eq!(
        facts(&program, &checked, "keep_all", "list"),
        (true, vec!["A".to_string(), "B".to_string()])
    );
    assert_eq!(
        facts(&program, &checked, "strip_all", "list"),
        (true, Vec::<String>::new())
    );
}
