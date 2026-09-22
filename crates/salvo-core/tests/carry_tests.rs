//! [cmp-carry] Structures that **hold** an ordering: the identity in the
//! type, the fn slot that declares where it goes, and the `?cmp` binder.
//!
//! The ordering round's last step (user decisions 2026-09-21, ORDERING.md
//! decisions 1, 11 and 12). What these tests pin down is that the identity
//! travels *in the type* — two heaps ordered differently are different types
//! and refuse to mix — and that a signature's `?cmp` is one binding: filled by
//! resolution where the fn declares it (and published by the result type), and
//! captured from the argument types where it does not.

use std::path::Path;

use salvo_core::{check_program, resolve, Program, SourceSet, Symbols};

/// [intrinsic-std-only] The capability groups and the intrinsic canonicals, as
/// `std/core/compare.sv` declares them, plus the list surface a heap is built
/// on.
const STD_PRELUDE: &str = concat!(
    "export intrinsic type Int\n",
    "export intrinsic type Long\n",
    "export intrinsic type Bool\n",
    "export intrinsic type Str canbe Mut\n",
    "export intrinsic type List<T> canbe Mut\n",
    "export params Ordered<T> {\n    fn cmp(a: T, b: T) -> Int\n}\n",
    "export params Eq<T> {\n    fn eq(a: T, b: T) -> Bool\n}\n",
    "export intrinsic fn cmp(a: Int, b: Int) [] -> Int => a, b\n",
    "export intrinsic fn cmp(a: Str, b: Str) [] -> Int => a, b\n",
    "export intrinsic fn eq(a: Int, b: Int) [] -> Bool => a, b\n",
    "export intrinsic fn mut_list_of<T>(...elems: T[]) [] -> Mut List<T>\n",
    "export intrinsic fn add<T>(list: Mut List<T>, elem: T) [] -> None => list: Mut, !elem\n",
    "export intrinsic fn size<T>(list: List<T>) [] -> Int => list\n",
);

/// The user code every test shares: a struct with a canonical ordering
/// [cmp-canonical], a second ordering of its own, and the heap qualifier with
/// its fn slot.
const HEAP: &str = r#"
export struct Person { name: Str, age: Int }

export fn cmp@Person(a: Person, b: Person) [] -> Int => a, b {
    return cmp(a.age, b.age)
}

export fn by_name(a: Person, b: Person) [] -> Int => a, b {
    return cmp(a.name, b.name)
}

export qualifier Heap<T, ?cmp: (T, T) -> Int> of List<T>

export fn empty_heap<T>(?cmp: (T, T) -> Int) [] -> Mut List<T> as Heap<?cmp> {
    return mut_list_of()
}

export fn heap_push<T>(heap: Heap<?cmp> Mut List<T>, elem: T) [] -> Mut List<T> as Heap<?cmp> => !heap, !elem {
    add(heap, elem)
    return heap
}

export fn heap_size<T>(heap: Heap Mut List<T>) [] -> Int => heap {
    return size(heap)
}
"#;

fn errors(src: &str) -> Vec<String> {
    errors_in(&[("main.sv", src)])
}

/// The shared declarations plus one file of test code.
fn errors_with_heap(src: &str) -> Vec<String> {
    errors_in(&[("heap.sv", HEAP), ("main.sv", src)])
}

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
    let mut parse_errors: Vec<String> = Vec::new();
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
        .chain(
            resolution
                .errors
                .iter()
                .chain(checked.errors.iter())
                .filter(|d| d.is_error())
                .map(|d| d.message.clone()),
        )
        .collect()
}

// ===== the shape itself =====

/// The declarations compile: a fn slot in a qualifier's generics list, a
/// binder in a parameter type, in a constructor's `as` clause, and a bare
/// `Heap` where the body never needs the identity.
#[test]
fn a_qualifier_may_hold_an_ordering() {
    let errs = errors_in(&[("heap.sv", HEAP)]);
    assert!(errs.is_empty(), "unexpected errors: {errs:?}");
}

/// The round trip: build under an ordering, push under the same one, read the
/// size without naming it. What the caller never writes is the ordering — it
/// is in the type from the construction on.
#[test]
fn a_heap_is_built_and_pushed_under_one_ordering() {
    let errs = errors_with_heap(
        r#"
import heap.Person
import heap.Heap
import heap.empty_heap
import heap.heap_push
import heap.heap_size

fn run() [] -> Int {
    let h = empty_heap<Person>()
    let h2 = heap_push(h, Person {name: "a", age: 3})
    return heap_size(h2)
}
"#,
    );
    assert!(errs.is_empty(), "unexpected errors: {errs:?}");
}

/// Two orderings are two types. A heap built by `by_name` does not fit a
/// parameter demanding the canonical one, and the diagnostic reads the
/// identity out of the type — which is the whole reason it is in the type.
#[test]
fn two_orderings_are_two_types() {
    let errs = errors_with_heap(
        r#"
import heap.Person
import heap.Heap
import heap.by_name
import heap.empty_heap

fn wants_canonical(h: Heap<cmp@Person> Mut List<Person>) [] -> Int => h {
    return 0
}

fn run() [] -> Int {
    let h = empty_heap<Person>(cmp = by_name)
    return wants_canonical(h)
}
"#,
    );
    assert!(
        errs.iter().any(|e| e.contains("Heap<by_name>")),
        "expected the carried ordering named: {errs:?}"
    );
    // And the other way round, where an annotation shows both sides.
    let errs = errors_with_heap(
        r#"
import heap.Person
import heap.Heap
import heap.by_name
import heap.empty_heap

fn run() [] -> Int {
    let h: Heap<cmp@Person> Mut List<Person> = empty_heap<Person>(cmp = by_name)
    return 0
}
"#,
    );
    assert!(
        errs.iter()
            .any(|e| e.contains("Heap<cmp@Person>") && e.contains("Heap<by_name>")),
        "expected both identities named: {errs:?}"
    );
}

/// [cmp-binder] The same signature's `?cmp` twice is **one** binding, so two arguments must
/// carry the same ordering: `heap_merge`'s rule (ORDERING.md decision 11).
#[test]
fn one_binder_forces_two_arguments_to_agree() {
    let errs = errors_with_heap(
        r#"
import heap.Person
import heap.Heap
import heap.by_name
import heap.empty_heap

fn merge<T>(a: Heap<?cmp> Mut List<T>, b: Heap<?cmp> Mut List<T>) [] -> Int => a, b {
    return cmp(a.size(), b.size())
}

fn run() [] -> Int {
    let one = empty_heap<Person>()
    let other = empty_heap<Person>(cmp = by_name)
    return merge(one, other)
}
"#,
    );
    assert!(!errs.is_empty(), "two orderings should not merge: {errs:?}");
}

/// [cmp-binder] Two heaps under the *same* ordering merge without the caller
/// saying so.
#[test]
fn one_binder_accepts_two_arguments_that_agree() {
    let errs = errors_with_heap(
        r#"
import heap.Person
import heap.Heap
import heap.empty_heap

fn merge<T>(a: Heap<?cmp> Mut List<T>, b: Heap<?cmp> Mut List<T>) [] -> Int => a, b {
    return cmp(a.size(), b.size())
}

fn run() [] -> Int {
    let one = empty_heap<Person>()
    let other = empty_heap<Person>()
    return merge(one, other)
}
"#,
    );
    assert!(errs.is_empty(), "unexpected errors: {errs:?}");
}

/// [cmp-binder] The binder is callable in the body, like any implicit:
/// the ordering the *caller's* structure was built with is what the body
/// compares with.
#[test]
fn the_binder_is_callable_in_the_body() {
    let errs = errors_with_heap(
        r#"
import heap.Person
import heap.Heap
import heap.empty_heap

fn least<T>(heap: Heap<?cmp> Mut List<T>, a: T, b: T) [] -> Int => heap, a, b {
    return cmp(a, b)
}

fn run() [] -> Int {
    let h = empty_heap<Person>()
    return least(h, Person {name: "a", age: 1}, Person {name: "b", age: 2})
}
"#,
    );
    assert!(errs.is_empty(), "unexpected errors: {errs:?}");
}

// ===== static identity =====

/// [cmp-carry] A fn bound into a type must be **named, top-level and
/// capture-free** (decision 12): a lambda has no identity a type can carry,
/// and the error says so rather than losing the claim silently.
#[test]
fn a_lambda_has_no_identity_a_type_can_carry() {
    let errs = errors_with_heap(
        r#"
import heap.Person
import heap.Heap
import heap.empty_heap

fn run() [] -> Int {
    let h = empty_heap<Person>(cmp = (a, b) -> 0)
    return 0
}
"#,
    );
    assert!(
        errs.iter().any(|e| e.contains("no identity a type can carry")),
        "expected the static-identity refusal: {errs:?}"
    );
}

/// Nor does a local of fn type, written in the type itself.
#[test]
fn a_local_is_not_an_identity() {
    let errs = errors_with_heap(
        r#"
import heap.Person
import heap.Heap
import heap.by_name

fn run() [] -> Int {
    let mine = by_name
    let h: Heap<mine> Mut List<Person> = mut_list_of()
    return 0
}
"#,
    );
    assert!(
        errs.iter().any(|e| e.contains("no identity a type can carry")),
        "expected the static-identity refusal: {errs:?}"
    );
}

/// A slot filled with a name nothing declares is the ordinary unresolved-name
/// error, naming what was looked for.
#[test]
fn an_unknown_identity_is_reported() {
    let errs = errors_with_heap(
        r#"
import heap.Person
import heap.Heap

fn run(h: Heap<nowhere> Mut List<Person>) [] -> Int => h {
    return 0
}
"#,
    );
    assert!(
        errs.iter().any(|e| e.contains("nowhere")),
        "expected the unknown-function error: {errs:?}"
    );
}

/// `cmp@Person` names the canonical [cmp-canonical]; `cmp@Nothing` names
/// nowhere, and the error says where one would live.
#[test]
fn a_selector_must_name_a_declaration() {
    let errs = errors_with_heap(
        r#"
import heap.Person
import heap.Heap

fn run(h: Heap<cmp@Nothing> Mut List<Person>) [] -> Int => h {
    return 0
}
"#,
    );
    assert!(
        errs.iter().any(|e| e.contains("cmp@Nothing")),
        "expected the selector error: {errs:?}"
    );
}

// ===== the slot declaration =====

/// A fn slot must have a fn type — the same requirement an implicit parameter
/// has, and for the same reason [implicit-param].
#[test]
fn a_slot_must_be_a_function() {
    let errs = errors(
        r#"
qualifier Bad<T, ?cmp: Int> of List<T>
"#,
    );
    assert!(
        errs.iter().any(|e| e.contains("function slot")),
        "expected the slot-type error: {errs:?}"
    );
}

/// Only a qualifier or an `intrinsic type` may declare one: on a fn the
/// binder binds bare in the signature instead (ORDERING.md decision 11).
#[test]
fn a_fn_declares_no_slot() {
    let errs = errors(
        r#"
fn bad<T, ?cmp: (T, T) -> Int>(a: T) [] -> Int => a {
    return 0
}
"#,
    );
    assert!(
        errs.iter().any(|e| e.contains("function slot")),
        "expected the slot-position error: {errs:?}"
    );
}
