//! [is-narrow-guard] Narrowing survives an **early-returning guard** (user
//! decision 2026-09-09, closing the defect found by the R5 probe).
//!
//! `if e is None { return … }` leaves the rest of the block on the *else*
//! path, so every condition's else-narrows hold there — the same facts
//! [is-narrowing] already gives an `else` branch, carried past a statement
//! whose branches cannot fall through. That is what makes the ordinary way of
//! writing a `next` over a container — guard on the end, then use the
//! element — expressible without an `else` block or a two-armed `when`.
//!
//! The rule keys on *every* branch exiting: `return`, `break`, `continue`,
//! and a diverging call ([type-any-never]) all count, and a branch that can
//! fall through narrows nothing after the `if`.

use std::path::Path;

use salvo_core::{check_program, resolve, Program, SourceSet, Symbols};

const STD_PRELUDE: &str = "export intrinsic type Int\nexport intrinsic type Str\nexport intrinsic type Bool\n\
     export intrinsic type Never\n";

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

/// The shape the defect was found in: guard on the empty case, then use the
/// value at the narrowed type. Assigning it to a `Str` local is the
/// assertion — it only type-checks if the fact survived the `if`.
#[test]
fn a_returning_guard_narrows_the_rest_of_the_block() {
    let errs = errors(
        "fn describe(s: Str?) -> Str => s {\n    \
         if s is None {\n        return \"none\"\n    }\n    \
         let text: Str = s\n    return \"got\"\n}\n",
    );
    assert!(errs.is_empty(), "expected no errors, got {errs:?}");
}

/// A `break` leaves the block just as a `return` does.
#[test]
fn a_breaking_guard_narrows_the_rest_of_the_body() {
    let errs = errors(
        "fn count(s: Str?) -> Int => s {\n    \
         let n = 0\n    \
         while n < 3 {\n        \
         if s is None {\n            break\n        }\n        \
         let text: Str = s\n        n = n + 1\n    }\n    return n\n}\n",
    );
    assert!(errs.is_empty(), "expected no errors, got {errs:?}");
}

/// So does a `continue`.
#[test]
fn a_continuing_guard_narrows_the_rest_of_the_body() {
    let errs = errors(
        "fn count(s: Str?) -> Int => s {\n    \
         let n = 0\n    \
         while n < 3 {\n        \
         n = n + 1\n        \
         if s is None {\n            continue\n        }\n        \
         let text: Str = s\n    }\n    return n\n}\n",
    );
    assert!(errs.is_empty(), "expected no errors, got {errs:?}");
}

/// [type-any-never] A diverging *call* is an exit too: nothing after it
/// runs, so the branch cannot fall through.
#[test]
fn a_diverging_call_in_the_guard_counts_as_an_exit() {
    let errs = errors(
        "fn give_up(reason: Str) -> Never => !reason {\n    return give_up(reason)\n}\n\
         fn describe(s: Str?) -> Str => s {\n    \
         if s is None {\n        give_up(\"none\")\n    }\n    \
         let text: Str = s\n    return \"got\"\n}\n",
    );
    assert!(errs.is_empty(), "expected no errors, got {errs:?}");
}

/// An `elif` chain accumulates: with both arms exiting, what is left is the
/// third.
#[test]
fn an_elif_chain_leaves_the_remaining_arm() {
    let errs = errors(
        "fn pick(v: Int | Str | Bool) -> Bool => v {\n    \
         if v is Int {\n        return false\n    } elif v is Str {\n        return true\n    }\n    \
         let flag: Bool = v\n    return true\n}\n",
    );
    assert!(errs.is_empty(), "expected no errors, got {errs:?}");
}

/// The negative half: a branch that *can* fall through leaves the value at
/// its declared type, since either path may have been taken.
#[test]
fn a_branch_that_falls_through_narrows_nothing() {
    let errs = errors(
        "fn describe(s: Str?) -> Str => s {\n    \
         let seen = false\n    \
         if s is None {\n        seen = true\n    }\n    \
         let text: Str = s\n    return \"got\"\n}\n",
    );
    assert!(
        errs.iter().any(|e| e.contains("expected `Str`, found `Str?`")),
        "got {errs:?}"
    );
}

/// One exiting branch is not enough when another falls through.
#[test]
fn a_mixed_if_narrows_nothing() {
    let errs = errors(
        "fn pick(v: Int | Str | Bool) -> Bool => v {\n    \
         let n = 0\n    \
         if v is Int {\n        return false\n    } elif v is Str {\n        n = 1\n    }\n    \
         let flag: Bool = v\n    return true\n}\n",
    );
    assert!(
        errs.iter().any(|e| e.contains("expected `Bool`")),
        "got {errs:?}"
    );
}

/// An explicit `else` that falls through is the path taken, and it carries
/// the same facts — so the narrowing holds there too, and after the `if`.
#[test]
fn an_exiting_then_branch_with_an_else_still_narrows_after() {
    let errs = errors(
        "fn describe(s: Str?) -> Str => s {\n    \
         let seen = false\n    \
         if s is None {\n        return \"none\"\n    } else {\n        seen = true\n    }\n    \
         let text: Str = s\n    return \"got\"\n}\n",
    );
    assert!(errs.is_empty(), "expected no errors, got {errs:?}");
}

/// [narrow-assign-reset] An assignment *on the surviving path* still resets:
/// the fact was about the old value.
#[test]
fn an_assignment_on_the_surviving_path_resets_the_narrowing() {
    let errs = errors(
        "fn describe(s: Str?, other: Str?) -> Str => s, other {\n    \
         if s is None {\n        return \"none\"\n    } else {\n        s = other\n    }\n    \
         let text: Str = s\n    return \"got\"\n}\n",
    );
    assert!(
        errs.iter().any(|e| e.contains("expected `Str`, found `Str?`")),
        "got {errs:?}"
    );
}

/// But an assignment inside the *exiting* branch does not: that path never
/// reaches the code below (the same reason a fall-through merge ignores it).
#[test]
fn an_assignment_in_the_exiting_branch_does_not_reset() {
    let errs = errors(
        "fn describe(s: Str?, other: Str?) -> Str => s, other {\n    \
         if s is None {\n        s = other\n        return \"none\"\n    }\n    \
         let text: Str = s\n    return \"got\"\n}\n",
    );
    assert!(errs.is_empty(), "expected no errors, got {errs:?}");
}

/// [is-qualifies] [fn-overload-rank] The same rule through a **negated** guard
/// over a *predicate qualifier*, which is the shape `demo/heap.sv` reaches for
/// (HEAP_QUALIFIER.md item 5): `if !(xs is NonEmpty) { return … }` puts the
/// claim in the condition's else-narrows — `Not` swaps them — so the rest of the
/// block holds it, and the call below routes to the overload that *demands* it.
///
/// Verified rather than assumed (2026-09-22): [is-qualifies]' "no else
/// information" note reads as though nobody had tested the negated path. Both
/// halves already worked; what the demo still waits for is only the `!is`
/// spelling.
#[test]
fn a_negated_guard_narrows_a_predicate_qualifier_and_routes_the_call() {
    let errs = errors(
        "qualifier NonEmpty of Str {\n    \
         fn qualifies(s: Str) -> Bool {\n        return true\n    }\n}\n\n\
         fn taste(s: Str) -> Int => s {\n    return 0\n}\n\n\
         fn taste(s: NonEmpty Str) -> Str => s {\n    return \"nonempty\"\n}\n\n\
         fn probe(s: Str) -> Str => s {\n    \
         if !(s is NonEmpty) {\n        return \"empty\"\n    }\n    \
         let text: Str = taste(s)\n    return text\n}\n",
    );
    assert!(errs.is_empty(), "expected no errors, got {errs:?}");
}

/// The negative half, so the test above is about the *guard* and not about
/// overload luck: without it, `taste` answers the plain overload's `Int`.
#[test]
fn without_the_guard_the_plain_overload_wins() {
    let errs = errors(
        "qualifier NonEmpty of Str {\n    \
         fn qualifies(s: Str) -> Bool {\n        return true\n    }\n}\n\n\
         fn taste(s: Str) -> Int => s {\n    return 0\n}\n\n\
         fn taste(s: NonEmpty Str) -> Str => s {\n    return \"nonempty\"\n}\n\n\
         fn probe(s: Str) -> Str => s {\n    \
         let text: Str = taste(s)\n    return text\n}\n",
    );
    assert!(
        errs.iter().any(|e| e.contains("expected `Str`, found `Int`")),
        "got {errs:?}"
    );
}

/// [is-not] The same guard in the spelling it is usually reached for
/// (`!is`, user decision 2026-09-22): sugar for `!(x is Q)`, so it narrows the
/// fall-through exactly as the parenthesized form does — one `Expr::Is` under a
/// `Not`, which is all the checker ever sees.
#[test]
fn the_not_is_spelling_narrows_the_same_way() {
    let errs = errors(
        "fn describe(s: Str?) -> Str {\n    \
         if s !is Str {\n        return \"none\"\n    }\n    \
         let text: Str = s\n    return text\n}\n",
    );
    assert!(errs.is_empty(), "expected no errors, got {errs:?}");
}
