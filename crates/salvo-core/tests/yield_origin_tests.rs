//! [yield-fn-origin] The `yield fn` origin-struct sugar (roadmap R3, user
//! direction 2026-09-08): the second way to discharge a `: Yield<self, T>`
//! obligation.
//!
//! ```
//! struct Counter : Yield<self, Int> { start: Int }
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
     intrinsic type List<T> canbe Mut\n\
     intrinsic fn copy<T>(value: T) [] -> [value] T\n\
     intrinsic fn mutable_list<T>(...elems: T[]) [] -> [] Mut List<T>\n\
     intrinsic fn add<T>(list: Mut List<T>, elem: T) [] -> [list: Mut] None\n\
     intrinsic fn size<T>(list: List<T>) [] -> [list] Int\n\
     qualifier Emitted<T> of T\n\
     struct Finished {}\n\
     fn emitted<T>(value: T) [] -> [] T as Emitted {\n    return value\n}\n\
     fn finished() [] -> [] Finished {\n    return Finished {}\n}\n\
     params Yield<It, T> {\n    fn next(it: Mut It) -> [it: Mut] Emitted T | Finished\n}\n\
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
struct Counter : Yield<self, Int> {
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
struct Chatty : Yield<self, Int> {
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

// --- the origin may not be mutated while it is driven [yield-fn-origin] ------

/// The origin does not have to be *immutable* — it must be **stable for the
/// duration of a drive**. A hidden machine reads its origin across
/// suspensions, so a write while the loop runs has two defensible meanings
/// (the machine's own copy, or the caller's object) and the two backends each
/// pick one. Refused rather than sided with [backend-parity].
///
/// Reported through the mutation choke point, so a write through a
/// *projection* counts too: everything reachable from the origin is what the
/// machine may read.
const MUT_ORIGIN: &str = r#"
struct B : Yield<self, Int> {
    rows: Mut List<Int>
}

yield fn next(b: B) -> Int {
    let before = size(b.rows)
    yield copy(before)
    let after = size(b.rows)
    yield copy(after)
}

fn make_b() -> B {
    return B { rows: mutable_list(1, 2) }
}

// A `Mut`-capable origin, for the whole-value mutation case.
struct C : Yield<self, Int> canbe Mut {
    n: Int
}

yield fn next(c: C) -> Int {
    yield copy(c.n)
    yield copy(c.n)
}

fn make_c() -> Mut C {
    return Mut C { n: 1 }
}
"#;

#[test]
fn mutating_a_driven_origin_is_refused() {
    let errs = errors(&format!(
        "{MUT_ORIGIN}\n\
         fn go() -> [] None {{\n\
         let b = make_b()\n\
         for n in b {{\n\
         add(b.rows, 9)\n\
         }}\n\
         }}\n"
    ));
    assert!(
        errs.iter().any(|e| e.contains("`b` is being iterated")
            && e.contains("cannot be mutated here")),
        "got {errs:?}"
    );
}

/// A mutable-origin *type* is fine — this is the case that must keep working,
/// since an origin is ordinary data the caller holds. Mutating it before and
/// after the drive is unremarkable.
#[test]
fn a_transitively_mutable_origin_is_allowed() {
    let errs = errors(&format!(
        "{MUT_ORIGIN}\n\
         fn go() -> [] None {{\n\
         let b = make_b()\n\
         add(b.rows, 3)\n\
         for n in b {{}}\n\
         add(b.rows, 4)\n\
         for n in b {{}}\n\
         }}\n"
    ));
    assert!(errs.is_empty(), "expected no errors, got {errs:?}");
}

/// The refusal is scoped to the loop, not to the function: an origin driven by
/// one loop may be mutated inside a *different* one.
#[test]
fn the_refusal_ends_with_the_loop() {
    let errs = errors(&format!(
        "{MUT_ORIGIN}\n\
         fn go() -> [] None {{\n\
         let b = make_b()\n\
         let c = make_b()\n\
         for n in c {{\n\
         add(b.rows, 9)\n\
         }}\n\
         }}\n"
    ));
    assert!(errs.is_empty(), "expected no errors, got {errs:?}");
}

/// Passing the origin to a mutating function is the same event, so it is
/// caught by the same rule.
#[test]
fn passing_a_driven_origin_to_a_mutator_is_refused() {
    let errs = errors(&format!(
        "{MUT_ORIGIN}\n\
         fn bump(x: Mut C) -> [x: Mut] None {{\n\
         x.n = x.n + 1\n\
         }}\n\
         fn go() -> [] None {{\n\
         let c = make_c()\n\
         for v in c {{\n\
         bump(c)\n\
         }}\n\
         }}\n"
    ));
    assert!(
        errs.iter().any(|e| e.contains("`c` is being iterated")),
        "got {errs:?}"
    );
}

// --- declaration rules ------------------------------------------------------

/// It is the sugared *member* of an obligation, so it answers to the member's
/// name. A free-standing `yield fn` would reintroduce an anonymous generator
/// type, which is exactly what the origin model removes.
#[test]
fn a_yield_fn_must_be_called_next() {
    let errs = errors(
        "struct Counter : Yield<self, Int> {\n    start: Int\n}\n\
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
        "struct Counter : Yield<self, Int> {\n    start: Int\n}\n\
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
        "struct Counter : Yield<self, Int> canbe Mut {\n    start: Int\n}\n\
         yield fn next(c: Mut Counter) -> Int {\n    yield 1\n}\n",
    );
    assert!(
        errs.iter()
            .any(|e| e.contains("read, not advanced") && e.contains("drop the `Mut`")),
        "got {errs:?}"
    );
}

/// The origin has to carry the clause: the sugar is how a `: Yield<self, T>` type
/// satisfies its obligation, not a way to make any struct iterable.
#[test]
fn an_origin_without_the_clause_is_refused() {
    let errs = errors(
        "struct Plain {\n    n: Int\n}\n\
         yield fn next(p: Plain) -> Int {\n    yield 1\n}\n",
    );
    assert!(
        errs.iter()
            .any(|e| e.contains("is not a pass") && e.contains(": Yield<self, T>")),
        "got {errs:?}"
    );
}

/// The return type *is* the element type, so it has to be the one the clause
/// declares — reported once, at the return type, which is where the remedy is.
#[test]
fn the_return_type_must_match_the_clause() {
    let errs = errors(
        "struct Counter : Yield<self, Int> {\n    start: Int\n}\n\
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
        "struct Counter : Yield<self, Int> {\n    start: Int\n}\n\
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
        "struct Both : Yield<self, Int> canbe Mut {\n    at: Int\n}\n\
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

/// [yield-fn-origin] The **mint** (user decision 2026-09-09): anything whose
/// declaration says `: Yield<self, T>` can be passed as is. A generic pass
/// position — `it: Mut It` with a `?Yield<It, T>` spread — accepts an
/// *origin*, because the argument is rewritten to a fresh instance of its
/// hidden pass; the author never names the machine.
#[test]
fn an_origin_fits_a_generic_pass_position() {
    let errs = errors(&format!(
        "{COUNTER}\n\
         fn drain<It, T>(it: Mut It, ?Yield<It, T>) -> [it: Mut] Int {{\n    \
         let n = 0\n    let going = true\n    while going {{\n        \
         let step = next(it)\n        when step {{\n            \
         is Emitted {{\n                n = n + 1\n            }}\n            \
         is Finished {{\n                going = false\n            }}\n        }}\n    }}\n    \
         return n\n}}\n\
         fn go() -> [] Int {{\n    return drain(Counter {{ start: 2 }})\n}}\n"
    ));
    assert!(errs.is_empty(), "expected no errors, got {errs:?}");
}

/// The other half: a value whose type declares nothing is still refused, so
/// the mint is not a hole in selection.
#[test]
fn a_plain_value_still_does_not_fit_a_pass_position() {
    let errs = errors(&format!(
        "{COUNTER}\n\
         struct Plain {{\n    n: Int\n}}\n\
         fn drain<It, T>(it: Mut It, ?Yield<It, T>) -> [it: Mut] Int {{\n    return 0\n}}\n\
         fn go() -> [] Int {{\n    return drain(Plain {{ n: 1 }})\n}}\n"
    ));
    assert!(
        errs.iter().any(|e| e.contains("no matching overload")
            || e.contains("no `next`")),
        "got {errs:?}"
    );
}
