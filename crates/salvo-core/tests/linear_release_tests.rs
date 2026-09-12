//! Releasing a linear value **on every path** [linear-obligation].
//!
//! This file was `defer_tests.rs` until 2026-09-10, when `defer` was removed
//! from the language (user decision): a deferred block discharged an
//! obligation on every exit, but linearity is what *checks* that the
//! obligation is discharged, so `defer` was the partial solution to a problem
//! the full one already covers. What is left to verify is that the checker
//! demands the release on each path and accepts it when written — including
//! the paths an author is most likely to forget: an early `return`, a `break`,
//! and a call that may throw.

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


struct Label {
    text: Str? = None
}

fn open_handle(fd: Int) [] -> Handle {
    return Handle { fd: fd }
}
fn close_handle(h: Handle) [] -> None => !h {
    close(h)
}
fn note(text: Str) [] -> None => text {}
fn take(text: Str) [] -> None => !text {}
fn shout(text: Str) [] -> Str => text {
    return ""
}
"#;

/// Wraps a body in a `[]`-effect fn returning `None`.
fn check(body: &str) -> Vec<String> {
    errors(&format!(
        "{PRELUDE}\nfn probe(flag: Bool) [] -> None {{\n{body}\n}}\n"
    ))
}

// ===== linear obligations [linear-obligation] =====

/// The motivating case, and the reason `defer` is not needed: a path that
/// leaves without releasing is an error naming the value.
#[test]
fn a_path_that_leaves_without_releasing_is_reported() {
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

/// Written out on both paths, it is accepted — which is what "linearity is the
/// full solution" means in practice: the compiler names the path you missed.
#[test]
fn releasing_on_every_path_is_accepted() {
    let errs = check(
        r#"
    let h = open_handle(3)
    if flag {
        close(h)
        return
    }
    note("still open here")
    close(h)
"#,
    );
    assert!(errs.is_empty(), "unexpected errors: {errs:?}");
}

/// A `break` is an exit like any other, and the value the *body* created dies
/// with the iteration.
#[test]
fn a_break_must_release_what_the_iteration_owns() {
    let leaks = check(
        r#"
    while flag {
        let h = open_handle(1)
        if flag {
            break
        }
        close(h)
    }
"#,
    );
    assert!(
        leaks.iter().any(|e| e.contains("still owns a linear value")),
        "expected the break path to be reported, got: {leaks:?}"
    );
    let ok = check(
        r#"
    while flag {
        let h = open_handle(1)
        if flag {
            close(h)
            break
        }
        close(h)
    }
"#,
    );
    assert!(ok.is_empty(), "unexpected errors: {ok:?}");
}

/// The path `defer` was most useful for: a call that may throw. The code after
/// it does not run on the throw path, so the release has to precede the call —
/// and the diagnostic says so rather than naming a construct that no longer
/// exists.
#[test]
fn a_call_that_may_throw_must_release_first() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn risky(flag: Bool) [Throw<Str>] -> None {{\n\
         if flag {{ return throw(\"no\") }}\n\
         }}\n\
         fn probe2(flag: Bool) [Throw<Str>] -> None {{\n\
         let h = open_handle(3)\n\
         risky(flag)\n\
         close(h)\n\
         }}\n"
    ));
    assert!(
        errs.iter().any(|e| e.contains("across a call that may throw")
            && e.contains("release it before the call")),
        "expected the throw-path diagnostic naming the remedy, got: {errs:?}"
    );
}
