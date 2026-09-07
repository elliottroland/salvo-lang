//! [seq-iterable] [implicit-group] [implicit-infer] [fn-overload-specific] The sequence
//! functions — `map`, `filter`, `reduce` — and what makes them work over
//! *anything iterable*.
//!
//! There is no `Iterable` trait: `params Iterable<It, T> { fn iter(it: It) ->
//! Iter<T> }` is a bundle of implicit parameters, and a call fills it with
//! whichever `iter` fits its subject. Two inference facts carry the whole
//! design:
//!
//! * the *implicit* resolution has to feed back into the call's type
//!   arguments — `It` comes from the subject, and `T` comes from **which
//!   `iter` filled the implicit** — or a bare lambda would be typed against
//!   an unbound `T` and `U` would be undeterminable [call-type-args];
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
intrinsic type Int\n\
intrinsic type Bool\n\
intrinsic type Char\n\
intrinsic type Str\n\
intrinsic type Iter<T>\n\
intrinsic type List<T> canbe Mut\n\
intrinsic fn mutable_list<T>(...elems: T[]) [] -> [] Mut List<T>\n\
intrinsic fn list<T>(...elems: T[]) [] -> [] List<T>\n\
intrinsic fn add<T>(list: Mut List<T>, elem: T) [] -> [list: Mut] None\n\
intrinsic fn size<T>(list: List<T>) [] -> [list] Int\n\
intrinsic fn iter<T>(list: List<T>) [] -> [list] Iter<T>\n\
intrinsic fn iter<T>(array: T[]) [] -> [array] Iter<T>\n\
intrinsic fn iter(str: Str) [] -> [str] Iter<Char>\n\
intrinsic fn iter<T>(it: Iter<T>) [] -> [it] Iter<T>\n\
params Iterable<It, T> {\n\
    fn iter(it: It) -> Iter<T>\n\
}\n\
fn map<It, T, U>(xs: It, f: (T) -> U, ?Iterable<It, T>) [] -> [xs, f] Mut List<U> {\n\
    let out = mutable_list<U>()\n\
    for x in iter(xs) {\n\
        add(out, f(x))\n\
    }\n\
    return out\n\
}\n\
fn reduce<It, T, A>(xs: It, init: A, f: (A, T) -> A, ?Iterable<It, T>) [] -> [xs, f] A {\n\
    let acc = init\n\
    for x in iter(xs) {\n\
        acc = f(acc, x)\n\
    }\n\
    return acc\n\
}\n\
intrinsic fn map<T, U>(list: List<T>, f: (T) -> U) [] -> [list, f] Mut List<U>\n\
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
    checked(src).1.errors
}

fn messages(src: &str) -> Vec<String> {
    errors(src).iter().map(|d| d.message.clone()).collect()
}

fn probe(body: &str) -> String {
    format!("fn probe() -> [] None {{\n{body}\n}}\n")
}

/// Which `map` overload a call resolved to: `true` for the intrinsic `List`
/// fast path, `false` for the generic body.
fn picked_fast_path(src: &str, callee: &str) -> bool {
    let (program, out) = checked(src);
    assert!(out.errors.is_empty(), "unexpected errors: {:?}", out.errors);
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

// ===== the generic path: anything with an `iter` =====

/// [implicit-infer] A `List` subject: `It` from the argument, `T` from the
/// `iter` that fills the implicit, `U` from the lambda's body.
#[test]
fn a_list_subject_infers_everything() {
    let src = probe("    let xs = list(1, 2)\n    let ys: Mut List<Int> = map(xs, n -> n * 2)");
    assert!(messages(&src).is_empty(), "{:?}", messages(&src));
}

/// An **array** subject — the case that failed before implicit resolution
/// fed back into the call's type arguments, while the identical `List` call
/// worked.
#[test]
fn an_array_subject_infers_everything() {
    let src = probe("    let arr = [1, 2]\n    let ys: Mut List<Int> = map(arr, n -> n + 1)");
    assert!(messages(&src).is_empty(), "{:?}", messages(&src));
}

/// A `Str` subject iterates its *characters*, so the lambda's parameter is a
/// `Char` — which is only knowable from the `iter` that fills the implicit.
#[test]
fn a_str_subject_iterates_characters() {
    let src = probe("    let ys: Mut List<Bool> = map(\"ab\", c -> c == 'a')");
    assert!(messages(&src).is_empty(), "{:?}", messages(&src));
    // ... and the element really is a `Char`, not a `Str`.
    let bad = probe("    let ys: Mut List<Int> = map(\"ab\", c -> size(c))");
    assert!(!messages(&bad).is_empty(), "a `Char` is not a `List`");
}

/// An `Iter<T>` is iterable through the identity overload, which is what
/// makes a chain compose: `map`'s result is a `Mut List`, and that has an
/// `iter` too.
#[test]
fn iterators_and_chains_compose() {
    let src = probe(
        "    let xs = list(1, 2)\n    \
         let ys: Mut List<Int> = map(iter(xs), n -> n * 2)\n    \
         let zs: Mut List<Int> = map(map(xs, n -> n * 2), n -> n + 1)",
    );
    assert!(messages(&src).is_empty(), "{:?}", messages(&src));
}

/// A type of your own becomes iterable by declaring `fn iter` for it —
/// nothing else, and no trait.
#[test]
fn a_user_type_becomes_iterable_by_declaring_iter() {
    let src = format!(
        "struct Bag {{\n    items: List<Int>\n}}\n\n\
         fn iter(bag: Bag) -> [bag] Iter<Int> {{\n    return iter(bag.items)\n}}\n\n{}",
        probe(
            "    let b = Bag {items: list(1, 2)}\n    \
             let total: Int = reduce(b, 0, (a, x) -> a + x)"
        )
    );
    assert!(messages(&src).is_empty(), "{:?}", messages(&src));
}

/// A subject with no `iter` at all is an error naming the implicit — the
/// diagnostic [implicit-resolve] already had, now reached by the thing most
/// likely to trip on it.
#[test]
fn a_subject_with_no_iter_is_an_error() {
    let src = probe("    let ys = map(1, n -> n)");
    let msgs = messages(&src);
    assert!(
        msgs.iter().any(|m| m.contains("`iter`")),
        "the diagnostic should name the missing `iter`: {msgs:?}"
    );
}

// ===== the fast path, and what picks it =====

/// [fn-overload-specific] With a `List` subject the *intrinsic* overload
/// wins: `List<T>` is more specific than a bare `It`. This is the case O1
/// was taken for.
#[test]
fn a_list_subject_picks_the_fast_path() {
    let src = probe("    let xs = list(1, 2)\n    let ys: Mut List<Int> = map(xs, n -> n * 2)");
    assert!(picked_fast_path(&src, "map"), "expected the `List` overload");
}

/// And with any other subject the generic body takes it — the fast path is
/// not viable, so specificity never gets to prefer it.
#[test]
fn other_subjects_pick_the_generic_body() {
    let src = probe("    let arr = [1, 2]\n    let ys: Mut List<Int> = map(arr, n -> n + 1)");
    assert!(!picked_fast_path(&src, "map"), "expected the generic overload");
}

/// [fn-overload-specific] The lead candidate is re-narrowed *per argument*,
/// which is what makes the array case above work at all: the `List`
/// candidate leads on specificity, and typing the subject has to drop it
/// before the lambda is checked against `List<T>`'s element type.
#[test]
fn the_lead_narrows_before_the_lambda_is_typed() {
    // A `Str` subject with a `Char` lambda: if the `List` fast path had
    // still been leading, the lambda would have been typed against `T` from
    // `List<T>` — unbound — and `U` would not have been inferable.
    let src = probe("    let ys: Mut List<Bool> = map(\"ab\", c -> c == 'a')");
    assert!(messages(&src).is_empty(), "{:?}", messages(&src));
    assert!(!picked_fast_path(&src, "map"));
}
