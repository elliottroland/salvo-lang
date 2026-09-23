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
