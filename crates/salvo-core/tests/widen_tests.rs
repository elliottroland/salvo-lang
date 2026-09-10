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

/// [intrinsic-std-only] The intrinsic declarations these sources rely on.
/// `intrinsic` is the compiler's modifier and only std may write it, so this
/// is loaded as a *std* file rather than pasted into the source under test.
/// Module `core.prelude`: `core.*` is implicitly imported, so the test source
/// sees these names without an `import`.
const STD_PRELUDE: &str = "intrinsic type Int\nintrinsic type Str\nintrinsic type Bool\n";

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
        .map(|d| d.message.clone())
        .collect()
}

const PRELUDE: &str = r#"

struct Person canbe Mut {
    name: Str
}

qualifier Ok<T> of T
qualifier Err<T> of T
qualifier Surname of Person

fn note(text: Str) [] -> [text] None {}
fn read(p: Person) [] -> [p] None {}
fn touch(p: Mut Person) [] -> [p: Mut] None {}

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
         fn outcome() [] -> [] Ok (Ok Int | Err Str) | Err Str {{ return err(\"e\") }}\n\
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
         fn outcome() [] -> [] Ok Int | Err Str {{ return err(\"e\") }}\n\
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
         fn pair() [] -> [] Ok Int | Ok Str {{ return ok(1) }}\n\
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
    sources.add(
        "main.sv",
        SourceSet::classify(Path::new("main.sv")).unwrap(),
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
         fn outcome() [] -> [] Ok Int | Err Str {{ return err(\"e\") }}\n\
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

// ===== constructing a group [qual-group] =====
//
// The other half of the qualified union: `^` *reads* a group, and these build
// one. `Ty::Qualified` keeps a flat, sorted, deduplicated qualifier list whose
// base is never itself `Qualified`, so applying a qualifier to an
// already-qualified value appends rather than nests — `emitted(ok(1))` is
// `Qualified { quals: [Emitted, Ok], base: Int }`, shape-identical to "two
// qualifiers on an `Int`". Where the expected arm is `Q (A | B)`, the value is
// read the other way round: the group's qualifiers come off the list and the
// remainder tries the union's arms. Fixed 2026-09-10; before that the
// construction needed an annotated intermediate `let`.

/// The declarations that make a nested group reachable: a second qualifier
/// (`Ok` alone cannot nest, see the deduplication test below) and its
/// constructor, in this file as [qual-ctor-same-file] requires.
const GROUP_PRELUDE: &str = r#"
qualifier Emitted<T> of T

fn emitted<T>(value: T) [] -> [] T as Emitted {
    return value
}
"#;

/// [qual-group] The defect this closed: a qualifier applied to an
/// already-qualified value flattens, and the arm it must land in is a
/// qualified union group.
#[test]
fn a_qualifier_applied_to_a_qualified_value_reaches_a_group_arm() {
    let errs = errors(&format!(
        "{PRELUDE}{GROUP_PRELUDE}\n\
         fn step(flag: Bool) [] -> [] Emitted (Ok Int | Err Str) | Err Str {{\n\
         if flag {{ return emitted(ok(1)) }}\n\
         return err(\"done\")\n\
         }}\n"
    ));
    assert!(errs.is_empty(), "unexpected errors: {errs:?}");
}

/// [qual-group] Either inner arm, so the arm index really is derived from the
/// remainder rather than assumed to be the first.
#[test]
fn the_remainder_picks_the_inner_arm() {
    let errs = errors(&format!(
        "{PRELUDE}{GROUP_PRELUDE}\n\
         fn step(flag: Bool) [] -> [] Emitted (Ok Int | Err Str) | Ok Bool {{\n\
         if flag {{ return emitted(err(\"bad\")) }}\n\
         return emitted(ok(1))\n\
         }}\n"
    ));
    assert!(errs.is_empty(), "unexpected errors: {errs:?}");
}

/// [qual-group] The remainder still has to fit: the group's qualifier being
/// present is not on its own a licence to enter the arm. (The outer union's
/// other arm is deliberately unrelated — `Emitted` is droppable, so an
/// `Err Str` arm would legitimately accept `Emitted Err Str` and this would
/// test nothing.)
#[test]
fn a_remainder_that_fits_no_inner_arm_is_rejected() {
    let errs = errors(&format!(
        "{PRELUDE}{GROUP_PRELUDE}\n\
         fn step() [] -> [] Emitted (Ok Int | Ok Str) | Ok Bool {{\n\
         return emitted(err(\"bad\"))\n\
         }}\n"
    ));
    assert!(
        errs.iter()
            .any(|e| e.contains("no arm of") && e.contains("Emitted Err Str")),
        "expected the no-arm error, got: {errs:?}"
    );
}

/// [qual-group] The limit of the flat representation, and the reason the
/// annotated-`let` leftover survives: the *same* qualifier twice deduplicates
/// to one, so `ok(ok(x))` is indistinguishable from `ok(x)` and no rule can
/// recover the nesting. The diagnostic says so, since the type it prints
/// looks like it should fit.
#[test]
fn the_same_qualifier_twice_deduplicates_and_says_so() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn step() [] -> [] Ok (Ok Int | Err Str) | Err Str {{\n\
         return ok(ok(1))\n\
         }}\n"
    ));
    assert!(
        errs.iter()
            .any(|e| e.contains("deduplicates") && e.contains("annotated local")),
        "expected the deduplication hint, got: {errs:?}"
    );
}
