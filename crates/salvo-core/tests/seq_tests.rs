//! [seq-pass] [implicit-group] [implicit-infer] [fn-overload-rank] The sequence
//! functions — `map`, `filter`, `reduce` — and what makes them work over
//! *any pass*.
//!
//! There is no `Iterable` trait, and since R5 there is no `Iterable` group
//! either: a combinator's subject **is** the pass, and its `next` arrives
//! through a `?Yield<It, T>` spread [iter-protocol] that each call fills with
//! whichever `next` fits. A container is iterated by writing its `iter`
//! (`map(iter(xs), f)`), which is what keeps the inference ordinary. Two facts
//! carry the design:
//!
//! * `It` is bound by an ordinary argument, so the spread resolves with the
//!   plain implicit machinery — and `T` comes from **which `next` filled the
//!   implicit**, which is what lets a bare lambda be typed [call-type-args];
//! * expected types have to reach a lambda even though the name is
//!   *overloaded* (each function also has a `List` fast path), which is what
//!   the specificity ranking's *lead candidate* is for — re-narrowed per
//!   argument, so the subject decides.

use std::path::Path;

use salvo_core::{check_program, resolve, FileDiagnostic, FnKey, Program, SourceSet, Symbols};

/// [intrinsic-std-only] A miniature of std's iterable surface: the group,
/// the `iter` overloads, and the generic sequence functions beside their
/// `List` fast paths. Loaded as *std* files, since only std may write
/// `intrinsic`.
const STD_PRELUDE: &str = "\
export intrinsic type Int\n\
export intrinsic type Bool\n\
export intrinsic type Char\n\
export intrinsic type Str\n\
export intrinsic type List<T> canbe Mut\n\
export intrinsic fn copy<T>(value: T) [] -> T => value\n\
export intrinsic fn mut_list_of<T>(...elems: T[]) [] -> Mut List<T>\n\
export intrinsic fn list_of<T>(...elems: T[]) [] -> List<T>\n\
export intrinsic fn add<T>(list: Mut List<T>, elem: T) [] -> None => list: Mut, !elem\n\
export intrinsic fn size<T>(list: List<T>) [] -> Int => list\n\
export intrinsic fn get<T>(list: List<T>, index: Int) [] -> T? => list, index\n\
export intrinsic fn char_at(str: Str, index: Int) [] -> Char? => str, index\n\
export qualifier Emitted<T> of T\n\
export struct Finished {}\n\
export fn emitted<T>(value: T) [] -> +Emitted T => !value { return value }\n\
export fn finished() [] -> Finished { return Finished {} }\n\
export params Yield<It, T> {\n\
    fn next(it: Mut It) -> Emitted T | Finished => it: Mut\n\
}\n\
export struct ListYield<T> : Yield<self, T> canbe Mut {\n\
    items: List<T>,\n\
    at: Int\n\
}\n\
export fn iter<T>(list: List<T>) [] -> Mut ListYield<T> => !list {\n\
    return Mut ListYield<T> { items: list, at: 0 }\n\
}\n\
export fn next<T>(p: Mut ListYield<T>) [] -> Emitted T | Finished => p: Mut {\n\
    let elem = get(p.items, p.at)\n\
    if elem is None {\n\
        return finished()\n\
    }\n\
    p.at = p.at + 1\n\
    return emitted(elem)\n\
}\n\
export struct StrYield : Yield<self, Char> canbe Mut {\n\
    text: Str,\n\
    at: Int\n\
}\n\
export fn iter(str: Str) [] -> Mut StrYield => !str {\n\
    return Mut StrYield { text: str, at: 0 }\n\
}\n\
export fn next(p: Mut StrYield) [] -> Emitted Char | Finished => p: Mut {\n\
    let chr = char_at(p.text, p.at)\n\
    if chr is None {\n\
        return finished()\n\
    }\n\
    p.at = p.at + 1\n\
    return emitted(chr)\n\
}\n\
export fn map<It, T, U>(it: Mut It, f: (T) -> U, ?Yield<It, T>) [] -> Mut List<U> => it: Mut, f {\n\
    let out = mut_list_of<U>()\n\
    let going = true\n\
    while going {\n\
        let step = next(it)\n\
        when step {\n\
            is Emitted {\n\
                add(out, f(step))\n\
            }\n\
            is Finished {\n\
                going = false\n\
            }\n\
        }\n\
    }\n\
    return out\n\
}\n\
export fn reduce<It, T, A>(it: Mut It, init: A, f: (A, T) -> A, ?Yield<It, T>) [] -> A => it: Mut, f, !init {\n\
    let acc = init\n\
    let going = true\n\
    while going {\n\
        let step = next(it)\n\
        when step {\n\
            is Emitted {\n\
                acc = f(acc, step)\n\
            }\n\
            is Finished {\n\
                going = false\n\
            }\n\
        }\n\
    }\n\
    return acc\n\
}\n\
export intrinsic fn map<T, U>(list: List<T>, f: (T) -> U) [] -> Mut List<U> => list, f\n\
";

fn checked(src: &str) -> (Program, salvo_core::Checked) {
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
    let out = check_program(&program, &resolution, &symbols);
    (program, out)
}

fn errors(src: &str) -> Vec<FileDiagnostic> {
    // Errors only: an unused-variable *warning* [unused-var] is a different
    // severity, and these tests are about legality.
    let mut diags = checked(src).1.errors;
    diags.retain(|d| d.is_error());
    diags
}

fn messages(src: &str) -> Vec<String> {
    errors(src).iter().map(|d| d.message.clone()).collect()
}

fn probe(body: &str) -> String {
    format!("fn probe() -> None {{\n{body}\n}}\n")
}

/// Which `map` overload a call resolved to: `true` for the intrinsic `List`
/// fast path, `false` for the generic body.
fn picked_fast_path(src: &str, callee: &str) -> bool {
    let (program, out) = checked(src);
    assert!(
        out.errors.iter().all(|d| !d.is_error()),
        "unexpected errors: {:?}",
        out.errors
    );
    let mut found: Option<bool> = None;
    for (_, key) in out.call_fn.iter() {
        let FnKey { file, item, .. } = *key;
        if let Some(salvo_syntax::ast::Item::Fn(f)) = program.modules[file].items.get(item) {
            if f.name.name == callee {
                // The user file declares nothing here, so both candidates
                // live in std; the fast path is the `intrinsic` one.
                found = Some(f.intrinsic);
            }
        }
    }
    found.unwrap_or_else(|| panic!("no call to `{callee}` was resolved"))
}

// ===== the generic path: any pass =====

/// [implicit-infer] A list, iterated by writing its `iter`: `It` from the
/// argument, `T` from the `next` that fills the implicit, `U` from the
/// lambda's body.
#[test]
fn a_pass_subject_infers_everything() {
    let src = probe(
        "    let xs = list_of(1, 2)\n    let ys: Mut List<Int> = map(iter(xs), n -> n * 2)",
    );
    assert!(messages(&src).is_empty(), "{:?}", messages(&src));
}

/// A `Str` pass yields *characters*, so the lambda's parameter is a `Char` —
/// which is only knowable from the `next` that fills the implicit.
#[test]
fn a_str_pass_iterates_characters() {
    let src = probe("    let ys: Mut List<Bool> = map(iter(\"ab\"), c -> c == 'a')");
    assert!(messages(&src).is_empty(), "{:?}", messages(&src));
    // ... and the element really is a `Char`, not a `Str`.
    let bad = probe("    let ys: Mut List<Int> = map(iter(\"ab\"), c -> size(c))");
    assert!(!messages(&bad).is_empty(), "a `Char` is not a `List`");
}

/// Chains compose, because `map`'s result is a `Mut List` and that has an
/// `iter` like any other list — one call more than the old `?Iterable` surface,
/// and no cross-implicit inference (user decision 2026-09-09).
#[test]
fn chains_compose_through_iter() {
    let src = probe(
        "    let xs = list_of(1, 2)\n    \
         let zs: Mut List<Int> = map(iter(map(xs, n -> n * 2)), n -> n + 1)",
    );
    assert!(messages(&src).is_empty(), "{:?}", messages(&src));
}

/// A type of your own joins in by declaring an `iter` that answers a pass —
/// nothing else, and no trait.
#[test]
fn a_user_type_becomes_iterable_by_declaring_iter() {
    let src = format!(
        "struct Bag {{\n    items: List<Int>\n}}\n\n\
         fn iter(bag: Bag) -> Mut ListYield<Int> => bag {{\n    return iter(bag.items)\n}}\n\n{}",
        probe(
            "    let b = Bag {items: list_of(1, 2)}\n    \
             let total: Int = reduce(iter(b), 0, (a, x) -> a + x)"
        )
    );
    assert!(messages(&src).is_empty(), "{:?}", messages(&src));
}

/// A subject that is not a pass is an error. It surfaces as *no matching
/// overload* rather than as the missing implicit: the pass position is
/// `Mut It`, and an `Int` cannot be `Mut`, so the candidate is out before its
/// spread is ever resolved.
#[test]
fn a_subject_that_is_not_a_pass_is_an_error() {
    let src = probe("    let ys = map(1, n -> n)");
    let msgs = messages(&src);
    assert!(
        msgs.iter().any(|m| m.contains("no matching overload for `map(Int")),
        "expected the overload rejection: {msgs:?}"
    );
}

// ===== the fast path, and what picks it =====

/// [fn-overload-rank] With a `List` subject the *intrinsic* overload wins:
/// `List<T>` is more specific than a bare `It`. It is also what keeps the short
/// spelling for the type people map most.
#[test]
fn a_list_subject_picks_the_fast_path() {
    let src = probe("    let xs = list_of(1, 2)\n    let ys: Mut List<Int> = map(xs, n -> n * 2)");
    assert!(picked_fast_path(&src, "map"), "expected the `List` overload");
}

/// And with a pass subject the generic body takes it — the fast path is not
/// viable, so specificity never gets to prefer it.
#[test]
fn a_pass_subject_picks_the_generic_body() {
    let src = probe(
        "    let xs = list_of(1, 2)\n    let ys: Mut List<Int> = map(iter(xs), n -> n + 1)",
    );
    assert!(
        !picked_fast_path(&src, "map"),
        "expected the generic overload"
    );
}

/// [fn-overload-rank] The lead candidate is re-narrowed *per argument*: the
/// `List` candidate leads on specificity, and typing the subject has to drop it
/// before the lambda is checked against `List<T>`'s element type.
#[test]
fn the_lead_narrows_before_the_lambda_is_typed() {
    // A `Str` pass with a `Char` lambda: if the `List` fast path had still been
    // leading, the lambda would have been typed against `T` from `List<T>` —
    // unbound — and `U` would not have been inferable.
    let src = probe("    let ys: Mut List<Bool> = map(iter(\"ab\"), c -> c == 'a')");
    assert!(messages(&src).is_empty(), "{:?}", messages(&src));
    assert!(!picked_fast_path(&src, "map"));
}

// ===== [iter-generic-drive] a generic pass is driven by `for` =====

/// The rule that lets std's own combinators read like ordinary Salvo (user
/// decision 2026-09-09): when a body has a `?Yield<It, T>` spread, the position
/// *is* the declaration `for` needs — it says "this call supplies a `next` for
/// `It`" — so the loop drives by calling that implicit parameter, and the
/// element type comes off its result.
#[test]
fn a_generic_pass_is_driven_by_for() {
    let src = "fn total<It>(it: Mut It, ?Yield<It, Int>) [] -> Int => it: Mut {\n\
               \x20   let sum = 0\n\
               \x20   for n in it {\n\
               \x20       sum = sum + n\n\
               \x20   }\n\
               \x20   return sum\n\
               }\n";
    assert!(messages(src).is_empty(), "{:?}", messages(src));
}

/// The element is the pass's to give: `next` returns `Emitted T` **by value**,
/// so the loop binding is an owned value and not a projection of the pass —
/// which is what lets a combinator move each element into its output.
#[test]
fn a_driven_element_is_owned_not_derived() {
    let src = "fn keep<It, T>(it: Mut It, ?Yield<It, T>) [] -> Mut List<T> => it: Mut {\n\
               \x20   let out = mut_list_of<T>()\n\
               \x20   for x in it {\n\
               \x20       add(out, x)\n\
               \x20   }\n\
               \x20   return out\n\
               }\n";
    assert!(messages(src).is_empty(), "{:?}", messages(src));
}

/// Without the spread there is nothing to drive with, and the diagnostic is the
/// ordinary not-iterable one [iter-resolve]: a bare type parameter says nothing.
#[test]
fn a_type_parameter_without_the_spread_is_not_iterable() {
    let src = "fn total<It>(it: Mut It) [] -> Int => it: Mut {\n\
               \x20   let sum = 0\n\
               \x20   for n in it {\n\
               \x20       sum = sum + 1\n\
               \x20   }\n\
               \x20   return sum\n\
               }\n";
    let msgs = messages(src);
    assert!(
        msgs.iter().any(|m| m.contains("`It` is not iterable")),
        "expected the not-iterable error, got: {msgs:?}"
    );
}

/// The spread's position has to take its state as `Mut`, for the same reason a
/// declared `next` does: advancing a pass mutates its position [iter-protocol].
#[test]
fn a_non_mut_position_cannot_be_driven() {
    let src = "fn total<It, T>(it: Mut It, ?next: (It) -> Emitted T | Finished) [] -> Int => it: Mut {\n\
               \x20   let sum = 0\n\
               \x20   for n in it {\n\
               \x20       sum = sum + 1\n\
               \x20   }\n\
               \x20   return sum\n\
               }\n";
    let msgs = messages(src);
    assert!(
        msgs.iter()
            .any(|m| m.contains("has to take its state as `Mut It`")),
        "expected the `Mut` requirement, got: {msgs:?}"
    );
}

/// [implicit-resolve] [fn-overload-scope] A program declaring its own pass
/// under a name std also uses — its own `ListYield` plus a `next` for it —
/// used to make every drive of that name ambiguous: implicit resolution
/// pooled core's `next` and the module's own without the scope ladder every
/// named call walks. The most specific rung wins now, so the module's own
/// `next` fills the spread and the program checks.
#[test]
fn an_own_pass_named_like_stds_resolves_to_the_own_next() {
    let src = "struct ListYield<T> : Yield<self, T> canbe Mut {\n\
               \x20   items: List<T>,\n\
               \x20   at: Int\n\
               }\n\
               fn mine<T>(items: List<T>) [] -> Mut ListYield<T> => !items {\n\
               \x20   return Mut ListYield<T> { items: items, at: 0 }\n\
               }\n\
               fn next<T>(p: Mut ListYield<T>) [] -> Emitted T | Finished => p: Mut {\n\
               \x20   let elem = get(p.items, p.at)\n\
               \x20   if elem is None {\n\
               \x20       return finished()\n\
               \x20   }\n\
               \x20   p.at = p.at + 1\n\
               \x20   return emitted(elem)\n\
               }\n\
               fn main() [] -> None {\n\
               \x20   let p = mine(list_of(1, 2))\n\
               \x20   for x in p {\n\
               \x20       let y = x\n\
               \x20   }\n\
               \x20   let q = mine(list_of(3, 4))\n\
               \x20   let doubled: Mut List<Int> = map(q, (n: Int) -> { return n + n })\n\
               \x20   return None\n\
               }\n";
    let msgs = messages(src);
    assert!(msgs.is_empty(), "expected a clean check, got: {msgs:?}");
}
