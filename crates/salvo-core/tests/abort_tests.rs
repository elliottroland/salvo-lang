//! Non-resumption: `abort` and the intrinsic `try` [abort] [try].
//!
//! `abort(message)` returns `Nothing`, so the code between it and its
//! delimiter does not run; a function that may abort says so with
//! `[Abort<M>]` and returns its *own* type. The delimiter is `try { ... }`,
//! a compiler intrinsic whose value is `Ok T | Aborted M` — an ordinary
//! union, so `is`/`when`/exhaustiveness need no new rules. `M` is the union
//! of the message types the body performs (user decision 2026-09-04).

use std::path::Path;

use salvo_core::{check_program, resolve, Program, SourceSet, Symbols};

/// Parses + resolves + checks one file (no std) and returns every error
/// message.
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

/// The two names the intrinsic needs from core, plus a pair of aborting
/// functions with *different* message types.
const PRELUDE: &str = r#"
internal type Int
internal type Str
internal type Bool
internal type Nothing

effect Abort<M> {
    fn abort(message: M) -> [] Nothing
}

qualifier Ok<T> of T
qualifier Aborted<M> of M

struct Handle canbe Linear {
    fd: Int
}

external fn open_handle(fd: Int) [] -> [] Handle
external fn close_handle(h: Handle) [] -> [] None
external fn note(text: Str) [] -> [text] None

fn parse(line: Str) [Abort<Str>] -> [] Int {
    if flagged(line) {
        abort("bad line")
    }
    return 1
}

fn limit(n: Int) [Abort<Int>] -> [] Int {
    if n > 3 {
        abort(n)
    }
    return n
}

external fn flagged(line: Str) [] -> [line] Bool
"#;

/// Wraps a body in a `[]`-effect fn returning `None`.
fn check(body: &str) -> Vec<String> {
    errors(&format!(
        "{PRELUDE}\nfn probe(flag: Bool) [] -> [] None {{\n{body}\n}}\n"
    ))
}

// ===== the outcome type [try] =====

/// [try] The delimiter's value is `Ok T | Aborted M` — read off a
/// deliberate annotation mismatch, which is the cheapest way to see the
/// type the checker computed.
#[test]
fn try_yields_ok_and_aborted_arms() {
    let errs = check(
        r#"
    let outcome: Bool = try {
        parse("x")
    }
"#,
    );
    assert!(
        errs.iter()
            .any(|e| e.contains("found `Ok Int | Aborted Str`")),
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
            .any(|e| e.contains("found `Ok Int | Aborted (Str | Int)`")),
        "expected a union message type, got: {errs:?}"
    );
}

/// [try] A body whose every path leaves still has an `Ok` arm: `Ok None`.
#[test]
fn an_always_aborting_body_still_has_an_ok_arm() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn probe() [] -> [] None {{\n\
         let outcome: Bool = try {{\n\
         parse(\"x\")\n\
         note(\"unreachable\")\n\
         }}\n\
         }}\n"
    ));
    assert!(
        errs.iter().any(|e| e.contains("found `Ok None | Aborted Str`")),
        "expected `Ok None`, got: {errs:?}"
    );
}

/// [try] Nothing in the body can abort, so the delimiter has no outcome to
/// produce (user decision 2026-09-04: an error, not `Aborted None`).
#[test]
fn a_try_that_cannot_abort_is_rejected() {
    let errs = check(
        r#"
    let outcome = try {
        note("nothing to abort here")
    }
"#,
    );
    assert!(
        errs.iter()
            .any(|e| e.contains("nothing in this `try` block can abort")),
        "expected the no-abort rejection, got: {errs:?}"
    );
}

// ===== propagation [abort] =====

/// [abort] An abort needs somewhere to land: a `try` or the enclosing fn's
/// declared effect. The diagnostic names both remedies.
#[test]
fn an_abort_with_nowhere_to_land_is_rejected() {
    let errs = check(
        r#"
    let n = parse("x")
    note("${n}")
"#,
    );
    assert!(
        errs.iter()
            .any(|e| e.contains("nothing here can receive an abort")),
        "expected the no-delimiter rejection, got: {errs:?}"
    );
}

/// [abort] Declaring the effect passes it on: no `try` needed, and the fn's
/// own return type is unchanged (no `Result` plumbing).
#[test]
fn declaring_the_effect_propagates() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn forward(line: Str) [Abort<Str>] -> [] Int {{\n\
         return parse(line)\n\
         }}\n"
    ));
    assert!(errs.is_empty(), "unexpected errors: {errs:?}");
}

/// [abort] The message must fit what the landing site carries.
#[test]
fn a_message_the_target_cannot_carry_is_rejected() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn forward(line: Str) [Abort<Int>] -> [] Int {{\n\
         return parse(line)\n\
         }}\n"
    ));
    assert!(
        errs.iter().any(|e| e.contains("carries a `Str` message")),
        "expected a message-type mismatch, got: {errs:?}"
    );
}

/// [abort-not-main] The entry point has no caller to receive an abort, and
/// no Rust lowering (`main` cannot return `ControlFlow`).
#[test]
fn main_cannot_declare_the_abort_effect() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn main() [Abort<Str>] -> [] None {{\n\
         note(\"hi\")\n\
         }}\n"
    ));
    assert!(
        errs.iter()
            .any(|e| e.contains("`main` cannot declare `Abort`")),
        "expected the main rejection, got: {errs:?}"
    );
}

/// [abort] There is no handler for aborting: a handler would have to
/// *resume*, which `Nothing` forbids.
#[test]
fn a_handler_for_abort_is_rejected() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         handler Swallow of Abort<Str> {{\n\
         fn abort(message: Str) -> [] Nothing {{\n\
         note(message)\n\
         }}\n\
         }}\n"
    ));
    assert!(
        errs.iter().any(|e| e.contains("`Abort` has no handlers")),
        "expected the handler rejection, got: {errs:?}"
    );
}

// ===== divergence [type-any-nothing] [fn-must-return] =====

/// [fn-must-return] `abort` returns `Nothing`, so a branch ending in one
/// satisfies "every path returns" — the path ends there.
#[test]
fn an_aborting_branch_counts_as_returning() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn strict(n: Int) [Abort<Str>] -> [] Int {{\n\
         if n > 0 {{\n\
         return n\n\
         }} else {{\n\
         abort(\"not positive\")\n\
         }}\n\
         }}\n"
    ));
    assert!(errs.is_empty(), "unexpected errors: {errs:?}");
}

/// [type-any-nothing] The same rule at the level of flow analysis: a
/// branch that aborts never falls through, so a value it consumed is still
/// live afterwards.
#[test]
fn an_aborting_branch_does_not_leak_its_consumption() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn strict(text: Str) [Abort<Str>] -> [] None {{\n\
         if flagged(text) {{\n\
         abort(text)\n\
         }}\n\
         note(text)\n\
         }}\n"
    ));
    assert!(errs.is_empty(), "unexpected errors: {errs:?}");
}

// ===== linear obligations across an abort [linear-obligation] =====

/// [abort] [linear-obligation] The code after a may-abort call does not run
/// on the abort path, so a linear value live across it would leak — the
/// diagnostic names `defer`, the one way to discharge on a path the author
/// does not write.
#[test]
fn a_linear_value_across_a_may_abort_call_is_rejected() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn risky(line: Str) [Abort<Str>] -> [] Int {{\n\
         let h = open_handle(1)\n\
         let n = parse(line)\n\
         close_handle(h)\n\
         return n\n\
         }}\n"
    ));
    assert!(
        errs.iter()
            .any(|e| e.contains("across a call that may abort")),
        "expected the linear-across-abort rejection, got: {errs:?}"
    );
}

/// [defer] The remedy, and the reason `defer` was built first: the deferred
/// release runs on the abort path too, so the obligation is discharged on
/// every path.
#[test]
fn a_deferred_release_discharges_it() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn safe(line: Str) [Abort<Str>] -> [] Int {{\n\
         let h = open_handle(1)\n\
         defer {{ close_handle(h) }}\n\
         return parse(line)\n\
         }}\n"
    ));
    assert!(errs.is_empty(), "unexpected errors: {errs:?}");
}

// ===== aborting from a deferred block [defer-no-escape] =====

/// [defer-no-escape] A deferred block runs *while* its scope is being left:
/// unwinding out of an unwind path is a hole neither lowering wants.
#[test]
fn aborting_in_a_deferred_block_is_rejected() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn bad(line: Str) [Abort<Str>] -> [] None {{\n\
         defer {{ abort(\"on the way out\") }}\n\
         note(line)\n\
         }}\n"
    ));
    assert!(
        errs.iter()
            .any(|e| e.contains("an `abort` is not allowed in a deferred block")),
        "expected the deferred-abort rejection, got: {errs:?}"
    );
}

/// [defer-no-escape] Same for a call that merely *may* abort: the deferred
/// block cannot know whether it will.
#[test]
fn a_may_abort_call_in_a_deferred_block_is_rejected() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn bad(line: Str) [Abort<Str>] -> [] None {{\n\
         defer {{ note(\"${{parse(line)}}\") }}\n\
         note(line)\n\
         }}\n"
    ));
    assert!(
        errs.iter()
            .any(|e| e.contains("a call that may abort is not allowed in a deferred block")),
        "expected the deferred-propagation rejection, got: {errs:?}"
    );
}

// ===== nesting [try-innermost] =====

/// [try-innermost] An abort lands in the innermost delimiter, so an inner
/// `try` does not need the outer one's message type — and the outer fn's
/// `[Abort<M>]` is unaffected by what the inner one catches.
#[test]
fn an_inner_try_takes_only_its_own_aborts() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn nested(line: Str) [Abort<Str>] -> [] Int {{\n\
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
            .any(|e| e.contains("found `Ok Int | Aborted Int`")),
        "expected the inner delimiter's own message type, got: {errs:?}"
    );
    assert_eq!(errs.len(), 1, "expected only the annotation error: {errs:?}");
}
