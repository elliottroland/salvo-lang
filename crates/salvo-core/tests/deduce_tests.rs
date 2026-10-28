//! Deduction inference and validation [deduce-syntax] [deduce-infer]: what
//! the checker *records* about a clause — whether a parameter is kept, which
//! qualifiers survive, the effect's form — read from its tables, which
//! annotations cannot. The diagnostics are test data, `tests/check/deduce/`
//! (TESTS.md T1). The `std = true` tests at the end keep a prelude of their
//! own: their point is a tree that *is* std.

use std::path::Path;

use salvo_core::{check_program, resolve, Checked, DeclId, Program, SourceSet, Symbols};
use salvo_syntax::ast::Item;

/// `src` checked over the real std, asserting it reports no error.
fn checked(src: &str) -> (Program, Checked) {
    let out = salvo_testkit::front::check_main(src);
    let errors = out.errors();
    assert!(errors.is_empty(), "unexpected errors: {errors:#?}");
    (out.program, out.checked)
}

/// Finds the `DeclId` of a top-level fn of the user's files by name.
fn fn_key(program: &Program, checked: &Checked, name: &str) -> DeclId {
    for (file, ast) in program.modules.iter().enumerate() {
        if program.files[file].is_std {
            continue;
        }
        for (item, it) in ast.items.iter().enumerate() {
            if let Item::Fn(f) = it {
                if f.name.name == name {
                    return checked.decls.fn_at(file, item);
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
    let key = fn_key(program, checked, fn_name);
    let ded = checked.deductions[&key]
        .iter()
        .find(|d| d.param == param)
        .unwrap_or_else(|| panic!("no deduction entry for `{param}` in `{fn_name}`"));
    let Some(decl) = checked.decls.fn_decl(&program.modules, key) else {
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
    let key = fn_key(program, checked, fn_name);
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
qualifier B<T> of List<T>
qualifier C<T> of List<T>

fn drop_a<T>(list: A B List<T>) [] -> None => list: B {}
fn consume<T>(list: List<T>) [] -> None => !list {}
"#;

// [deduce-default] A call that only *reads* its argument keeps it; the
// caller's own parameter keeps its default (keep-all), independent of what
// the callee's qualifiers are — there is no inference that subtracts the
// callee's removal set from the caller any more (user decision 2026-10-11).
// To drop a qualifier the caller must say so with a written entry.
#[test]
fn a_reading_caller_keeps_its_default() {
    let src = format!(
        "{QUALIFIED_LISTS}
fn caller<T>(list: A B List<T>) -> None => list: B {{
    drop_a(list)
}}
"
    );
    let (program, checked) = checked(&src);
    let (kept, quals) = facts(&program, &checked, "caller", "list");
    assert!(kept);
    assert_eq!(quals, vec!["B".to_string()]);
}

// [deduce-syntax] A *plain* (exhaustive) written list is exactly what
// survives: a qualifier not named is dropped. The body is checked against it
// — here `drop_a` keeps `list`, so promising `B` is sound.
#[test]
fn exhaustive_lists_drop_undeclared_qualifiers() {
    let src = format!(
        "{QUALIFIED_LISTS}
fn caller<T>(list: A B C List<T>) -> None => list: B {{
    drop_a(list)
}}
"
    );
    let (program, checked) = checked(&src);
    let (kept, quals) = facts(&program, &checked, "caller", "list");
    assert!(kept);
    assert_eq!(quals, vec!["B".to_string()]);
    assert_eq!(effect_form(&program, &checked, "caller", "list"), "exhaustive: B");
}

// [deduce-syntax] A *delta* list (`-A`) drops only what it names, so the
// other declared qualifiers survive.
#[test]
fn delta_lists_pass_undeclared_qualifiers_through() {
    let src = format!(
        "{QUALIFIED_LISTS}
fn shed_a<T>(list: A B List<T>) [] -> None => list: -A {{}}

fn caller<T>(list: A B C List<T>) -> None => list: -A {{
    shed_a(list)
}}
"
    );
    let (program, checked) = checked(&src);
    let (kept, quals) = facts(&program, &checked, "caller", "list");
    assert!(kept);
    assert_eq!(quals, vec!["B".to_string(), "C".to_string()]);
    assert_eq!(effect_form(&program, &checked, "caller", "list"), "remove: A");
}

// [deduce-default] A `Mut` parameter defaults to the exhaustive `: Mut`
// form — only `Mut` survives, since a mutable parameter the body may change
// cannot promise other qualifiers back (user decision 2026-10-11). The
// exhaustive `=> list: Mut` written out is the same form. (A written
// `=> list` promising more is refused: `deduce/clauses.sv`'s `keeps_all`.)
#[test]
fn a_mut_parameter_defaults_to_exhaustive_mut() {
    let src = "

qualifier A<T> of List<T>

fn mutate<T>(list: Mut List<T>) [] -> None => list: Mut {}

fn defaulted<T>(list: Mut A List<T>) -> None {
    mutate(list)
}

fn exhaustive<T>(list: Mut A List<T>) -> None => list: Mut {
    mutate(list)
}
";
    let (program, checked) = checked(src);
    // The default keeps only `Mut`, never `A`.
    assert_eq!(effect_form(&program, &checked, "defaulted", "list"), "exhaustive: Mut");
    assert_eq!(effect_form(&program, &checked, "exhaustive", "list"), "exhaustive: Mut");
}

// [deduce-syntax] `[p: Never]` / `!p` is the moved form — the caller loses
// the value.
#[test]
fn nothing_in_a_deduction_means_moved() {
    let src = format!(
        "{QUALIFIED_LISTS}
fn eat<T>(list: List<T>) [] -> None => !list {{}}

fn caller<T>(list: List<T>) -> None => !list {{
    eat(list)
}}
"
    );
    let (program, checked) = checked(&src);
    let (kept, _) = facts(&program, &checked, "caller", "list");
    assert!(!kept, "the parameter should be moved");
}

// [deduce-default] …declaring the move with `=> !p` settles an undeclared
// move (`deduce/clauses.sv`), on both shapes.
// [fate-move-mode] S2: a binding that is later *moved* takes ownership; a
// move — returning a parameter, or passing it to a consuming call — must now
// be **declared** with `=> !p` (user decision 2026-10-11, replacing the claim
// that used to make an unwritten parameter moved). With the clause the body
// compiles, and the parameter is recorded moved.
#[test]
fn a_declared_move_is_accepted() {
    let src = format!(
        "{QUALIFIED_LISTS}
fn pass_back<T>(list: List<T>) -> List<T> => !list {{
    return list
}}

fn caller<T>(list: List<T>) -> None => !list {{
    consume(list)
}}
"
    );
    let (program, checked) = checked(&src);
    assert!(!facts(&program, &checked, "pass_back", "list").0);
    assert!(!facts(&program, &checked, "caller", "list").0);
}

// [call-resolve] An effect member's *declared* deduction list is the callee
// contract: a member that takes ownership moves its argument, so a fn passing
// its own parameter to that member must declare the move too (`=> !list`).
#[test]
fn effect_member_moves_must_be_declared() {
    let src = format!(
        r#"{QUALIFIED_LISTS}
effect Sink {{
    fn eat(list: List<Int>) -> None => !list
}}

fn forward(list: List<Int>) [Sink] -> None => !list {{
    eat(list)
}}
"#
    );
    let (program, checked) = checked(&src);
    let (kept, _) = facts(&program, &checked, "forward", "list");
    assert!(!kept, "`forward` declares the move the member forces");
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

fn peek_at<T>(list: List<T>) [] -> Int => list {{ return 0 }}

fn caller<T>(list: A B List<T>) [Logger] -> Int {{
    let _s = peek_at(list)
    log(list)
    return 1
}}
"#
    );
    let (program, checked) = checked(&src);
    assert_eq!(facts(&program, &checked, "caller", "list"), (true, vec!["A".to_string(), "B".to_string()]));
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
    let (program, checked) = checked(&src);
    assert_eq!(facts(&program, &checked, "stricter", "list"), (true, vec!["B".to_string()]));
    assert!(!facts(&program, &checked, "moves_anyway", "list").0);
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
    let (program, checked) = checked(&src);
    assert_eq!(facts(&program, &checked, "keep_all", "list"), (true, vec!["A".to_string(), "B".to_string()]));
    assert_eq!(facts(&program, &checked, "strip_all", "list"), (true, Vec::<String>::new()));
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
    let _alias = list
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
    let (program, checked) = checked(&src);
    assert_eq!(facts(&program, &checked, "links", "list"), (true, vec!["A".to_string()]));
    assert_eq!(facts(&program, &checked, "reads", "list"), (true, vec!["A".to_string()]));
    assert!(facts(&program, &checked, "escapes", "list").0);
}

// [deduce-default] A lambda capture of a parameter declared moved may own
// it; a **read**-only capture keeps the parameter. (The mutating capture of
// a kept parameter is refused: `deduce/clauses.sv`'s `mutates`.)
#[test]
fn a_lambda_capture_of_a_moved_or_read_parameter() {
    let src = format!(
        "{QUALIFIED_LISTS}

fn runs(f: () -> None) -> None {{
    f()
}}

fn moved<T>(store: Mut List<T>) -> None => !store {{
    let g = () -> {{ bump(store) }}
    runs(g)
}}

fn reads<T>(store: Mut List<T>) -> None {{
    let g = () -> {{ let _n = store_size(store) }}
    runs(g)
}}

fn bump<T>(store: Mut List<T>) [] -> None => store: Mut {{}}

fn store_size<T>(store: List<T>) [] -> Int => store {{ return 0 }}
"
    );
    let (program, checked) = checked(&src);
    // `moved` declares the move, so the capture may own it.
    assert!(!facts(&program, &checked, "moved", "store").0);
    // A read-only capture keeps it, no error.
    assert!(facts(&program, &checked, "reads", "store").0);
}

// ---- [deduce-syntax] the `=>` clause (user decisions 2026-09-11) ----

/// [deduce-default] A written clause is *partial*: the parameters it mentions
/// are overridden, the rest take their **default** (not inferred from the
/// body). So a consumed parameter the clause omits must still be declared.
#[test]
fn unmentioned_parameters_take_their_default_beside_written_ones() {
    let src = format!(
        "{QUALIFIED_LISTS}
fn mixed<T>(kept: A List<T>, eaten: List<T>, stripped: A B List<T>) -> None => stripped: B, !eaten {{
    consume(eaten)
}}
"
    );
    let (program, checked) = checked(&src);
    // Written: `stripped` keeps exactly `B`.
    let (kept, quals) = facts(&program, &checked, "mixed", "stripped");
    assert!(kept && quals == vec!["B".to_string()], "{kept} {quals:?}");
    // `eaten` is moved (written); `kept` takes its default (keep-all), so it
    // keeps its declared `A`.
    assert!(!facts(&program, &checked, "mixed", "eaten").0);
    let (kept, quals) = facts(&program, &checked, "mixed", "kept");
    assert!(kept && quals == vec!["A".to_string()], "{kept} {quals:?}");
}

/// [proj-readonly-return] A derived return's refusal points at the returned
/// call itself — the column, which an annotation's line cannot say (the
/// diagnostic is `deduce/clauses.sv`'s `leak`).
#[test]
fn a_derived_return_error_spans_the_returned_call() {
    let src = "fn owned<T>(list: List<T>) -> List<T> {\n    return copy(list)\n}\n\n\
               fn leak<T>(list: List<T>) -> proj(list) List<T> {\n    return owned(list)\n}\n";
    let out = salvo_testkit::front::check_main(src);
    assert_eq!(out.ids(), vec!["proj-readonly-return:not-derived"], "{:#?}", out.errors());
    let d = out.diagnostics().next().unwrap();
    assert!(src[d.span.start as usize..d.span.end as usize].starts_with("owned("), "the error must point at the returned call itself");
}

// ===== [deduce-std-explicit] A `std = true` tree needs total clauses =====
//
// These keep a prelude of their own: their point is a tree that *is* std
// (`mark_std_tree`), which the real std's front does not build.

/// [platform-intrinsic-std-only] The intrinsic declarations these sources rely on.
/// `intrinsic` is the compiler's modifier and only std may write it, so this
/// is loaded as a *std* file rather than pasted into the source under test.
const STD_PRELUDE: &str = concat!(
    "export intrinsic type Int\nexport intrinsic type List<T> canbe Mut\nexport intrinsic fn copy<T>(value: T) [] -> T => value\n",
    // [op-arith] What the arithmetic operators call.
    "export intrinsic fn plus(a: Int, b: Int) [] -> Int => a, b\n",
);

/// Checks `src` as the one file of a `std = true` tree (so it is
/// `is_std && is_shadow`, exactly what `mark_std_tree` sets), with the
/// prelude as embedded std. Returns the rendered error messages.
fn check_std_tree(src: &str) -> Vec<(Option<&'static str>, String)> {
    let mut sources = SourceSet::default();
    sources.add(
        "std/core/prelude.sv",
        SourceSet::classify(Path::new("std/core/prelude.sv")).unwrap(),
        STD_PRELUDE.to_string(),
        true,
    );
    sources.add("main.sv", SourceSet::classify(Path::new("main.sv")).unwrap(), src.to_string(), false);
    // The whole tree *is* std, like std's own manifest (`std = true`).
    sources.mark_std_tree();
    let mut modules = Vec::new();
    for file in &sources.files {
        let (module, diagnostics) = salvo_syntax::parse_module(&file.content);
        let errors: Vec<_> = diagnostics.iter().filter(|d| d.is_error()).collect();
        assert!(errors.is_empty(), "parse errors: {errors:?}");
        modules.push(module);
    }
    let _expansion = salvo_core::expand(&sources.files, &mut modules);
    let program = Program { files: sources.files, modules: Program::share(modules), companions: Vec::new() };
    let symbols = Symbols::collect(&program);
    let resolution = resolve(&program);
    let checked = check_program(&program, &resolution, &symbols);
    checked.errors.iter().filter(|d| d.is_error()).map(|d| (d.id, d.message.clone())).collect()
}

/// [deduce-std-explicit] The refusal of an exported std fn's partial clause.
const UNCOVERED: Option<&str> = Some("deduce-std-explicit:uncovered-params");

/// [deduce-std-explicit] An exported bodied fn with a non-scalar parameter
/// and no clause is refused, and the diagnostic names the missing parameter
/// and the inferred clause to write.
#[test]
fn an_exported_std_fn_without_a_clause_is_refused() {
    let errs = check_std_tree("export fn keep(s: List<Int>) [] -> List<Int> {\n    return s\n}\n");
    assert!(
        errs.iter().any(|(id, _)| *id == UNCOVERED),
        "expected the missing-clause refusal: {errs:?}"
    );
    // `keep` returns its parameter, so the inferred clause moves it. Kept on
    // the text: the point is the clause the message suggests.
    assert!(errs.iter().any(|(_, m)| m.contains("=> !s")), "the diagnostic should suggest the inferred total clause: {errs:?}");
}

/// [deduce-std-explicit] A total clause (every non-scalar parameter named)
/// satisfies the rule.
#[test]
fn an_exported_std_fn_with_a_total_clause_is_accepted() {
    let errs = check_std_tree("export fn keep(s: List<Int>) [] -> List<Int> => !s {\n    return s\n}\n");
    assert!(!errs.iter().any(|(id, _)| *id == UNCOVERED), "a total clause should satisfy the rule: {errs:?}");
}

/// [deduce-std-explicit] A *partial* clause — present, but not naming every
/// parameter — is still refused: an unmentioned parameter is still inferred,
/// so it is not pinned, which is the whole point of the rule.
#[test]
fn a_partial_std_clause_is_still_refused() {
    let errs = check_std_tree("export fn two(a: List<Int>, b: List<Int>) [] -> List<Int> => !a {\n    return a\n}\n");
    assert!(
        errs.iter().any(|(id, _)| *id == UNCOVERED),
        "the unmentioned parameter should still be refused: {errs:?}"
    );
}

/// [deduce-std-explicit] [copy-scalar-free] A Copy scalar needs no entry —
/// its keep-all is invariant, so leaving it out is sound and not flagged.
#[test]
fn a_scalar_parameter_needs_no_entry() {
    let errs = check_std_tree("export fn id(n: Int) [] -> Int {\n    return n\n}\n");
    assert!(!errs.iter().any(|(id, _)| *id == UNCOVERED), "a scalar-only exported fn should not be flagged: {errs:?}");
}

/// [deduce-std-explicit] The rule is about the public surface: a
/// module-private fn is not flagged (it is not something the cache keys on).
#[test]
fn a_private_std_fn_is_not_flagged() {
    let errs = check_std_tree("fn helper(s: List<Int>) [] -> List<Int> {\n    return s\n}\n");
    assert!(!errs.iter().any(|(id, _)| *id == UNCOVERED), "a private fn should not be flagged: {errs:?}");
}
