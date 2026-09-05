//! Deferred blocks [defer] [defer-no-escape].
//!
//! `defer { ... }` runs when the enclosing *block* ends. Its meaning is
//! *splice at exit*: the body is checked once, where the `defer` stands,
//! and what it does is applied at every exit of that block — the end of
//! the block and each `return`/`break`/`continue` that leaves it. So a
//! linear obligation a deferred block discharges is discharged on every
//! path, and giving the same value away twice is an error even when one of
//! the two is deferred.

use std::path::Path;

use salvo_core::{check_program, resolve, Program, SourceSet, Symbols};

/// [intrinsic-std-only] The intrinsic declarations these sources rely on.
/// `intrinsic` is the compiler's modifier and only std may write it, so this
/// is loaded as a *std* file rather than pasted into the source under test.
/// Module `core.prelude`: `core.*` is implicitly imported, so the test source
/// sees these names without an `import`.
const STD_PRELUDE: &str = "intrinsic type Int\nintrinsic type Str\nintrinsic type Bool\nintrinsic fn discard<T canbe Linear>(value: T) [] -> [] None\n";

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
        .map(|d| d.message.clone())
        .collect()
}

const PRELUDE: &str = r#"

struct Handle canbe Linear {
    fd: Int
}

struct Label {
    text: Str? = None
}

fn open_handle(fd: Int) [] -> [] Handle {
    return Handle { fd: fd }
}
fn close_handle(h: Handle) [] -> [] None {
    discard(h)
}
fn note(text: Str) [] -> [text] None {}
fn take(text: Str) [] -> [] None {}
fn shout(text: Str) [] -> [text] Str {
    return ""
}
"#;

/// Wraps a body in a `[]`-effect fn returning `None`.
fn check(body: &str) -> Vec<String> {
    errors(&format!(
        "{PRELUDE}\nfn probe(flag: Bool) [] -> [] None {{\n{body}\n}}\n"
    ))
}

// ===== linear obligations [linear-obligation] =====

/// [defer] The motivating case: a linear value live across the body is
/// discharged by a deferred call, so *no* path leaks it — including the
/// early `return`.
#[test]
fn defer_discharges_a_linear_obligation() {
    let errs = check(
        r#"
    let h = open_handle(3)
    defer { close_handle(h) }
    if flag {
        return
    }
    note("still open here")
"#,
    );
    assert!(errs.is_empty(), "unexpected errors: {errs:?}");
}

/// The control for the test above: without the `defer` the same body
/// leaks the handle, which is what makes the acceptance meaningful.
#[test]
fn without_defer_the_obligation_is_reported() {
    let errs = check(
        r#"
    let h = open_handle(3)
    if flag {
        return
    }
    note("still open here")
"#,
    );
    assert!(
        errs.iter().any(|e| e.contains("still owns a linear value")),
        "expected a linear-obligation error, got: {errs:?}"
    );
}

/// [defer] A deferred block registered *inside* a loop body discharges the
/// value that body created — on the `continue` path too.
#[test]
fn defer_in_a_loop_body_discharges_per_iteration() {
    let errs = check(
        r#"
    while flag {
        let h = open_handle(1)
        defer { close_handle(h) }
        if flag {
            continue
        }
        note("used")
    }
"#,
    );
    assert!(errs.is_empty(), "unexpected errors: {errs:?}");
}

// ===== giving the same value away twice =====

/// [defer] The deferred call runs at the exit, so a manual call before it
/// consumed the value first: the deferred one would use a value that is
/// gone.
#[test]
fn deferring_a_second_consume_is_an_error() {
    let errs = check(
        r#"
    let h = open_handle(3)
    defer { close_handle(h) }
    close_handle(h)
"#,
    );
    assert!(
        errs.iter()
            .any(|e| e.contains("no longer holds a value at this exit")),
        "expected a double-consume error, got: {errs:?}"
    );
}

/// [defer] The same, one diagnostic only: the block is applied at the
/// `return` *and* at the end of the block, and both find the same fact
/// broken.
#[test]
fn a_broken_fact_is_reported_once_per_defer() {
    let errs: Vec<String> = check(
        r#"
    let h = open_handle(3)
    defer { close_handle(h) }
    close_handle(h)
    return
"#,
    )
    .into_iter()
    .filter(|e| e.contains("no longer holds a value at this exit"))
    .collect();
    assert_eq!(errs.len(), 1, "expected exactly one diagnostic: {errs:?}");
}

/// [defer] A value moved *into* the deferred block is consumed at the
/// exit, so using it afterwards in the same block is fine (the code after
/// the `defer` runs first) — but consuming it twice in the body is not.
#[test]
fn a_kept_use_after_a_deferred_consume_is_fine() {
    let errs = check(
        r#"
    let h = open_handle(3)
    defer { close_handle(h) }
    note("fd is ${h.fd}")
"#,
    );
    assert!(errs.is_empty(), "unexpected errors: {errs:?}");
}

// ===== narrowing the body relied on [defer] =====

/// [defer] [flow-place] A deferred block checked where a place is narrowed
/// is rejected if the fact does not survive to the exit: the lowering
/// recorded for it (a Kotlin `!!`, a Rust unwrap) would be wrong there
/// [backend-never-wrong].
#[test]
fn a_narrowing_the_body_relied_on_must_survive() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn label() [] -> [] Label {{ return Label {{}} }}\n\
         fn touch(l: Mut Label) [] -> [l: Mut] None {{}}\n\
         fn probe(l: Mut Label) [] -> [l: Mut] None {{\n\
         if l.text is Str {{\n\
         defer {{ take(l.text) }}\n\
         touch(l)\n\
         }}\n\
         }}\n"
    ));
    assert!(
        errs.iter().any(|e| e.contains("no longer holds at this exit")),
        "expected a stale-narrowing error, got: {errs:?}"
    );
}

/// The control: with nothing invalidating the fact, the same deferred
/// block is accepted.
#[test]
fn a_surviving_narrowing_is_accepted() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn label() [] -> [] Label {{ return Label {{}} }}\n\
         fn probe(l: Label) [] -> [l] None {{\n\
         if l.text is Str {{\n\
         defer {{ take(l.text) }}\n\
         note(\"fine\")\n\
         }}\n\
         }}\n"
    ));
    assert!(errs.is_empty(), "unexpected errors: {errs:?}");
}

// ===== control flow out of a deferred block [defer-no-escape] =====

/// [defer-no-escape] The body runs on the way out of its block: there is
/// no path for it to `return` through.
#[test]
fn return_in_a_deferred_block_is_rejected() {
    let errs = check(
        r#"
    defer { return }
    note("body")
"#,
    );
    assert!(
        errs.iter()
            .any(|e| e.contains("`return` is not allowed in a deferred block")),
        "expected a `return` rejection, got: {errs:?}"
    );
}

/// [defer-no-escape] `break`/`continue` in a deferred block do not bind an
/// enclosing loop.
#[test]
fn break_and_continue_in_a_deferred_block_are_rejected() {
    let errs = check(
        r#"
    while flag {
        defer { break }
        note("body")
    }
"#,
    );
    assert!(
        errs.iter()
            .any(|e| e.contains("`break` is not allowed in a deferred block")),
        "expected a `break` rejection, got: {errs:?}"
    );
    let errs = check(
        r#"
    while flag {
        defer { continue }
        note("body")
    }
"#,
    );
    assert!(
        errs.iter()
            .any(|e| e.contains("`continue` is not allowed in a deferred block")),
        "expected a `continue` rejection, got: {errs:?}"
    );
}

/// [defer-no-escape] A loop *written inside* the body owns its own
/// `break`/`continue`, and a lambda owns its own `return`.
#[test]
fn control_flow_inside_the_body_is_fine() {
    let errs = check(
        r#"
    defer {
        while flag {
            if flag {
                continue
            }
            break
        }
    }
    note("body")
"#,
    );
    assert!(errs.is_empty(), "unexpected errors: {errs:?}");
}

// ===== effects and scoping =====

/// [defer] [linear-obligation] The body is an ordinary block with its own
/// scope: a linear value it creates must be discharged inside it.
#[test]
fn the_body_owns_what_it_creates() {
    let errs = check(
        r#"
    defer {
        let inner = open_handle(1)
        note("leaked")
    }
    note("body")
"#,
    );
    assert!(
        errs.iter().any(|e| e.contains("still owns a linear value")),
        "expected the body's own obligation to be reported, got: {errs:?}"
    );
}
