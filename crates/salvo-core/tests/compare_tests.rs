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
    "export params Hashed<T> {\n    fn hash(value: T) -> Long\n}\n",
    "export intrinsic fn cmp(a: Int, b: Int) [] -> Int => a, b\n",
    "export intrinsic fn cmp(a: Str, b: Str) [] -> Int => a, b\n",
    "export intrinsic fn eq(a: Int, b: Int) [] -> Bool => a, b\n",
    "export intrinsic fn eq(a: Str, b: Str) [] -> Bool => a, b\n",
    "export intrinsic fn eq(a: Double, b: Double) [] -> Bool => a, b\n",
    "export intrinsic fn hash(value: Int) [] -> Long => value\n",
    "export intrinsic fn hash(value: Str) [] -> Long => value\n",
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

// ===== [cmp-canonical] the canonical implementation, `@`-scoped to its type =====

/// `fn cmp@Person(…)` is an ordinary overload — bare calls and dot-notation
/// reach it [fn-dot] — declared in the type's own file.
#[test]
fn a_canonical_is_an_ordinary_overload() {
    let errs = errors(
        r#"
struct Person {
    name: Str,
    age: Int
}

fn cmp@Person(a: Person, b: Person) [] -> Int => a, b {
    return cmp(a.age, b.age)
}

fn older(a: Person, b: Person) [] -> Bool => a, b {
    return a.cmp(b) > 0
}
"#,
    );
    assert!(errs.is_empty(), "unexpected errors: {errs:?}");
}

/// The selector spelling **is** the declaration spelling: `cmp@Person` names it
/// at a call, and — following the *module* precedent [fn-overload-at] rather
/// than [effect-at]'s call-only form — as a value, which is what
/// `cmp = cmp@Person` needs [implicit-override].
#[test]
fn a_canonical_is_named_by_the_selector_it_is_declared_with() {
    let errs = errors(
        r#"
struct Person {
    name: Str,
    age: Int
}

fn cmp@Person(a: Person, b: Person) [] -> Int => a, b {
    return cmp(a.age, b.age)
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

/// [cmp-canonical] It **travels with the type**: a module that imports `Person`
/// and nothing else can still compare two, which is what closes
/// [implicit-resolve]'s per-call-site visibility hole.
#[test]
fn a_canonical_is_imported_with_its_type() {
    let errs = errors_in(&[
        (
            "people.sv",
            r#"
export struct Person {
    name: Str,
    age: Int
}

export fn cmp@Person(a: Person, b: Person) [] -> Int => a, b {
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
    assert!(errs.is_empty(), "unexpected errors: {errs:?}");
}

/// The rule that makes "imported with the type" applicable without searching
/// the program: a canonical lives in the type's **own file**.
#[test]
fn a_canonical_must_live_in_its_types_file() {
    let errs = errors_in(&[
        (
            "people.sv",
            r#"
export struct Person {
    name: Str
}
"#,
        ),
        (
            "other.sv",
            r#"
import people.Person

export fn cmp@Person(a: Person, b: Person) [] -> Int => a, b {
    return cmp(a.name, b.name)
}
"#,
        ),
    ]);
    assert!(
        errs.iter().any(|e| e.contains("not a type declared in this file")),
        "expected the same-file rule to be named, got: {errs:?}"
    );
}

/// [mod-export] `export` is explicit on a canonical and must **match the
/// type's** (user decision 2026-09-21): no inheritance, either way.
#[test]
fn a_canonicals_export_must_match_its_types() {
    let exported_fn_private_type = errors(
        r#"
struct Point {
    x: Int
}

export fn cmp@Point(a: Point, b: Point) [] -> Int => a, b {
    return cmp(a.x, b.x)
}
"#,
    );
    assert!(
        exported_fn_private_type
            .iter()
            .any(|e| e.contains("must agree on `export`")),
        "expected the export-match error, got: {exported_fn_private_type:?}"
    );

    let exported_type_private_fn = errors(
        r#"
export struct Point {
    x: Int
}

fn cmp@Point(a: Point, b: Point) [] -> Int => a, b {
    return cmp(a.x, b.x)
}
"#,
    );
    assert!(
        exported_type_private_fn
            .iter()
            .any(|e| e.contains("must agree on `export`")),
        "expected the export-match error, got: {exported_type_private_fn:?}"
    );

    let matching = errors(
        r#"
export struct Point {
    x: Int
}

export fn cmp@Point(a: Point, b: Point) [] -> Int => a, b {
    return cmp(a.x, b.x)
}
"#,
    );
    assert!(matching.is_empty(), "unexpected errors: {matching:?}");
}

/// [cmp-canonical] Decision 9: ambiguity around a canonical is **an error,
/// explicit and implicit alike** — no scope rung silently wins. This is the one
/// carve-out of [fn-overload-scope]'s Own-beats-Import silence.
#[test]
fn ambiguity_around_a_canonical_is_an_error_on_every_rung() {
    let files = &[
        (
            "people.sv",
            r#"
export struct Person {
    name: Str,
    age: Int
}

export fn cmp@Person(a: Person, b: Person) [] -> Int => a, b {
    return cmp(a.age, b.age)
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
    // The *call* says both selector spellings…
    assert!(
        errs.iter()
            .any(|e| e.contains("ambiguous call to `cmp(Person, Person)`")
                && e.contains("cmp@Person")
                && e.contains("cmp@main")),
        "expected the call ambiguity to name both spellings, got: {errs:?}"
    );
    // …and so does implicit resolution, which used to resolve by rung.
    assert!(
        errs.iter()
            .any(|e| e.contains("is ambiguous for `min_of`") && e.contains("cmp@Person")),
        "expected the implicit ambiguity to name the canonical, got: {errs:?}"
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
}

export fn cmp@Person(a: Person, b: Person) [] -> Int => a, b {
    return cmp(a.age, b.age)
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

/// [cmp-canonical] The canonical is the **default selection** even where a
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
}

export fn cmp@Person(a: Person, b: Person) [] -> Int => a, b {
    return cmp(a.age, b.age)
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
/// anything there is a type's.
#[test]
fn a_declaration_selector_must_name_a_type() {
    let (_, diags) = salvo_syntax::parse_module(
        "fn cmp@people(a: Int, b: Int) -> Int {\n    return 0\n}\n",
    );
    let errs: Vec<String> = diags
        .iter()
        .filter(|d| d.is_error())
        .map(|d| d.message.clone())
        .collect();
    assert!(
        errs.iter().any(|e| e.contains("scopes a function to a *type*")),
        "expected the casing rule to be named, got: {errs:?}"
    );
}
