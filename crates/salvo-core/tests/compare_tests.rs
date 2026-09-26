//! [cmp-groups] Comparison, equality and hashing as **params groups**.
//!
//! The ordering round's foundation (user decisions 2026-09-21): `Ordered`,
//! `Eq` and `Hashed` are ordinary `params` groups in `core.compare`, so a
//! capability is "a fn of this shape is in scope" and nothing more. Which
//! means these tests are mostly about the machinery the language already has
//! — [implicit-resolve] picking the canonical overload for the type a call
//! instantiates, [implicit-forward] colouring a generic fn that needs one,
//! and [group-obligation] checking a promise at a struct — applied to the
//! three groups. Nothing here is new *mechanism*; what is new is that the
//! three capabilities are expressed with it rather than being built in.
//!
//! The prelude mirrors `std/core/compare.sv`: the same three groups, and the
//! canonical overloads for the intrinsic types this file compares.

use std::path::Path;

use salvo_core::{check_program, resolve, Program, SourceSet, Symbols};

/// [intrinsic-std-only] Loaded as a *std* file, since only std writes
/// `intrinsic` — and `core.*` is implicitly visible, which is how a canonical
/// `cmp` is in scope without an import.
const STD_PRELUDE: &str = concat!(
    "export intrinsic type Int\n",
    "export intrinsic type Long\n",
    "export intrinsic type Bool\n",
    "export intrinsic type Double\n",
    "export intrinsic type Str canbe Mut\n",
    "export params Ordered<T> {\n    fn cmp(a: T, b: T) -> Int\n}\n",
    "export params Eq<T> {\n    fn eq(a: T, b: T) -> Bool\n}\n",
    "export params Hashed<T> {\n    fn hash(value: T) -> Long\n    fn eq(a: T, b: T) -> Bool\n}\n",
    "export intrinsic fn cmp(a: Int, b: Int) [] -> Int => a, b\n",
    "export intrinsic fn cmp(a: Str, b: Str) [] -> Int => a, b\n",
    "export intrinsic fn eq(a: Int, b: Int) [] -> Bool => a, b\n",
    "export intrinsic fn eq(a: Str, b: Str) [] -> Bool => a, b\n",
    "export intrinsic fn eq(a: Double, b: Double) [] -> Bool => a, b\n",
    "export intrinsic fn hash(value: Int) [] -> Long => value\n",
    "export intrinsic fn hash(value: Str) [] -> Long => value\n",
    "export intrinsic fn to_long(value: Int) [] -> Long => value\n",
);

fn errors(src: &str) -> Vec<String> {
    errors_in(&[("main.sv", src)])
}

/// The same over several user files, which is what the canonical rules need:
/// "the type's own file" and "imported with the type" are both about *files*.
fn errors_in(files: &[(&str, &str)]) -> Vec<String> {
    let mut sources = SourceSet::default();
    sources.add(
        "std/core/compare.sv",
        SourceSet::classify(Path::new("core/compare.sv")).unwrap(),
        STD_PRELUDE.to_string(),
        true,
    );
    for (name, src) in files {
        sources.add(
            *name,
            SourceSet::classify(Path::new(name)).unwrap(),
            src.to_string(),
            false,
        );
    }
    let mut modules = Vec::with_capacity(sources.files.len());
    for file in &sources.files {
        let (ast, diagnostics) = salvo_syntax::parse_module(&file.content);
        let parse_errors: Vec<_> = diagnostics.iter().filter(|d| d.is_error()).collect();
        assert!(
            parse_errors.is_empty(),
            "unexpected parse errors: {parse_errors:?}"
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

// ===== the capability resolves at a concrete type =====

/// A direct call picks the canonical overload for its argument types, like any
/// other overload [fn-overload] — the groups add nothing to that path.
#[test]
fn the_canonical_implementations_are_ordinary_overloads() {
    let errs = errors(
        r#"
fn describe(a: Str, b: Str) [] -> Int => a, b {
    if eq(a, b) {
        return 0
    }
    return cmp(a, b)
}

fn bucket(n: Int) [] -> Long => n {
    return hash(n)
}
"#,
    );
    assert!(errs.is_empty(), "unexpected errors: {errs:?}");
}

/// Dot notation reaches it too — `a.cmp(b)` *is* `cmp(a, b)` [fn-dot], which
/// is what makes a capability read like a method without being one.
#[test]
fn a_capability_reads_as_a_method() {
    let errs = errors(
        r#"
fn compare(a: Int, b: Int) [] -> Int => a, b {
    return a.cmp(b)
}
"#,
    );
    assert!(errs.is_empty(), "unexpected errors: {errs:?}");
}

// ===== the capability at a generic, through a group spread =====

/// [implicit-group] [implicit-resolve] A `?Ordered<T>` spread makes the
/// ordering a parameter, and the *call site* resolves it: at `Int` and at
/// `Str` the canonical overloads fit, and the generic body needs no bound.
#[test]
fn a_group_spread_is_filled_by_the_canonical_overload() {
    let errs = errors(
        r#"
fn min_of<T>(a: T, b: T, ?Ordered<T>) [] -> T {
    if cmp(a, b) <= 0 {
        return a
    }
    return b
}

fn pick() [] -> Int {
    let s = "b"
    let t = "ab"
    let smaller = min_of(s, t)
    return min_of(2, 3)
}
"#,
    );
    assert!(errs.is_empty(), "unexpected errors: {errs:?}");
}

/// All three groups in one signature, which is what a keyed collection's
/// operations will ask for (`?Hashed<K>` plus `?Eq<K>`): the members are
/// implicit parameters in their own right, so two spreads compose.
#[test]
fn several_groups_compose_in_one_signature() {
    let errs = errors(
        r#"
fn keyed<T>(a: T, b: T, ?Eq<T>, ?Hashed<T>) [] -> Bool => a, b {
    if eq(a, b) {
        return hash(a) == hash(b)
    }
    return true
}

fn use_it() [] -> Bool {
    return keyed(1, 2)
}
"#,
    );
    assert!(errs.is_empty(), "unexpected errors: {errs:?}");
}

/// [implicit-forward] A generic fn that asks for nothing cannot compare:
/// nothing about an opaque `T` is knowable [call-resolve], so there is no
/// default to resolve and the diagnostic names the two remedies. This is the
/// colouring the ordering operators will inherit in the next step.
#[test]
fn a_generic_fn_without_the_capability_cannot_compare() {
    let errs = errors(
        r#"
fn min_of<T>(a: T, b: T) [] -> T {
    if cmp(a, b) <= 0 {
        return a
    }
    return b
}
"#,
    );
    assert!(
        errs.iter().any(|e| e.contains("cmp")),
        "expected a missing-`cmp` diagnostic, got: {errs:?}"
    );
}

/// The capability forwards by name and type, not by how it was declared
/// [implicit-forward]: a `?Ordered<T>` spread here fills an individually
/// declared `?cmp` there.
#[test]
fn the_capability_forwards_through_a_generic_call() {
    let errs = errors(
        r#"
fn ranked<T>(a: T, b: T, ?cmp: (T, T) -> Int) [] -> Bool => a, b {
    return cmp(a, b) < 0
}

fn sorted_pair<T>(a: T, b: T, ?Ordered<T>) [] -> Bool => a, b {
    return ranked(a, b)
}

fn go() [] -> Bool {
    return sorted_pair(1, 2)
}
"#,
    );
    assert!(errs.is_empty(), "unexpected errors: {errs:?}");
}

// ===== a type of your own =====

/// A struct joins in by declaring the function: no trait, no `impl`, and the
/// overload is reached by the same resolution the primitives use.
#[test]
fn a_struct_joins_a_capability_by_declaring_a_fn() {
    let errs = errors(
        r#"
struct Person {
    name: Str,
    age: Int
}

fn cmp(a: Person, b: Person) [] -> Int => a, b {
    return cmp(a.age, b.age)
}

fn older(a: Person, b: Person) [] -> Bool => a, b {
    return cmp(a, b) > 0
}

fn min_of<T>(a: T, b: T, ?Ordered<T>) [] -> T {
    if cmp(a, b) <= 0 {
        return a
    }
    return b
}

fn younger(a: Person, b: Person) [] -> Person => !a, !b {
    return min_of(a, b)
}
"#,
    );
    assert!(errs.is_empty(), "unexpected errors: {errs:?}");
}

/// [group-obligation] The obligation form works on the three groups as it does
/// on `Yield`: the promise is checked **at the struct**, so a missing `cmp` is
/// an error where the promise is written rather than at some distant call.
/// (The `default` spelling that *generates* the implementation is the round's
/// next step; this is the hand-written half.)
#[test]
fn an_obligation_is_checked_at_the_struct() {
    let errs = errors(
        r#"
struct Point : Ordered<self> {
    x: Int,
    y: Int
}
"#,
    );
    assert!(
        errs.iter().any(|e| e.contains("cmp")),
        "expected the unmet obligation to name `cmp`, got: {errs:?}"
    );

    let ok = errors(
        r#"
struct Point : Ordered<self>, Eq<self> {
    x: Int,
    y: Int
}

fn cmp(a: Point, b: Point) [] -> Int => a, b {
    return cmp(a.x, b.x)
}

fn eq(a: Point, b: Point) [] -> Bool => a, b {
    return eq(a.x, b.x)
}
"#,
    );
    assert!(ok.is_empty(), "unexpected errors: {ok:?}");
}

// ===== what has no canonical implementation =====

/// `Double` has an `eq` and no `cmp`: `NaN` ties with nothing, so no total
/// order exists — the same reason a sorted collection of floats stays refused
/// [col-sorted]. The refusal is the ordinary "no overload" one.
#[test]
fn floats_compare_for_equality_but_have_no_ordering() {
    let ok = errors(
        r#"
fn same(a: Double, b: Double) [] -> Bool => a, b {
    return eq(a, b)
}
"#,
    );
    assert!(ok.is_empty(), "unexpected errors: {ok:?}");

    let errs = errors(
        r#"
fn ranked(a: Double, b: Double) [] -> Int => a, b {
    return cmp(a, b)
}
"#,
    );
    assert!(
        errs.iter().any(|e| e.contains("cmp")),
        "expected no `cmp` for `Double`, got: {errs:?}"
    );
}

/// A capability a type does not have is refused at the *call*, naming the fn
/// — which is what makes "declare one" the obvious remedy.
#[test]
fn a_type_without_the_capability_is_refused_at_the_call() {
    let errs = errors(
        r#"
struct Opaque {
    at: Int
}

fn ranked(a: Opaque, b: Opaque) [] -> Int => a, b {
    return cmp(a, b)
}
"#,
    );
    assert!(
        errs.iter().any(|e| e.contains("cmp")),
        "expected the missing `cmp` to be named, got: {errs:?}"
    );
}

// ===== [fn-attached] a function declared *on* a type =====

/// A fn written **inside** the struct body is an ordinary overload — bare calls
/// and dot-notation reach it [fn-dot] — and it is declared on the type.
#[test]
fn a_fn_inside_the_body_is_an_ordinary_overload() {
    let errs = errors(
        r#"
struct Person {
    name: Str,
    age: Int

    fn cmp(a: Person, b: Person) [] -> Int => a, b {
        return cmp(a.age, b.age)
    }
}

fn older(a: Person, b: Person) [] -> Bool => a, b {
    return a.cmp(b) > 0
}
"#,
    );
    assert!(errs.is_empty(), "unexpected errors: {errs:?}");
}

/// [fn-attached] The other route: the file's fulfilment of an obligation the
/// struct declares. Nothing moves — the fn stays top-level — and it is attached
/// all the same.
#[test]
fn an_obligation_fulfilment_is_attached() {
    let errs = errors(
        r#"
struct Person : Ordered<self> {
    name: Str,
    age: Int
}

fn cmp(a: Person, b: Person) [] -> Int => a, b {
    return cmp(a.age, b.age)
}

fn older(a: Person, b: Person) [] -> Bool => a, b {
    return a.cmp(b) > 0
}
"#,
    );
    assert!(errs.is_empty(), "unexpected errors: {errs:?}");
}

/// [fn-attached] [fn-attached] The selector still *reads* the attachment:
/// `cmp@Person` names the fn declared on `Person`, at a call and as a value,
/// which is what `cmp = cmp@Person` needs [implicit-override]. Only the
/// declaration spelling changed.
#[test]
fn an_attached_fn_is_named_by_the_type_selector() {
    let errs = errors(
        r#"
struct Person {
    name: Str,
    age: Int

    fn cmp(a: Person, b: Person) [] -> Int => a, b {
        return cmp(a.age, b.age)
    }
}

fn min_of<T>(a: T, b: T, ?Ordered<T>) [] -> T {
    if cmp(a, b) <= 0 {
        return a
    }
    return b
}

fn pick(a: Person, b: Person) [] -> Person => !a, !b {
    let explicit = cmp@Person(a, b)
    return min_of(a, b, cmp = cmp@Person)
}
"#,
    );
    assert!(errs.is_empty(), "unexpected errors: {errs:?}");
}

/// [fn-attached] It **travels with the type**: a module that imports `Person`
/// and nothing else can still compare two, which is what closes
/// [implicit-resolve]'s per-call-site visibility hole. Both routes travel.
#[test]
fn an_attached_fn_is_imported_with_its_type() {
    let inner = errors_in(&[
        (
            "people.sv",
            r#"
export struct Person {
    name: Str,
    age: Int

    fn cmp(a: Person, b: Person) [] -> Int => a, b {
        return cmp(a.age, b.age)
    }
}
"#,
        ),
        (
            "main.sv",
            r#"
import people.Person

fn min_of<T>(a: T, b: T, ?Ordered<T>) [] -> T {
    if cmp(a, b) <= 0 {
        return a
    }
    return b
}

fn go(a: Person, b: Person) [] -> Person => !a, !b {
    let direct = cmp(a, b)
    return min_of(a, b)
}
"#,
        ),
    ]);
    assert!(inner.is_empty(), "unexpected errors: {inner:?}");

    let fulfilment = errors_in(&[
        (
            "people.sv",
            r#"
export struct Person : Ordered<self> {
    name: Str,
    age: Int
}

export fn cmp(a: Person, b: Person) [] -> Int => a, b {
    return cmp(a.age, b.age)
}
"#,
        ),
        (
            "main.sv",
            r#"
import people.Person

fn min_of<T>(a: T, b: T, ?Ordered<T>) [] -> T {
    if cmp(a, b) <= 0 {
        return a
    }
    return b
}

fn go(a: Person, b: Person) [] -> Person => !a, !b {
    let direct = cmp(a, b)
    return min_of(a, b)
}
"#,
        ),
    ]);
    assert!(fulfilment.is_empty(), "unexpected errors: {fulfilment:?}");
}

/// [fn-attached] [mod-export] A **detached** fulfilment of an exported type's
/// obligation has to `export` too: it is declared on a type every importer can
/// use, and a private one would be unreachable there.
#[test]
fn a_detached_fulfilment_must_export_with_its_type() {
    let private_fn = errors(
        r#"
export struct Point : Ordered<self> {
    x: Int
}

fn cmp(a: Point, b: Point) [] -> Int => a, b {
    return cmp(a.x, b.x)
}
"#,
    );
    assert!(
        private_fn.iter().any(|e| e.contains("has to say so")),
        "expected the export rule, got: {private_fn:?}"
    );

    let matching = errors(
        r#"
export struct Point : Ordered<self> {
    x: Int
}

export fn cmp(a: Point, b: Point) [] -> Int => a, b {
    return cmp(a.x, b.x)
}
"#,
    );
    assert!(matching.is_empty(), "unexpected errors: {matching:?}");

    // A *private* type needs nothing: there is no importer to hide it from.
    let private_type = errors(
        r#"
struct Point : Ordered<self> {
    x: Int
}

fn cmp(a: Point, b: Point) [] -> Int => a, b {
    return cmp(a.x, b.x)
}
"#,
    );
    assert!(private_type.is_empty(), "unexpected errors: {private_type:?}");
}

/// [fn-overload-ambiguous] An attached fn is an ordinary overload, so two that
/// fit are an ordinary ambiguity: refused, naming the places. Until 2026-09-26
/// this needed a carve-out, because scope otherwise decided in silence.
#[test]
fn ambiguity_around_a_canonical_is_an_error_on_every_rung() {
    let files = &[
        (
            "people.sv",
            r#"
export struct Person {
    name: Str,
    age: Int

    fn cmp(a: Person, b: Person) [] -> Int => a, b {
        return cmp(a.age, b.age)
    }
}

"#,
        ),
        (
            "main.sv",
            r#"
import people.Person

// This module's own `cmp` for the same shape: today's ladder would take it
// silently, because `Own` outranks `Import`.
fn cmp(a: Person, b: Person) [] -> Int => a, b {
    return cmp(a.name, b.name)
}

fn min_of<T>(a: T, b: T, ?Ordered<T>) [] -> T {
    if cmp(a, b) <= 0 {
        return a
    }
    return b
}

fn go(a: Person, b: Person) [] -> Person => !a, !b {
    let direct = cmp(a, b)
    return min_of(a, b)
}
"#,
        ),
    ];
    let errs = errors_in(files);
    // The *call* is refused and names the places [fn-overload-ambiguous]: the
    // rule is general now, so an attached fn needs no carve-out of its own.
    assert!(
        errs.iter()
            .any(|e| e.contains("ambiguous call to `cmp(Person, Person)`")
                && e.contains("cmp@people")
                && e.contains("cmp@main")),
        "expected the call ambiguity to name both places, got: {errs:?}"
    );
}

/// The remedy the diagnostics name, which is the point of the selector being
/// one token: both spellings resolve.
#[test]
fn the_selector_resolves_an_ambiguity_around_a_canonical() {
    let errs = errors_in(&[
        (
            "people.sv",
            r#"
export struct Person {
    name: Str,
    age: Int

    fn cmp(a: Person, b: Person) [] -> Int => a, b {
        return cmp(a.age, b.age)
    }
}

"#,
        ),
        (
            "main.sv",
            r#"
import people.Person

fn cmp(a: Person, b: Person) [] -> Int => a, b {
    return cmp(a.name, b.name)
}

fn min_of<T>(a: T, b: T, ?Ordered<T>) [] -> T {
    if cmp(a, b) <= 0 {
        return a
    }
    return b
}

fn go(a: Person, b: Person) [] -> Person => !a, !b {
    let by_age = cmp@Person(a, b)
    let by_name = cmp@main(a, b)
    return min_of(a, b, cmp = cmp@Person)
}
"#,
        ),
    ]);
    assert!(errs.is_empty(), "unexpected errors: {errs:?}");
}

/// [fn-attached] The canonical is the **default selection** even where a
/// nearer rung has a fitting candidate of a *different* shape: only candidates
/// that fit compete, so an unrelated `cmp` in this module is not an ambiguity.
#[test]
fn an_unrelated_overload_is_not_an_ambiguity() {
    let errs = errors_in(&[
        (
            "people.sv",
            r#"
export struct Person {
    name: Str,
    age: Int

    fn cmp(a: Person, b: Person) [] -> Int => a, b {
        return cmp(a.age, b.age)
    }
}

"#,
        ),
        (
            "main.sv",
            r#"
import people.Person

struct Box {
    at: Int
}

fn cmp(a: Box, b: Box) [] -> Int => a, b {
    return cmp(a.at, b.at)
}

fn go(a: Person, b: Person) [] -> Int => a, b {
    return cmp(a, b)
}
"#,
        ),
    ]);
    assert!(errs.is_empty(), "unexpected errors: {errs:?}");
}

/// A lowercase name after `@` on a *declaration* is a mistake with its own
/// message: a fn is already scoped to its module, so the only `@` that means
/// [fn-attached] A `@` on a *declaration* is refused outright now: attachment
/// is declaring the function on the type, not selecting from it. The diagnostic
/// names both routes.
#[test]
fn a_declaration_takes_no_selector() {
    let (_m, diags) = salvo_syntax::parse_module(
        "struct Person {\n    name: Str\n}\n\nfn cmp@Person(a: Person, b: Person) -> Int => a, b {\n    return 0\n}\n",
    );
    let msgs: Vec<String> = diags
        .iter()
        .filter(|d| d.is_error())
        .map(|d| d.message.clone())
        .collect();
    assert!(
        msgs.iter().any(|m| m.contains("does not take a `@` selector")
            && m.contains("inside `Person`'s body")
            && m.contains("obligations")),
        "got: {msgs:?}"
    );
}

// ===== [cmp-auto] the generated structural implementations =====

/// `: auto Ordered<self>` writes the members: `cmp` resolves at the type,
/// fills a `?Ordered<T>` position, and is reached by the canonical selector
/// like a hand-written one [fn-attached].
#[test]
fn a_default_obligation_generates_the_members() {
    let errs = errors(
        r#"
struct Point : auto Ordered<self>, auto Hashed<self> {
    x: Int,
    y: Int
}

fn min_of<T>(a: T, b: T, ?Ordered<T>) [] -> T {
    if cmp(a, b) <= 0 {
        return a
    }
    return b
}

fn go(p: Point, q: Point) [] -> Bool => p, q {
    let order = cmp(p, q)
    let same = eq(p, q)
    let digest = hash(p)
    let selected = cmp@Point(p, q)
    return eq(order, selected)
}

// `min_of` answers one of its arguments, so it **moves** them [deduce-infer]:
// filling its `?cmp` with the canonical is the point here.
fn smaller(p: Point, q: Point) [] -> Point => !p, !q {
    return min_of(p, q, cmp = cmp@Point)
}
"#,
    );
    assert!(errs.is_empty(), "unexpected errors: {errs:?}");
}

/// [cmp-auto] **`auto Hashed<self>` brings `eq` with it, and `auto
/// Ordered<self>` does not** (user decision 2026-09-22, revising decision 7): a
/// hash container buckets by `hash` and confirms by `eq`, so the pair is the
/// unit — while no sorted container consults equality at all, so an `eq` in
/// `Ordered` would be a member nothing reads.
#[test]
fn hashing_brings_eq_and_ordering_does_not() {
    // `auto Hashed<self>` generates `hash` *and* `eq`.
    let errs = errors(
        r#"
struct Key : auto Hashed<self> {
    x: Int
}

fn same(a: Key, b: Key) [] -> Bool => a, b {
    return eq(a, b)
}
"#,
    );
    assert!(errs.is_empty(), "unexpected errors: {errs:?}");

    // `auto Ordered<self>` generates only `cmp`, so equality is a separate ask.
    let errs = errors(
        r#"
struct Point : auto Ordered<self> {
    x: Int
}

fn same(a: Point, b: Point) [] -> Bool => a, b {
    return eq(a, b)
}
"#,
    );
    assert!(
        errs.iter()
            .any(|e| e.contains("no matching overload for `eq(Point, Point)`")),
        "ordering alone should not supply equality, got: {errs:?}"
    );

    // …and asking for it is one line, which is what the function-level form is
    // for: some members generated, others not.
    let errs = errors(
        r#"
struct Point : Ordered<self>, Eq<self> {
    x: Int

    auto fn eq(a: Point, b: Point) [] -> Bool => a, b

    auto fn cmp(a: Point, b: Point) [] -> Int => a, b
}

fn same(a: Point, b: Point) [] -> Bool => a, b {
    return eq(a, b) && cmp(a, b) == 0
}
"#,
    );
    assert!(errs.is_empty(), "unexpected errors: {errs:?}");
}

/// [cmp-auto] `default` is legal only where the compiler has a generator: on
/// any other group the word would promise an implementation nothing provides.
#[test]
fn default_is_refused_on_a_group_with_no_generator() {
    let errs = errors(
        r#"
params Step<It, T> {
    fn advance(it: Mut It) -> T
}

struct Counter : auto Step<self, Int> canbe Mut {
    at: Int
}
"#,
    );
    assert!(
        errs.iter()
            .any(|e| e.contains("`cmp`, `eq`, `hash`")),
        "expected the generable members to be named, got: {errs:?}"
    );
}

/// The generator writes the signature over the declaring type, so the argument
/// is `self` [group-self].
#[test]
fn a_default_obligation_takes_self() {
    let errs = errors(
        r#"
struct Point : auto Ordered<Int> {
    x: Int
}
"#,
    );
    assert!(
        errs.iter().any(|e| e.contains("its argument is `self`")),
        "expected the `self` rule to be named, got: {errs:?}"
    );
}

/// [cmp-auto] `default` inherits today's validation, because it inherits
/// today's lowering: a float field has no hash the two backends agree on, and a
/// mutable struct cannot be a key. Both are reported at the clause.
#[test]
fn a_default_obligation_validates_the_fields() {
    let float_field = errors(
        r#"
struct Sample : auto Hashed<self> {
    at: Double
}
"#,
    );
    assert!(
        float_field
            .iter()
            .any(|e| e.contains("auto fn hash") && e.contains("not hashed")),
        "expected the float field to be refused, got: {float_field:?}"
    );

    let mutable = errors(
        r#"
struct Counter : auto Ordered<self> canbe Mut {
    at: Int
}
"#,
    );
    assert!(
        mutable
            .iter()
            .any(|e| e.contains("auto fn cmp") && e.contains("canbe Mut")),
        "expected the mutable struct to be refused, got: {mutable:?}"
    );
}

/// [cmp-auto] [implicit-group] `Hashed<T>` declares the **pair** a hash
/// container needs — `hash` to bucket, `eq` to confirm the bucket hit — and two
/// spreads asking for the same position ask for one parameter (user decisions
/// 2026-09-22). `?Eq<T>` beside `?Hashed<T>` is one `eq`, not a collision.
#[test]
fn overlapping_spreads_merge() {
    let errs = errors(
        r#"
fn digest_agrees<T>(a: T, b: T, ?Eq<T>, ?Hashed<T>) [] -> Bool => a, b {
    if eq(a, b) {
        return hash(a) == hash(b)
    }
    return true
}

fn probe(x: Int, y: Int) [] -> Bool => x, y {
    return digest_agrees(x, y)
}
"#,
    );
    assert!(errs.is_empty(), "unexpected errors: {errs:?}");
}

/// …while a name clash at two *different* types stays an error: nothing tells
/// those two apart, and the message names both types and the remedy.
#[test]
fn a_clash_at_two_types_is_still_an_error() {
    let errs = errors(
        r#"
params Weird<T> {
    fn eq(a: T, b: T) -> Int
}

fn probe<T>(a: T, b: T, ?Eq<T>, ?Weird<T>) [] -> Bool => a, b {
    return eq(a, b)
}
"#,
    );
    assert!(
        errs.iter()
            .any(|e| e.contains("two different types") && e.contains("distinct names")),
        "expected the clash to be reported: {errs:?}"
    );
}

/// [cmp-auto] A spread resolves **every** member, used or not (decision 19), so
/// `?Hashed<T>` at a type with a `hash` and no `eq` is the ordinary
/// missing-implicit error naming `eq`. Asking for less is a narrower spread, or
/// the members written individually.
#[test]
fn a_spread_resolves_every_member() {
    let errs = errors(
        r#"
struct Key {
    x: Int

    auto fn hash(value: Key) [] -> Long => value
}

fn bucket<T>(v: T, ?Hashed<T>) [] -> Long => v {
    return hash(v)
}

fn probe(k: Key) [] -> Long => k {
    return bucket(k)
}
"#,
    );
    assert!(
        errs.iter().any(|e| e.contains("`eq`")),
        "expected the unfilled `eq` to be named: {errs:?}"
    );

    // Declaring the member it actually needs asks for less, and works.
    let errs = errors(
        r#"
struct Key {
    x: Int

    auto fn hash(value: Key) [] -> Long => value
}

fn bucket<T>(v: T, ?hash: (T) -> Long) [] -> Long => v {
    return hash(v)
}

fn probe(k: Key) [] -> Long => k {
    return bucket(k)
}
"#,
    );
    assert!(errs.is_empty(), "unexpected errors: {errs:?}");
}

/// [cmp-auto] The **function-level** form, and the reason it exists (user
/// decision 2026-09-22): a type generates the members it wants generated and
/// writes the one it wants written. Here `Person` is ordered by name, hashed
/// structurally, and equal exactly when it ties — which no single obligation
/// clause can say.
#[test]
fn auto_fns_mix_with_hand_written_ones() {
    let errs = errors(
        r#"
struct Person : Ordered<self>, Hashed<self> {
    name: Str,
    age: Int

    fn eq(a: Person, b: Person) [] -> Bool => a, b {
        return cmp(a, b) == 0
    }

    auto fn hash(value: Person) [] -> Long => value

    auto fn cmp(a: Person, b: Person) [] -> Int => a, b
}

fn probe(a: Person, b: Person) [] -> Bool => a, b {
    return a == b && a < b && hash(a) == hash(b)
}
"#,
    );
    assert!(errs.is_empty(), "unexpected errors: {errs:?}");
}

/// [cmp-auto] `auto Group<self>` is sugar for exactly those declarations, so
/// the two spellings are interchangeable — and mixing them on one member is the
/// ordinary duplicate.
#[test]
fn the_clause_and_the_fn_form_are_one_thing() {
    let errs = errors(
        r#"
struct A : auto Hashed<self> {
    x: Int
}

struct B {
    x: Int

    auto fn eq(a: B, b: B) [] -> Bool => a, b

    auto fn hash(value: B) [] -> Long => value
}

fn probe(p: A, q: A, r: B, s: B) [] -> Bool => p, q, r, s {
    return eq(p, q) && eq(r, s) && hash(p) == hash(r)
}
"#,
    );
    assert!(errs.is_empty(), "unexpected errors: {errs:?}");

    let dup = errors(
        r#"
struct A : auto Hashed<self> {
    x: Int

    auto fn hash(value: A) [] -> Long => value
}

"#,
    );
    assert!(
        dup.iter().any(|e| e.contains("hash")),
        "expected the duplicate to be reported: {dup:?}"
    );
}

/// [cmp-auto] What an `auto fn` must be: `@`-scoped to a visible struct, named
/// after a member the compiler can write, bodiless, and of the member's own
/// shape. Each refusal names the remedy rather than leaving a body unwritten.
#[test]
fn an_auto_fn_is_checked_at_its_declaration() {
    // Not declared on a type: there are no fields to read.
    let errs = errors("auto fn cmp(a: Int, b: Int) [] -> Int => a, b
");
    assert!(
        errs.iter().any(|e| e.contains("declared *on* one")),
        "expected the attachment requirement: {errs:?}"
    );

    // A member the compiler has no generator for.
    let errs = errors(
        r#"
struct Point {
    x: Int

    auto fn describe(p: Point) [] -> Str => p
}

"#,
    );
    assert!(
        errs.iter()
            .any(|e| e.contains("`cmp`, `eq`, `hash`") && e.contains("describe")),
        "expected the generable members to be named: {errs:?}"
    );

    // A body, which the compiler was going to write.
    let errs = errors(
        r#"
struct Point {
    x: Int

    auto fn cmp(a: Point, b: Point) [] -> Int => a, b {
        return 0
    }
}

"#,
    );
    assert!(
        errs.iter().any(|e| e.contains("takes none")),
        "expected the body to be refused: {errs:?}"
    );

    // The wrong shape for the member it names.
    let errs = errors(
        r#"
struct Point {
    x: Int

    auto fn hash(a: Point, b: Point) [] -> Long => a, b
}

"#,
    );
    assert!(
        errs.iter().any(|e| e.contains("1 parameter")),
        "expected the arity to be named: {errs:?}"
    );

    let errs = errors(
        r#"
struct Point {
    x: Int

    auto fn cmp(a: Point, b: Point) [] -> Bool => a, b
}

"#,
    );
    assert!(
        errs.iter().any(|e| e.contains("must be declared")),
        "expected the signature to be named: {errs:?}"
    );

    // An `auto` obligation on a type with no fields to read: the generator has
    // nothing to write the body from, and only a struct has fields.
    let errs = errors("auto fn cmp(a: Str, b: Str) [] -> Int => a, b
");
    assert!(
        errs.iter().any(|e| e.contains("declared *on* one")),
        "expected the attachment requirement: {errs:?}"
    );
}

/// A hand-written member of the same shape beside a generated one is the
/// ordinary duplicate, with the message naming the half to delete.
#[test]
fn a_hand_written_member_beside_a_generated_one_is_a_duplicate() {
    let errs = errors(
        r#"
struct Point : auto Eq<self> {
    x: Int

    fn eq(a: Point, b: Point) [] -> Bool => a, b {
        return eq(a.x, b.x)
    }
}

"#,
    );
    assert!(
        errs.iter()
            .any(|e| e.contains("is declared twice") && e.contains("remove `default`")),
        "expected the duplicate to name both remedies, got: {errs:?}"
    );
}

/// A generic struct's generated members are generic too, over the struct's own
/// parameters — and a type variable is not checked at the declaration
/// [col-hashed-ordered], so generic code over keyed collections stays writable.
#[test]
fn a_generic_struct_generates_generic_members() {
    let errs = errors(
        r#"
struct Box<T> : auto Eq<self> {
    item: T
}

fn same(a: Box<Int>, b: Box<Int>) [] -> Bool => a, b {
    return eq(a, b)
}
"#,
    );
    assert!(errs.is_empty(), "unexpected errors: {errs:?}");
}

/// The obligation is still checked at the struct [group-obligation]: `default`
/// satisfies it by generating, and a bare `Eq<self>` with nothing to satisfy it
/// still fails — the two spellings answer the same question.
#[test]
fn default_satisfies_the_obligation_it_is_written_on() {
    let generated = errors(
        r#"
struct Point : auto Eq<self> {
    x: Int
}
"#,
    );
    assert!(generated.is_empty(), "unexpected errors: {generated:?}");

    let bare = errors(
        r#"
struct Point : Eq<self> {
    x: Int
}
"#,
    );
    assert!(
        bare.iter().any(|e| e.contains("eq")),
        "expected the unmet obligation to name `eq`, got: {bare:?}"
    );
}

// ===== [op-order] [op-equality] the operators through the groups =====

/// `a < b` is `cmp(a, b) < 0` and `a == b` is `eq(a, b)`, so both work wherever
/// the function is — including at a `Str`, which had **no** ordering before
/// (the operator was refused): `cmp(Str, Str)` is code-point order on both
/// backends [kt-ordered].
#[test]
fn the_operators_resolve_through_the_groups() {
    let errs = errors(
        r#"
struct Point : auto Ordered<self>, auto Eq<self> {
    x: Int
}

fn probe(p: Point, q: Point, s: Str, t: Str) [] -> Bool => p, q, s, t {
    let structs = p < q
    let strs = s <= t
    let both = p == q
    let nope = s != t
    // Numerics keep the native path, mixed widths included [op-promote].
    let numbers = 1 < 2 && 1 <= to_long(2) && 1 == 1
    return structs && strs && both && nope && numbers
}
"#,
    );
    assert!(errs.is_empty(), "unexpected errors: {errs:?}");
}

/// [op-order] The hole this closes: comparing an **unconstrained generic** used
/// to compile silently (`op_lenient` included `Ty::Var` — "a documented
/// leftover") and emit `a < b` on a boundless generic, which rustc then refused.
/// It is now the ordinary missing-capability error, and the remedy it names is
/// the signature.
#[test]
fn comparing_an_unconstrained_generic_is_an_error() {
    let errs = errors(
        r#"
fn largest<T>(a: T, b: T) [] -> Bool => a, b {
    return a < b
}
"#,
    );
    assert!(
        errs.iter()
            .any(|e| e.contains("`<` on `T`") && e.contains("?Ordered<T>")),
        "expected the missing-capability error naming the spread, got: {errs:?}"
    );

    let with_capability = errors(
        r#"
fn largest<T>(a: T, b: T, ?Ordered<T>) [] -> Bool => a, b {
    return a < b
}

fn go() [] -> Bool {
    return largest(1, 2)
}
"#,
    );
    assert!(
        with_capability.is_empty(),
        "unexpected errors: {with_capability:?}"
    );
}

/// [op-equality] The same for equality, which is the half that makes it *opt-in*
/// — and the same colouring at a generic.
#[test]
fn equality_is_opt_in_at_every_type() {
    let missing = errors(
        r#"
struct Note {
    text: Str
}

fn same(a: Note, b: Note) [] -> Bool => a, b {
    return a == b
}
"#,
    );
    assert!(
        missing
            .iter()
            .any(|e| e.contains("`==` on `Note`") && e.contains(": auto")),
        "expected the opt-in to be named, got: {missing:?}"
    );

    let generic = errors(
        r#"
fn same<T>(a: T, b: T) [] -> Bool => a, b {
    return a == b
}
"#,
    );
    assert!(
        generic.iter().any(|e| e.contains("?Eq<T>")),
        "expected the spread to be named, got: {generic:?}"
    );
}

/// [cmp-auto] `canbe hashed` and `canbe ordered` are **gone**, and the
/// diagnostic names what replaced them rather than reporting an unknown opt-in.
#[test]
fn the_canbe_optins_are_deleted() {
    for (written, replacement) in [
        ("canbe hashed", ": auto Hashed<self>"),
        ("canbe ordered", ": auto Ordered<self>"),
    ] {
        let errs = errors(&format!("struct Point {written} {{\n    x: Int\n}}\n"));
        assert!(
            errs.iter()
                .any(|e| e.contains("no longer exists") && e.contains(replacement)),
            "expected `{written}` to name `{replacement}`, got: {errs:?}"
        );
    }
}

/// [op-order] Comparing two different base types is still refused — the check
/// runs before resolution, so the message is about the *operands* rather than a
/// missing function.
#[test]
fn the_operands_must_still_be_one_type() {
    let errs = errors(
        r#"
struct A : auto Eq<self> {
    v: Int
}

struct B : auto Eq<self> {
    v: Int
}

fn probe(a: A, b: B) [] -> Bool => a, b {
    return a == b
}
"#,
    );
    assert!(
        errs.iter().any(|e| e.contains("must be the same type")),
        "expected the same-type rule, got: {errs:?}"
    );
}
