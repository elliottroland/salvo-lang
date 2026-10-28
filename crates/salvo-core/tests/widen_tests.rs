//! The widening check `^` [qual-lift].
//!
//! `^` is the dual of `is`: where a successful `is` narrows the subject (a
//! qualifier added, a union arm picked), a successful `^` **generalizes** it
//! by removing the listed qualifiers. Boolean-valued, same places, same
//! runtime test — only the type in the branch differs. Its reason to exist
//! is the qualified union: `Ok (Ok Int | Err Str)` is a claim *about* a
//! union, and `is ^Ok` is what lets a branch `when` the union inside it.
//!
//! The diagnostics are test data, `tests/check/widening/` (TESTS.md T1).
//! What stays here could not move to the real std without a different
//! result; each says why.

use std::path::Path;

use salvo_core::{check_program, resolve, Program, SourceSet, Symbols};

/// [platform-intrinsic-std-only] The intrinsic declarations these sources rely on.
/// `intrinsic` is the compiler's modifier and only std may write it, so this
/// is loaded as a *std* file rather than pasted into the source under test.
/// Module `std.core.prelude`: `std.core.*` is implicitly imported, so the test source
/// sees these names without an `import`.
const STD_PRELUDE: &str = concat!(
    "export intrinsic type Int\nexport intrinsic type Str\nexport intrinsic type Bool\nexport intrinsic type List<T> canbe Mut\nexport intrinsic fn first<T>(list: List<T>) [] -> proj(list) T? => list\n",
    // [op-equality] [op-order] What the comparison operators call.
    "export intrinsic fn eq(a: Int, b: Int) [] -> Bool => a, b\n",
    "export intrinsic fn eq(a: Bool, b: Bool) [] -> Bool => a, b\n",
    "export intrinsic fn eq(a: Str, b: Str) [] -> Bool => a, b\n",
    "export intrinsic fn cmp(a: Int, b: Int) [] -> Int => a, b\n",
    // [op-arith] [op-pow] What the arithmetic operators call.
    "export intrinsic fn plus(a: Int, b: Int) [] -> Int => a, b\n",
    "export intrinsic fn minus(a: Int, b: Int) [] -> Int => a, b\n",
    "export intrinsic fn times(a: Int, b: Int) [] -> Int => a, b\n",
    "export intrinsic fn divide(a: Int, b: Int) [] -> Int => a, b\n",
);

fn errors(src: &str) -> Vec<String> {
    let mut sources = SourceSet::default();
    sources.add(
        "std/core/prelude.sv",
        SourceSet::classify(Path::new("std/core/prelude.sv")).unwrap(),
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
    // [comptime-instantiate] The `by` sites stamp before resolution.
    let _expansion = salvo_core::expand(&sources.files, &mut modules);
    let program = Program {
        files: sources.files,
        modules: Program::share(modules),
        companions: Vec::new(),
    };
    let symbols = Symbols::collect(&program);
    let resolution = resolve(&program);
    let checked = check_program(&program, &resolution, &symbols);
    let mut all: Vec<_> = resolution.errors.iter().chain(checked.errors.iter()).filter(|d| d.is_error()).cloned().collect();
    // [fate-poison] As the gates do, the IR analyses run on every
    // declaration the checker's errors leave clean [ir-analyze]: a poisoned
    // read is the IR flow pass's since R8 piece
    // 3c [flow-use-after-move].
    all.extend(salvo_core::ir::analyze::analyze(&program, &symbols, &resolution, &checked).into_iter().filter(|d| d.is_error()));
    all.into_iter().map(|d| d.message).collect()
}

const PRELUDE: &str = r#"

struct Person canbe Mut {
    name: Str
}

qualifier Ok<T> of T
qualifier Err<T> of T
qualifier Surname of Person

fn note(text: Str) [] -> None => text {}
fn read(p: Person) [] -> None => p {}
fn touch(p: Mut Person) [] -> None => p: Mut {}

fn ok(value: Int) [] -> +Ok Int {
    return value
}

fn err(value: Str) [] -> +Err Str => !value {
    return value
}
"#;

// Kept in Rust, on this prelude: over the real std, whose `Ok` is a
// provenance qualifier, `return ok(ok(1))` into `Ok (Ok Int | Err Str) | Err
// Str` checks clean and the Rust backend emits ill-typed code
// (`Union2::U1(ok(ok(1i32)))`, rustc E0308): a compiler bug, to be fixed
// before this case can move.
/// [qual-group] The limit of the flat representation, and the reason the
/// annotated-`let` leftover survives: the *same* qualifier twice deduplicates
/// to one, so `ok(ok(x))` is indistinguishable from `ok(x)` and no rule can
/// recover the nesting. The diagnostic says so, since the type it prints
/// looks like it should fit.
#[test]
fn the_same_qualifier_twice_deduplicates_and_says_so() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn step() [] -> Ok (Ok Int | Err Str) | Err Str {{\n\
         return ok(ok(1))\n\
         }}\n"
    ));
    assert!(
        errs.iter()
            .any(|e| e.contains("deduplicates") && e.contains("annotated local")),
        "expected the deduplication hint, got: {errs:?}"
    );
}

// Kept in Rust, parse-only: checked as a whole program, the mixed check is
// reported twice — the parse's `qual-lift:partial` and then the checker's
// `qual-lift:not-qualifiers` for the same `Int` — so the data file would pin a
// second diagnostic for one mistake.
/// [qual-lift] A check **mixes** lifted and kept terms — `is ^Ok Int` marks
/// the qualifier but not the base type — and that is refused at the parse, not
/// the check: some terms marked and some not is a syntactic property, and the
/// parser cannot tell a qualifier from a type name (both are uppercase). The
/// old spelling `^ Ok Int` was refused too, by the checker's qualifiers-only
/// rule, so nothing expressible has been lost. Lifting a qualifier *while*
/// naming the arm's base type stays unexpressible for now.
#[test]
fn a_check_mixing_lifted_and_kept_terms_is_rejected() {
    let src = "fn probe(o: Ok Int | Err Str) {\n    if o is ^Ok Int {\n        note(o)\n    }\n}\n";
    let (_ast, diagnostics) = salvo_syntax::parse_module(src);
    assert!(
        diagnostics
            .iter()
            .any(|d| d.is_error() && d.message.contains("lifts every qualifier or none")),
        "expected the mixed-check error, got {:?}",
        diagnostics.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
}
