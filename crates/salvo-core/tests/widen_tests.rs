//! The widening check `^` [qual-widen].
//!
//! `^` is the dual of `is`: where a successful `is` narrows the subject (a
//! qualifier added, a union arm picked), a successful `^` **generalizes** it
//! by removing the listed qualifiers. Boolean-valued, same places, same
//! runtime test — only the type in the branch differs. Its reason to exist
//! is the qualified union: `Ok (Ok Int | Err Str)` is a claim *about* a
//! union, and `^ Ok` is what lets a branch `when` the union inside it.

use std::path::Path;

use salvo_core::{check_program, resolve, Program, SourceSet, Symbols};

fn errors(src: &str) -> Vec<String> {
    let mut sources = SourceSet::default();
    let (module, kind) = SourceSet::classify(Path::new("main.sv"), "kotlin").unwrap();
    sources.add("main.sv", module, kind, src.to_string(), false);
    let (ast, diagnostics) = salvo_syntax::parse_module(&sources.files[0].content);
    let parse_errors: Vec<_> = diagnostics.iter().filter(|d| d.is_error()).collect();
    assert!(parse_errors.is_empty(), "parse errors: {parse_errors:?}");
    let program = Program {
        files: sources.files,
        modules: vec![ast],
        companions: Vec::new(),
    };
    let symbols = Symbols::collect(&program);
    let resolution = resolve(&program);
    let checked = check_program(&program, &resolution, &symbols);
    resolution
        .errors
        .iter()
        .chain(checked.errors.iter())
        .map(|d| d.message.clone())
        .collect()
}

const PRELUDE: &str = r#"
intrinsic type Int
intrinsic type Str
intrinsic type Bool

struct Person canbe Mut {
    name: Str
}

qualifier Ok<T> of T
qualifier Err<T> of T
qualifier Surname of Person

external type List<T> canbe Mut

external fn note(text: Str) [] -> [text] None
external fn read(p: Person) [] -> [p] None
external fn touch(p: Mut Person) [] -> [p: Mut] None

fn ok(value: Int) [] -> [] Int as Ok {
    return value
}

fn err(value: Str) [] -> [] Str as Err {
    return value
}
"#;

/// [qual-widen] The motivating case: a `^` branch head tests the arm *and*
/// removes the claim, so the branch can `when` the union inside it — no
/// intermediate binding and no repeated type.
#[test]
fn a_widening_branch_opens_a_nested_union() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         external fn outcome() [] -> [] Ok (Ok Int | Err Str) | Err Str\n\
         fn probe() [] -> [] None {{\n\
         let o = outcome()\n\
         when o {{\n\
         ^ Ok {{\n\
         when o {{\n\
         is Ok {{ note(\"value\") }}\n\
         is Err {{ note(\"inner error\") }}\n\
         }}\n\
         }}\n\
         is Err {{ note(\"outer error\") }}\n\
         }}\n\
         }}\n"
    ));
    assert!(errs.is_empty(), "unexpected errors: {errs:?}");
}

/// [qual-widen] In an `if`, the same check on a plain qualified value: the
/// qualifier is statically present, so it cannot fail — and the subject
/// reads without it inside the branch, which is what picks the *unqualified*
/// overload.
#[test]
fn widening_strips_a_qualifier_in_an_if() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn probe(p: Mut Person) [] -> [p: Mut] None {{\n\
         if p ^ Mut {{\n\
         read(p)\n\
         }}\n\
         }}\n"
    ));
    assert!(errs.is_empty(), "unexpected errors: {errs:?}");
}

/// [qual-widen] And the point of it: after widening, the value no longer
/// satisfies what the qualifier granted.
#[test]
fn a_widened_value_loses_what_the_qualifier_granted() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn probe(p: Mut Person) [] -> [p: Mut] None {{\n\
         if p ^ Mut {{\n\
         touch(p)\n\
         }}\n\
         }}\n"
    ));
    assert!(
        !errs.is_empty(),
        "expected the `Mut`-needing call to be rejected after widening"
    );
}

// ===== the exclusion list =====

/// [qual-widen] The capability qualifiers that may never be dropped, read
/// from the *same* list `Qual T <: T` uses (user decision 2026-09-05), so
/// the two cannot drift.
#[test]
fn intrinsic_capability_qualifiers_cannot_be_widened_away() {
    // `ReadOnly` is in the same list but cannot be *written* in source (it
    // is a presentation-only compiler qualifier), so it is unreachable from
    // a `^` — the list carries it for the day that changes.
    for (qual, needle) in [("Once", "once-callable"), ("Linear", "use obligation")] {
        let errs = errors(&format!(
            "{PRELUDE}\n\
             fn probe(p: Person) [] -> [p] None {{\n\
             if p ^ {qual} {{\n\
             read(p)\n\
             }}\n\
             }}\n"
        ));
        assert!(
            errs.iter()
                .any(|e| e.contains(&format!("`{qual}` cannot be removed with `^`"))
                    && e.contains(needle)),
            "expected `{qual}` to be rejected with its reason, got: {errs:?}"
        );
    }
}

// ===== the error cases =====

/// [qual-widen] Nothing to remove is an error, not a silently-false test
/// (user decision 2026-09-05).
#[test]
fn widening_a_qualifier_the_value_lacks_is_rejected() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn probe(p: Person) [] -> [p] None {{\n\
         if p ^ Surname {{\n\
         read(p)\n\
         }}\n\
         }}\n"
    ));
    assert!(
        errs.iter()
            .any(|e| e.contains("does not carry `Surname`")),
        "expected the nothing-to-remove error, got: {errs:?}"
    );
}

/// [qual-widen] `^` removes qualifiers; a *type* on the right is an `is`
/// question.
#[test]
fn a_type_on_the_right_of_widening_is_rejected() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         external fn outcome() [] -> [] Ok Int | Err Str\n\
         fn probe() [] -> [] None {{\n\
         let o = outcome()\n\
         if o ^ Ok Int {{\n\
         note(\"ok\")\n\
         }}\n\
         }}\n"
    ));
    assert!(
        errs.iter()
            .any(|e| e.contains("qualifier names only")),
        "expected the qualifiers-only error, got: {errs:?}"
    );
}

/// [qual-widen] One widened view cannot stand for two arms: each would peel
/// a different wrapper position.
#[test]
fn widening_more_than_one_arm_is_rejected() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         external fn pair() [] -> [] Ok Int | Ok Str\n\
         fn probe() [] -> [] None {{\n\
         let o = pair()\n\
         if o ^ Ok {{\n\
         note(\"ok\")\n\
         }}\n\
         }}\n"
    ));
    assert!(
        errs.iter()
            .any(|e| e.contains("matches more than one arm")),
        "expected the multi-arm rejection, got: {errs:?}"
    );
}

/// [qual-widen] No binding form: the subject itself reads widened, so a
/// second spelling would be redundant (user decision 2026-09-05).
#[test]
fn a_widening_check_takes_no_binding() {
    let mut sources = SourceSet::default();
    let (module, kind) = SourceSet::classify(Path::new("main.sv"), "kotlin").unwrap();
    sources.add(
        "main.sv",
        module,
        kind,
        "fn f(p: Mut Person) {\n    if p ^ Mut plain {\n        read(plain)\n    }\n}\n"
            .to_string(),
        false,
    );
    let (_ast, diagnostics) = salvo_syntax::parse_module(&sources.files[0].content);
    assert!(
        diagnostics
            .iter()
            .any(|d| d.is_error() && d.message.contains("`^` takes no binding")),
        "expected the no-binding error, got {:?}",
        diagnostics.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
}

/// [qual-widen] A `^` branch consumes the arms it matched, exactly as `is`
/// does, so exhaustiveness needs no new rule.
#[test]
fn a_widening_branch_consumes_its_arms() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         external fn outcome() [] -> [] Ok Int | Err Str\n\
         fn probe() [] -> [] None {{\n\
         let o = outcome()\n\
         when o {{\n\
         ^ Ok {{ note(\"ok\") }}\n\
         }}\n\
         }}\n"
    ));
    assert!(
        errs.iter()
            .any(|e| e.contains("non-exhaustive `when`") && e.contains("Err Str")),
        "expected the unhandled arm to be reported, got: {errs:?}"
    );
}
