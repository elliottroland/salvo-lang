//! [elem-distinct] Distinct awareness for mutable element handles: poison
//! consults a live `NotSame` claim between two bound handles before killing
//! the sibling [ref-notsame], the claim dies when either handle is rebound,
//! and one call may take two handles only when they are proven apart —
//! which statement-scoped mints never are.

use std::path::Path;

use salvo_core::{check_program, resolve, Program, SourceSet, Symbols};

const STD_PRELUDE: &str =
    "export intrinsic type Str\nexport intrinsic type Int\nexport intrinsic type Bool\n";

/// A miniature `core.list`: the mint recognition is nominal — `get` and
/// `at` declared by the `core.list` module — so the tests declare exactly the
/// surface the feature reads [elem-distinct].
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
export fn at<T>(list: List<Mut T>, index: Int) [] -> ref(list) Mut T?\n\
=> list, index {\n\
    return get(list, index)\n\
}\n";

/// A miniature `core.ref`, verbatim the declarations of `std/core/ref.sv`:
/// the proof that two handles name different elements [ref-notsame].
const STD_REF: &str = "\
export intrinsic fn same<T>(a: T, b: T) [] -> Bool => a, b\n\
export qualifier NotSame<T>(a: T) of T {\n\
    fn qualifies(b: T, a: T) -> Bool {\n\
        return !same(a, b)\n\
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
        "std/core/ref.sv",
        SourceSet::classify(Path::new("core/ref.sv")).unwrap(),
        STD_REF.to_string(),
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
    // [comptime-instantiate] The `by` sites stamp before resolution.
    let _expansion = salvo_core::expand(&sources.files, &mut modules);
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

/// [elem-distinct] [ref-notsame] Two bound handles a live `NotSame` claim
/// proves apart survive each other's mutations: the sibling names disjoint
/// storage, so the poison spares it — and a write *through* a handle leaves
/// the claim standing, so the second round of writes is accepted too.
#[test]
fn two_proven_distinct_handles_coexist_across_mutation() {
    let src = format!(
        "{PRELUDE}\
         fn f(es: List<Mut Entity>, i: Int, j: Int) [] -> None {{\n    \
         let a = at(es, i)!\n    \
         let d = at(es, j)!\n    \
         if d is NotSame(a) {{\n        \
         a.hp = a.hp + 1\n        \
         d.hp = d.hp + 1\n        \
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
         let a = at(es, i)!\n    \
         let d = at(es, j)!\n    \
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

/// [elem-distinct] [ref-notsame] [qual-depend] The claim is about the
/// handle a name holds *now*: rebinding `d` to another mint strips it, and
/// the conservative poison returns — even though both names still exist.
#[test]
fn rebinding_a_handle_strips_the_claim() {
    let src = format!(
        "{PRELUDE}\
         fn f(es: List<Mut Entity>, i: Int, j: Int) [] -> None => es, i, j {{\n    \
         let a = at(es, i)!\n    \
         let d = at(es, j)!\n    \
         if d is NotSame(a) {{\n        \
         d = at(es, i)!\n        \
         a.hp = a.hp + 1\n        \
         d.hp = d.hp + 1\n    \
         }}\n    \
         return None\n}}\n"
    );
    let errs = errors(&src);
    assert!(
        errs.iter().any(|e| e.contains("mutated after the binding")),
        "expected the rebinding to restore the poison: {errs:?}"
    );
}

/// [elem-distinct] The same index minted twice with no proof is the same
/// element as far as the checker is concerned: the sibling poisons. (Proving
/// such a pair apart is a run-time test that answers false — the
/// compile-and-run cases on both backends cover it.)
#[test]
fn the_same_index_without_a_proof_still_poisons() {
    let src = format!(
        "{PRELUDE}\
         fn f(es: List<Mut Entity>, i: Int) [] -> None => es, i {{\n    \
         let a = at(es, i)!\n    \
         let d = at(es, i)!\n    \
         a.hp = a.hp + 1\n    \
         d.hp = d.hp + 1\n    \
         return None\n}}\n"
    );
    let errs = errors(&src);
    assert!(
        errs.iter().any(|e| e.contains("mutated after the binding")),
        "expected the same-element poison: {errs:?}"
    );
}

/// [elem-distinct] [ref-notsame] Statement-scoped mints cannot carry the
/// proof — a claim is about two *bound* handles — so two of them in one call
/// are refused even inside a `NotSame` block over other handles of the same
/// elements, naming the remedy.
#[test]
fn statement_scoped_mints_cannot_carry_the_proof() {
    let src = format!(
        "{PRELUDE}\
         fn f(es: List<Mut Entity>, i: Int, j: Int) [] -> None => es, i, j {{\n    \
         let a = at(es, i)!\n    \
         let d = at(es, j)!\n    \
         if d is NotSame(a) {{\n        \
         attack(at(es, i)!, at(es, j)!)\n    \
         }}\n    \
         return None\n}}\n"
    );
    let errs = errors(&src);
    assert!(
        errs.iter().any(|e| e.contains("b is NotSame(a)")),
        "expected the unproven-pair refusal naming `NotSame`: {errs:?}"
    );
}

/// [elem-distinct] Without the proof the pair is refused at the call,
/// naming the remedy — the shape used to pass the checker and die at
/// rustc (E0499), a checker/emitter disagreement closed with this rule.
#[test]
fn an_unproven_pair_in_one_call_is_refused() {
    let src = format!(
        "{PRELUDE}\
         fn f(es: List<Mut Entity>, i: Int, j: Int) [] -> None => es, i, j {{\n    \
         attack(at(es, i)!, at(es, j)!)\n    \
         return None\n}}\n"
    );
    let errs = errors(&src);
    assert!(
        errs.iter().any(|e| e.contains("prove them apart")),
        "expected the unproven-pair refusal: {errs:?}"
    );
}

/// [elem-distinct] [ref-notsame] Two *bound* handles proven apart land in
/// one call — the shape the Rust backend renders through `salvo_pair_mut`
/// [rs-elem-mut] — and both survive it: writes through either afterwards
/// are accepted.
#[test]
fn a_proven_bound_pair_may_land_in_one_call() {
    let src = format!(
        "{PRELUDE}\
         fn f(es: List<Mut Entity>, i: Int, j: Int) [] -> None {{\n    \
         let a = at(es, i)!\n    \
         let d = at(es, j)!\n    \
         if d is NotSame(a) {{\n        \
         attack(a, d)\n        \
         a.hp = a.hp + 1\n        \
         d.hp = d.hp + 1\n        \
         attack(d, a)\n    \
         }}\n    \
         return None\n}}\n"
    );
    assert!(errors(&src).is_empty(), "got {:?}", errors(&src));
}

/// [elem-distinct] …and the same bound pair *without* the proof is refused
/// at the call.
#[test]
fn an_unproven_bound_pair_in_one_call_is_refused() {
    let src = format!(
        "{PRELUDE}\
         fn f(es: List<Mut Entity>, i: Int, j: Int) [] -> None => es, i, j {{\n    \
         let a = at(es, i)!\n    \
         let d = at(es, j)!\n    \
         attack(a, d)\n    \
         return None\n}}\n"
    );
    let errs = errors(&src);
    assert!(
        errs.iter().any(|e| e.contains("prove them apart")),
        "expected the unproven-pair refusal: {errs:?}"
    );
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
         f(at(es, i)!, at(es, j)!)\n    \
         return None\n}}\n"
    );
    let errs = errors(&src);
    assert!(
        errs.iter().any(|e| e.contains("prove them apart")),
        "expected the unproven-pair refusal: {errs:?}"
    );
}

/// [elem-distinct] [ref-notsame] …and a `NotSame` proof over two bound
/// handles legalizes it, ordinary Salvo end to end.
#[test]
fn a_notsame_proof_carries_a_fn_value_pair() {
    let src = format!(
        "{PRELUDE}\
         fn touch2(es: List<Mut Entity>, i: Idx(es) Int, j: Idx(es) Int,\n\
                   f: (a: Mut Entity, b: Mut Entity) -> None) [] -> None {{\n    \
         let a = at(es, i)!\n    \
         let d = at(es, j)!\n    \
         if d is NotSame(a) {{\n        \
         f(a, d)\n    \
         }}\n    \
         return None\n}}\n"
    );
    assert!(errors(&src).is_empty(), "got {:?}", errors(&src));
}

/// A callee taking two handles anchored at one container parameter
/// [ref-anchor]: `ref(c)` on both says they may be the same element.
const DUEL: &str = "\
fn duel(c: List<Mut Entity>, a: ref(c) Mut Entity, d: ref(c) Mut Entity) [] -> None {\n    \
a.hp = a.hp - 1\n    \
d.hp = d.hp - 2\n    \
return None\n}\n";

/// [ref-anchor] ④b — a callee whose two handle parameters are both
/// `ref(c)` takes two element handles of one container with **no
/// disjointness proof**: the same-call rule stands down exactly for the
/// pair anchored at one container parameter — including the self-strike.
#[test]
fn a_covered_pair_needs_no_proof() {
    let src = format!(
        "{PRELUDE}{DUEL}\
         fn f(es: List<Mut Entity>, i: Idx(es) Int, j: Idx(es) Int) [] -> None {{\n    \
         duel(es, at(es, i)!, at(es, j)!)\n    \
         duel(es, at(es, i)!, at(es, i)!)\n    \
         return None\n}}\n"
    );
    assert!(errors(&src).is_empty(), "got {:?}", errors(&src));
}

/// [ref-anchor] …and an *uncovered* callee still refuses the pair: the
/// exemption is exactly anchor-shaped (P-6).
#[test]
fn an_uncovered_callee_still_refuses_the_pair() {
    let src = format!(
        "{PRELUDE}\
         fn duel(a: Mut Entity, d: Mut Entity) [] -> None => a: Mut, d: Mut {{\n    \
         a.hp = a.hp - 1\n    \
         d.hp = d.hp - 2\n    \
         return None\n}}\n\
         fn f(es: List<Mut Entity>, i: Idx(es) Int, j: Idx(es) Int) [] -> None {{\n    \
         duel(at(es, i)!, at(es, j)!)\n    \
         return None\n}}\n"
    );
    let errs = errors(&src);
    assert!(
        errs.iter().any(|e| e.contains("prove them apart")),
        "expected the uncovered refusal: {errs:?}"
    );
}

/// [GB-fix-1b] [ref-anchor] A `ref(c)` pair drawn from **two different
/// containers** is refused at the call: each handle must come from the
/// very container passed for its `ref(…)` parameter. (The `canbe` shape
/// this replaces used to compile and silently miscompile on Rust — one
/// list's handle redirected onto the other — a backend-parity break.)
#[test]
fn a_covered_pair_from_two_containers_is_refused() {
    let src = format!(
        "{PRELUDE}{DUEL}\
         fn f(es: List<Mut Entity>, fs: List<Mut Entity>, i: Idx(es) Int, j: Idx(fs) Int) [] -> None {{\n    \
         duel(es, at(es, i)!, at(fs, j)!)\n    \
         return None\n}}\n"
    );
    let errs = errors(&src);
    assert!(
        errs.iter().any(|e| e.contains("[ref-anchor]")),
        "expected the two-container refusal: {errs:?}"
    );
}

/// [GB-fix-1b] [ref-anchor] …and the same pair drawn from **one
/// container** is accepted — the refusal is only for the
/// different-container case, nothing else.
#[test]
fn a_covered_pair_from_one_container_is_accepted() {
    let src = format!(
        "{PRELUDE}{DUEL}\
         fn f(es: List<Mut Entity>, i: Idx(es) Int, j: Idx(es) Int) [] -> None {{\n    \
         duel(es, at(es, i)!, at(es, j)!)\n    \
         return None\n}}\n"
    );
    assert!(errors(&src).is_empty(), "got {:?}", errors(&src));
}

/// [ref-handle] A plain `get` is read-only even through a fn value: a
/// `Mut` position of the fn type refuses it, as a named callee does.
#[test]
fn a_read_only_projection_does_not_fit_a_fn_values_mut_position() {
    let src = format!(
        "{PRELUDE}\
         fn touch(es: List<Mut Entity>, i: Idx(es) Int, f: (a: Mut Entity) -> None) [] -> None {{\n    \
         f(get(es, i))\n    \
         return None\n}}\n"
    );
    let errs = errors(&src);
    assert!(
        errs.iter().any(|e| e.contains("plain projection") && e.contains("[ref-handle]")),
        "expected the read-only refusal: {errs:?}"
    );
}

/// [ref-anchor] A handle parameter's container must be another parameter.
#[test]
fn a_ref_parameter_naming_no_parameter_is_refused() {
    let src = format!(
        "{PRELUDE}\
         fn bad(c: List<Mut Entity>, a: ref(nosuch) Mut Entity) [] -> None {{\n    \
         a.hp = a.hp - 1\n    \
         return None\n}}\n"
    );
    let errs = errors(&src);
    assert!(
        errs.iter().any(|e| e.contains("`ref(nosuch)` names no other parameter")),
        "expected the unknown-container refusal: {errs:?}"
    );
}

/// [proj-opt-slot] A borrowed optional fits a kept owned optional
/// parameter — of a fn and of an effect member — whether it is a read-only
/// `get` or an optional handle (user decision 2026-10-10).
#[test]
fn a_borrowed_optional_fits_a_kept_owned_optional() {
    let src = format!(
        "{PRELUDE}\
         fn hp_of(e: Entity?) [] -> Int => e {{\n    \
         if e is Entity {{\n        return e.hp\n    }}\n    \
         return 0\n}}\n\
         effect Reader {{\n    fn read(e: Entity?) -> Int => e\n}}\n\
         fn f(es: List<Mut Entity>) [Reader] -> Int => es {{\n    \
         let h = at(es, 0)\n    \
         return hp_of(get(es, 0)) + hp_of(at(es, 1)) + hp_of(h) + read(get(es, 2)) + read(h)\n}}\n"
    );
    assert!(errors(&src).is_empty(), "got {:?}", errors(&src));
}

/// [proj-opt-slot] A `Mut` optional position still needs the owned value:
/// the lift is for reading positions only.
#[test]
fn a_borrowed_optional_does_not_fit_a_mut_optional() {
    let src = format!(
        "{PRELUDE}\
         fn heal(e: Mut Entity?) [] -> None => e: Mut {{\n    \
         if e is Entity {{\n        e.hp = e.hp + 1\n    }}\n    \
         return None\n}}\n\
         fn f(es: List<Mut Entity>) [] -> None => es {{\n    \
         heal(get(es, 0))\n    \
         return None\n}}\n"
    );
    let errs = errors(&src);
    assert!(errs.iter().any(|e| e.contains("borrowed value") || e.contains("projection")), "got {errs:?}");
}
