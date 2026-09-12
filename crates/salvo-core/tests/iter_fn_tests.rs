//! [iter-fn] The `iter fn` form (user decision 2026-09-09): a hand-written
//! `next` whose **pass struct is generated**.
//!
//! ```
//! struct Countdown { from: Int }
//!
//! iter fn next(c: Countdown) -> Emitted Int | Finished {
//!     state {
//!         at: Int = c.from
//!     }
//!     if at <= 0 { return finished() }
//!     at = at - 1
//!     return emitted(at + 1)
//! }
//! ```
//!
//! The third way to be iterable, and the one with the least to declare: the
//! subject stays ordinary data, the `state` block is the pass's own fields, and
//! the compiler writes the struct plus the `iter` that mints it. Unlike a
//! `yield fn` there is no state machine — the author's body *is* the `next` —
//! so what this file checks is mostly that the generated declarations behave
//! like the hand-written ones they stand for.
//!
//! The expansion happens in `salvo_syntax::desugar`, before resolution, so
//! every test here goes through the ordinary checker.

use std::path::Path;

use salvo_core::{check_program, resolve, Program, SourceSet, Symbols};

const STD_PRELUDE: &str =
    "intrinsic type Int\nintrinsic type Str\nintrinsic type Bool\n\
     intrinsic type List<T> canbe Mut\n\
     intrinsic fn copy<T>(value: T) [] -> T => value\n\
     intrinsic fn mutable_list<T>(...elems: T[]) [] -> Mut List<T>\n\
     intrinsic fn list<T>(...elems: T[]) [] -> List<T>\n\
     intrinsic fn add<T>(list: Mut List<T>, elem: T) [] -> None => list: Mut, !elem\n\
     intrinsic fn get<T>(list: List<T>, index: Int) [] -> T? => list, index\n\
     intrinsic fn size<T>(list: List<T>) [] -> Int => list\n\
     qualifier Emitted<T> of T\n\
     struct Finished {}\n\
     fn emitted<T>(value: T) [] -> T as Emitted => !value {\n    return value\n}\n\
     fn finished() [] -> Finished {\n    return Finished {}\n}\n\
     params Yield<It, T> {\n    fn next(it: Mut It) -> Emitted T | Finished => it: Mut\n}\n\
     effect Console {\n    fn print(message: Str) -> None => !message\n}\n\
     handler StdOutConsole of Console {\n    fn print(message: Str) -> None => !message {}\n}\n\
     fn println(message: Str) [Console] -> None => !message {}\n";

/// Every diagnostic — parse, resolve and check — because an `iter fn`'s own
/// rules are reported by the desugaring, which runs inside the parser.
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
    let mut parse_errors = Vec::new();
    for file in &sources.files {
        let (ast, diagnostics) = salvo_syntax::parse_module(&file.content);
        parse_errors.extend(
            diagnostics
                .iter()
                .filter(|d| d.is_error())
                .map(|d| d.message.clone()),
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
    parse_errors
        .into_iter()
        .chain(resolution.errors.iter().map(|d| d.message.clone()))
        // Errors only: an unused-variable *warning* [unused-var] is a
        // different severity, and these tests are about legality.
        .chain(
            checked
                .errors
                .iter()
                .filter(|d| d.is_error())
                .map(|d| d.message.clone()),
        )
        .collect()
}

const COUNTDOWN: &str = r#"
struct Countdown {
    from: Int
}

iter fn next(c: Countdown) -> Emitted Int | Finished {
    state {
        at: Int = c.from
    }
    if at <= 0 {
        return finished()
    }
    at = at - 1
    return emitted(at + 1)
}
"#;

// --- what the form buys -----------------------------------------------------

/// One declaration makes the subject iterable: no pass struct, no constructor.
#[test]
fn an_iter_fn_makes_its_subject_iterable() {
    let errs = errors(&format!(
        "{COUNTDOWN}\nfn go() -> None {{\n    let c = Countdown {{ from: 3 }}\n    \
         for n in c {{}}\n}}\n"
    ));
    assert!(errs.is_empty(), "expected no errors, got {errs:?}");
}

/// Driving does not consume the subject: the pass holds a *copy*, so a second
/// `for` starts over. This is the property that separates a subject from a
/// pass [iter-pass].
#[test]
fn driving_leaves_the_subject_usable() {
    let errs = errors(&format!(
        "{COUNTDOWN}\nfn go() -> None {{\n    let c = Countdown {{ from: 3 }}\n    \
         for n in c {{}}\n    for n in c {{}}\n}}\n"
    ));
    assert!(errs.is_empty(), "expected no errors, got {errs:?}");
}

/// The generated `iter` is an ordinary function, so a pass can be *held* — the
/// capability a `yield fn` origin does not have, since each `for` re-mints.
#[test]
fn the_generated_iter_hands_back_a_pass_you_can_hold() {
    let errs = errors(&format!(
        "{COUNTDOWN}\nfn go() -> None {{\n    let c = Countdown {{ from: 3 }}\n    \
         let p = iter(c)\n    let step = next(p)\n}}\n"
    ));
    assert!(errs.is_empty(), "expected no errors, got {errs:?}");
}

/// A `state` field is an ordinary struct field of the pass, so the ordinary
/// type rules apply to it.
#[test]
fn a_state_field_is_type_checked() {
    let errs = errors(
        "struct S { n: Int }\n\
         iter fn next(s: S) -> Emitted Int | Finished {\n    \
         state {\n        at: Int = \"not a number\"\n    }\n    \
         return finished()\n}\n",
    );
    assert!(
        errs.iter().any(|e| e.contains("expects `Int`, found `Str`")),
        "{errs:?}"
    );
}

/// The subject is **read-only** (user decision 2026-09-09): it is a field of
/// the pass typed as the subject, so writing through it is the standing
/// [struct-mut] refusal rather than a rule of its own.
#[test]
fn the_subject_is_read_only() {
    let errs = errors(
        "struct S canbe Mut { n: Int }\n\
         iter fn next(s: S) -> Emitted Int | Finished {\n    \
         state {\n        at: Int = 0\n    }\n    \
         s.n = 5\n    return finished()\n}\n",
    );
    assert!(
        errs.iter().any(|e| e.contains("cannot assign to field")),
        "{errs:?}"
    );
}

/// A `state` initializer runs when the pass is minted, and the mint is an
/// effect-free call — so an effectful initializer is refused where it is
/// written, with no rule of its own [iter-fn].
#[test]
fn a_state_initializer_may_not_perform_effects() {
    let errs = errors(
        "struct S { n: Int }\n\
         fn noisy() [Console] -> Int {\n    println(\"hi\")\n    return 1\n}\n\
         iter fn next(s: S) -> Emitted Int | Finished {\n    \
         state {\n        at: Int = noisy()\n    }\n    \
         return finished()\n}\n",
    );
    assert!(
        errs.iter().any(|e| e.contains("no handler for effect `Console`")),
        "{errs:?}"
    );
}

// --- the refusals -----------------------------------------------------------

/// The name is the obligation's member name: `for` reads a declaration.
#[test]
fn an_iter_fn_must_be_called_next() {
    let errs = errors(
        "struct S { n: Int }\n\
         iter fn step(s: S) -> Emitted Int | Finished {\n    return finished()\n}\n",
    );
    assert!(
        errs.iter().any(|e| e.contains("must be called `next`")),
        "{errs:?}"
    );
}

#[test]
fn an_iter_fn_takes_exactly_one_parameter() {
    let errs = errors(
        "struct S { n: Int }\n\
         iter fn next(s: S, k: Int) -> Emitted Int | Finished {\n    return finished()\n}\n",
    );
    assert!(
        errs.iter().any(|e| e.contains("exactly one parameter")),
        "{errs:?}"
    );
}

/// `Mut` on the subject would promise that advancing writes through it, which
/// is the opposite of what a mint does.
#[test]
fn a_mut_subject_is_refused() {
    let errs = errors(
        "struct S canbe Mut { n: Int }\n\
         iter fn next(s: Mut S) -> Emitted Int | Finished {\n    return finished()\n}\n",
    );
    assert!(
        errs.iter().any(|e| e.contains("read, not advanced")),
        "{errs:?}"
    );
}

/// The result shape is the obligation's own, and the element type is read out
/// of it — so anything else has no element type to declare.
#[test]
fn the_result_must_be_emitted_or_finished() {
    let errs = errors(
        "struct S { n: Int }\n\
         iter fn next(s: S) -> Int {\n    return 1\n}\n",
    );
    assert!(
        errs.iter()
            .any(|e| e.contains("returns `Emitted T | Finished`")),
        "{errs:?}"
    );
}

/// A binding of a field's name would silently mean something else, so it is
/// reported rather than renamed [iter-fn].
#[test]
fn a_binding_may_not_shadow_a_state_field() {
    let errs = errors(
        "struct S { n: Int }\n\
         iter fn next(s: S) -> Emitted Int | Finished {\n    \
         state {\n        at: Int = 0\n    }\n    \
         let at = 3\n    return finished()\n}\n",
    );
    assert!(
        errs.iter().any(|e| e.contains("would shadow it")),
        "{errs:?}"
    );
}

#[test]
fn a_binding_may_not_shadow_the_subject() {
    let errs = errors(
        "struct S { n: Int }\n\
         iter fn next(s: S) -> Emitted Int | Finished {\n    \
         state {\n        at: Int = 0\n    }\n    \
         for s in list(1, 2) {}\n    return finished()\n}\n",
    );
    assert!(
        errs.iter().any(|e| e.contains("would shadow it")),
        "{errs:?}"
    );
}

/// A `state` field named like the subject is the same collision, caught at the
/// declaration rather than at a use.
#[test]
fn a_state_field_may_not_be_named_like_the_subject() {
    let errs = errors(
        "struct S { n: Int }\n\
         iter fn next(s: S) -> Emitted Int | Finished {\n    \
         state {\n        s: Int = 0\n    }\n    \
         return finished()\n}\n",
    );
    assert!(
        errs.iter().any(|e| e.contains("subject's name")),
        "{errs:?}"
    );
}

/// The generated struct is named after the subject, so the subject needs a
/// name: an array, tuple or union has to be wrapped.
#[test]
fn a_structural_subject_is_refused() {
    let errs = errors(
        "iter fn next(items: Int[]) -> Emitted Int | Finished {\n    return finished()\n}\n",
    );
    assert!(
        errs.iter().any(|e| e.contains("must be a named type")),
        "{errs:?}"
    );
}

/// Every `state` field is a declaration with a type and an initializer — the
/// block is the pass's shape, not code that runs (user decision 2026-09-09).
#[test]
fn a_state_field_needs_an_initializer() {
    let errs = errors(
        "struct S { n: Int }\n\
         iter fn next(s: S) -> Emitted Int | Finished {\n    \
         state {\n        at: Int\n    }\n    \
         return finished()\n}\n",
    );
    assert!(
        errs.iter().any(|e| e.contains("needs an initializer")),
        "{errs:?}"
    );
}

#[test]
fn an_empty_state_block_is_refused() {
    let errs = errors(
        "struct S { n: Int }\n\
         iter fn next(s: S) -> Emitted Int | Finished {\n    \
         state {\n    }\n    \
         return finished()\n}\n",
    );
    assert!(
        errs.iter().any(|e| e.contains("declares nothing")),
        "{errs:?}"
    );
}

/// The generated pass is **not nameable**: `__Pass_S` is the compiler's, and a
/// written type reference may not reach it. (Without this, the first boundary
/// the design rests on — a pass you must name is written by hand — would leak.)
#[test]
fn the_generated_pass_type_cannot_be_written() {
    let errs = errors(&format!(
        "{COUNTDOWN}\nfn hold(p: __Pass_Countdown) -> None => !p {{}}\n"
    ));
    assert!(
        errs.iter()
            .any(|e| e.contains("compiler-generated name and cannot be written")),
        "{errs:?}"
    );
}

// --- how much of the subject the pass holds ---------------------------------

/// [iter-fn] Tier 2: reading a subject field on every turn is fine — the mint
/// snapshots that field. What matters here is that it still *checks*: the
/// snapshot carries the field's declared type.
#[test]
fn a_subject_field_read_per_turn_is_typed_by_its_declaration() {
    let errs = errors(
        "struct S { limit: Int }\n\
         iter fn next(s: S) -> Emitted Int | Finished {\n    \
         state {\n        at: Int = 0\n    }\n    \
         if at >= s.limit {\n        return finished()\n    }\n    \
         at = at + 1\n    return emitted(copy(at))\n}\n",
    );
    assert!(errs.is_empty(), "expected no errors, got {errs:?}");
}

/// A field the subject does not have is reported against the subject's own
/// type, not against the generated pass — which is why the whole subject is
/// kept when a read cannot be resolved to a declared field.
#[test]
fn an_unknown_subject_field_is_reported_against_the_subject() {
    let errs = errors(
        "struct S { limit: Int }\n\
         iter fn next(s: S) -> Emitted Int | Finished {\n    \
         state {\n        at: Int = 0\n    }\n    \
         if at >= s.limmit {\n        return finished()\n    }\n    \
         return finished()\n}\n",
    );
    assert!(
        errs.iter().any(|e| e.contains("limmit") && e.contains("S")),
        "{errs:?}"
    );
}

/// Tier 3: the body hands the subject on as a value, so no per-field snapshot
/// can stand in for it and the pass keeps a copy. Legality is what is asserted
/// here; which shape was emitted is each backend's test.
#[test]
fn the_whole_subject_may_be_handed_on() {
    let errs = errors(
        "struct S { n: Int }\n\
         fn describe(s: S) -> Int => s {\n    return s.n\n}\n\
         iter fn next(s: S) -> Emitted Int | Finished {\n    \
         state {\n        left: Int = s.n\n    }\n    \
         if left <= 0 {\n        return finished()\n    }\n    \
         left = left - 1\n    return emitted(describe(s))\n}\n",
    );
    assert!(errs.is_empty(), "expected no errors, got {errs:?}");
}

/// A `state` field named like a subject field the body also reads: the snapshot
/// moves into the compiler's namespace rather than colliding.
#[test]
fn a_state_field_may_share_a_name_with_a_subject_field() {
    let errs = errors(
        "struct S { at: Int, limit: Int }\n\
         iter fn next(s: S) -> Emitted Int | Finished {\n    \
         state {\n        at: Int = s.at\n    }\n    \
         if at >= s.limit {\n        return finished()\n    }\n    \
         at = at + 1\n    return emitted(copy(at))\n}\n",
    );
    assert!(errs.is_empty(), "expected no errors, got {errs:?}");
}

// --- a generic function over *containers* ------------------------------------

/// [implicit-infer] The container-shaped combinator (user decision 2026-09-10):
/// a generic fn takes the container and asks for its `iter` as an implicit, so
/// the pass type `It` is decided by *which `iter` fills it* — and then the
/// `?Yield<It, T>` beside it resolves against what that taught.
///
/// The reason this had to work: with an `iter fn` the pass type is **unnameable**,
/// so writing the type arguments is not an available workaround. Before the
/// learning sweep ran after the arguments were typed, `next` was reported as
/// ambiguous with `It` still `?`.
const CONTAINER_COMBINATOR: &str = r#"
fn total<C, It>(c: C, ?iter: (c: C) -> Mut It, ?Yield<It, Int>) -> Int =>[iter] c, proj[from: c] => c {
    let sum = 0
    let p = iter(c)
    for n in p {
        sum = sum + n
    }
    return sum
}
"#;

/// The case that was impossible: the pass is the one an `iter fn` generated, so
/// no type argument could name it.
#[test]
fn a_generic_fn_infers_the_pass_type_of_an_iter_fn_subject() {
    let errs = errors(&format!(
        "{COUNTDOWN}{CONTAINER_COMBINATOR}\n\
         fn go() -> Int {{\n    let c = Countdown {{ from: 3 }}\n    return total(c)\n}}\n"
    ));
    assert!(errs.is_empty(), "expected no errors, got {errs:?}");
}

/// The same function over a container with a *written* `iter`.
#[test]
fn a_generic_fn_infers_the_pass_type_of_a_written_iter() {
    let errs = errors(&format!(
        "struct Bag {{ items: List<Int> }}\n\
         struct BagYield : Yield<self, Int> canbe Mut {{ items: proj List<Int>, at: Int }}\n\
         fn iter(bag: Bag) -> Mut BagYield => bag {{\n    \
         return Mut BagYield {{ items: bag.items, at: 0 }}\n}}\n\
         fn next(p: Mut BagYield) -> Emitted Int | Finished => p: Mut {{\n    \
         let e = get(p.items, p.at)\n    if e is None {{\n        return finished()\n    }}\n    \
         p.at = p.at + 1\n    return emitted(e)\n}}\n\
         {CONTAINER_COMBINATOR}\n\
         fn go() -> Int {{\n    let b = Bag {{ items: list(4, 5) }}\n    return total(b)\n}}\n"
    ));
    assert!(errs.is_empty(), "expected no errors, got {errs:?}");
}

/// And the ambiguity the user accepted as the price (2026-09-10): when nothing
/// determines the container type either, several `iter`s match and the choice
/// would be a guess. The remedies are the ordinary ones — a `rename`, or passing
/// the member by name.
#[test]
fn an_undetermined_container_reports_the_ambiguity() {
    let errs = errors(&format!(
        "{CONTAINER_COMBINATOR}\n\
         fn forward<D>(d: D) -> Int => !d {{\n    return total(d)\n}}\n"
    ));
    assert!(
        errs.iter().any(|e| e.contains("ambiguous") || e.contains("no `iter`")),
        "{errs:?}"
    );
}

/// [iter-fn] The integration guard: a driver that parses with
/// `parse_module_deferred` owes the module a program-level
/// `expand_iter_fns_with` before resolution. A surviving `iter fn` must
/// never be checked as an ordinary fn (silently different behavior), so
/// resolve reports it loudly instead.
#[test]
fn an_unexpanded_iter_fn_is_a_loud_resolution_error() {
    let src = "struct Countdown {\n    from: Int\n}\n\
               iter fn next(c: Countdown) -> Emitted Int | Finished {\n\
               \x20   state {\n        at: Int = 0\n    }\n\
               \x20   if at >= c.from {\n        return finished()\n    }\n\
               \x20   at = at + 1\n\
               \x20   return emitted(at)\n\
               }\n";
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
        // Deliberately deferred, and never expanded: the mistake under test.
        let (ast, diagnostics) = salvo_syntax::parse_module_deferred(&file.content);
        assert!(diagnostics.iter().all(|d| !d.is_error()), "{diagnostics:?}");
        modules.push(ast);
    }
    let program = Program {
        files: sources.files,
        modules,
        companions: Vec::new(),
    };
    let resolution = resolve(&program);
    assert!(
        resolution
            .errors
            .iter()
            .any(|d| d.message.contains("reached resolution unexpanded")),
        "expected the loud guard, got: {:?}",
        resolution.errors
    );
}
