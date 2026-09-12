//! Qualifier refinements [qual-refn]: a qualifier states what functions it
//! does not own do to *its* claim, which is what recovers the precision
//! [deduce-syntax] gives up when it forbids a mutating function from
//! promising a qualifier it has never heard of.
//!
//! The observable in most of these tests is *overload resolution*: a
//! `needs_nonempty` overload matches only while the checker still believes
//! the value is `NonEmpty`, so "clean" means the refinement applied and
//! "no matching overload" means it did not.

use std::path::Path;

use salvo_core::{check_program, resolve, Program, SourceSet, Symbols};

/// [intrinsic-std-only] The intrinsic declarations these sources rely on.
const STD_PRELUDE: &str = "intrinsic type Str\nintrinsic type Int\nintrinsic type Bool\n";

/// Parses + resolves + checks the given files (the first is `main.sv`) and
/// returns every diagnostic message, warnings included — a suppressed
/// refinement conflict is a *warning* [qual-refn-conflict], so the tests
/// have to see both severities.
fn diagnostics(files: &[(&str, &str)]) -> Vec<String> {
    let mut sources = SourceSet::default();
    sources.add(
        "std/core/prelude.sv",
        SourceSet::classify(Path::new("core/prelude.sv")).unwrap(),
        STD_PRELUDE.to_string(),
        true,
    );
    for (name, content) in files {
        sources.add(
            *name,
            SourceSet::classify(Path::new(name)).unwrap(),
            content.to_string(),
            false,
        );
    }
    let mut modules = Vec::with_capacity(sources.files.len());
    for file in &sources.files {
        let (ast, parse) = salvo_syntax::parse_module(&file.content);
        let parse_errors: Vec<_> = parse.iter().filter(|d| d.is_error()).collect();
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

fn errors(src: &str) -> Vec<String> {
    diagnostics(&[("main.sv", src)])
}

/// A list-like type with a mutator whose exhaustive deduction list
/// necessarily drops every claim it does not name [deduce-syntax] — the
/// situation refinements exist for.
const PRELUDE: &str = r#"
struct Store<T> canbe Mut {
    value: T
}

fn push<T>(s: Mut Store<T>, value: T) [] -> None => s: Mut, !value {
    s.value = value
}

fn needs_nonempty<T>(s: NonEmpty Store<T>) -> Int => s {
    return 1
}

"#;

/// `NonEmpty` with the refinement that makes `push` establish it.
const NONEMPTY: &str = r#"
qualifier NonEmpty<T> of Store<T> {
    fn qualifies(s: Store<T>) -> Bool {
        return true
    }

    // Pushing a value leaves the store non-empty.
    refn push(s: Mut Store<T>, value: T) => s: +NonEmpty
}
"#;

// --- The motivating case ---

/// [qual-refn] The claim's owner states what a mutating call it does not own
/// does to its claim, and the call site believes it.
#[test]
fn a_refinement_establishes_a_qualifier_at_a_call_site() {
    let src = format!(
        "{PRELUDE}{NONEMPTY}\nfn f(s: Mut Store<Int>) -> None {{\n    \
         push(s, 1)\n    \
         let _n = needs_nonempty(s)\n}}\n"
    );
    assert!(errors(&src).is_empty(), "{:?}", errors(&src));
}

/// The control: without the refinement the same program is rejected, so the
/// acceptance above is the refinement's doing and not something else.
#[test]
fn without_the_refinement_the_qualifier_is_gone() {
    let bare = "\nqualifier NonEmpty<T> of Store<T> {\n    \
                fn qualifies(s: Store<T>) -> Bool {\n        return true\n    }\n}\n";
    let src = format!(
        "{PRELUDE}{bare}\nfn f(s: Mut Store<Int>) -> None {{\n    \
         push(s, 1)\n    \
         let _n = needs_nonempty(s)\n}}\n"
    );
    assert!(
        errors(&src)
            .iter()
            .any(|e| e.contains("no matching overload for `needs_nonempty")),
        "{:?}",
        errors(&src)
    );
}

/// [qual-refn] `-Q` is the other direction: the qualifier declares that a
/// call *invalidates* its claim, even one that keeps everything else.
#[test]
fn a_refinement_can_invalidate_a_qualifier() {
    let src = format!(
        "{PRELUDE}\n\
         qualifier NonEmpty<T> of Store<T> {{\n    \
         fn qualifies(s: Store<T>) -> Bool {{\n        return true\n    }}\n\n    \
         refn read(s: Store<T>) => s: -NonEmpty\n}}\n\n\
         fn read<T>(s: Store<T>) [] -> Int => s {{\n    return 0\n}}\n\n\
         fn nonempty<T>(s: Store<T>) -> Store<T> as NonEmpty {{\n    return s\n}}\n\n\
         fn f(s: NonEmpty Store<Int>) -> None {{\n    \
         let a = read(s)\n    \
         let b = needs_nonempty(s)\n}}\n"
    );
    assert!(
        errors(&src)
            .iter()
            .any(|e| e.contains("no matching overload for `needs_nonempty")),
        "{:?}",
        errors(&src)
    );
}

// --- Conflicts [qual-refn-conflict] ---

/// [qual-refn-conflict] Two qualifiers that cannot co-apply [qual-with]
/// cannot both be established by one call, so *neither* refinement applies —
/// no error, but a warning, because an imported refinement silently doing
/// nothing would be undiscoverable.
#[test]
fn conflicting_refinements_all_stand_down_with_a_warning() {
    let src = format!(
        "{PRELUDE}{NONEMPTY}\n\
         qualifier Sorted<T> of Store<T> {{\n    \
         fn qualifies(s: Store<T>) -> Bool {{\n        return true\n    }}\n\n    \
         refn push(s: Mut Store<T>, value: T) => s: +Sorted\n}}\n\n\
         fn f(s: Mut Store<Int>) -> None {{\n    \
         push(s, 1)\n    \
         let _n = needs_nonempty(s)\n}}\n"
    );
    let diags = errors(&src);
    assert!(
        diags.iter().any(|d| d.contains("disagree about `s`")
            && d.contains("`NonEmpty` and `Sorted`")
            && d.contains("reconcile them in a top-level `refn`")),
        "expected a conflict warning: {diags:?}"
    );
    // Nothing applied, so the caller is back to the unrefined behaviour.
    assert!(
        diags
            .iter()
            .any(|d| d.contains("no matching overload for `needs_nonempty")),
        "{diags:?}"
    );
}

/// [qual-refn-conflict] Two *compatible* qualifiers (one declares `with` the
/// other) both apply: a conflict is precisely "these could not have been
/// written together", not "there are two of them".
#[test]
fn compatible_refinements_both_apply() {
    let src = format!(
        "{PRELUDE}{NONEMPTY}\n\
         qualifier Sorted<T> of Store<T> with NonEmpty {{\n    \
         fn qualifies(s: Store<T>) -> Bool {{\n        return true\n    }}\n\n    \
         refn push(s: Mut Store<T>, value: T) => s: +Sorted\n}}\n\n\
         fn needs_sorted<T>(s: Sorted Store<T>) -> Int => s {{\n    return 2\n}}\n\n\
         fn f(s: Mut Store<Int>) -> None {{\n    \
         push(s, 1)\n    \
         let _a = needs_nonempty(s)\n    \
         let _b = needs_sorted(s)\n}}\n"
    );
    assert!(errors(&src).is_empty(), "{:?}", errors(&src));
}

/// [qual-refn-reconcile] The consumer's remedy: a top-level `refn`
/// *replaces* the qualifiers' own refinements for that parameter, so the
/// disagreement is gone and the reconciled result applies.
#[test]
fn a_top_level_refinement_reconciles_a_conflict() {
    let src = format!(
        "{PRELUDE}{NONEMPTY}\n\
         qualifier Sorted<T> of Store<T> {{\n    \
         fn qualifies(s: Store<T>) -> Bool {{\n        return true\n    }}\n\n    \
         refn push(s: Mut Store<T>, value: T) => s: +Sorted\n}}\n\n\
         refn push<T>(s: Mut Store<T>, value: T) => s: +NonEmpty\n\n\
         fn f(s: Mut Store<Int>) -> None {{\n    \
         push(s, 1)\n    \
         let _n = needs_nonempty(s)\n}}\n"
    );
    assert!(errors(&src).is_empty(), "{:?}", errors(&src));
}

// --- Scope [qual-refn-scope] ---

/// [qual-refn-scope] A qualifier's refinements travel with it: a file that
/// does not import the qualifier does not get its refinements, which is what
/// "the user opts into the refinements when they opt into the qualifier"
/// means.
#[test]
fn a_refinement_is_only_in_scope_with_its_qualifier() {
    let quals = format!(
        "{PRELUDE}{NONEMPTY}\nfn qualified<T>(s: Store<T>) -> Store<T> as NonEmpty {{\n    \
         return s\n}}\n"
    );
    // Importing the qualifier: the refinement applies.
    let with_import = "import quals.NonEmpty\nimport quals.Store\n\
                       import quals.push\nimport quals.needs_nonempty\n\
                       fn g(s: Mut Store<Int>) -> None {\n    \
                       push(s, 1)\n    \
                       let _n = needs_nonempty(s)\n}\n";
    assert!(
        diagnostics(&[("quals.sv", &quals), ("user.sv", with_import)]).is_empty(),
        "{:?}",
        diagnostics(&[("quals.sv", &quals), ("user.sv", with_import)])
    );
    // Not importing it: `needs_nonempty` cannot be named either, so the
    // observable is the *other* overload — the value stays unqualified, and
    // a call that needs the claim is unavailable by name. What this test
    // pins is that the refinement is not applied program-wide: the file
    // without the import reports the unresolved qualifier rather than
    // silently benefiting from it.
    let without = "import quals.Store\nimport quals.push\n\
                   fn g(s: Mut Store<Int>) -> None {\n    \
                   push(s, 1)\n    \
                   let t: NonEmpty Store<Int> = s\n}\n";
    let diags = diagnostics(&[("quals.sv", &quals), ("user.sv", without)]);
    assert!(
        diags.iter().any(|d| d.contains("NonEmpty")),
        "expected the unimported qualifier to be unknown here: {diags:?}"
    );
}

/// [qual-refn-scope] A top-level refinement is module-scoped and *not*
/// importable: reconciling is the consumer's call, and a library shipping
/// its own reconciliation would move the conflict one level up.
#[test]
fn a_top_level_refinement_does_not_leave_its_module() {
    let lib = format!(
        "{PRELUDE}\nqualifier NonEmpty<T> of Store<T> {{\n    \
         fn qualifies(s: Store<T>) -> Bool {{\n        return true\n    }}\n}}\n\n\
         refn push<T>(s: Mut Store<T>, value: T) => s: +NonEmpty\n"
    );
    // The same module, second file: module scope, so it applies.
    let same_module = "fn g(s: Mut Store<Int>) -> None {\n    \
                       push(s, 1)\n    \
                       let _n = needs_nonempty(s)\n}\n";
    assert!(
        diagnostics(&[("lib.sv", &lib), ("lib2.sv", same_module)])
            .iter()
            .all(|d| !d.contains("no matching overload")),
        "a top-level refinement should apply across its own module: {:?}",
        diagnostics(&[("lib.sv", &lib), ("lib2.sv", same_module)])
    );
    // Another module, importing everything it can: the refinement stays
    // behind, so the call fails.
    let other = "import lib.NonEmpty\nimport lib.Store\n\
                 import lib.push\nimport lib.needs_nonempty\n\
                 fn g(s: Mut Store<Int>) -> None {\n    \
                 push(s, 1)\n    \
                 let _n = needs_nonempty(s)\n}\n";
    let diags = diagnostics(&[("lib.sv", &lib), ("other/user.sv", other)]);
    assert!(
        diags
            .iter()
            .any(|d| d.contains("no matching overload for `needs_nonempty")),
        "a top-level refinement must not be importable: {diags:?}"
    );
}

// --- Declaration rules ---

/// [qual-refn-match] The parameter list picks one overload, by names *and*
/// types: a mismatch is an error at the refinement rather than a refinement
/// that silently never fires.
#[test]
fn a_refinement_must_match_an_overload_exactly() {
    let cases = [
        // Wrong parameter name.
        (
            "refn push(store: Mut Store<T>, value: T) => store: +NonEmpty ",
            "matches no `push`",
        ),
        // Wrong type.
        (
            "refn push(s: Store<T>, value: T) => s: +NonEmpty ",
            "matches no `push`",
        ),
        // No such function.
        (
            "refn shove(s: Mut Store<T>, value: T) => s: +NonEmpty ",
            "no function `shove` is visible here",
        ),
        // A parameter the function does not have.
        (
            "refn push(s: Mut Store<T>, value: T) => other: +NonEmpty",
            "has no parameter `other`",
        ),
    ];
    for (refn, expected) in cases {
        let src = format!(
            "{PRELUDE}\nqualifier NonEmpty<T> of Store<T> {{\n    \
             fn qualifies(s: Store<T>) -> Bool {{\n        return true\n    }}\n\n    \
             {refn}\n}}\n"
        );
        let diags = errors(&src);
        assert!(
            diags.iter().any(|d| d.contains(expected)),
            "expected {expected:?} for {refn:?}, got {diags:?}"
        );
    }
}

/// [qual-refn-match] A type parameter has to be bound. A top-level
/// refinement has no qualifier to borrow `T` from, and the diagnostic names
/// the remedy rather than reporting a shape mismatch.
#[test]
fn an_unbound_type_parameter_names_the_remedy() {
    let src = format!(
        "{PRELUDE}\nqualifier NonEmpty<T> of Store<T> {{\n    \
         fn qualifies(s: Store<T>) -> Bool {{\n        return true\n    }}\n}}\n\n\
         refn push(s: Mut Store<T>, value: T) => s: +NonEmpty\n"
    );
    let diags = errors(&src);
    assert!(
        diags
            .iter()
            .any(|d| d.contains("unknown type `T`") && d.contains("refn push<T>")),
        "{diags:?}"
    );
}

/// [qual-refn] A qualifier may only speak about its *own* claim. That is
/// what makes "opting into a qualifier opts into its refinements" honest,
/// and it is why conflicts reduce to two qualifiers that cannot co-apply.
#[test]
fn a_qualifier_may_only_refine_its_own_claim() {
    let src = format!(
        "{PRELUDE}\nqualifier Sorted<T> of Store<T> {{\n    \
         fn qualifies(s: Store<T>) -> Bool {{\n        return true\n    }}\n}}\n\n\
         qualifier NonEmpty<T> of Store<T> {{\n    \
         fn qualifies(s: Store<T>) -> Bool {{\n        return true\n    }}\n\n    \
         refn push(s: Mut Store<T>, value: T) => s: +Sorted\n}}\n"
    );
    let diags = errors(&src);
    assert!(
        diags.iter().any(|d| d
            .contains("a refinement declared by `NonEmpty` can only establish `NonEmpty`")
            && d.contains("top-level `refn`")),
        "{diags:?}"
    );
}

/// [qual-refn] Only *state* qualifiers can be refined [qual-subject]:
/// provenance cannot be invalidated in the first place, and the compiler's
/// own qualifiers carry representation choices — `Mut` is not even erased,
/// so handing one out is not a refinement's business.
#[test]
fn only_state_qualifiers_can_be_refined() {
    let provenance = format!(
        "{PRELUDE}\nprovenance qualifier Trusted<T> of Store<T>\n\n\
         qualifier NonEmpty<T> of Store<T> {{\n    \
         fn qualifies(s: Store<T>) -> Bool {{\n        return true\n    }}\n}}\n\n\
         refn push<T>(s: Mut Store<T>, value: T) => s: +Trusted\n"
    );
    assert!(
        errors(&provenance)
            .iter()
            .any(|d| d.contains("`Trusted` is a provenance qualifier")),
        "{:?}",
        errors(&provenance)
    );
    let intrinsic = format!(
        "{PRELUDE}\nrefn push<T>(s: Mut Store<T>, value: T) => s: +Mut\n"
    );
    assert!(
        errors(&intrinsic)
            .iter()
            .any(|d| d.contains("unknown qualifier `Mut`")
                || d.contains("compiler's own qualifiers")),
        "{:?}",
        errors(&intrinsic)
    );
}

/// [qual-refn] There is nothing to refine about a parameter the call takes
/// away: a caller knows nothing about a moved value afterwards.
#[test]
fn refining_a_moved_parameter_is_an_error() {
    let src = format!(
        "{PRELUDE}\nqualifier NonEmpty<T> of Store<T> {{\n    \
         fn qualifies(s: Store<T>) -> Bool {{\n        return true\n    }}\n\n    \
         refn push(s: Mut Store<T>, value: T) => value: +NonEmpty\n}}\n"
    );
    let diags = errors(&src);
    assert!(
        diags
            .iter()
            .any(|d| d.contains("moves `value`") && d.contains("nothing to refine")),
        "{diags:?}"
    );
}

/// [qual-refn] The claim has to be about the parameter's type [qual-of].
#[test]
fn a_refinement_qualifier_must_apply_to_the_parameter() {
    let src = format!(
        "{PRELUDE}\nqualifier NonEmpty<T> of Store<T> {{\n    \
         fn qualifies(s: Store<T>) -> Bool {{\n        return true\n    }}\n\n    \
         refn count(text: Str) => text: +NonEmpty\n}}\n\n\
         fn count(text: Str) [] -> Int => text {{\n    return 0\n}}\n"
    );
    let diags = errors(&src);
    assert!(
        diags
            .iter()
            .any(|d| d.contains("`NonEmpty` applies to `Store<T>`, not to `Str`")),
        "{diags:?}"
    );
}

// --- Inference [qual-refn-infer] ---

/// [qual-refn-infer] A refinement reaches the *inferred* contract, so the
/// fact propagates one frame outward: a fn whose parameter declares the
/// qualifier and whose body pushes may promise it back.
#[test]
fn a_refinement_reaches_an_inferred_deduction() {
    let src = format!(
        "{PRELUDE}{NONEMPTY}\n\
         fn refill<T>(s: Mut NonEmpty Store<T>, v: T) -> None => !v {{\n    \
         push(s, v)\n}}\n\n\
         fn f(s: Mut Store<Int>) -> None {{\n    \
         push(s, 1)\n    \
         refill(s, 2)\n    \
         let _n = needs_nonempty(s)\n}}\n"
    );
    assert!(errors(&src).is_empty(), "{:?}", errors(&src));
}

/// [qual-refn-infer] …and a *written* list promising it back validates
/// against the same body facts, which is the same mechanism seen from the
/// other side ([deduce-infer] rejects a promise the body may break).
#[test]
fn a_written_list_may_promise_a_refined_qualifier() {
    let with_refn = format!(
        "{PRELUDE}{NONEMPTY}\n\
         fn refill<T>(s: Mut NonEmpty Store<T>, v: T) -> None => s: Mut NonEmpty, !v {{\n    \
         push(s, v)\n}}\n"
    );
    assert!(errors(&with_refn).is_empty(), "{:?}", errors(&with_refn));

    let bare = "\nqualifier NonEmpty<T> of Store<T> {\n    \
                fn qualifies(s: Store<T>) -> Bool {\n        return true\n    }\n}\n";
    let without = format!(
        "{PRELUDE}{bare}\n\
         fn refill<T>(s: Mut NonEmpty Store<T>, v: T) -> None => s: Mut NonEmpty, !v {{\n    \
         push(s, v)\n}}\n"
    );
    assert!(
        errors(&without).iter().any(|d| d
            .contains("promises qualifier `NonEmpty`")
            && d.contains("body may remove it")),
        "{:?}",
        errors(&without)
    );
}

/// [qual-refn-infer] An addition inside a block that may not run does *not*
/// reach the inferred contract: the deduction pass is a meet over all uses
/// rather than a flow analysis, so a conditional call cannot establish a
/// fact the signature then promises. The call site, which *is*
/// flow-sensitive, still narrows inside the branch.
#[test]
fn a_conditional_refinement_does_not_reach_the_contract() {
    let src = format!(
        "{PRELUDE}{NONEMPTY}\n\
         fn maybe<T>(s: Mut NonEmpty Store<T>, v: T, c: Bool) -> None => s: Mut NonEmpty, !v {{\n    \
         if c {{\n        push(s, v)\n    }}\n}}\n"
    );
    assert!(
        errors(&src).iter().any(|d| d
            .contains("promises qualifier `NonEmpty`")
            && d.contains("body may remove it")),
        "{:?}",
        errors(&src)
    );
}
