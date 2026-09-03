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

/// The `(kept, surviving qualifiers)` facts for one parameter of one fn.
/// The surviving set is the effect resolved against the parameter's
/// declared qualifiers [deduce-syntax].
fn facts(program: &Program, checked: &Checked, fn_name: &str, param: &str) -> (bool, Vec<String>) {
    let key = fn_key(program, fn_name);
    let ded = checked.deductions[&key]
        .iter()
        .find(|d| d.param == param)
        .unwrap_or_else(|| panic!("no deduction entry for `{param}` in `{fn_name}`"));
    let Item::Fn(decl) = &program.modules[key.file].items[key.item] else {
        panic!("not a fn");
    };
    let declared = decl
        .params
        .iter()
        .find(|p| p.name.name == param)
        .map(|p| salvo_core::deduce::declared_quals(&p.ty))
        .unwrap_or_default();
    (ded.kept, ded.effect.kept_quals(&declared))
}

/// The *form* of one parameter's effect, for the [deduce-syntax] polarity
/// tests: `"keep-all"`, `"exhaustive: A B"`, or `"remove: A"`.
fn effect_form(program: &Program, checked: &Checked, fn_name: &str, param: &str) -> String {
    let key = fn_key(program, fn_name);
    let ded = checked.deductions[&key]
        .iter()
        .find(|d| d.param == param)
        .unwrap_or_else(|| panic!("no deduction entry for `{param}` in `{fn_name}`"));
    match &ded.effect {
        salvo_core::QualEffect::KeepAll => "keep-all".to_string(),
        salvo_core::QualEffect::Exhaustive(q) => format!("exhaustive: {}", q.join(" ")),
        salvo_core::QualEffect::Remove(q) => format!("remove: {}", q.join(" ")),
    }
}

const QUALIFIED_LISTS: &str = r#"
internal type List<T>

qualifier A<T> of List<T>
qualifier B<T> of List<T> with A<T>
qualifier C<T> of List<T> with A<T>, B<T>

external fn drop_a<T>(list: A B List<T>) [] -> [list: B] None
external fn consume<T>(list: List<T>) [] -> [] None
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

// [deduce-syntax] A *plain* (exhaustive) list is exactly what survives:
// a qualifier the callee never declared (here `C`) is dropped too, because
// the callee cannot have preserved what it never knew about. This is the
// fix for the qualifier-preservation unsoundness (D1).
#[test]
fn exhaustive_lists_drop_undeclared_qualifiers() {
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
    assert_eq!(quals, vec!["B".to_string()]);
    // Exhaustiveness is contagious: `caller` can no longer promise its own
    // caller's extras either.
    assert_eq!(
        effect_form(&program, &checked, "caller", "list"),
        "exhaustive: B"
    );
}

// [deduce-syntax] A *delta* list (`-A`) drops only what it names, so
// qualifiers the callee never declared pass through — the old behavior,
// now opt-in and only legal where nothing can invalidate them.
#[test]
fn delta_lists_pass_undeclared_qualifiers_through() {
    let src = format!(
        "{QUALIFIED_LISTS}
external fn shed_a<T>(list: A B List<T>) [] -> [list: -A] None

fn caller<T>(list: A B C List<T>) -> None {{
    shed_a(list)
}}
"
    );
    let (program, checked) = check_src(&src);
    assert!(checked.errors.is_empty(), "errors: {:?}", checked.errors);
    let (kept, quals) = facts(&program, &checked, "caller", "list");
    assert!(kept);
    assert_eq!(quals, vec!["B".to_string(), "C".to_string()]);
    assert_eq!(
        effect_form(&program, &checked, "caller", "list"),
        "remove: A"
    );
}

// [deduce-syntax] The soundness rule: a body that *mutates* a parameter
// may not keep everything, nor drop selectively — mutation can invalidate
// qualifiers the caller has and the signature never mentions.
#[test]
fn mutating_bodies_require_an_exhaustive_list() {
    let src = "
internal type List<T> canbe Mut

qualifier A<T> of List<T>

external fn mutate<T>(list: Mut List<T>) [] -> [list: Mut] None

fn keeps_all<T>(list: Mut A List<T>) -> [list] None {
    mutate(list)
}

fn drops_one<T>(list: Mut A List<T>) -> [list: -A] None {
    mutate(list)
}

fn exhaustive<T>(list: Mut A List<T>) -> [list: Mut] None {
    mutate(list)
}
"
    .to_string();
    let (_, checked) = check_src(&src);
    let messages: Vec<String> = checked.errors.iter().map(|e| e.message.clone()).collect();
    assert_eq!(
        messages
            .iter()
            .filter(|m| m.contains("cannot keep every qualifier"))
            .count(),
        1,
        "errors: {messages:?}"
    );
    assert_eq!(
        messages
            .iter()
            .filter(|m| m.contains("cannot drop qualifiers selectively"))
            .count(),
        1,
        "errors: {messages:?}"
    );
    // `keeps_all` also trips the promise check (it claims `A` survives a
    // mutation), which is the same fact reported from the body side; the
    // exhaustive form produces nothing.
    assert!(
        messages.iter().all(|m| !m.contains("exhaustive")),
        "the exhaustive form must be accepted: {messages:?}"
    );
}

// [deduce-syntax] `[p: Nothing]` is the moved form (`Nothing` is
// uninhabited, so it withdraws use without claiming anything about
// content) — the same facts as omitting the entry.
#[test]
fn nothing_in_a_deduction_means_moved() {
    let src = format!(
        "{QUALIFIED_LISTS}
external fn eat<T>(list: List<T>) [] -> [list: Nothing] None

fn caller<T>(list: List<T>) -> None {{
    eat(list)
}}
"
    );
    let (program, checked) = check_src(&src);
    let (kept, _) = facts(&program, &checked, "caller", "list");
    assert!(!kept, "the parameter should be moved: {:?}", checked.errors);
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
external fn keep_all<T>(list: A B List<T>) [] -> [list] None
external fn strip_all<T>(list: A B List<T>) [] -> [list:] None
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

// [fate-link] [deduce-infer] Binding a bare parameter (or a projection
// of one) with `let`/assignment fate-links the variable instead of
// moving the parameter: the parameter stays kept, and reading through
// the derived variable is fine. Escapes of the derived variable are
// checker errors [fate-derived-readonly], and `copy` severs the link so
// the body can return an independent value while keeping the parameter.
#[test]
fn let_bindings_link_instead_of_moving() {
    let src = format!(
        "{QUALIFIED_LISTS}
fn links<T>(list: A List<T>) -> None {{
    let alias = list
}}

fn reads<T>(list: A List<T>) -> Int {{
    let alias = list
    return list_size(alias)
}}

fn escapes<T>(list: List<T>) -> List<T> {{
    let alias = list
    return copy(alias)
}}

internal fn copy<T>(value: T) [] -> [value] T

external fn list_size<T>(list: List<T>) [] -> [list] Int
"
    );
    let (program, checked) = check_src(&src);
    assert!(checked.errors.is_empty(), "errors: {:?}", checked.errors);
    assert_eq!(
        facts(&program, &checked, "links", "list"),
        (true, vec!["A".to_string()])
    );
    assert_eq!(
        facts(&program, &checked, "reads", "list"),
        (true, vec!["A".to_string()])
    );
    assert_eq!(facts(&program, &checked, "escapes", "list").0, true);
}

// [fate-move-mode] S2: a binding that is later *moved* takes ownership —
// binding a bare parameter (or a projection of one) and then moving the
// derived variable claims the parameter as moved, transitively through
// binding chains and loop bindings. The claim feeds the interprocedural
// fixpoint: callers see the argument consumed. A written list keeping
// the parameter blocks the claim (the S1 error stands instead).
#[test]
fn move_mode_bindings_claim_parameters() {
    let src = format!(
        "{QUALIFIED_LISTS}
fn claims<T>(list: List<T>) -> List<T> {{
    let alias = list
    return alias
}}

fn chain_claims<T>(list: List<T>) -> List<T> {{
    let a = list
    let b = a
    return b
}}

fn caller<T>(list: List<T>) -> Int {{
    let result = claims(list)
    return list_size(result)
}}

external fn list_size<T>(list: List<T>) [] -> [list] Int
"
    );
    let (program, checked) = check_src(&src);
    assert!(checked.errors.is_empty(), "errors: {:?}", checked.errors);
    // The move-mode binding claims the parameter: not kept.
    assert_eq!(facts(&program, &checked, "claims", "list").0, false);
    assert_eq!(facts(&program, &checked, "chain_claims", "list").0, false);
    // The claim propagates through the call graph: `caller` passes its
    // own parameter to `claims`, which now moves it.
    assert_eq!(facts(&program, &checked, "caller", "list").0, false);
}

// [fate-lambda] A lambda that captures and mutates a parameter claims it
// as moved (the closure takes ownership at creation); a read-only
// capture keeps the parameter kept.
#[test]
fn lambda_capture_mutation_claims_parameters() {
    let src = format!(
        "{QUALIFIED_LISTS}
internal type Store<T> canbe Mut

fn runs(f: () -> None) -> None {{
    let unused = f
}}

fn mutates<T>(store: Mut Store<T>) -> None {{
    let g = () -> {{ bump(store) }}
    runs(g)
}}

fn reads<T>(store: Mut Store<T>) -> None {{
    let g = () -> {{ let n = store_size(store) }}
    runs(g)
}}

external fn bump<T>(store: Mut Store<T>) [] -> [store: Mut] None

external fn store_size<T>(store: Store<T>) [] -> [store] Int
"
    );
    let (program, checked) = check_src(&src);
    assert!(checked.errors.is_empty(), "errors: {:?}", checked.errors);
    // The mutating capture claims the parameter.
    assert_eq!(facts(&program, &checked, "mutates", "store").0, false);
    // A read-only capture keeps it.
    assert_eq!(facts(&program, &checked, "reads", "store").0, true);
}

// [deduce-fixpoint] L3: checking iterates to a fixpoint (capped) — a
// move-mode candidate first discovered in round two (when inferred
// deductions are first enforced) is applied by a third round, so the
// diagnostic lands at the true problem site instead of the raw
// derived-move error.
#[test]
fn late_candidates_converge() {
    let src = format!(
        "{QUALIFIED_LISTS}
fn steal<T>(list: List<T>) -> List<T> {{
    return list
}}

fn late<T>(seed: List<T>) -> Int {{
    let xs = seed
    let ys = xs
    let zs = steal(ys)
    return list_size(xs)
}}

external fn list_size<T>(list: List<T>) [] -> [list] Int
"
    );
    let (_, checked) = check_src(&src);
    let messages: Vec<&str> = checked.errors.iter().map(|e| e.message.as_str()).collect();
    assert!(
        messages
            .iter()
            .any(|m| m.contains("`ys` was bound from it and later moves the value")),
        "errors: {messages:?}"
    );
    assert!(
        !messages.iter().any(|m| m.contains("cannot move `ys`")),
        "round-two error should have been superseded: {messages:?}"
    );
}
