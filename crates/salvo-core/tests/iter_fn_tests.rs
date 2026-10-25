//! [iter-fn] The `iter fn` form (user decisions 2026-09-09, 2026-09-26): a
//! minter under any name, with any parameters, whose body is the step and whose
//! **iterator struct is generated**.
//!
//! ```
//! struct Countdown { from: Int }
//!
//! iter fn iter(c: Countdown) -> Emitted Int | Finished {
//!     state CountdownIter {
//!         at: Int = c.from
//!     }
//!     if at <= 0 { return finished() }
//!     at = at - 1
//!     return emitted(at + 1)
//! }
//! ```
//!
//! The way to be iterable with the least to declare: the parameters stay
//! ordinary data, the `state` block names the iterator struct and declares its
//! own fields, and the
//! compiler writes the struct plus its `next`. The author's body *is* the
//! `next`, so what this file checks is mostly that the generated declarations
//! behave like the hand-written ones they stand for — plus the `Iter` group
//! [iter-group] and the step-call `for` [iter-step-call].
//!
//! The expansion happens in `salvo_syntax::desugar`, before resolution, so
//! every test here goes through the ordinary checker.

use std::path::Path;

use salvo_core::{check_program, resolve, Program, SourceSet, Symbols};

const STD_PRELUDE: &str =
    "export intrinsic type Int\nexport intrinsic type Str\nexport intrinsic type Bool\n\
     export intrinsic type List<T> canbe Mut\n\
     export intrinsic fn copy<T>(value: T) [] -> T => value\n\
     export intrinsic fn Mut List<T>(...elems: T[]) [] -> Mut List<T>\n\
     export intrinsic fn List<T>(...elems: T[]) [] -> List<T>\n\
     export intrinsic fn add<T>(list: Mut List<T>, elem: T) [] -> None => list: Mut, !elem\n\
     export intrinsic fn get<T>(list: List<T>, index: Int) [] -> T? => list, index\n\
     export intrinsic fn size<T>(list: List<T>) [] -> Int => list\n\
     export qualifier Emitted<T> of T\n\
     export struct Finished {}\n\
     export fn emitted<T>(value: T) [] -> +Emitted T => !value {\n    return value\n}\n\
     export fn finished() [] -> Finished {\n    return Finished {}\n}\n\
     export params Yield<It, T> {\n    fn next(it: Mut It) -> Emitted T | Finished => it: Mut\n}\n\
     export params Iter<C, It, T> => iter with next {\n    fn iter(collection: C) -> Mut It\n    fn next(iterator: Mut It) -> Emitted T | Finished => iterator: Mut\n}\n\
     export effect Console {\n    fn print(message: Str) -> None => !message\n}\n\
     export handler StdConsole of Console {\n    fn print(message: Str) -> None => !message {}\n}\n\
     export fn println(message: Str) [Console] -> None => !message {}\n";

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
        let (ast, diagnostics) = salvo_syntax::parse_module_deferred(&file.content);
        parse_errors.extend(
            diagnostics
                .iter()
                .filter(|d| d.is_error())
                .map(|d| d.message.clone()),
        );
        modules.push(ast);
    }
    // The program-level expansion, as every driver runs it.
    let expansion = salvo_core::expand(&sources.files, &mut modules);
    parse_errors.extend(
        expansion
            .diagnostics
            .iter()
            .filter(|d| d.is_error())
            .map(|d| d.message.clone()),
    );
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

iter fn iter(c: Countdown) -> Emitted Int | Finished {
    state CountdownIter {
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
/// pass [iter-mint].
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
         iter fn iter(s: S) -> Emitted Int | Finished {\n    \
         state SIter {\n        at: Int = \"not a number\"\n    }\n    \
         return finished()\n}\n",
    );
    assert!(
        errs.iter().any(|e| e.contains("expects `Int`, found `Str`")),
        "{errs:?}"
    );
}

/// [state-infer] [interface-pass] An inferred state field's type is the
/// iterator struct's, an interface: a fn declared *above* the `iter fn`
/// reads it as one declared below does. It was read off the minter's body
/// walk, so the fn above saw `Unknown` and returned an `Int` field from a
/// `-> Str` unopposed (found 2026-10-20).
#[test]
fn an_inferred_state_field_is_typed_before_any_body() {
    for (above, below) in [(true, false), (false, true)] {
        let reader = "fn peek(c: Counts) -> Str {\n    return c.at\n}\n";
        let iter = "iter fn counts(n: Int) -> Emitted Int | Finished {\n    \
                    state Counts { at = 0 }\n    \
                    if at >= n { return finished() }\n    \
                    at = at + 1\n    \
                    return emitted(at)\n}\n";
        let src = format!(
            "{}{iter}{}",
            if above { reader } else { "" },
            if below { reader } else { "" }
        );
        let errs = errors(&src);
        assert!(
            errs.iter().any(|e| e.contains("expected return type `Str`, found `Int`")),
            "reader above: {above}: {errs:?}"
        );
    }
}

/// The subject is **read-only** (user decision 2026-09-09): it is a field of
/// the pass typed as the subject, so writing through it is the standing
/// [struct-mut] refusal rather than a rule of its own.
#[test]
fn the_subject_is_read_only() {
    let errs = errors(
        "struct S canbe Mut { n: Int }\n\
         iter fn iter(s: S) -> Emitted Int | Finished {\n    \
         state SIter2 {\n        at: Int = 0\n    }\n    \
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
         iter fn iter(s: S) -> Emitted Int | Finished {\n    \
         state SIter3 {\n        at: Int = noisy()\n    }\n    \
         return finished()\n}\n",
    );
    assert!(
        errs.iter().any(|e| e.contains("no handler for effect `Console`")),
        "{errs:?}"
    );
}

// --- the refusals -----------------------------------------------------------

/// [iter-fn] Any name: the declaration is the minter, so `countdown(3)` mints
/// an iterator and `for n in countdown(3)` drives it (user decision
/// 2026-09-26). A parameter that is a Copy scalar is held by value.
#[test]
fn an_iter_fn_may_have_any_name_and_scalar_parameters() {
    let errs = errors(
        "iter fn countdown(from: Int) -> Emitted Int | Finished {\n    \
         state Countdown {\n        at: Int = from\n    }\n    \
         if at <= 0 {\n        return finished()\n    }\n    \
         at = at - 1\n    return emitted(at + 1)\n}\n\
         fn go() -> Int {\n    let sum = 0\n    for n in countdown(3) {\n        sum = sum + n\n    }\n    \
         let p = countdown(2)\n    for n in p {\n        sum = sum + n\n    }\n    return sum\n}\n",
    );
    assert!(errs.is_empty(), "expected no errors, got {errs:?}");
}

/// [iter-fn] Several parameters, of which the non-scalar ones are borrowed by
/// the iterator struct and the scalars owned.
#[test]
fn an_iter_fn_may_take_several_parameters() {
    let errs = errors(
        "iter fn take(xs: List<Int>, count: Int) -> Emitted Int | Finished {\n    \
         state Take {\n        at: Int = 0\n    }\n    \
         if at >= count {\n        return finished()\n    }\n    \
         let e = get(xs, at)\n    if e is None {\n        return finished()\n    }\n    \
         at = at + 1\n    return emitted(copy(e))\n}\n\
         fn go() -> Int {\n    let xs = List(1, 2, 3)\n    let sum = 0\n    \
         for n in take(xs, 2) {\n        sum = sum + n\n    }\n    return sum + size(xs)\n}\n",
    );
    assert!(errs.is_empty(), "expected no errors, got {errs:?}");
}

/// [iter-fn] A variadic parameter has no field to be held in.
#[test]
fn an_iter_fn_refuses_a_variadic_parameter() {
    let errs = errors(
        "iter fn each(...xs: Int[]) -> Emitted Int | Finished {\n    state Each {}\n    return finished()\n}\n",
    );
    assert!(
        errs.iter().any(|e| e.contains("cannot take a variadic parameter")),
        "{errs:?}"
    );
}

/// `Mut` on the subject would promise that advancing writes through it, which
/// is the opposite of what a mint does.
#[test]
fn a_mut_subject_is_refused() {
    let errs = errors(
        "struct S canbe Mut { n: Int }\n\
         iter fn iter(s: Mut S) -> Emitted Int | Finished {\n    state SIter {}\n    return finished()\n}\n",
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
         iter fn iter(s: S) -> Int {\n    state SIter4 {}\n    return 1\n}\n",
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
         iter fn iter(s: S) -> Emitted Int | Finished {\n    \
         state SIter5 {\n        at: Int = 0\n    }\n    \
         let at = 3\n    return finished()\n}\n",
    );
    assert!(
        errs.iter().any(|e| e.contains("may not reuse it")),
        "{errs:?}"
    );
}

#[test]
fn a_binding_may_not_shadow_the_subject() {
    let errs = errors(
        "struct S { n: Int }\n\
         iter fn iter(s: S) -> Emitted Int | Finished {\n    \
         state SIter6 {\n        at: Int = 0\n    }\n    \
         for s in List(1, 2) {}\n    return finished()\n}\n",
    );
    assert!(
        errs.iter().any(|e| e.contains("may not reuse it")),
        "{errs:?}"
    );
}

/// A `state` field named like a parameter is the same collision, caught at the
/// declaration rather than at a use.
#[test]
fn a_state_field_may_not_be_named_like_the_subject() {
    let errs = errors(
        "struct S { n: Int }\n\
         iter fn iter(s: S) -> Emitted Int | Finished {\n    \
         state SIter7 {\n        s: Int = 0\n    }\n    \
         return finished()\n}\n",
    );
    assert!(
        errs.iter().any(|e| e.contains("parameter's name")),
        "{errs:?}"
    );
}

/// The generated struct is named after the subject, so the subject needs a
/// name: an array, tuple or union has to be wrapped.
#[test]
fn a_structural_subject_is_refused() {
    let errs = errors(
        "iter fn iter(items: Int[]) -> Emitted Int | Finished {\n    state ItemsIter {}\n    return finished()\n}\n",
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
         iter fn iter(s: S) -> Emitted Int | Finished {\n    \
         state SIter8 {\n        at: Int\n    }\n    \
         return finished()\n}\n",
    );
    assert!(
        errs.iter().any(|e| e.contains("needs an initializer")),
        "{errs:?}"
    );
}

/// An empty block is a struct holding the parameters only.
#[test]
fn an_empty_state_block_holds_the_parameters() {
    let errs = errors(
        "struct S { n: Int }\n\
         iter fn iter(s: S) -> Emitted Int | Finished {\n    \
         state SIter9 {\n    }\n    \
         return finished()\n}\n",
    );
    assert!(errs.is_empty(), "expected no errors, got {errs:?}");
}

// --- reading the subject ------------------------------------------------------

/// [iter-fn] Reading a subject field on every turn reads it through the
/// iterator's borrow of the subject, typed by the field's declaration.
#[test]
fn a_subject_field_read_per_turn_is_typed_by_its_declaration() {
    let errs = errors(
        "struct S { limit: Int }\n\
         iter fn iter(s: S) -> Emitted Int | Finished {\n    \
         state SIter10 {\n        at: Int = 0\n    }\n    \
         if at >= s.limit {\n        return finished()\n    }\n    \
         at = at + 1\n    return emitted(copy(at))\n}\n",
    );
    assert!(errs.is_empty(), "expected no errors, got {errs:?}");
}

/// A field the subject does not have is reported against the subject's own
/// type, not against the generated iterator struct.
#[test]
fn an_unknown_subject_field_is_reported_against_the_subject() {
    let errs = errors(
        "struct S { limit: Int }\n\
         iter fn iter(s: S) -> Emitted Int | Finished {\n    \
         state SIter11 {\n        at: Int = 0\n    }\n    \
         if at >= s.limmit {\n        return finished()\n    }\n    \
         return finished()\n}\n",
    );
    assert!(
        errs.iter().any(|e| e.contains("limmit") && e.contains("S")),
        "{errs:?}"
    );
}

/// The body may hand the subject on as a value.
#[test]
fn the_whole_subject_may_be_handed_on() {
    let errs = errors(
        "struct S { n: Int }\n\
         fn describe(s: S) -> Int => s {\n    return s.n\n}\n\
         iter fn iter(s: S) -> Emitted Int | Finished {\n    \
         state SIter12 {\n        left: Int = s.n\n    }\n    \
         if left <= 0 {\n        return finished()\n    }\n    \
         left = left - 1\n    return emitted(describe(s))\n}\n",
    );
    assert!(errs.is_empty(), "expected no errors, got {errs:?}");
}

/// A `state` field named like a subject field the body also reads: the
/// subject is held whole, so the two never meet.
#[test]
fn a_state_field_may_share_a_name_with_a_subject_field() {
    let errs = errors(
        "struct S { at: Int, limit: Int }\n\
         iter fn iter(s: S) -> Emitted Int | Finished {\n    \
         state SIter13 {\n        at: Int = s.at\n    }\n    \
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
/// Before the learning sweep ran after the arguments were typed, `next` was
/// reported as ambiguous with `It` still `?`.
const CONTAINER_COMBINATOR: &str = r#"
fn total<C, It>(c: C, ?iter: (c: C) -> Mut It holds proj(c), ?Yield<It, Int>) -> Int =>[iter] c => c {
    let sum = 0
    let p = iter(c)
    for n in p {
        sum = sum + n
    }
    return sum
}
"#;

/// The pass is the one an `iter fn` generated.
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
         fn go() -> Int {{\n    let b = Bag {{ items: List(4, 5) }}\n    return total(b)\n}}\n"
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
/// `parse_module_deferred` owes the module the program-level expansion
/// (`salvo_core::expand`) before resolution. A surviving `iter fn` must
/// never be checked as an ordinary fn (silently different behavior), so
/// resolve reports it loudly instead.
#[test]
fn an_unexpanded_iter_fn_is_a_loud_resolution_error() {
    let src = "struct Countdown {\n    from: Int\n}\n\
               iter fn iter(c: Countdown) -> Emitted Int | Finished {\n\
               \x20   state CountdownIter {\n        at: Int = 0\n    }\n\
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

// --- [iter-group] the `Iter` group -------------------------------------------

/// `: Iter<self, It, T>` is satisfied by an `iter fn iter` over the type whose
/// state is `It`.
#[test]
fn an_iter_obligation_is_satisfied_by_an_iter_fn() {
    let errs = errors(
        "struct Bag : Iter<self, BagIter, Int> { items: List<Int> }\n\
         iter fn iter(bag: Bag) -> Emitted Int | Finished {\n    \
         state BagIter {\n        at: Int = 0\n    }\n    \
         let e = get(bag.items, at)\n    if e is None {\n        return finished()\n    }\n    \
         at = at + 1\n    return emitted(copy(e))\n}\n\
         fn go() -> Int {\n    let b = Bag { items: List(1, 2) }\n    let sum = 0\n    \
         for n in b {\n        sum = sum + n\n    }\n    return sum\n}\n",
    );
    assert!(errs.is_empty(), "expected no errors, got {errs:?}");
}

/// …or by a hand-written `iter` returning the named iterator struct.
#[test]
fn an_iter_obligation_is_satisfied_by_a_minter_of_a_named_struct() {
    let errs = errors(
        "struct Bag : Iter<self, BagIter, Int> { items: List<Int> }\n\
         struct BagIter : Yield<self, Int> canbe Mut { items: proj List<Int>, at: Int }\n\
         fn iter(bag: Bag) -> Mut BagIter => bag {\n    \
         return Mut BagIter { items: bag.items, at: 0 }\n}\n\
         fn next(p: Mut BagIter) -> Emitted Int | Finished => p: Mut {\n    \
         let e = get(p.items, p.at)\n    if e is None {\n        return finished()\n    }\n    \
         p.at = p.at + 1\n    return emitted(copy(e))\n}\n",
    );
    assert!(errs.is_empty(), "expected no errors, got {errs:?}");
}

/// An obligation with no minter behind it is an error at the struct.
#[test]
fn an_unsatisfied_iter_obligation_is_reported_at_the_struct() {
    let errs = errors(
        "struct Bag : Iter<self, BagIter, Int> { items: List<Int> }\n\
         struct BagIter : Yield<self, Int> canbe Mut { at: Int }\n\
         fn next(p: Mut BagIter) -> Emitted Int | Finished => p: Mut {\n    return finished()\n}\n",
    );
    assert!(
        errs.iter().any(|e| e.contains("no visible `iter` matches")),
        "{errs:?}"
    );
}

/// A type is a source or an iterator struct, never both: `for x in s` would
/// have two answers.
#[test]
fn iter_and_yield_on_one_struct_is_refused() {
    let errs = errors(
        "struct Both : Iter<self, BothIter, Int>, Yield<self, Int> canbe Mut { at: Int }\n\
         iter fn iter(b: Both) -> Emitted Int | Finished {\n    state BothIter {}\n    return finished()\n}\n\
         fn next(b: Mut Both) -> Emitted Int | Finished => b: Mut {\n    return finished()\n}\n",
    );
    assert!(
        errs.iter().any(|e| e.contains("declares both")),
        "{errs:?}"
    );
}

/// `?Iter<C, It, T>`: `for x in c` mints with the `iter` implicit and drives
/// with the `next` beside it, and a call fills the pair from whatever `C`
/// turns out to be — an `iter fn`'s minter, a named struct's, or std's own.
#[test]
fn a_spread_over_a_generic_source_drives_through_its_iterator() {
    let errs = errors(&format!(
        "{COUNTDOWN}\n\
         fn total<C, It>(c: C, ?Iter<C, It, Int>) -> Int => c {{\n    let sum = 0\n    \
         for n in c {{\n        sum = sum + n\n    }}\n    return sum\n}}\n\
         fn go() -> Int {{\n    let c = Countdown {{ from: 3 }}\n    return total(c)\n}}\n"
    ));
    assert!(errs.is_empty(), "expected no errors, got {errs:?}");
}

/// Inside the spreading fn, `iter(c)` yields the `It`, which a
/// `?Yield` combinator accepts with the paired `next` forwarded.
#[test]
fn a_spread_forwards_its_pair_to_a_yield_combinator() {
    let errs = errors(&format!(
        "{COUNTDOWN}\n\
         fn count_it<It>(it: Mut It, ?Yield<It, Int>) -> Int => it: Mut {{\n    let n = 0\n    \
         for x in it {{\n        n = n + 1\n    }}\n    return n\n}}\n\
         fn count<C, It>(c: C, ?Iter<C, It, Int>) -> Int => c {{\n    let p = iter(c)\n    return count_it(p)\n}}\n\
         fn go() -> Int {{\n    return count(Countdown {{ from: 3 }})\n}}\n"
    ));
    assert!(errs.is_empty(), "expected no errors, got {errs:?}");
}

// --- [iter-step-call] `for` over a step call ---------------------------------

/// `for i in next(p)` re-invokes the call each turn — which is what lets a step
/// under another name be driven at all.
#[test]
fn a_for_over_a_step_call_drives_it() {
    let errs = errors(
        "struct Down : Yield<self, Int> canbe Mut { at: Int }\n\
         fn next(d: Mut Down) -> Emitted Int | Finished => d: Mut {\n    \
         if d.at <= 0 {\n        return finished()\n    }\n    d.at = d.at - 1\n    return emitted(d.at + 1)\n}\n\
         fn skip2(d: Mut Down) -> Emitted Int | Finished => d: Mut {\n    \
         if d.at <= 1 {\n        return finished()\n    }\n    d.at = d.at - 2\n    return emitted(d.at + 2)\n}\n\
         fn go() -> Int {\n    let d = Mut Down { at: 6 }\n    let sum = 0\n    \
         for n in skip2(d) {\n        sum = sum + n\n    }\n    \
         for n in next(d) {\n        sum = sum + n\n    }\n    return sum\n}\n",
    );
    assert!(errs.is_empty(), "expected no errors, got {errs:?}");
}

/// Each argument is re-evaluated per turn, so it has to be a place or a
/// literal.
#[test]
fn a_step_call_argument_must_be_a_place() {
    let errs = errors(
        "struct Down : Yield<self, Int> canbe Mut { at: Int }\n\
         fn next(d: Mut Down) -> Emitted Int | Finished => d: Mut {\n    return finished()\n}\n\
         fn mk() -> Mut Down {\n    return Mut Down { at: 3 }\n}\n\
         fn go() -> None {\n    for n in next(mk()) {}\n}\n",
    );
    assert!(
        errs.iter().any(|e| e.contains("has to be a place")),
        "{errs:?}"
    );
}

/// A step's *result* held in a variable is a union, not a loop.
#[test]
fn a_step_result_value_is_not_iterable() {
    let errs = errors(
        "struct Down : Yield<self, Int> canbe Mut { at: Int }\n\
         fn next(d: Mut Down) -> Emitted Int | Finished => d: Mut {\n    return finished()\n}\n\
         fn go() -> None {\n    let d = Mut Down { at: 3 }\n    let r = next(d)\n    for n in r {}\n}\n",
    );
    assert!(
        errs.iter().any(|e| e.contains("is a step's result")),
        "{errs:?}"
    );
}

// --- [iter-fn] the named state struct, and every parameter held -----------

/// The struct is named by the `state` block, so a fn handing an iterator out
/// writes its type (what `-> iter T` used to fill from the body).
#[test]
fn the_state_struct_is_an_ordinary_type() {
    let errs = errors(&format!(
        "{COUNTDOWN}\n\
         fn from(c: Countdown) -> Mut CountdownIter => c {{\n    return iter(c)\n}}\n\
         fn go() -> Int {{\n    let c = Countdown {{ from: 3 }}\n    let sum = 0\n    for x in from(c) {{\n        sum = sum + x\n    }}\n    \
         let p: Mut CountdownIter = from(c)\n    let s = next(p)\n    return sum\n}}\n"
    ));
    assert!(errs.is_empty(), "expected no errors, got {errs:?}");
}

/// The name is declared beside the fn, so it collides with a type of the same
/// name there — and with another `iter fn`'s state.
#[test]
fn a_state_name_collides_with_a_type() {
    let errs = errors(
        "struct Range { a: Int }\n\
         iter fn upto(n: Int) -> Emitted Int | Finished {\n    state Range {\n        at: Int = 0\n    }\n    return finished()\n}\n",
    );
    assert!(
        errs.iter().any(|e| e.contains("duplicate struct `Range`")),
        "{errs:?}"
    );
    let errs = errors(
        "iter fn upto(n: Int) -> Emitted Int | Finished {\n    state Upto {}\n    return finished()\n}\n\
         iter fn downto(n: Int) -> Emitted Int | Finished {\n    state Upto {}\n    return finished()\n}\n",
    );
    assert!(
        errs.iter().any(|e| e.contains("duplicate struct `Upto`")),
        "{errs:?}"
    );
}

/// Every `iter fn` names its struct: no `state` block is an error.
#[test]
fn a_missing_state_block_is_refused() {
    let errs = errors("iter fn ones(n: Int) -> Emitted Int | Finished {\n    return finished()\n}\n");
    assert!(
        errs.iter().any(|e| e.contains("names its iterator struct")),
        "{errs:?}"
    );
    let errs = errors("iter fn ones(n: Int) -> Emitted Int | Finished {\n    state {}\n    return finished()\n}\n");
    assert!(
        errs.iter().any(|e| e.contains("names the iterator struct")),
        "{errs:?}"
    );
}

/// `iter T` is gone: what wrote it writes a generic with its spread.
#[test]
fn iter_t_is_not_a_type() {
    let errs = errors("fn first(it: iter Int) -> Int => it: Mut {\n    return 0\n}\n");
    assert!(!errs.is_empty(), "expected `iter Int` to be refused");
}
