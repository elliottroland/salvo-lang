//! [fn-overload-specific] Overload specificity — decision O1 (user,
//! 2026-09-06): when several candidates match a call, a **concrete**
//! parameter type beats a type variable. Before this, two candidates that
//! both matched were separated only by declaration order, so
//! `describe<T>(T)` declared first shadowed `describe(Int)` for
//! `describe(3)` — a bug that went unnoticed because every overload set in
//! existence had disjoint parameter types.
//!
//! The relation is a *partial* order: it ranks the genericity axis and
//! nothing else, so a set of best candidates that it cannot rank is an
//! ambiguity error, while a pair it has nothing to say about (differing
//! only in qualifiers, unions or variadics — axes still undesigned) keeps
//! declaration order.

use std::path::Path;

use salvo_core::{check_program, resolve, FileDiagnostic, Program, SourceSet, Symbols};

/// [intrinsic-std-only] The intrinsic declarations these sources rely on,
/// loaded as a *std* file (only std may write `intrinsic`).
const STD_PRELUDE: &str = "intrinsic type Int\nintrinsic type Str\nintrinsic type Bool\nintrinsic type List<T> canbe Mut\nintrinsic fn of_list<T>(...elems: T[]) [] -> [] Mut List<T>\n";

fn checked(src: &str) -> salvo_core::Checked {
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
    check_program(&program, &resolution, &symbols)
}

fn errors(src: &str) -> Vec<FileDiagnostic> {
    checked(src).errors
}

fn messages(src: &str) -> Vec<String> {
    errors(src).iter().map(|d| d.message.clone()).collect()
}

/// The return type of the *selected* overload is the observable proof of
/// which one won: each candidate returns a distinct type, and the call's
/// result is annotated with the expected one.
fn picks(decls: &str, call: &str, expect_ty: &str) -> Vec<String> {
    let src = format!("{decls}\nfn probe() -> [] None {{\n    let picked: {expect_ty} = {call}\n}}\n");
    messages(&src)
}

// ===== the concrete/generic pair, in both declaration orders =====

/// [fn-overload-specific] The generic overload declared *first* no longer
/// shadows the concrete one.
#[test]
fn a_concrete_parameter_beats_a_type_variable() {
    let decls = "fn describe<T>(value: T) [] -> [] Bool { return true }\n\
                 fn describe(value: Int) [] -> [] Str { return \"concrete\" }\n";
    // The concrete overload wins, so the result is `Str`.
    assert!(picks(decls, "describe(3)", "Str").is_empty());
    // ... and it is genuinely selected rather than both being accepted.
    let msgs = picks(decls, "describe(3)", "Bool");
    assert!(!msgs.is_empty(), "the generic overload must not be selected");
}

/// The same, with the concrete overload declared first: the answer must not
/// depend on declaration order at all.
#[test]
fn declaration_order_does_not_decide() {
    let decls = "fn describe(value: Int) [] -> [] Str { return \"concrete\" }\n\
                 fn describe<T>(value: T) [] -> [] Bool { return true }\n";
    assert!(picks(decls, "describe(3)", "Str").is_empty());
}

/// A call the concrete overload does not accept still reaches the generic
/// one — specificity ranks the *viable* candidates, it does not discard.
#[test]
fn the_generic_overload_still_takes_what_the_concrete_one_cannot() {
    let decls = "fn describe<T>(value: T) [] -> [] Bool { return true }\n\
                 fn describe(value: Int) [] -> [] Str { return \"concrete\" }\n";
    assert!(picks(decls, "describe(\"text\")", "Bool").is_empty());
}

/// [fn-overload-specific] Specificity is structural: `List<Int>` is more
/// specific than `List<T>`, which is exactly the shape of S-Seq's `List`
/// fast path (a generic body over `iter` plus a `List` overload).
#[test]
fn specificity_is_structural() {
    let decls = "fn each<T>(xs: List<T>) [] -> [xs] Bool { return true }\n\
                 fn each(xs: List<Int>) [] -> [xs] Str { return \"ints\" }\n";
    let src = format!(
        "{decls}\nfn probe() -> [] None {{\n    let xs: Mut List<Int> = of_list(1, 2)\n    \
         let picked: Str = each(xs)\n}}\n"
    );
    assert!(messages(&src).is_empty(), "{:?}", messages(&src));
}

/// A partially generic candidate beats a wholly generic one, and loses to a
/// wholly concrete one: dominance is per-position with one strict win.
#[test]
fn partial_specificity_ranks_between() {
    let decls = "fn pair<T, U>(a: T, b: U) [] -> [] Bool { return true }\n\
                 fn pair<U>(a: Int, b: U) [] -> [] Str { return \"half\" }\n";
    assert!(picks(decls, "pair(1, \"x\")", "Str").is_empty());
    let decls = format!("{decls}fn pair(a: Int, b: Str) [] -> [] List<Int> {{ return of_list(1) }}\n");
    let src = format!(
        "{decls}\nfn probe() -> [] None {{\n    let picked: List<Int> = pair(1, \"x\")\n}}\n"
    );
    assert!(messages(&src).is_empty(), "{:?}", messages(&src));
}

// ===== the error path =====

/// [fn-overload-specific] Two candidates that disagree about *which*
/// parameter is concrete are unrankable, and an unrankable best set is an
/// error naming both — not a silent pick.
#[test]
fn a_specificity_tie_is_an_error() {
    let decls = "fn mix<T>(a: T, b: Int) [] -> [] Bool { return true }\n\
                 fn mix<T>(a: Int, b: T) [] -> [] Bool { return true }\n";
    let src = format!("{decls}\nfn probe() -> [] None {{\n    let x = mix(1, 2)\n}}\n");
    let msgs = messages(&src);
    assert_eq!(msgs.len(), 1, "expected exactly one diagnostic: {msgs:?}");
    assert!(
        msgs[0].contains("ambiguous call to `mix(Int, Int)`")
            && msgs[0].contains("`mix(T, Int)`")
            && msgs[0].contains("`mix(Int, T)`"),
        "the diagnostic should name the call and both candidates: {}",
        msgs[0]
    );
}

/// An explicit type-argument list is one of the two remedies the diagnostic
/// names, and it does not help here — the ambiguity is about *parameters*,
/// not bindings — so the other one must: annotating the argument so only one
/// candidate is viable.
#[test]
fn narrowing_an_argument_resolves_the_tie() {
    let decls = "fn mix<T>(a: T, b: Int) [] -> [] Bool { return true }\n\
                 fn mix<T>(a: Int, b: T) [] -> [] Bool { return true }\n";
    let src = format!(
        "{decls}\nfn probe() -> [] None {{\n    let s: Str = \"x\"\n    \
         let x = mix(s, 2)\n}}\n"
    );
    assert!(messages(&src).is_empty(), "{:?}", messages(&src));
}

/// [fn-overload-specific] O1 ranks the genericity axis and no other, so a
/// pair that differs only in *qualifiers* is untouched by it: the existing
/// score already prefers the qualified parameter, and no ambiguity is
/// reported.
#[test]
fn qualifier_specificity_is_left_to_the_score() {
    let decls = "qualifier Even of Int {\n    fn qualifies(n: Int) -> Bool { return true }\n}\n\
                 fn label(n: Int) [] -> [] Bool { return true }\n\
                 fn label(n: Even Int) [] -> [] Str { return \"even\" }\n";
    let src = format!(
        "{decls}\nfn probe() -> [] None {{\n    let n = 4\n    if n is Even {{\n        \
         let picked: Str = label(n)\n    }}\n}}\n"
    );
    assert!(messages(&src).is_empty(), "{:?}", messages(&src));
}

/// [type-unknown-lenient] An argument the checker could not infer fits
/// *every* candidate, so the ranking must stay silent: one mistake, one
/// diagnostic. (Before this guard, `size(xs)` on a poisoned local reported
/// the fate error *and* an ambiguity between std's three `size` overloads.)
#[test]
fn an_un_inferred_argument_produces_no_ambiguity() {
    let decls = "fn mix<T>(a: T, b: Int) [] -> [] Bool { return true }\n\
                 fn mix<T>(a: Int, b: T) [] -> [] Bool { return true }\n";
    let src = format!("{decls}\nfn probe() -> [] None {{\n    let x = mix(nope(), 2)\n}}\n");
    let msgs = messages(&src);
    assert_eq!(msgs.len(), 1, "expected only the unresolved-call error: {msgs:?}");
    assert!(msgs[0].contains("no function named `nope`"), "{}", msgs[0]);
}

/// A single candidate is unaffected by any of this — the ranking only runs
/// when the score leaves several standing.
#[test]
fn one_candidate_needs_no_ranking() {
    let decls = "fn only<T>(value: T) [] -> [] Str { return \"generic\" }\n";
    assert!(picks(decls, "only(1)", "Str").is_empty());
}
