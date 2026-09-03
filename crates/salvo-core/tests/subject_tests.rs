//! Qualifier subjects [qual-subject]: state claims describe contents and a
//! mutating call may strip them; provenance claims describe where the
//! handle came from, survive every call, compose without `with`, and are
//! mint-only.

use std::path::Path;

use salvo_core::{check_program, resolve, Program, SourceSet, Symbols};

/// Parses + resolves + checks one file (no std) and returns the checker's
/// and resolver's error messages.
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
internal type Int
internal type Str
external type Store<T> canbe Mut

qualifier NonEmpty<T> of Store<T>
provenance qualifier Authenticated<T> of Store<T>

fn nonempty<T>(s: Store<T>) -> Store<T> as NonEmpty {
    return s
}

fn authenticate<T>(s: Store<T>) -> Store<T> as Authenticated {
    return s
}

external fn touch<T>(s: Mut Store<T>) [] -> [s: Mut] None

fn needs_nonempty<T>(s: NonEmpty Store<T>) -> None {
}

fn needs_auth<T>(s: Authenticated Store<T>) -> None {
}
"#;

// [qual-subject] [deduce-syntax] A mutating call strips a *state* claim:
// the stripping rule exists because mutation can invalidate a claim about
// contents.
#[test]
fn a_mutating_call_strips_a_state_qualifier() {
    let src = format!(
        "{PRELUDE}\nfn f(s: Mut NonEmpty Store<Int>) -> None {{\n    \
         touch(s)\n    needs_nonempty(s)\n}}\n"
    );
    let errors = errors(&src);
    assert!(
        errors.iter().any(|m| m.contains("no matching overload for `needs_nonempty")),
        "got {errors:?}"
    );
}

// [qual-subject] Provenance survives it: mutating the contents cannot
// change where the handle came from. This is the payoff of the subject
// axis — without it, D3 refinements would be the only escape.
#[test]
fn provenance_survives_a_mutating_call() {
    let src = format!(
        "{PRELUDE}\nfn f(s: Mut Authenticated Store<Int>) -> None {{\n    \
         touch(s)\n    needs_auth(s)\n}}\n"
    );
    assert!(errors(&src).is_empty(), "got {:?}", errors(&src));
}

// [qual-subject] [qual-with] Provenance composes without a `with`
// declaration — with state claims and with other provenance claims —
// while two state claims still need one.
#[test]
fn provenance_composes_without_with() {
    let ok = "internal type Str\n\
               struct Request { body: Str }\n\
              qualifier Validated of Request\n\
              provenance qualifier Authenticated of Request\n\
              provenance qualifier FromCache of Request\n\
              fn a(r: Authenticated Validated Request) -> Str { return r.body }\n\
              fn b(r: Authenticated FromCache Request) -> Str { return r.body }\n";
    assert!(errors(ok).is_empty(), "got {:?}", errors(ok));

    let bad = "internal type Str\n\
               struct Request { body: Str }\n\
               qualifier Validated of Request\n\
               qualifier Checked of Request\n\
               fn a(r: Validated Checked Request) -> Str { return r.body }\n";
    assert!(
        errors(bad).iter().any(|m| m.contains("are not compatible")),
        "got {:?}",
        errors(bad)
    );
}

// [qual-subject] Provenance is mint-only: nothing in the bits establishes
// where a handle came from, so a body (a `qualifies` predicate or field
// overrides, which are claims about contents) is a contradiction.
#[test]
fn provenance_cannot_have_a_body() {
    let src = "provenance qualifier Positive of Int {\n    \
               fn qualifies(v: Int) -> Bool { return v > 0 }\n}\n";
    assert!(
        errors(src)
            .iter()
            .any(|m| m.contains("cannot have a body")),
        "got {:?}",
        errors(src)
    );
}

// [qual-subject] [qual-constructive] `is` on a non-union provenance value
// is an error — it falls out of provenance being constructive, and is the
// same message constructive qualifiers already give.
#[test]
fn provenance_cannot_be_tested_with_is() {
    let src = "internal type Str\n\
               struct Request { body: Str }\n\
               provenance qualifier Authenticated of Request\n\
               fn f(r: Request) -> Bool {\n    return r is Authenticated\n}\n";
    assert!(
        errors(src)
            .iter()
            .any(|m| m.contains("constructive qualifier")),
        "got {:?}",
        errors(src)
    );
}

// [qual-subject] Provenance is droppable (forgetting an origin is safe)
// and survives being stored into another value.
#[test]
fn provenance_is_droppable_and_survives_storage() {
    let src = "internal type Str\n\
               struct Request { body: Str }\n\
               struct Wrapper { req: Authenticated Request }\n\
               provenance qualifier Authenticated of Request\n\
               fn authenticate(r: Request) -> Request as Authenticated { return r }\n\
               fn plain(r: Request) -> Str { return r.body }\n\
               fn f(r: Request) -> Str {\n    \
               let a = authenticate(r)\n    \
               let w = Wrapper {req: a}\n    \
               return plain(w.req)\n}\n";
    assert!(errors(src).is_empty(), "got {:?}", errors(src));
}
