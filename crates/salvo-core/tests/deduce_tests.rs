//! Deduction inference and validation tests [deduce-syntax] [deduce-infer].

use std::path::Path;

use salvo_core::{check_program, resolve, Checked, FnKey, Program, SourceSet, Symbols};
use salvo_syntax::ast::Item;

/// [intrinsic-std-only] The intrinsic declarations these sources rely on.
/// `intrinsic` is the compiler's modifier and only std may write it, so this
/// is loaded as a *std* file rather than pasted into the source under test.
/// Module `core.prelude`: `core.*` is implicitly imported, so the test source
/// sees these names without an `import`.
const STD_PRELUDE: &str = "export intrinsic type Int\nexport intrinsic type List<T> canbe Mut\nexport intrinsic fn copy<T>(value: T) [] -> T => value\nexport intrinsic type Store<T> canbe Mut\nexport intrinsic fn fresh() [] -> List<Int>\nexport intrinsic fn add<T>(list: Mut List<T>, elem: T) [] -> None => list: Mut, !elem\n";

/// Parses + resolves + checks a single-file program (no std).
fn check_src(src: &str) -> (Program, Checked) {
    let mut sources = SourceSet::default();
    sources.add(
        "std/core/prelude.sv",
        SourceSet::classify(Path::new("core/prelude.sv")).unwrap(),
        STD_PRELUDE.to_string(),
        true,
    );
    sources.add(
        "main.sv",
        SourceSet::classify(Path::new("main.sv")).unwrap(),
        src.to_string(),
        false,
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

qualifier A<T> of List<T>
qualifier B<T> of List<T> with A<T>
qualifier C<T> of List<T> with A<T>, B<T>

fn drop_a<T>(list: A B List<T>) [] -> None => list: B {}
fn consume<T>(list: List<T>) [] -> None => !list {}
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
    assert!(
        checked.errors.iter().all(|e| !e.is_error()),
        "errors: {:?}",
        checked.errors
    );
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
    assert!(
        checked.errors.iter().all(|e| !e.is_error()),
        "errors: {:?}",
        checked.errors
    );
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
fn shed_a<T>(list: A B List<T>) [] -> None => list: -A {{}}

fn caller<T>(list: A B C List<T>) -> None {{
    shed_a(list)
}}
"
    );
    let (program, checked) = check_src(&src);
    assert!(
        checked.errors.iter().all(|e| !e.is_error()),
        "errors: {:?}",
        checked.errors
    );
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

qualifier A<T> of List<T>

fn mutate<T>(list: Mut List<T>) [] -> None => list: Mut {}

fn keeps_all<T>(list: Mut A List<T>) -> None => list {
    mutate(list)
}

fn drops_one<T>(list: Mut A List<T>) -> None => list: -A {
    mutate(list)
}

fn exhaustive<T>(list: Mut A List<T>) -> None => list: Mut {
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

// [deduce-syntax] `[p: Never]` is the moved form (`Never` is
// uninhabited, so it withdraws use without claiming anything about
// content) — the same facts as omitting the entry.
#[test]
fn nothing_in_a_deduction_means_moved() {
    let src = format!(
        "{QUALIFIED_LISTS}
fn eat<T>(list: List<T>) [] -> None => !list {{}}

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
    assert!(
        checked.errors.iter().all(|e| !e.is_error()),
        "errors: {:?}",
        checked.errors
    );
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
    assert!(
        checked.errors.iter().all(|e| !e.is_error()),
        "errors: {:?}",
        checked.errors
    );
    assert_eq!(
        facts(&program, &checked, "top", "list"),
        (true, vec!["B".to_string()])
    );
}

// [call-resolve] [deduce-infer] An effect member's *declared* deduction
// list is the callee contract for **inference** too, not just at the call
// site: a member that takes ownership makes the enclosing fn move the
// argument, so its own callers must hand over ownership. Before this,
// deduce treated member calls as borrows (no `FnKey` to resolve), so the
// checker said "consumed" inside the body while the inferred contract said
// "kept" — the two disagreed about the same call.
#[test]
fn effect_member_moves_propagate_to_the_caller() {
    let src = format!(
        r#"{QUALIFIED_LISTS}
effect Sink {{
    fn eat(list: List<Int>) -> None => !list
}}

fn forward(list: List<Int>) [Sink] -> None {{
    eat(list)
}}
"#
    );
    let (program, checked) = check_src(&src);
    assert!(
        checked.errors.iter().all(|e| !e.is_error()),
        "errors: {:?}",
        checked.errors
    );
    let (kept, _) = facts(&program, &checked, "forward", "list");
    assert!(
        !kept,
        "the member takes ownership, so `forward` must be inferred to move it"
    );
}

// [deduce-infer] [call-resolve] A member that *keeps* its parameter still
// borrows, and preserves its qualifiers; plain reads never move.
#[test]
fn reads_and_effect_member_calls_borrow() {
    let src = format!(
        r#"{QUALIFIED_LISTS}
effect Logger {{
    fn log<T>(list: List<T>) -> None => list
}}

fn peek<T>(list: List<T>) [] -> Int => list {{ return 0 }}

fn caller<T>(list: A B List<T>) [Logger] -> Int {{
    let _s = peek(list)
    log(list)
    return 1
}}
"#
    );
    let (program, checked) = check_src(&src);
    assert!(
        checked.errors.iter().all(|e| !e.is_error()),
        "errors: {:?}",
        checked.errors
    );
    assert_eq!(
        facts(&program, &checked, "caller", "list"),
        (true, vec!["A".to_string(), "B".to_string()])
    );
}

// [effect-state-store] [deduce-infer] Storing a value into a handler *state*
// field is a move — the field outlives every member call — so a member that
// promises its parameter back while storing it is rejected. This was a real
// parity divergence: the member declared `[list: Mut]`, Kotlin aliased the
// list into the state (later caller mutations visible) while Rust cloned it
// (invisible), and the same program printed 2 and 1. Handler member bodies
// were never validated against their contracts, because the deduction pass
// only collected top-level fns.
#[test]
fn handler_state_stores_are_moves_and_members_are_validated() {
    let src = format!(
        r#"{QUALIFIED_LISTS}
effect Sink {{
    fn keep(list: A List<Int>) -> None => list: A
}}

handler Bin of Sink {{
    held: List<Int> = fresh()

    fn keep(list: A List<Int>) -> None => list: A {{
        held = list
    }}
}}
"#
    );
    let (_, checked) = check_src(&src);
    let messages: Vec<String> = checked.errors.iter().map(|e| e.message.clone()).collect();
    assert!(
        messages
            .iter()
            .any(|m| m == "deduction promises `list` back to the caller, but the body moves it"),
        "got {messages:?}"
    );
}

// [effect-state-store] A member that declares the parameter *moved* may store
// it: that is the honest contract, and callers then give up ownership.
#[test]
fn handler_state_stores_are_legal_when_the_contract_moves() {
    let src = format!(
        r#"{QUALIFIED_LISTS}
effect Sink {{
    fn keep(list: A List<Int>) -> None => !list
}}

handler Bin of Sink {{
    held: List<Int> = fresh()

    fn keep(list: A List<Int>) -> None => !list {{
        held = list
    }}
}}
"#
    );
    let (_, checked) = check_src(&src);
    let messages: Vec<String> = checked.errors.iter().map(|e| e.message.clone()).collect();
    assert!(messages.is_empty(), "got {messages:?}");
}

// [effect-state-store] The **read** direction of the same rule, and the same
// evidence: a member cannot move a value *out* of the handler's storage
// either, because the handler still owns it when the member returns. Rust
// silently `.clone()`d it (so the handler kept a copy and mutable data
// diverged: `eat(items)` then `size(items)` printed 2 on Rust and 3 on Kotlin)
// while Kotlin shared the reference, and where the clone was missing — the
// consuming *intrinsics* — rustc reported a bare E0507 with no Salvo
// diagnostic at all. Found 2026-09-15. Both kinds of storage are covered: a
// `state` field and a constructor parameter.
#[test]
fn a_member_cannot_move_a_value_out_of_handler_state() {
    let src = format!(
        r#"{QUALIFIED_LISTS}
fn eat(list: List<Int>) -> None => !list {{ }}

effect Sink {{
    fn go() -> None
}}

handler Bin of Sink {{
    held: List<Int> = fresh()

    fn go() -> None {{
        eat(held)
    }}
}}
"#
    );
    let (_, checked) = check_src(&src);
    let messages: Vec<String> = checked.errors.iter().map(|e| e.message.clone()).collect();
    assert!(
        messages.iter().any(|m| m.contains(
            "cannot move `held`: it is a state field of handler `Bin`"
        ) && m.contains("copy(held)")),
        "got {messages:?}"
    );
}

#[test]
fn a_member_cannot_move_a_value_out_of_a_constructor_parameter() {
    let src = format!(
        r#"{QUALIFIED_LISTS}
fn eat(list: List<Int>) -> None => !list {{ }}

effect Sink {{
    fn go() -> None
}}

handler Bin(seed: List<Int>) of Sink {{
    fn go() -> None {{
        eat(seed)
    }}
}}
"#
    );
    let (_, checked) = check_src(&src);
    let messages: Vec<String> = checked.errors.iter().map(|e| e.message.clone()).collect();
    assert!(
        messages.iter().any(|m| m.contains(
            "cannot move `seed`: it is a constructor parameter of handler `Bin`"
        )),
        "got {messages:?}"
    );
}

// [effect-state-store] Returning it is the same move, and the shape most
// likely to be written: a getter. `copy` is the remedy there too.
#[test]
fn a_member_cannot_return_handler_storage_uncopied() {
    let src = format!(
        r#"{QUALIFIED_LISTS}
effect Sink {{
    fn read() -> List<Int>
}}

handler Bin of Sink {{
    held: List<Int> = fresh()

    fn read() -> List<Int> {{
        return held
    }}
}}
"#
    );
    let (_, checked) = check_src(&src);
    let messages: Vec<String> = checked.errors.iter().map(|e| e.message.clone()).collect();
    assert!(
        messages
            .iter()
            .any(|m| m.contains("cannot return `held`") && m.contains("copy(held)")),
        "got {messages:?}"
    );
}

// [effect-state-store] …and `copy` settles it, on both counts.
#[test]
fn copying_handler_storage_out_is_accepted() {
    let src = format!(
        r#"{QUALIFIED_LISTS}
fn eat(list: List<Int>) -> None => !list {{ }}

effect Sink {{
    fn read() -> List<Int>
    fn go() -> None
}}

handler Bin(seed: List<Int>) of Sink {{
    held: List<Int> = fresh()

    fn read() -> List<Int> {{
        return copy(held)
    }}

    fn go() -> None {{
        eat(copy(seed))
    }}
}}
"#
    );
    let (_, checked) = check_src(&src);
    let messages: Vec<String> = checked.errors.iter().map(|e| e.message.clone()).collect();
    assert!(messages.is_empty(), "got {messages:?}");
}

// [copy-scalar-free] A native scalar is exempt: its copy is free and
// indistinguishable from a move, both backends agree, and requiring `copy`
// around every `Int` a member answers with would be noise. This is why the
// first asynchronous test (`sum: Int`, `out.send(sum)`) never met the rule.
#[test]
fn a_copy_scalar_state_field_may_be_handed_over() {
    let src = format!(
        r#"{QUALIFIED_LISTS}
fn eat(n: Int) -> None => !n {{ }}

effect Sink {{
    fn read() -> Int
    fn go() -> None
}}

handler Bin of Sink {{
    count: Int = 0

    fn read() -> Int {{
        return count
    }}

    fn go() -> None {{
        eat(count)
    }}
}}
"#
    );
    let (_, checked) = check_src(&src);
    let messages: Vec<String> = checked.errors.iter().map(|e| e.message.clone()).collect();
    assert!(messages.is_empty(), "got {messages:?}");
}

// [deduce-syntax] A written list promising a parameter back that the body
// moves is an error; so is promising a qualifier the body may remove.
#[test]
fn written_lists_are_validated_against_the_body() {
    let src = format!(
        "{QUALIFIED_LISTS}
fn bad_move<T>(list: List<T>) -> None => list {{
    consume(list)
}}

fn bad_qual<T>(list: A B List<T>) -> None => list: A B {{
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
fn unknown_param(x: Int) -> None => y {{
}}

fn duplicate(x: Int) -> None => x, x {{
}}

fn undeclared_qual<T>(list: List<T>) -> None => list: A {{
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
fn stricter<T>(list: A B List<T>) -> None => list: B {{
}}

fn moves_anyway<T>(list: List<T>) -> None => !list {{
}}
"
    );
    let (program, checked) = check_src(&src);
    assert!(
        checked.errors.iter().all(|e| !e.is_error()),
        "errors: {:?}",
        checked.errors
    );
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
fn keep_all<T>(list: A B List<T>) [] -> None => list {{}}
fn strip_all<T>(list: A B List<T>) [] -> None => list: None {{}}
"
    );
    let (program, checked) = check_src(&src);
    assert!(
        checked.errors.iter().all(|e| !e.is_error()),
        "errors: {:?}",
        checked.errors
    );
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


fn list_size<T>(list: List<T>) [] -> Int => list {{ return 0 }}
"
    );
    let (program, checked) = check_src(&src);
    assert!(
        checked.errors.iter().all(|e| !e.is_error()),
        "errors: {:?}",
        checked.errors
    );
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

fn list_size<T>(list: List<T>) [] -> Int => list {{ return 0 }}
"
    );
    let (program, checked) = check_src(&src);
    assert!(
        checked.errors.iter().all(|e| !e.is_error()),
        "errors: {:?}",
        checked.errors
    );
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

fn bump<T>(store: Mut Store<T>) [] -> None => store: Mut {{}}

fn store_size<T>(store: Store<T>) [] -> Int => store {{ return 0 }}
"
    );
    let (program, checked) = check_src(&src);
    assert!(
        checked.errors.iter().all(|e| !e.is_error()),
        "errors: {:?}",
        checked.errors
    );
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

fn list_size<T>(list: List<T>) [] -> Int => list {{ return 0 }}
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

// ===== [fate-field-disjoint] L5: field-disjoint precision =====
//
// A fate link records *which projection* of the root the derived value came
// from, and an event poisons a link only when the two places overlap
// ([flow-place]'s `Place::overlaps`, the substrate P1 built). So reading one
// field while another is mutated is legal, and everything that could name the
// same storage still poisons: the same field, a prefix, the whole variable, and
// a dynamic index (which may alias any element).
//
// Before this, all four accept cases below were rejected and the remedy was
// `copy`, which costs a real clone on the Rust backend.

const DISJOINT_PRELUDE: &str = r#"
struct Person canbe Mut {
    name: Str,
    tags: Mut List<Str>
}

struct Outer {
    inner: Person,
    other: Str
}

fn touch(list: Mut List<Str>) [] -> None => list: Mut {}
fn touch_all(p: Mut Person) [] -> None => p: Mut {}
fn read(s: Str) [] -> Int => s { return 0 }
fn count(list: List<Str>) [] -> Int => list { return 0 }
fn add_tag(list: Mut List<Str>, s: Str) [] -> None => list: Mut, s {}
fn eat(list: Mut List<Str>) [] -> None => !list {}
fn take_person(p: Person) [] -> None => !p {}
"#;

fn disjoint_errors(body: &str) -> Vec<String> {
    // `Str` and `mut_list_of` are not in this file's shared std prelude, so
    // they are declared alongside it here (as std, since `intrinsic` is the
    // compiler's modifier [intrinsic-std-only]).
    let mut sources = SourceSet::default();
    sources.add(
        "std/core/prelude.sv",
        SourceSet::classify(Path::new("core/prelude.sv")).unwrap(),
        format!(
            "{STD_PRELUDE}export intrinsic type Str\n\
             export intrinsic fn mut_list_of<T>(...elems: T[]) [] -> Mut List<T>\n"
        ),
        true,
    );
    sources.add(
        "main.sv",
        SourceSet::classify(Path::new("main.sv")).unwrap(),
        format!("{DISJOINT_PRELUDE}\nfn probe() -> None {{\n{body}\n}}\n"),
        false,
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
    // Errors only: an unused-variable *warning* [unused-var] is a different
    // severity and these tests are about legality.
    checked
        .errors
        .iter()
        .filter(|e| e.is_error())
        .map(|e| e.message.clone())
        .collect()
}

/// [fate-field-disjoint] Reading `p.name` and mutating `p.tags` are disjoint,
/// so the derived value survives — the shape that motivated L5.
#[test]
fn a_disjoint_field_survives_a_mutation() {
    let errs = disjoint_errors(
        "    let p = Person { name: \"a\", tags: mut_list_of() }\n\
         \x20   let n = p.name\n\
         \x20   add_tag(p.tags, \"y\")\n\
         \x20   let k = read(n)",
    );
    assert!(errs.is_empty(), "expected a clean check, got: {errs:?}");
}

/// [fate-field-disjoint] Two disjoint fields of one struct, each reached
/// independently.
#[test]
fn two_disjoint_fields_are_independent() {
    let errs = disjoint_errors(
        "    let o = Outer { inner: Person { name: \"a\", tags: mut_list_of() }, other: \"o\" }\n\
         \x20   let n = o.inner.name\n\
         \x20   add_tag(o.inner.tags, \"y\")\n\
         \x20   let k = read(n)",
    );
    assert!(errs.is_empty(), "expected a clean check, got: {errs:?}");
}

/// [fate-field-disjoint] An assignment is an event on the field it writes:
/// writing `p.tags` leaves a value derived from `p.name` alone.
#[test]
fn an_assignment_to_a_disjoint_field_does_not_poison() {
    let errs = disjoint_errors(
        "    let p = Mut Person { name: \"a\", tags: mut_list_of() }\n\
         \x20   let n = p.name\n\
         \x20   p.tags = mut_list_of()\n\
         \x20   let k = read(n)",
    );
    assert!(errs.is_empty(), "expected a clean check, got: {errs:?}");
}

/// [fate-field-disjoint] The *same* field still poisons — the precision is
/// about disjointness, not about weakening the rule.
#[test]
fn the_same_field_still_poisons() {
    let errs = disjoint_errors(
        "    let p = Person { name: \"a\", tags: mut_list_of() }\n\
         \x20   let t = p.tags\n\
         \x20   add_tag(p.tags, \"y\")\n\
         \x20   let k = count(t)",
    );
    assert!(
        errs.iter().any(|e| e.contains("`t` cannot be used here")),
        "expected the poison error, got: {errs:?}"
    );
}

/// [fate-field-disjoint] A mutation of the **whole variable** is a prefix of
/// every projection, so it poisons everything derived from it — the
/// pre-L5 behavior, unchanged.
#[test]
fn a_whole_variable_mutation_still_poisons_every_field() {
    let errs = disjoint_errors(
        "    let p = Mut Person { name: \"a\", tags: mut_list_of() }\n\
         \x20   let n = p.name\n\
         \x20   touch_all(p)\n\
         \x20   let k = read(n)",
    );
    assert!(
        errs.iter().any(|e| e.contains("`n` cannot be used here")),
        "expected the poison error, got: {errs:?}"
    );
}

/// [fate-field-disjoint] A **prefix** overlaps: a value derived from
/// `o.inner` is poisoned by a mutation of `o.inner.tags`, since the mutation
/// changes part of what it names.
#[test]
fn a_prefix_projection_is_poisoned_by_a_deeper_mutation() {
    let errs = disjoint_errors(
        "    let o = Outer { inner: Person { name: \"a\", tags: mut_list_of() }, other: \"o\" }\n\
         \x20   let i = o.inner\n\
         \x20   add_tag(o.inner.tags, \"y\")\n\
         \x20   let k = read(i.name)",
    );
    assert!(
        errs.iter().any(|e| e.contains("`i` cannot be used here")),
        "expected the poison error, got: {errs:?}"
    );
}

/// [fate-field-disjoint] A **dynamic index** may land on any element, so
/// `arr[i]` and `arr[j]` are treated as possibly the same storage
/// ([`proj::Element`]'s may-alias rule) — precision stops where the analysis
/// cannot tell two places apart.
#[test]
fn a_dynamic_index_stays_conservative() {
    let errs = disjoint_errors(
        "    let arr: Mut List<Str>[] = [mut_list_of(), mut_list_of()]\n\
         \x20   let one = arr[0]\n\
         \x20   add_tag(arr[1], \"z\")\n\
         \x20   let k = count(one)",
    );
    assert!(
        errs.iter().any(|e| e.contains("`one` cannot be used here")),
        "expected the conservative poison, got: {errs:?}"
    );
}

// ===== [fate-partial-move] L5's move half: partial moves =====
//
// Moving a projection out leaves the *rest* of the root readable, instead of
// consuming the whole variable. The moved place is recorded per root, a read
// overlapping it is an error naming the field that left, and a whole-value use
// is always refused (a value missing a part cannot be handed on). Reassigning
// the place puts it back.
//
// This matches Rust, where partial moves are function-local: a borrowed
// parameter refuses a move out of it (kept deductions still error), and an
// owned one may be partially moved because the caller already gave it up.

/// [fate-partial-move] The shape the move half exists for: hand one field to
/// a consuming fn, keep reading the other.
#[test]
fn a_disjoint_field_survives_a_projection_move() {
    let errs = disjoint_errors(
        "    let p = Person { name: \"a\", tags: mut_list_of() }\n\
         \x20   eat(p.tags)\n\
         \x20   let k = read(p.name)",
    );
    assert!(errs.is_empty(), "expected a clean check, got: {errs:?}");
}

/// [fate-partial-move] The same through a move-mode *binding* rather than a
/// consuming call: `let t = p.tags` takes ownership of that field only.
#[test]
fn a_move_mode_binding_of_a_field_leaves_its_siblings() {
    let errs = disjoint_errors(
        "    let p = Person { name: \"a\", tags: mut_list_of() }\n\
         \x20   let t = p.tags\n\
         \x20   add_tag(t, \"z\")\n\
         \x20   let k = read(p.name)",
    );
    assert!(errs.is_empty(), "expected a clean check, got: {errs:?}");
}

/// [fate-partial-move] Reading the moved field back is the error, and it
/// names the field rather than the variable.
#[test]
fn reading_a_moved_field_back_is_an_error() {
    let errs = disjoint_errors(
        "    let p = Person { name: \"a\", tags: mut_list_of() }\n\
         \x20   eat(p.tags)\n\
         \x20   let k = count(p.tags)",
    );
    assert!(
        errs.iter()
            .any(|e| e.contains("`p.tags` cannot be used here")
                && e.contains("was moved out of `p`")),
        "expected the partial-move error, got: {errs:?}"
    );
}

/// [fate-partial-move] The whole value can never be used once a part has
/// left — the rule that keeps an incomplete struct from being handed on.
#[test]
fn the_whole_value_cannot_be_used_after_a_partial_move() {
    let errs = disjoint_errors(
        "    let p = Person { name: \"a\", tags: mut_list_of() }\n\
         \x20   eat(p.tags)\n\
         \x20   take_person(p)",
    );
    assert!(
        errs.iter()
            .any(|e| e.contains("`p` cannot be used as a whole here")),
        "expected the whole-value refusal, got: {errs:?}"
    );
}

/// [fate-partial-move] Assigning the place puts data back, so the variable is
/// whole again — including reads of the field that had left.
#[test]
fn reassigning_a_moved_field_revives_it() {
    let errs = disjoint_errors(
        "    let p = Mut Person { name: \"a\", tags: mut_list_of() }\n\
         \x20   eat(p.tags)\n\
         \x20   p.tags = mut_list_of()\n\
         \x20   let k = count(p.tags)\n\
         \x20   let j = read(p.name)",
    );
    assert!(errs.is_empty(), "expected a clean check, got: {errs:?}");
}

/// [fate-partial-move] Flow-sensitive like consumption: moved on *some* path
/// is moved after the join.
#[test]
fn a_move_on_one_branch_is_moved_after_the_join() {
    let errs = disjoint_errors(
        "    let p = Person { name: \"a\", tags: mut_list_of() }\n\
         \x20   if read(p.name) > 0 {\n\
         \x20       eat(p.tags)\n\
         \x20   }\n\
         \x20   let k = count(p.tags)",
    );
    assert!(
        errs.iter().any(|e| e.contains("`p.tags` cannot be used here")),
        "expected the merged move to poison, got: {errs:?}"
    );
    // ...but a disjoint field is still readable on every path.
    let ok = disjoint_errors(
        "    let p = Person { name: \"a\", tags: mut_list_of() }\n\
         \x20   if 1 > 0 {\n\
         \x20       eat(p.tags)\n\
         \x20   }\n\
         \x20   let k = read(p.name)",
    );
    assert!(ok.is_empty(), "expected a clean check, got: {ok:?}");
}

/// [fate-partial-move] A **kept** parameter still refuses the move outright:
/// the caller keeps the value, so nothing may be taken from it. This is
/// Rust's rule for a borrowed parameter (E0507), and it is why partial moves
/// never need to appear in a signature.
#[test]
fn a_kept_parameter_still_refuses_a_projection_move() {
    let src = format!(
        "{DISJOINT_PRELUDE}\nfn kept(p: Person) -> None => p {{\n    eat(p.tags)\n    return None\n}}\n"
    );
    let mut sources = SourceSet::default();
    sources.add(
        "std/core/prelude.sv",
        SourceSet::classify(Path::new("core/prelude.sv")).unwrap(),
        format!(
            "{STD_PRELUDE}export intrinsic type Str\n\
             export intrinsic fn mut_list_of<T>(...elems: T[]) [] -> Mut List<T>\n"
        ),
        true,
    );
    sources.add(
        "main.sv",
        SourceSet::classify(Path::new("main.sv")).unwrap(),
        src,
        false,
    );
    let mut modules = Vec::new();
    for file in &sources.files {
        let (module, diagnostics) = salvo_syntax::parse_module(&file.content);
        assert!(
            diagnostics.iter().all(|d| !d.is_error()),
            "parse errors: {diagnostics:?}"
        );
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
    let errs: Vec<String> = checked.errors.iter().map(|e| e.message.clone()).collect();
    assert!(
        errs.iter()
            .any(|e| e.contains("cannot move mutable data out of `p`")
                && e.contains("kept parameter")),
        "expected the kept-parameter refusal, got: {errs:?}"
    );
}

// ---- [deduce-syntax] the `=>` clause (user decisions 2026-09-11) ----

/// A written clause is *partial*: the parameters it mentions are fixed, the
/// rest are inferred from the body exactly as an unwritten clause's are.
#[test]
fn unmentioned_parameters_are_inferred_beside_written_ones() {
    let src = format!(
        "{QUALIFIED_LISTS}
fn mixed<T>(kept: A List<T>, eaten: List<T>, stripped: A B List<T>) -> None => stripped: B {{
    consume(eaten)
}}
"
    );
    let (program, checked) = check_src(&src);
    assert!(checked.errors.iter().all(|e| !e.is_error()), "{:?}", checked.errors);
    // Written: `stripped` keeps exactly `B`.
    let (kept, quals) = facts(&program, &checked, "mixed", "stripped");
    assert!(kept && quals == vec!["B".to_string()], "{kept} {quals:?}");
    // Inferred: `eaten` is consumed (the body hands it to `consume`),
    // `kept` is kept with its `A`.
    assert!(!facts(&program, &checked, "mixed", "eaten").0);
    let (kept, quals) = facts(&program, &checked, "mixed", "kept");
    assert!(kept && quals == vec!["A".to_string()], "{kept} {quals:?}");
}

/// A written entry is validated against the body; an unmentioned parameter
/// is not (there is nothing written to be wrong).
#[test]
fn written_entries_are_validated_and_inferred_ones_are_not() {
    let src = format!(
        "{QUALIFIED_LISTS}
fn bad<T>(list: List<T>, other: List<T>) -> None => list {{
    consume(list)
    consume(other)
}}
"
    );
    let (_, checked) = check_src(&src);
    let errs: Vec<&str> = checked.errors.iter().map(|e| e.message.as_str()).collect();
    assert!(
        errs.iter().any(|e| e.contains("promises `list` back to the caller, but the body moves it")),
        "{errs:?}"
    );
    assert!(!errs.iter().any(|e| e.contains("`other`")), "{errs:?}");
}

/// A bodiless declaration must mention every parameter except Copy scalars.
#[test]
fn a_bodiless_declaration_must_write_its_whole_clause() {
    let src = format!(
        "{QUALIFIED_LISTS}
effect Sink {{
    fn eat<T>(list: List<T>, n: Int) -> None
    fn fine<T>(list: List<T>, n: Int) -> None => !list
}}
"
    );
    let (_, checked) = check_src(&src);
    let errs: Vec<&str> = checked.errors.iter().map(|e| e.message.as_str()).collect();
    assert!(
        errs.iter().any(|e| e.contains("must say what happens to `list`")),
        "{errs:?}"
    );
    assert!(!errs.iter().any(|e| e.contains("`n`")), "Copy scalars are exempt: {errs:?}");
    assert_eq!(errs.iter().filter(|e| e.contains("must say what happens")).count(), 1);
}

/// `=>[f] !t` scopes a consumption to a fn-typed parameter (its parameter
/// named in the type), and `proj(a, b)` joins two sources.
#[test]
fn fn_type_groups_and_multi_source_projections() {
    let src = format!(
        "{QUALIFIED_LISTS}
fn apply<T>(list: List<T>, f: (t: List<T>) -> Int) -> Int =>[f] !t {{
    return f(list)
}}
fn either<T>(a: List<T>, b: List<T>, flag: Bool) -> proj(a, b) List<T> {{
    if flag {{
        return a
    }}
    return b
}}
fn main<T>(a: List<T>, b: List<T>) -> None {{
    let v = either(a, b, true)
    consume(a)
    let n = v
}}
"
    );
    let (_, checked) = check_src(&src);
    let errs: Vec<&str> = checked.errors.iter().map(|e| e.message.as_str()).collect();
    // The call through `f` consumes `list` (the group says so), so `apply`
    // is inferred to consume it too — no error, just the fact.
    assert!(
        errs.iter().any(|e| e.contains("`v` cannot be used here") && e.contains("`a`")),
        "the projection must be linked to both sources: {errs:?}"
    );
    assert!(
        !errs.iter().any(|e| e.contains("returns `proj(a, b)`, so every returned value")),
        "either branch is a valid source: {errs:?}"
    );
}

/// [readonly-return] [proj-anywhere] A derived return checks the *returned
/// expression*, not the first argument of a one-argument call: dot-notation
/// (`list.pick(index)`) leaves exactly one written argument, and the
/// constructor-operand unwrap that a `proj`-arm union return needs must not
/// apply to a plain `proj(p) T` return — it used to, so the checker
/// asked the *index* where it came from and rejected the call, while
/// accepting a one-argument call that borrows nothing.
#[test]
fn derived_return_checks_the_call_not_its_only_argument() {
    let src = format!(
        "{QUALIFIED_LISTS}
fn pick<T>(list: List<T>, index: Int) -> proj(list) List<T> {{
    return list
}}
fn owned<T>(list: List<T>) -> List<T> {{
    return copy(list)
}}
fn forward<T>(list: List<T>, index: Int) -> proj(list) List<T> {{
    return list.pick(index)
}}
fn leak<T>(list: List<T>) -> proj(list) List<T> {{
    return owned(list)
}}
"
    );
    let (_, checked) = check_src(&src);
    let errs: Vec<&str> = checked.errors.iter().map(|e| e.message.as_str()).collect();
    let derived: Vec<&&str> = errs
        .iter()
        .filter(|e| e.contains("so every returned value must be derived from"))
        .collect();
    // One error, and it is `leak`'s: `forward` returns the borrow `pick`
    // handed back, however the call is spelled.
    assert_eq!(derived.len(), 1, "{errs:?}");
    let span_ok = checked
        .errors
        .iter()
        .filter(|d| d.message.contains("so every returned value must be derived from"))
        .all(|d| src[d.span.start as usize..d.span.end as usize].starts_with("owned("));
    assert!(span_ok, "the error must point at the returned call itself");
}

// ===== [deduce-reapply] a claim the function re-establishes =====

/// The whole point (user decision 2026-09-22, ROADMAP's D2): a mutator keeps a
/// user qualifier across a `Mut` parameter by saying it **establishes** it.
/// Plain `Q` stays what it always was — a claim the body must have preserved —
/// and that is the distinction the spelling exists to draw.
fn reapply_errors(src: &str) -> Vec<String> {
    let (_, checked) = check_src(src);
    checked
        .errors
        .iter()
        .filter(|d| d.is_error())
        .map(|d| d.message.clone())
        .collect()
}

/// The same over several files, for the same-file rule — which can only be
/// tested by putting the qualifier somewhere else.
fn reapply_errors_in(files: &[(&str, &str)]) -> Vec<String> {
    let mut sources = SourceSet::default();
    sources.add(
        "std/core/prelude.sv",
        SourceSet::classify(Path::new("core/prelude.sv")).unwrap(),
        STD_PRELUDE.to_string(),
        true,
    );
    for (name, src) in files {
        sources.add(
            *name,
            SourceSet::classify(Path::new(name)).unwrap(),
            src.to_string(),
            false,
        );
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
    let checked = check_program(&program, &resolution, &symbols);
    checked
        .errors
        .iter()
        .filter(|d| d.is_error())
        .map(|d| d.message.clone())
        .collect()
}

/// The shared shape: a qualifier whose file owns a mutator over it. `push`
/// mutates through `add`, whose own exhaustive clause strips the claim, and
/// nothing but `push` knows it is re-established.
const HEAPISH: &str = r#"
qualifier H<T> of List<T>

fn push<T>(heap: H<T> Mut List<T>, elem: T) [] -> None
=> heap: +H<T> Mut, !elem {
    add(heap, elem)
}
"#;

#[test]
fn a_reapplied_claim_survives_a_mutation() {
    let errs = reapply_errors(HEAPISH);
    assert!(errs.is_empty(), "unexpected errors: {errs:?}");
}

/// …and the plain spelling of the same signature still fails, which is what
/// makes `+` mean something.
#[test]
fn the_plain_spelling_of_the_same_claim_still_fails() {
    let errs = reapply_errors(&HEAPISH.replace("+H<T> Mut", "H Mut"));
    assert!(
        errs.iter()
            .any(|e| e.contains("promises qualifier `H` on `heap`, but the body may remove it")),
        "expected the body-validation error: {errs:?}"
    );
}

/// The trust is gated to the qualifier's **own file** [qual-ctor-same-file]:
/// the same party already trusted to mint the claim, and nobody else.
#[test]
fn only_the_qualifiers_own_file_may_reestablish_it() {
    let mut sources = vec![("q.sv", "export qualifier H<T> of List<T>\n")];
    let other = r#"
import q.H

fn push<T>(heap: H<T> Mut List<T>, elem: T) [] -> None
=> heap: +H<T> Mut, !elem {
    add(heap, elem)
}
"#;
    sources.push(("main.sv", other));
    let errs = reapply_errors_in(&sources);
    assert!(
        errs.iter()
            .any(|e| e.contains("which this file does not declare")),
        "expected the same-file refusal: {errs:?}"
    );
}

/// A compiler qualifier is not a claim about the value and nothing could
/// establish it, so `+Mut` is refused outright rather than ignored.
#[test]
fn a_compiler_qualifier_cannot_be_reestablished() {
    let errs = reapply_errors(
        "fn f(xs: Mut List<Int>) [] -> None => xs: +Mut {\n}\n",
    );
    assert!(
        errs.iter()
            .any(|e| e.contains("`Mut` cannot be established")),
        "expected the compiler-qualifier refusal: {errs:?}"
    );
}

/// Re-establishing is not *adding*: the qualifier has to be one the parameter
/// declares, so the claim the caller ends up with is the parameter's own.
/// [deduce-reapply] A claim the parameter does **not** hold can be established
/// too (user decision 2026-09-22): `heapify(list)` takes an ordinary list and
/// hands back a heap, which is the `+Q` form's other half — the same trust, and
/// the same same-file confinement. The plain spelling could never say this: it
/// may only preserve what the parameter declares.
#[test]
fn a_claim_the_parameter_lacks_can_be_established() {
    let errs = reapply_errors(
        "qualifier H<T> of List<T>\n\n\
         fn heapify<T>(xs: Mut List<T>) [] -> None\n=> xs: +H<T> Mut {\n}\n",
    );
    assert!(errs.is_empty(), "unexpected errors: {errs:?}");
}

/// [deduce-reapply] …and it reaches the caller **whatever the order of the
/// declarations**: what a call needs to know about a callee comes out of a side
/// table, and a table filled as the check walks makes the answer depend on where
/// the callee sits. That had already cost the implicit parameters once
/// (2026-09-22); this pins the same property for the claims a fn establishes,
/// with the callee written *below* its caller.
#[test]
fn an_established_claim_does_not_depend_on_declaration_order() {
    let errs = reapply_errors(
        "qualifier H<T> of List<T>\n\n\
         fn only_heap<T>(xs: H<T> Mut List<T>) [] -> Int => xs {\n    return 1\n}\n\n\
         fn caller(xs: Mut List<Int>) [] -> Int => xs: Mut {\n    \
         heapify(xs)\n    return only_heap(xs)\n}\n\n\
         fn heapify<T>(xs: Mut List<T>) [] -> None\n=> xs: +H<T> Mut {\n}\n",
    );
    assert!(errs.is_empty(), "unexpected errors: {errs:?}");
}

/// …and the claim has to make sense for the parameter's type, on the terms a
/// constructor's `as Q` answers to [qual-ctor-fn].
#[test]
fn an_established_claim_must_apply_to_the_parameter() {
    let errs = reapply_errors(
        "qualifier H<T> of List<T>\n\n\
         fn f(n: Int) [] -> None\n=> n: +H {\n}\n",
    );
    assert!(
        errs.iter()
            .any(|e| e.contains("does not apply to") && e.contains("cannot establish it")),
        "expected the of-type refusal: {errs:?}"
    );
}

/// [cmp-carry] A qualifier that **holds a function** cannot be established
/// without naming it: every operation that reads the claim binds the slot
/// [cmp-binder], so a claim with no identity is one nothing can use. The
/// remedy is in the message.
#[test]
fn an_established_claim_must_name_the_identity_it_holds() {
    let errs = reapply_errors(
        "qualifier H<T>(?cmp: (T, T) -> Int) of List<T>\n\n\
         fn heapify<T>(xs: Mut List<T>, ?cmp: (T, T) -> Int) [] -> None\n\
         => xs: +H Mut {\n}\n",
    );
    assert!(
        errs.iter()
            .any(|e| e.contains("establishes a claim with no identity") && e.contains("?cmp")),
        "expected the missing-identity refusal: {errs:?}"
    );
}
