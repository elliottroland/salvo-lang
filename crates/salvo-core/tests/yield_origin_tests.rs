//! [yield-fn-origin] The `yield fn` origin-struct sugar (roadmap R3, user
//! direction 2026-09-08): the second way to discharge a `: Yield<T>`
//! obligation.
//!
//! ```
//! struct Counter : Yield<Int> { start: Int }
//!
//! yield fn next(c: Counter) -> Int { … }
//! ```
//!
//! The subject is the **origin** — the starting data — and the state machine
//! the compiler builds from the body is *hidden*: not nameable, not
//! constructible, not readable. Whoever wants the state struct writes the raw
//! `next` instead. That is what makes the sugar free of the
//! partially-declared-type problem: `Counter` is wholly the author's, the
//! machine wholly the compiler's.

use std::path::Path;

use salvo_core::{check_program, resolve, Program, SourceSet, Symbols};

const STD_PRELUDE: &str =
    "intrinsic type Int\nintrinsic type Str\nintrinsic type Bool\n\
     intrinsic fn copy<T>(value: T) [] -> [value] T\n\
     qualifier Emitted<T> of T\n\
     struct Finished {}\n\
     fn emitted<T>(value: T) [] -> [] T as Emitted {\n    return value\n}\n\
     fn finished() [] -> [] Finished {\n    return Finished {}\n}\n\
     params Yield<T> {\n    fn next(s: Mut Self) -> [s: Mut] Emitted T | Finished\n}\n\
     effect Console {\n    fn print(message: Str) -> [] None\n}\n\
     handler StdOutConsole of Console {\n    fn print(message: Str) -> [] None {}\n}\n\
     fn println(message: Str) [Console] -> [] None {}\n";

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

/// The user's own example, with the `copy` the standing [deduce-consume] rule
/// wants (`yield num` *moves* `num`, so the decrement after it would read a
/// consumed variable — the same idiom the `Iter<T>` form has always used).
const COUNTER: &str = r#"
struct Counter : Yield<Int> {
    start: Int
}

yield fn next(c: Counter) -> Int {
    let num = copy(c.start)
    while num >= 0 {
        yield copy(num)
        num = num - 1
    }
}

fn counter(start: Int) -> Counter {
    return Counter { start: start }
}
"#;

// --- the sugar discharges the obligation ------------------------------------

#[test]
fn a_yield_fn_satisfies_the_obligation() {
    let errs = errors(&format!(
        "{COUNTER}\nfn go() -> [] None {{ for n in counter(3) {{}} }}\n"
    ));
    assert!(errs.is_empty(), "expected no errors, got {errs:?}");
}

/// The whole point of the origin model: driving does **not** consume the
/// origin. Each `for` mints a fresh hidden machine from it, so a second loop
/// over the same value starts over — where a real pass (the value that *holds*
/// the position) would be moved into the loop.
#[test]
fn driving_an_origin_twice_is_fine() {
    let errs = errors(&format!(
        "{COUNTER}\n\
         fn go() -> [] None {{\n\
         let c = counter(3)\n\
         for n in c {{}}\n\
         for n in c {{}}\n\
         }}\n"
    ));
    assert!(errs.is_empty(), "expected no errors, got {errs:?}");
}

/// A `yield fn` may declare effects normally — nothing calls it, so there is no
/// call site to burden — and they are performed *while driving*, so the `for`
/// is what needs the handler in scope.
#[test]
fn drive_site_effects_are_required_at_the_loop() {
    const CHATTY: &str = r#"
struct Chatty : Yield<Int> {
    limit: Int
}

yield fn next(c: Chatty) [Console] -> Int {
    println("open")
    yield copy(c.limit)
}

fn chatty(limit: Int) -> Chatty {
    return Chatty { limit: limit }
}
"#;
    let with_handler = errors(&format!(
        "{CHATTY}\nfn go() [Console] -> [] None {{ for n in chatty(1) {{}} }}\n"
    ));
    assert!(
        with_handler.is_empty(),
        "expected no errors with the effect declared, got {with_handler:?}"
    );
    let without = errors(&format!(
        "{CHATTY}\nfn go() -> [] None {{ for n in chatty(1) {{}} }}\n"
    ));
    assert!(
        without.iter().any(|e| e.contains("Console")),
        "expected the drive site to require the handler, got {without:?}"
    );
}

// --- declaration rules ------------------------------------------------------

/// It is the sugared *member* of an obligation, so it answers to the member's
/// name. A free-standing `yield fn` would reintroduce an anonymous generator
/// type, which is exactly what the origin model removes.
#[test]
fn a_yield_fn_must_be_called_next() {
    let errs = errors(
        "struct Counter : Yield<Int> {\n    start: Int\n}\n\
         yield fn generate(c: Counter) -> Int {\n    yield 1\n}\n",
    );
    assert!(
        errs.iter().any(|e| e.contains("must be called `next`")),
        "got {errs:?}"
    );
}

#[test]
fn a_yield_fn_takes_exactly_one_origin() {
    let errs = errors(
        "struct Counter : Yield<Int> {\n    start: Int\n}\n\
         yield fn next(c: Counter, extra: Int) -> Int {\n    yield 1\n}\n",
    );
    assert!(
        errs.iter().any(|e| e.contains("exactly one parameter")),
        "got {errs:?}"
    );
}

/// The origin is read, not advanced: the hidden machine holds the position.
#[test]
fn a_mut_origin_is_refused() {
    let errs = errors(
        "struct Counter : Yield<Int> canbe Mut {\n    start: Int\n}\n\
         yield fn next(c: Mut Counter) -> Int {\n    yield 1\n}\n",
    );
    assert!(
        errs.iter()
            .any(|e| e.contains("read, not advanced") && e.contains("drop the `Mut`")),
        "got {errs:?}"
    );
}

/// The origin has to carry the clause: the sugar is how a `: Yield<T>` type
/// satisfies its obligation, not a way to make any struct iterable.
#[test]
fn an_origin_without_the_clause_is_refused() {
    let errs = errors(
        "struct Plain {\n    n: Int\n}\n\
         yield fn next(p: Plain) -> Int {\n    yield 1\n}\n",
    );
    assert!(
        errs.iter()
            .any(|e| e.contains("is not a pass") && e.contains(": Yield<T>")),
        "got {errs:?}"
    );
}

/// The return type *is* the element type, so it has to be the one the clause
/// declares — reported once, at the return type, which is where the remedy is.
#[test]
fn the_return_type_must_match_the_clause() {
    let errs = errors(
        "struct Counter : Yield<Int> {\n    start: Int\n}\n\
         yield fn next(c: Counter) -> Str {\n    yield \"x\"\n}\n",
    );
    assert_eq!(errs.len(), 1, "got {errs:?}");
    assert!(
        errs[0].contains("has to return `Int`") && errs[0].contains("found `Str`"),
        "got {errs:?}"
    );
}

#[test]
fn a_yield_fn_without_a_yield_is_refused() {
    let errs = errors(
        "struct Counter : Yield<Int> {\n    start: Int\n}\n\
         yield fn next(c: Counter) -> Int {\n    return 1\n}\n",
    );
    assert!(
        errs.iter().any(|e| e.contains("has to `yield`")),
        "got {errs:?}"
    );
}

/// One type, one machine: the sugar *generates* the state struct a
/// hand-written `next` would be, so declaring both leaves `for` with two
/// machines to choose between.
#[test]
fn both_forms_at_once_are_refused() {
    let errs = errors(
        "struct Both : Yield<Int> canbe Mut {\n    at: Int\n}\n\
         yield fn next(b: Both) -> Int {\n    yield 1\n}\n\
         fn next(b: Mut Both) -> [b: Mut] Emitted Int | Finished {\n    return finished()\n}\n",
    );
    assert!(
        errs.iter()
            .any(|e| e.contains("both a `yield fn next` and a plain `next`")),
        "got {errs:?}"
    );
}

// --- it is not a function ---------------------------------------------------

/// [group-not-a-value]'s neighbour: the machine is unnameable, so there is
/// nothing a call could hand back. Iterating the origin is the only way in —
/// which is also what makes abandonment impossible for a generated pass, since
/// only the `for` sugar can hold one and the sugar always closes.
#[test]
fn a_yield_fn_cannot_be_called() {
    let errs = errors(&format!(
        "{COUNTER}\n\
         fn go() -> [] None {{\n\
         let c = counter(3)\n\
         let step = next(c)\n\
         }}\n"
    ));
    assert!(
        errs.iter()
            .any(|e| e.contains("is a `yield fn`, so it cannot be called")
                && e.contains("for x in <Counter value>")),
        "got {errs:?}"
    );
}
