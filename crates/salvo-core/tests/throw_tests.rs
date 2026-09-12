//! Non-resumption: `throw` and the intrinsic `try` [throw] [try].
//!
//! `throw(message)` returns `Nothing`, so the code between it and its
//! delimiter does not run; a function that may throw says so with
//! `[Throw<M>]` and returns its *own* type. The delimiter is `try { ... }`,
//! a compiler intrinsic whose value is `Ok T | Thrown M` — an ordinary
//! union, so `is`/`when`/exhaustiveness need no new rules. `M` is the union
//! of the message types the body performs (user decision 2026-09-04).

use std::path::Path;

use salvo_core::{check_program, resolve, Program, SourceSet, Symbols};

/// [intrinsic-std-only] The intrinsic declarations these sources rely on.
/// `intrinsic` is the compiler's modifier and only std may write it, so this
/// is loaded as a *std* file rather than pasted into the source under test.
/// Module `core.prelude`: `core.*` is implicitly imported, so the test source
/// sees these names without an `import`.
const STD_PRELUDE: &str = "intrinsic type Int\nintrinsic type Str\nintrinsic type Bool\nintrinsic type Nothing\nintrinsic fn discard<T canbe Linear>(value: T) [] -> None => !value\nparams Linear<It> {\n    fn close(it: It) -> None => !it\n}\n";

/// Parses + resolves + checks one file (no std) and returns every error
/// message.
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

/// The two names the intrinsic needs from core, plus a pair of throwing
/// functions with *different* message types.
const PRELUDE: &str = r#"

effect Throw<M> {
    fn throw(message: M) -> Nothing => !message
}

qualifier Ok<T> of T
qualifier Thrown<M> of M

struct Handle : Linear<self> {
    fd: Int
}

fn close(x: Handle) -> None => !x {}


fn open_handle(fd: Int) [] -> Handle {
    return Handle { fd: fd }
}
fn close_handle(h: Handle) [] -> None => !h {
    close(h)
}
fn note(text: Str) [] -> None => text {}

fn parse(line: Str) [Throw<Str>] -> Int => !line {
    if flagged(line) {
        throw("bad line")
    }
    return 1
}

fn limit(n: Int) [Throw<Int>] -> Int {
    if n > 3 {
        throw(n)
    }
    return n
}

fn flagged(line: Str) [] -> Bool => line {
    return true
}
"#;

/// Wraps a body in a `[]`-effect fn returning `None`.
fn check(body: &str) -> Vec<String> {
    errors(&format!(
        "{PRELUDE}\nfn probe(flag: Bool) [] -> None {{\n{body}\n}}\n"
    ))
}

// ===== the outcome type [try] =====

/// [try] The delimiter's value is `Ok T | Thrown M` — read off a
/// deliberate annotation mismatch, which is the cheapest way to see the
/// type the checker computed.
#[test]
fn try_yields_ok_and_thrown_arms() {
    let errs = check(
        r#"
    let outcome: Bool = try {
        parse("x")
    }
"#,
    );
    assert!(
        errs.iter()
            .any(|e| e.contains("found `Ok Int | Thrown Str`")),
        "expected the outcome union, got: {errs:?}"
    );
}

/// [try] `M` is the *union* of the message types performed in the body
/// (user decision 2026-09-04, for consistency with `if`/`when` branch
/// types) — a single type stays bare, several form a union.
#[test]
fn several_message_types_union_together() {
    let errs = check(
        r#"
    let outcome: Bool = try {
        let n = parse("x")
        limit(n)
    }
"#,
    );
    assert!(
        errs.iter()
            .any(|e| e.contains("found `Ok Int | Thrown (Str | Int)`")),
        "expected a union message type, got: {errs:?}"
    );
}

/// [try] A body whose every path leaves still has an `Ok` arm: `Ok None`.
#[test]
fn an_always_throwing_body_still_has_an_ok_arm() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn probe() [] -> None {{\n\
         let outcome: Bool = try {{\n\
         parse(\"x\")\n\
         note(\"unreachable\")\n\
         }}\n\
         }}\n"
    ));
    assert!(
        errs.iter().any(|e| e.contains("found `Ok None | Thrown Str`")),
        "expected `Ok None`, got: {errs:?}"
    );
}

/// [try] Nothing in the body can throw, so the delimiter has no outcome to
/// produce (user decision 2026-09-04: an error, not `Thrown None`).
#[test]
fn a_try_that_cannot_throw_is_rejected() {
    let errs = check(
        r#"
    let outcome = try {
        note("nothing to throw here")
    }
"#,
    );
    assert!(
        errs.iter()
            .any(|e| e.contains("nothing in this `try` block can throw")),
        "expected the no-throw rejection, got: {errs:?}"
    );
}

// ===== propagation [throw] =====

/// [throw] A throw needs somewhere to land: a `try` or the enclosing fn's
/// declared effect. The diagnostic names both remedies.
#[test]
fn a_throw_with_nowhere_to_land_is_rejected() {
    let errs = check(
        r#"
    let n = parse("x")
    note("${n}")
"#,
    );
    assert!(
        errs.iter()
            .any(|e| e.contains("nothing here can receive a throw")),
        "expected the no-delimiter rejection, got: {errs:?}"
    );
}

/// [throw] Declaring the effect passes it on: no `try` needed, and the fn's
/// own return type is unchanged (no `Result` plumbing).
#[test]
fn declaring_the_effect_propagates() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn forward(line: Str) [Throw<Str>] -> Int => !line {{\n\
         return parse(line)\n\
         }}\n"
    ));
    assert!(errs.is_empty(), "unexpected errors: {errs:?}");
}

/// [throw] The message must fit what the landing site carries.
#[test]
fn a_message_the_target_cannot_carry_is_rejected() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn forward(line: Str) [Throw<Int>] -> Int => !line {{\n\
         return parse(line)\n\
         }}\n"
    ));
    assert!(
        errs.iter().any(|e| e.contains("carries a `Str` message")),
        "expected a message-type mismatch, got: {errs:?}"
    );
}

/// [throw-not-main] The entry point has no caller to receive a throw, and
/// no Rust lowering (`main` cannot return `ControlFlow`).
#[test]
fn main_cannot_declare_the_throw_effect() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn main() [Throw<Str>] -> None {{\n\
         note(\"hi\")\n\
         }}\n"
    ));
    assert!(
        errs.iter()
            .any(|e| e.contains("`main` cannot declare `Throw`")),
        "expected the main rejection, got: {errs:?}"
    );
}

/// [throw] There is no handler for throwing: a handler would have to
/// *resume*, which `Nothing` forbids.
#[test]
fn a_handler_for_throw_is_rejected() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         handler Swallow of Throw<Str> {{\n\
         fn throw(message: Str) -> Nothing => !message {{\n\
         note(message)\n\
         }}\n\
         }}\n"
    ));
    assert!(
        errs.iter().any(|e| e.contains("`Throw` has no handlers")),
        "expected the handler rejection, got: {errs:?}"
    );
}

// ===== divergence [type-any-nothing] [fn-must-return] =====

/// [fn-must-return] `throw` returns `Nothing`, so a branch ending in one
/// satisfies "every path returns" — the path ends there.
#[test]
fn a_throwing_branch_counts_as_returning() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn strict(n: Int) [Throw<Str>] -> Int {{\n\
         if n > 0 {{\n\
         return n\n\
         }} else {{\n\
         throw(\"not positive\")\n\
         }}\n\
         }}\n"
    ));
    assert!(errs.is_empty(), "unexpected errors: {errs:?}");
}

/// [type-any-nothing] The same rule at the level of flow analysis: a
/// branch that throws never falls through, so a value it consumed is still
/// live afterwards.
#[test]
fn a_throwing_branch_does_not_leak_its_consumption() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn strict(text: Str) [Throw<Str>] -> None => !text {{\n\
         if flagged(text) {{\n\
         throw(text)\n\
         }}\n\
         note(text)\n\
         }}\n"
    ));
    assert!(errs.is_empty(), "unexpected errors: {errs:?}");
}

// ===== linear obligations across a throw [linear-obligation] =====

/// [throw] [linear-obligation] The code after a may-throw call does not run
/// on the throw path, so a linear value live across it would leak. Since
/// `defer` was removed (2026-09-10) there is no construct that discharges on a
/// path the author does not write, so the diagnostic names the two remedies
/// that remain: release before the call, or move the value onward.
#[test]
fn a_linear_value_across_a_may_throw_call_is_rejected() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn risky(line: Str) [Throw<Str>] -> Int => !line {{\n\
         let h = open_handle(1)\n\
         let n = parse(line)\n\
         close_handle(h)\n\
         return n\n\
         }}\n"
    ));
    assert!(
        errs.iter()
            .any(|e| e.contains("across a call that may throw")),
        "expected the linear-across-throw rejection, got: {errs:?}"
    );
}

/// The remedy that remains: release *before* the call, so the throw path owes
/// nothing. This is what removing `defer` costs — the release is written where
/// it happens rather than registered once — and what it buys: the obligation is
/// checked rather than delegated to a construct.
#[test]
fn releasing_before_the_call_discharges_it() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn safe(line: Str) [Throw<Str>] -> Int => !line {{\n\
         let h = open_handle(1)\n\
         close_handle(h)\n\
         return parse(line)\n\
         }}\n"
    ));
    assert!(errs.is_empty(), "unexpected errors: {errs:?}");
}


// ===== nesting [try-innermost] =====

/// [try-innermost] A throw lands in the innermost delimiter, so an inner
/// `try` does not need the outer one's message type — and the outer fn's
/// `[Throw<M>]` is unaffected by what the inner one catches.
#[test]
fn an_inner_try_takes_only_its_own_throws() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn nested(line: Str) [Throw<Str>] -> Int => !line {{\n\
         let inner: Bool = try {{\n\
         limit(2)\n\
         }}\n\
         return parse(line)\n\
         }}\n"
    ));
    // The inner delimiter's outcome carries `Int` (from `limit`), *not* the
    // `Str` the enclosing fn propagates.
    assert!(
        errs.iter()
            .any(|e| e.contains("found `Ok Int | Thrown Int`")),
        "expected the inner delimiter's own message type, got: {errs:?}"
    );
    assert_eq!(errs.len(), 1, "expected only the annotation error: {errs:?}");
}

// ===== nested qualification and union messages [try] =====
// The two shapes the design flagged as wanting a test rather than an
// assumption. Both are reachable: a *qualified union* is a claim about a
// union, so the inner arms are matched by binding at the inner type — the
// droppable-qualifier rule (`Qual T <: T`) does the unwrapping, which is
// also why `Once` (never droppable) needs no special case here.

/// [try] A body that already returns a result yields
/// `Ok (Ok Int | Err Str) | Thrown M`, and the inner result is reachable.
#[test]
fn a_nested_result_outcome_can_be_taken_apart() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         qualifier Err<M> of M\n\
         fn wrapped(n: Int) [Throw<Str>] -> Ok Int | Err Str {{\n\
         if flagged(\"x\") {{\n\
         throw(\"bad\")\n\
         }}\n\
         return ok(n)\n\
         }}\n\
         fn ok(value: Int) [] -> Int as Ok {{\n\
         return value\n\
         }}\n\
         fn probe() [] -> None {{\n\
         let outcome = try {{ wrapped(1) }}\n\
         when outcome {{\n\
         is Ok {{\n\
         let inner: Ok Int | Err Str = outcome\n\
         when inner {{\n\
         is Ok {{ note(\"value\") }}\n\
         is Err {{ note(\"error\") }}\n\
         }}\n\
         }}\n\
         is Thrown {{\n\
         note(\"thrown\")\n\
         }}\n\
         }}\n\
         }}\n"
    ));
    assert!(errs.is_empty(), "unexpected errors: {errs:?}");
}

/// [try] The same for a *union message*: `Thrown (Str | Int)`'s arms are
/// reachable by binding at the message type.
#[test]
fn a_union_message_can_be_taken_apart() {
    let errs = check(
        r#"
    let outcome = try {
        let n = parse("x")
        limit(n)
    }
    when outcome {
        is Ok {
            note("ok")
        }
        is Thrown {
            let message: Str | Int = outcome
            when message {
                is Str {
                    note("text")
                }
                is Int {
                    note("number")
                }
            }
        }
    }
"#,
    );
    assert!(errs.is_empty(), "unexpected errors: {errs:?}");
}

/// [when-union-subject] Matching the qualified union *directly* is rejected
/// with a diagnostic naming that remedy — the same qualifier name can appear
/// at both levels (`Ok (Ok Int | …)`), so the binding is what makes which
/// level is meant visible.
#[test]
fn matching_a_qualified_union_directly_names_the_remedy() {
    let errs = check(
        r#"
    let outcome = try {
        let n = parse("x")
        limit(n)
    }
    when outcome {
        is Ok {
            note("ok")
        }
        is Thrown {
            when outcome {
                is Str {
                    note("text")
                }
                is Int {
                    note("number")
                }
            }
        }
    }
"#,
    );
    assert!(
        errs.iter().any(|e| e.contains("is the claim `Thrown` *about* a union")
            && e.contains("bind the inner union to a local")),
        "expected the remedy-naming diagnostic, got: {errs:?}"
    );
}

// ===== a `try` body is ordinary code to every traversal =====

/// [try] [narrow-assign-reset] An assignment inside a `try` body resets an
/// earlier `is` narrowing, like an assignment anywhere else. (Flow-sensitive
/// checking of the body is what achieves this, not the syntactic
/// assigned-name scan — verified by reverting that scan's `Expr::Try` arm
/// and watching this test still pass. Kept as a behavioural regression: the
/// body is code, and `try` must not become a narrowing barrier.)
#[test]
fn an_assignment_inside_a_try_body_resets_narrowing() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn probe(value: Str | Int) [] -> None => !value {{\n\
         if value is Str {{\n\
         let outcome = try {{\n\
         value = 7\n\
         parse(\"x\")\n\
         }}\n\
         note(value)\n\
         }}\n\
         }}\n"
    ));
    assert!(
        errs.iter().any(|e| e.contains("note")),
        "expected the reset narrowing to reject the `Str` read, got: {errs:?}"
    );
    // The control: without the assignment the narrowing stands and the
    // read is fine, so the rejection above means what it says.
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn probe(value: Str | Int) [] -> None => !value {{\n\
         if value is Str {{\n\
         let outcome = try {{\n\
         parse(\"x\")\n\
         }}\n\
         note(value)\n\
         }}\n\
         }}\n"
    ));
    assert!(errs.is_empty(), "unexpected errors: {errs:?}");
}
