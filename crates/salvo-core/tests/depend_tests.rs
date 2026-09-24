//! [qual-depend] Dependent qualifiers — value slots, the filled `is` test,
//! fate-root binding, and conservative cross-value stripping (the
//! refinement-types sequence, step 2).

use std::path::Path;

use salvo_core::{check_program, resolve, Program, SourceSet, Symbols};

const STD_PRELUDE: &str = "export intrinsic type Str\nexport intrinsic type Int\nexport intrinsic type Bool\n";

/// Parses + resolves + checks one file (no std) and returns the checker's
/// and resolver's error messages.
fn errors(src: &str) -> Vec<String> {
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
struct Box canbe Mut { n: Int }\n\
struct Key { s: Str }\n\
qualifier Inside(box: Box) of Key {\n\
    fn qualifies(key: Key, box: Box) -> Bool {\n\
        return box.n > 0\n\
    }\n\
}\n\
fn bump(box: Mut Box) [] -> None => box: Mut {\n\
    box.n = box.n + 1\n\
}\n\
fn read(box: Box) [] -> Int => box {\n\
    return box.n\n\
}\n";

/// The filled test narrows: after `assert!(key is Inside(box))` the claim is
/// in the type, which the widen check observes [qual-lift].
#[test]
fn a_filled_is_test_narrows_to_a_dependent_claim() {
    let src = format!(
        "{PRELUDE}\
         fn f(key: Key, box: Box) [] -> Bool => key, box {{\n    \
         assert!(key is Inside(box))\n    \
         return key is ^Inside\n}}\n"
    );
    assert!(errors(&src).is_empty(), "got {:?}", errors(&src));
}

/// [qual-depend] Any `Mut` use of the depended-on value strips the claim —
/// the widen check below it no longer finds `Inside`.
#[test]
fn mutating_the_linked_value_strips_the_claim() {
    let src = format!(
        "{PRELUDE}\
         fn f(key: Key, box: Mut Box) [] -> Bool => key, box: Mut {{\n    \
         assert!(key is Inside(box))\n    \
         bump(box)\n    \
         return key is ^Inside\n}}\n"
    );
    let errs = errors(&src);
    assert!(
        errs.iter().any(|e| e.contains("Inside")),
        "expected the widen to miss the stripped claim: {errs:?}"
    );
}

/// A *read* of the depended-on value keeps the claim: only mutation can
/// invalidate a fact about contents [deduce-syntax].
#[test]
fn reading_the_linked_value_keeps_the_claim() {
    let src = format!(
        "{PRELUDE}\
         fn f(key: Key, box: Box) [] -> Bool => key, box {{\n    \
         assert!(key is Inside(box))\n    \
         let _n = read(box)\n    \
         return key is ^Inside\n}}\n"
    );
    assert!(errors(&src).is_empty(), "got {:?}", errors(&src));
}

/// Mutating a *different* value strips nothing: the claim is bound to the
/// fate roots of the place that filled the slot.
#[test]
fn mutating_an_unrelated_value_keeps_the_claim() {
    let src = format!(
        "{PRELUDE}\
         fn f(key: Key, box: Box, other: Mut Box) [] -> Bool => key, box, other: Mut {{\n    \
         assert!(key is Inside(box))\n    \
         bump(other)\n    \
         return key is ^Inside\n}}\n"
    );
    assert!(errors(&src).is_empty(), "got {:?}", errors(&src));
}

/// An unfilled test of a dependent qualifier names the filled form.
#[test]
fn an_unfilled_dependent_check_is_an_error() {
    let src = format!(
        "{PRELUDE}\
         fn f(key: Key) [] -> Bool => key {{\n    \
         return key is Inside\n}}\n"
    );
    let errs = errors(&src);
    assert!(
        errs.iter().any(|e| e.contains("filled form")),
        "expected the filled-form error: {errs:?}"
    );
}

/// The place must fit the slot's type.
#[test]
fn a_slot_mismatch_is_reported() {
    let src = format!(
        "{PRELUDE}\
         fn f(key: Key, wrong: Key) [] -> Bool => key, wrong {{\n    \
         return key is Inside(wrong)\n}}\n"
    );
    let errs = errors(&src);
    assert!(
        errs.iter().any(|e| e.contains("fills the `box` slot")),
        "expected the slot-type error: {errs:?}"
    );
}

/// [qual-depend] A dependent predicate's `qualifies` takes the subject and
/// then one parameter per value slot; fewer is an error naming the count.
#[test]
fn a_dependent_qualifies_must_take_its_slots() {
    let src = "\
        struct Box canbe Mut { n: Int }\n\
        struct Key { s: Str }\n\
        qualifier Inside(box: Box) of Key {\n\
            fn qualifies(key: Key) -> Bool {\n\
                return true\n\
            }\n\
        }\n";
    let errs = errors(src);
    assert!(
        errs.iter().any(|e| e.contains("one \
                         parameter per value slot")
            || e.contains("one parameter per value slot")),
        "expected the arity error: {errs:?}"
    );
}

const OVERLOADS: &str = "\
struct Box canbe Mut { n: Int }\n\
struct Key { s: Str }\n\
qualifier Inside(box: Box) of Key {\n\
    fn qualifies(key: Key, box: Box) -> Bool {\n\
        return box.n > 0\n\
    }\n\
}\n\
fn open(box: Box, key: Inside(box) Key) [] -> Int => box, key {\n\
    return box.n\n\
}\n\
fn open(box: Box, key: Key) [] -> Int? => box, key {\n\
    return None\n\
}\n";

/// [qual-depend] A parameter's dependent claim names a sibling parameter:
/// the argument's claim must be about *this call's* value, so a claim about
/// a different box falls back to the optional overload — visible in the
/// result type.
#[test]
fn a_claim_about_another_value_falls_to_the_plain_overload() {
    let src = format!(
        "{OVERLOADS}\
         fn f(key: Key, right: Box, wrong: Box) [] -> Int => key, right, wrong {{\n    \
         assert!(key is Inside(wrong))\n    \
         return open(right, key)\n}}\n"
    );
    let errs = errors(&src);
    // The optional overload answers `Int?`, which does not fit `-> Int`:
    // the total overload was correctly refused.
    assert!(
        errs.iter().any(|e| e.contains("Int?") || e.contains("expected")),
        "expected the optional result to surface: {errs:?}"
    );
}

/// The matching claim picks the total overload, and the call answers the
/// element type.
#[test]
fn a_matching_claim_picks_the_total_overload() {
    let src = format!(
        "{OVERLOADS}\
         fn f(key: Key, box: Box) [] -> Int => key, box {{\n    \
         assert!(key is Inside(box))\n    \
         return open(box, key)\n}}\n"
    );
    assert!(errors(&src).is_empty(), "got {:?}", errors(&src));
}

/// An alias of the depended-on value is the same value: the claim binds
/// fate roots, not names.
#[test]
fn an_alias_of_the_value_still_matches() {
    let src = format!(
        "{OVERLOADS}\
         fn f(key: Key, box: Box) [] -> Int => key, box {{\n    \
         let same = box\n    \
         assert!(key is Inside(same))\n    \
         return open(box, key)\n}}\n"
    );
    assert!(errors(&src).is_empty(), "got {:?}", errors(&src));
}

const PRESERVE: &str = "\
struct Box canbe Mut { n: Int }\n\
struct Key { s: Str }\n\
qualifier Inside(box: Box) of Key {\n\
    fn qualifies(key: Key, box: Box) -> Bool {\n\
        return box.n > 0\n\
    }\n\
    refn bump(box: Mut Box) => box: preserve Inside\n\
}\n\
fn bump(box: Mut Box) [] -> None => box: Mut {\n\
    box.n = box.n + 1\n\
}\n\
fn shrink(box: Mut Box) [] -> None => box: Mut {\n\
    box.n = 0\n\
}\n";

/// [qual-preserve] A refined call keeps the dependent claim alive: the
/// widen check still finds it below the mutation.
#[test]
fn a_preserving_call_keeps_the_claim() {
    let src = format!(
        "{PRESERVE}\
         fn f(key: Key, box: Mut Box) [] -> Bool => key, box: Mut {{\n    \
         assert!(key is Inside(box))\n    \
         bump(box)\n    \
         return key is ^Inside\n}}\n"
    );
    assert!(errors(&src).is_empty(), "got {:?}", errors(&src));
}

/// An unrefined mutator still strips: preservation is per call, opt-in.
#[test]
fn an_unrefined_mutator_still_strips() {
    let src = format!(
        "{PRESERVE}\
         fn f(key: Key, box: Mut Box) [] -> Bool => key, box: Mut {{\n    \
         assert!(key is Inside(box))\n    \
         shrink(box)\n    \
         return key is ^Inside\n}}\n"
    );
    let errs = errors(&src);
    assert!(
        errs.iter().any(|e| e.contains("Inside")),
        "expected the widen to miss the stripped claim: {errs:?}"
    );
}

/// [qual-preserve] A fn's own `preserve` entry is checked: a body that
/// hands the parameter to a non-preserving mutator is refused at that call.
#[test]
fn an_own_preserve_promise_is_checked_against_the_body() {
    let src = format!(
        "{PRESERVE}\
         fn wrap(box: Mut Box) [] -> None\n\
         => box: Mut, box: preserve Inside {{\n    \
         shrink(box)\n    \
         return None\n}}\n"
    );
    let errs = errors(&src);
    assert!(
        errs.iter().any(|e| e.contains("promises to preserve")),
        "expected the own-promise check: {errs:?}"
    );
}

/// A body whose mutating calls all preserve satisfies the promise, and a
/// caller of the wrapper keeps its claim through it.
#[test]
fn an_own_preserve_promise_carries_to_callers() {
    let src = format!(
        "{PRESERVE}\
         fn wrap(box: Mut Box) [] -> None\n\
         => box: Mut, box: preserve Inside {{\n    \
         bump(box)\n    \
         return None\n}}\n\
         fn f(key: Key, box: Mut Box) [] -> Bool => key, box: Mut {{\n    \
         assert!(key is Inside(box))\n    \
         wrap(box)\n    \
         return key is ^Inside\n}}\n"
    );
    assert!(errors(&src).is_empty(), "got {:?}", errors(&src));
}

/// [qual-const] Constant slots: `Percent(0, 100) Int` in a parameter demands
/// a claim with exactly those constants — carried, matched, or the plain
/// overload takes the call.
#[test]
fn constant_slots_match_exactly() {
    let src = "\
        qualifier Percent(lo: Int, hi: Int) of Int {\n\
            fn qualifies(n: Int, lo: Int, hi: Int) -> Bool {\n\
                return n >= lo && n <= hi\n\
            }\n\
        }\n\
        fn shade(n: Percent(0, 100) Int) [] -> Int => n {\n\
            return n + 0\n\
        }\n\
        fn f(n: Int) [] -> Int => n {\n\
            assert!(n is Percent(0, 100))\n\
            return shade(n)\n\
        }\n";
    assert!(errors(src).is_empty(), "got {:?}", errors(src));
}

/// A claim with different constants is a different fact: the call is
/// refused rather than silently accepted.
#[test]
fn different_constants_refuse() {
    let src = "\
        qualifier Percent(lo: Int, hi: Int) of Int {\n\
            fn qualifies(n: Int, lo: Int, hi: Int) -> Bool {\n\
                return n >= lo && n <= hi\n\
            }\n\
        }\n\
        fn shade(n: Percent(0, 100) Int) [] -> Int => n {\n\
            return n + 0\n\
        }\n\
        fn f(n: Int) [] -> Int => n {\n\
            assert!(n is Percent(0, 255))\n\
            return shade(n)\n\
        }\n";
    let errs = errors(src);
    assert!(
        !errs.is_empty(),
        "a Percent(0, 255) must not fill a Percent(0, 100) position"
    );
}

/// [pick-qualifies] The qualifier pick's runtime form: a predicate
/// qualifier on a non-union subject calls `qualifies`, and the picked
/// value carries the claim — the widen check sees it below a diverging
/// right side.
#[test]
fn a_predicate_pick_applies_the_claim() {
    let src = format!(
        "{PRESERVE}\
         fn f(key: Key, box: Box) [] -> Bool => key, box {{\n    \
         let inside = key Inside(box)?: return false\n    \
         return inside is ^Inside\n}}\n"
    );
    assert!(errors(&src).is_empty(), "got {:?}", errors(&src));
}

/// A `^` on a predicate pick is refused: there is no tag to lift — the
/// pick establishes.
#[test]
fn a_predicate_pick_refuses_the_lift() {
    let src = format!(
        "{PRESERVE}\
         fn f(key: Key, box: Box) [] -> Bool => key, box {{\n    \
         let inside = key ^Inside(box)?: return false\n    \
         return true\n}}\n"
    );
    let errs = errors(&src);
    assert!(
        errs.iter().any(|e| e.contains("drop the `^`")),
        "expected the lift refusal: {errs:?}"
    );
}

/// A constructive qualifier has no runtime test, so a pick cannot decide.
#[test]
fn a_constructive_pick_is_refused() {
    let src = "\
        struct Key { s: Str }\n\
        qualifier Blessed of Key\n\
        fn bless(key: Key) -> +Blessed Key { return key }\n\
        fn f(key: Key) [] -> Bool => key {\n    \
        let b = key Blessed?: return false\n    \
        return true\n}\n";
    let errs = errors(src);
    assert!(
        errs.iter().any(|e| e.contains("constructive")),
        "expected the constructive refusal: {errs:?}"
    );
}

/// [elvis-guard] A leaving right side takes its flow effects with it: a
/// `return elem` there must not consume `elem` on the fall-through path —
/// found on `std.heap`'s sift (2026-09-24).
#[test]
fn a_leaving_right_side_keeps_its_consumptions_to_itself() {
    let src = format!(
        "{PRESERVE}\
         fn f(key: Key, box: Box, prize: Key) [] -> Key => key, box, !prize {{\n    \
         let inside = key Inside(box)?: return prize\n    \
         return prize\n}}\n"
    );
    assert!(errors(&src).is_empty(), "got {:?}", errors(&src));
}

/// [qual-depend] **Reassignment strips dependent claims exactly as
/// mutation does**: the old value is gone, so a claim bound to it
/// describes nothing. Before the 2026-09-24 fix an `Idx(xs)` claim held
/// across `xs = [9]` kept resolving the total `get` — a checked
/// out-of-bounds read at runtime.
#[test]
fn reassigning_the_linked_value_strips_the_claim() {
    let src = format!(
        "{PRELUDE}\
         fn f(key: Key) [] -> Bool => key {{\n    \
         let box = Mut Box {{ n: 1 }}\n    \
         assert!(key is Inside(box))\n    \
         box = Mut Box {{ n: 2 }}\n    \
         return key is ^Inside\n}}\n"
    );
    let errs = errors(&src);
    assert!(
        errs.iter().any(|e| e.contains("Inside")),
        "expected the widen to miss the stripped claim: {errs:?}"
    );
}

/// [qual-depend] `i++` rebinds the variable, so dependent claims bound to
/// its old value strip too — the step is a reassignment in every flow
/// sense [inc-dec].
#[test]
fn stepping_a_linked_int_strips_the_claim() {
    let src = "\
        qualifier Above(floor: Int) of Int {\n\
            fn qualifies(n: Int, floor: Int) -> Bool {\n\
                return n > floor\n\
            }\n\
        }\n\
        fn f(n: Int, base: Int) [] -> Bool => n, base {\n    \
        let floor = base + 0\n    \
        assert!(n is Above(floor))\n    \
        floor++\n    \
        return n is ^Above\n}\n";
    let errs = errors(src);
    assert!(
        errs.iter().any(|e| e.contains("Above")),
        "expected the widen to miss the stripped claim: {errs:?}"
    );
}
