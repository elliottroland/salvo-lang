//! [elem-distinct] Distinct awareness for mutable element handles
//! (group-borrowing ladder step ②): element links carry the identity of
//! their minting index, poison consults a live `NotEq` claim before
//! killing a sibling handle, the identity dies with a reassignment of the
//! index, and one call may take two handles only when they are proven
//! apart.

use std::path::Path;

use salvo_core::{check_program, resolve, Program, SourceSet, Symbols};

const STD_PRELUDE: &str =
    "export intrinsic type Str\nexport intrinsic type Int\nexport intrinsic type Bool\n";

/// A miniature `core.list`: the mint recognition is nominal — `get`
/// declared by the `core.list` module — so the tests declare exactly the
/// surface the feature reads [elem-distinct], plus `NotEq` itself
/// [col-noteq].
const STD_LIST: &str = "\
export intrinsic type List<T> canbe Mut\n\
export intrinsic fn list_of<T>(first: T, ...rest: T[]) [] -> List<T> => !first\n\
export intrinsic fn size<T>(list: List<T>) [] -> Int => list\n\
export intrinsic fn get<T>(list: List<T>, index: Int) [] -> (proj(list) T)? => list, index\n\
export qualifier Idx<T>(list: List<T>) of Int {\n\
    fn qualifies(index: Int, list: List<T>) -> Bool {\n\
        return index >= 0 && index < size(list)\n\
    }\n\
}\n\
export fn get<T>(list: List<T>, index: Idx(list) Int) [] -> proj(list) T\n\
=> list, index {\n\
    return get(list, index + 0)!\n\
}\n\
export qualifier NotEq(i: Int) of Int with Idx {\n\
    fn qualifies(j: Int, i: Int) -> Bool {\n\
        return j != i\n\
    }\n\
}\n";

/// Parses + resolves + checks one file against the mini std and returns
/// every error message.
fn errors(src: &str) -> Vec<String> {
    let mut sources = SourceSet::default();
    sources.add(
        "std/core/prelude.sv",
        SourceSet::classify(Path::new("core/prelude.sv")).unwrap(),
        STD_PRELUDE.to_string(),
        true,
    );
    sources.add(
        "std/core/list.sv",
        SourceSet::classify(Path::new("core/list.sv")).unwrap(),
        STD_LIST.to_string(),
        true,
    );
    sources.add(
        "main.sv",
        SourceSet::classify(Path::new("main.sv")).unwrap(),
        src.to_string(),
        false,
    );
    let mut modules = Vec::with_capacity(sources.files.len());
    for file in &sources.files {
        let (ast, diagnostics) = salvo_syntax::parse_module(&file.content);
        let parse_errors: Vec<_> = diagnostics.iter().filter(|d| d.is_error()).collect();
        assert!(
            parse_errors.is_empty(),
            "parse errors in {}: {parse_errors:?}",
            file.name
        );
        modules.push(ast);
    }
    let program = Program {
        files: sources.files,
        modules,
        companions: Vec::new(),
    };
    let symbols = Symbols::collect(&program);
    let resolution = resolve(&program);
    let checked = check_program(&program, &resolution, &symbols);
    resolution
        .errors
        .iter()
        .chain(checked.errors.iter())
        .filter(|d| d.is_error())
        .map(|d| d.message.clone())
        .collect()
}

const PRELUDE: &str = "\
struct Entity canbe Mut { hp: Int }\n\
fn attack(a: Mut Entity, d: Mut Entity) [] -> None => a: Mut, d: Mut {\n\
    a.hp = a.hp - 1\n\
    d.hp = d.hp - 2\n\
    return None\n\
}\n";

/// [elem-distinct] Two bound handles whose minting indices a live
/// `NotEq` claim proves apart survive each other's mutations: the
/// sibling names disjoint storage, so the poison spares it.
#[test]
fn two_proven_distinct_handles_coexist_across_mutation() {
    let src = format!(
        "{PRELUDE}\
         fn f(es: List<Mut Entity>, i: Int, j: Int) [] -> None {{\n    \
         if j is NotEq(i) {{\n        \
         let a = get(es, i)!\n        \
         let d = get(es, j)!\n        \
         a.hp = a.hp + 1\n        \
         d.hp = d.hp + 1\n    \
         }}\n    \
         return None\n}}\n"
    );
    assert!(errors(&src).is_empty(), "got {:?}", errors(&src));
}

/// [elem-distinct] Without the claim, today's conservative rule stands: a
/// computed index may alias every element [fate-field-disjoint], so the
/// mutation through one handle poisons the sibling [fate-poison].
#[test]
fn without_a_claim_the_sibling_handle_poisons() {
    let src = format!(
        "{PRELUDE}\
         fn f(es: List<Mut Entity>, i: Int, j: Int) [] -> None => es, i, j {{\n    \
         let a = get(es, i)!\n    \
         let d = get(es, j)!\n    \
         a.hp = a.hp + 1\n    \
         d.hp = d.hp + 1\n    \
         return None\n}}\n"
    );
    let errs = errors(&src);
    assert!(
        errs.iter().any(|e| e.contains("mutated after the binding")),
        "expected the sibling poison: {errs:?}"
    );
}

/// [elem-distinct] The identity means "the element selected by the index
/// variable's *current* value": reassigning the index erases it, and the
/// conservative poison returns — even though the `NotEq` claim's
/// variables still exist.
#[test]
fn reassigning_the_index_restores_the_conservative_poison() {
    let src = format!(
        "{PRELUDE}\
         fn f(es: List<Mut Entity>, i: Int, j: Int) [] -> None => es, i, j {{\n    \
         let k = i + 0\n    \
         if j is NotEq(k) {{\n        \
         let a = get(es, k)!\n        \
         let d = get(es, j)!\n        \
         k = k + 1\n        \
         a.hp = a.hp + 1\n        \
         d.hp = d.hp + 1\n    \
         }}\n    \
         return None\n}}\n"
    );
    let errs = errors(&src);
    assert!(
        errs.iter().any(|e| e.contains("mutated after the binding")),
        "expected the erased identity to restore the poison: {errs:?}"
    );
}

/// [elem-distinct] The same index minted twice is certainly the same
/// element: no claim can spare it, and none is consulted — the identities
/// are equal, not distinct.
#[test]
fn the_same_index_still_poisons() {
    let src = format!(
        "{PRELUDE}\
         fn f(es: List<Mut Entity>, i: Int, j: Int) [] -> None => es, i, j {{\n    \
         if j is NotEq(i) {{\n        \
         let a = get(es, i)!\n        \
         let d = get(es, i)!\n        \
         a.hp = a.hp + 1\n        \
         d.hp = d.hp + 1\n    \
         }}\n    \
         return None\n}}\n"
    );
    let errs = errors(&src);
    assert!(
        errs.iter().any(|e| e.contains("mutated after the binding")),
        "expected the same-element poison: {errs:?}"
    );
}

/// [elem-distinct] One call may take two mutable element handles of one
/// container when they are proven apart — the shape the Rust backend
/// renders through `salvo_pair_mut` [rs-elem-mut].
#[test]
fn a_proven_pair_may_land_in_one_call() {
    let src = format!(
        "{PRELUDE}\
         fn f(es: List<Mut Entity>, i: Int, j: Int) [] -> None => es, i, j {{\n    \
         if j is NotEq(i) {{\n        \
         attack(get(es, i)!, get(es, j)!)\n    \
         }}\n    \
         return None\n}}\n"
    );
    assert!(errors(&src).is_empty(), "got {:?}", errors(&src));
}

/// [elem-distinct] Without the proof the pair is refused at the call,
/// naming the remedy — the shape used to pass the checker and die at
/// rustc (E0499), a checker/emitter disagreement closed with this rule.
#[test]
fn an_unproven_pair_in_one_call_is_refused() {
    let src = format!(
        "{PRELUDE}\
         fn f(es: List<Mut Entity>, i: Int, j: Int) [] -> None => es, i, j {{\n    \
         attack(get(es, i)!, get(es, j)!)\n    \
         return None\n}}\n"
    );
    let errs = errors(&src);
    assert!(
        errs.iter().any(|e| e.contains("prove them apart")),
        "expected the unproven-pair refusal: {errs:?}"
    );
}

/// [elem-distinct] Two *bound* handles land in one call the same way —
/// the proof travels with the links, not with the argument shape.
#[test]
fn a_proven_bound_pair_may_land_in_one_call() {
    let src = format!(
        "{PRELUDE}\
         fn f(es: List<Mut Entity>, i: Int, j: Int) [] -> None {{\n    \
         if j is NotEq(i) {{\n        \
         let a = get(es, i)!\n        \
         let d = get(es, j)!\n        \
         attack(a, d)\n    \
         }}\n    \
         return None\n}}\n"
    );
    assert!(errors(&src).is_empty(), "got {:?}", errors(&src));
}

/// [qual-depend] A parameter's **declared** dependent claim is live in the
/// body — its roots fill from the sibling parameters at declaration — so
/// the claim-demanding total overload resolves. (Before 2026-09-24 the
/// declared claim stayed a root-free template and only `is`-established
/// claims worked.)
#[test]
fn a_declared_claim_serves_the_total_overload_in_the_body() {
    let src = format!(
        "{PRELUDE}\
         fn read(es: List<Mut Entity>, i: Idx(es) Int) [] -> Int => es, i {{\n    \
         return get(es, i).hp\n}}\n"
    );
    assert!(errors(&src).is_empty(), "got {:?}", errors(&src));
}

/// [elem-distinct] The pair rule covers calls **through fn values** too
/// ([fn-contract]'s keep-in-sync duty): two unproven handles into one
/// container are refused at the second argument.
#[test]
fn an_unproven_pair_through_a_fn_value_is_refused() {
    let src = format!(
        "{PRELUDE}\
         fn touch2(es: List<Mut Entity>, i: Idx(es) Int, j: Idx(es) Int,\n\
                   f: (a: Mut Entity, b: Mut Entity) -> None) [] -> None {{\n    \
         f(get(es, i), get(es, j))\n    \
         return None\n}}\n"
    );
    let errs = errors(&src);
    assert!(
        errs.iter().any(|e| e.contains("prove them apart")),
        "expected the unproven-pair refusal: {errs:?}"
    );
}

/// [elem-distinct] …and the declared `NotEq` proof legalizes it — the
/// `update2` shape, ordinary Salvo end to end [col-update].
#[test]
fn a_declared_noteq_proof_carries_a_fn_value_pair() {
    let src = format!(
        "{PRELUDE}\
         fn touch2(es: List<Mut Entity>, i: Idx(es) Int, j: NotEq(i) Idx(es) Int,\n\
                   f: (a: Mut Entity, b: Mut Entity) -> None) [] -> None {{\n    \
         f(get(es, i), get(es, j))\n    \
         return None\n}}\n"
    );
    assert!(errors(&src).is_empty(), "got {:?}", errors(&src));
}

/// [canbe-entry] ④b — a callee declaring `=> a canbe d` takes two element
/// handles of one container with **no disjointness proof**: the same-call
/// rule stands down exactly for the covered pair.
#[test]
fn a_covered_pair_needs_no_proof() {
    let src = format!(
        "{PRELUDE}\
         fn duel(a: Mut Entity, d: Mut Entity) [] -> None\n\
         => a canbe d, a: Mut, d: Mut {{\n    \
         a.hp = a.hp - 1\n    \
         d.hp = d.hp - 2\n    \
         return None\n}}\n\
         fn f(es: List<Mut Entity>, i: Idx(es) Int, j: Idx(es) Int) [] -> None {{\n    \
         duel(get(es, i), get(es, j))\n    \
         return None\n}}\n"
    );
    assert!(errors(&src).is_empty(), "got {:?}", errors(&src));
}

/// [canbe-entry] …and an *uncovered* callee still refuses the pair: the
/// exemption is exactly coverage-shaped (P-6).
#[test]
fn an_uncovered_callee_still_refuses_the_pair() {
    let src = format!(
        "{PRELUDE}\
         fn duel(a: Mut Entity, d: Mut Entity) [] -> None => a: Mut, d: Mut {{\n    \
         a.hp = a.hp - 1\n    \
         d.hp = d.hp - 2\n    \
         return None\n}}\n\
         fn f(es: List<Mut Entity>, i: Idx(es) Int, j: Idx(es) Int) [] -> None {{\n    \
         duel(get(es, i), get(es, j))\n    \
         return None\n}}\n"
    );
    let errs = errors(&src);
    assert!(
        errs.iter().any(|e| e.contains("prove them apart")),
        "expected the uncovered refusal: {errs:?}"
    );
}
