//! [cmp-carry] Structures that **hold** an ordering: the identity in the
//! type, the fn slot that declares where it goes, and the `?cmp` binder.
//!
//! The ordering round's last step (user decisions 2026-09-21, the ordering round
//! decisions 1, 11 and 12). What these tests pin down is that the identity
//! travels *in the type* — two heaps ordered differently are different types
//! and refuse to mix — and that a signature's `?cmp` is one binding: filled by
//! resolution where the fn declares it (and published by the result type), and
//! captured from the argument types where it does not.

use std::path::Path;

use salvo_core::{check_program, resolve, Program, SourceSet, Symbols};

/// [intrinsic-std-only] The capability groups and the intrinsic canonicals, as
/// `lib/std/core/compare.sv` declares them, plus the list surface a heap is built
/// on.
const STD_PRELUDE: &str = concat!(
    "export intrinsic type Int\n",
    "export intrinsic type Long\n",
    "export intrinsic type Bool\n",
    "export intrinsic type Str canbe Mut\n",
    "export intrinsic type List<T> canbe Mut\n",
    "export fns Ordered<T> {\n    fn cmp(a: T, b: T) -> Int\n}\n",
    "export fns Eq<T> {\n    fn eq(a: T, b: T) -> Bool\n}\n",
    "export intrinsic fn cmp(a: Int, b: Int) [] -> Int => a, b\n",
    "export intrinsic fn cmp(a: Str, b: Str) [] -> Int => a, b\n",
    "export intrinsic fn eq(a: Int, b: Int) [] -> Bool => a, b\n",
    "export intrinsic fn Mut List<T>(...elems: T[]) [] -> Mut List<T>\n",
    "export intrinsic fn add<T>(list: Mut List<T>, elem: T) [] -> None => list: Mut, !elem\n",
    "export intrinsic fn size<T>(list: List<T>) [] -> Int => list\n",
);

/// The user code every test shares: a struct with a canonical ordering
/// [fn-attached], a second ordering of its own, and the heap qualifier with
/// its fn slot.
const HEAP: &str = r#"
// [fn-attached] The obligation declares the capability, and this file's `cmp`
// fulfils it — so `cmp` is declared on `Person` and travels with it.
export struct Person : Ordered<self> { name: Str, age: Int }

export fn cmp(a: Person, b: Person) [] -> Int => a, b {
    return cmp(a.age, b.age)
}

export fn by_name(a: Person, b: Person) [] -> Int => a, b {
    return cmp(a.name, b.name)
}

export qualifier Heap<T>(?cmp: (T, T) -> Int) of List<T>

// [cmp-carry] The same thing through a **group spread**: one slot per member of
// the group, which is what `?Ordered<T>` means in a slot list.
export qualifier Ranked<T>(?Ordered<T>) of List<T>

export fn empty_ranked<T>(?cmp: (T, T) -> Int) [] -> +Ranked<T>(?cmp) Mut List<T> {
    return Mut List()
}

export fn empty_heap<T>(?cmp: (T, T) -> Int) [] -> +Heap<T>(?cmp) Mut List<T> {
    return Mut List()
}

export fn heap_push<T>(heap: Heap<T>(?cmp) Mut List<T>, elem: T) [] -> +Heap<T>(?cmp) Mut List<T> => !heap, !elem {
    add(heap, elem)
    return heap
}

export fn heap_size<T>(heap: Heap Mut List<T>) [] -> Int => heap {
    return size(heap)
}

// [deduce-reapply] Establishing the claim on a parameter that arrives without
// it, which only this file may do.
export fn heapify<T>(list: Mut List<T>, elem: T, ?cmp: (T, T) -> Int) [] -> None
=> list: +Heap<T>(?cmp) Mut, !elem {
    add(list, elem)
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
        SourceSet::classify(Path::new("std/core/compare.sv")).unwrap(),
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
    // [comptime-instantiate] The `by` sites stamp before resolution.
    let _expansion = salvo_core::expand(&sources.files, &mut modules);
    let program = Program {
        files: sources.files,
        modules: Program::share(modules),
        companions: Vec::new(),
    };
    let symbols = Symbols::collect(&program);
    let resolution = resolve(&program);
    let checked = check_program(&program, &resolution, &symbols);
    let mut msgs: Vec<String> = parse_errors
        .into_iter()
        .chain(
            resolution
                .errors
                .iter()
                .chain(checked.errors.iter())
                .filter(|d| d.is_error())
                .map(|d| d.message.clone()),
        )
        .collect();
    msgs.extend(
        salvo_core::ir::analyze::analyze(&program, &symbols, &resolution, &checked)
            .into_iter()
            .map(|d| d.message),
    );
    msgs
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

fn wants_canonical(h: Heap<Person>(cmp@Person) Mut List<Person>) [] -> Int => h {
    return 0
}

fn run() [] -> Int {
    let h = empty_heap<Person>(cmp = by_name)
    return wants_canonical(h)
}
"#,
    );
    assert!(
        errs.iter().any(|e| e.contains("Heap<Person>(by_name)")),
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
    let h: Heap<Person>(cmp@Person) Mut List<Person> = empty_heap<Person>(cmp = by_name)
    return 0
}
"#,
    );
    assert!(
        errs.iter()
            .any(|e| e.contains("Heap<Person>(cmp@Person)")
                && e.contains("Heap<Person>(by_name)")),
        "expected both identities named: {errs:?}"
    );
}

/// [cmp-binder] The same signature's `?cmp` twice is **one** binding, so two arguments must
/// carry the same ordering: `heap_merge`'s rule (the ordering round's decision 11).
#[test]
fn one_binder_forces_two_arguments_to_agree() {
    let errs = errors_with_heap(
        r#"
import heap.Person
import heap.Heap
import heap.by_name
import heap.empty_heap

fn merge<T>(a: Heap<T>(?cmp) Mut List<T>, b: Heap<T>(?cmp) Mut List<T>) [] -> Int => a, b {
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

fn merge<T>(a: Heap<T>(?cmp) Mut List<T>, b: Heap<T>(?cmp) Mut List<T>) [] -> Int => a, b {
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

fn least<T>(heap: Heap<T>(?cmp) Mut List<T>, a: T, b: T) [] -> Int => heap, a, b {
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

/// [cmp-binder] A deduction entry names qualifiers, not their arguments — and
/// keeping the qualifier keeps the **identity**, since it lives in the type and
/// the fn could not have changed it. The proof is the second call: it demands
/// the identity by name, which a claim that had lost its argument could not
/// satisfy.
#[test]
fn keeping_the_qualifier_keeps_the_identity() {
    let errs = errors_with_heap(
        r#"
import heap.Person
import heap.Heap
import heap.empty_heap

fn only_canonical(r: Heap<Person>(cmp@Person) Mut List<Person>) [] -> Int => r {
    return 0
}

fn touch<T>(r: Heap<T>(?cmp) Mut List<T>) [] -> Int => r: Heap Mut {
    return size(r)
}

fn run() [] -> Int {
    let h = empty_heap<Person>()
    let n = touch(h)
    return n + only_canonical(h)
}
"#,
    );
    assert!(errs.is_empty(), "unexpected errors: {errs:?}");
}

/// [cmp-carry] A `fns` group spread into a **slot list** declares one slot
/// per member (user decision 2026-09-22), so `Ranked<T>(?Ordered<T>)` is the
/// `Heap` declaration written the short way — and a use site fills it by naming
/// the member's slot.
#[test]
fn a_group_spreads_into_a_slot_list() {
    let errs = errors_with_heap(
        r#"
import heap.Person
import heap.Ranked
import heap.empty_ranked

fn least<T>(r: Ranked<T>(?cmp) List<T>, a: T, b: T) [] -> Int => r, a, b {
    return cmp(a, b)
}

fn run() [] -> Int {
    let r = empty_ranked<Person>()
    return least(r, Person {name: "a", age: 1}, Person {name: "b", age: 2})
}
"#,
    );
    assert!(errs.is_empty(), "unexpected errors: {errs:?}");
}

/// [cmp-binder] A slot a signature does not mention is **not constrained** — the
/// value keeps carrying it — which is what makes a written slot list a pattern
/// rather than an exact type, and a bare `Heap` the empty case of one rule.
#[test]
fn an_unmentioned_slot_is_unconstrained() {
    let errs = errors_with_heap(
        r#"
import heap.Person
import heap.Heap
import heap.by_name
import heap.empty_heap

fn size_only<T>(h: Heap Mut List<T>) [] -> Int => h {
    return size(h)
}

fn run() [] -> Int {
    let a = empty_heap<Person>()
    let b = empty_heap<Person>(cmp = by_name)
    return size_only(a) + size_only(b)
}
"#,
    );
    assert!(errs.is_empty(), "unexpected errors: {errs:?}");
}

/// [cmp-binder] Two heaps in one signature need two names, and the **alias**
/// gives the second one: `?cmp: cmp2` fills the `cmp` slot under the name
/// `cmp2`.
#[test]
fn a_binder_may_be_aliased() {
    let errs = errors_with_heap(
        r#"
import heap.Person
import heap.Heap
import heap.by_name
import heap.empty_heap

fn compare_across<T>(a: Heap<T>(?cmp) List<T>, b: Heap<T>(?cmp: cmp2) List<T>, x: T, y: T) [] -> Int => a, b, x, y {
    // Each ordering is reachable by its own name.
    return cmp(x, y) + cmp2(x, y)
}

fn run() [] -> Int {
    let one = empty_heap<Person>()
    let other = empty_heap<Person>(cmp = by_name)
    return compare_across(one, other, Person {name: "a", age: 1}, Person {name: "b", age: 2})
}
"#,
    );
    assert!(errs.is_empty(), "unexpected errors: {errs:?}");
}

/// [cmp-binder] …and an alias does not make the *operator* unambiguous: two
/// orderings in one scope are two candidates for `<` whatever they are called,
/// so the body must name the one it means. Aliasing is how two orderings get
/// into one scope, not how the choice between them is dodged.
#[test]
fn two_orderings_in_scope_refuse_the_operator() {
    let errs = errors_with_heap(
        r#"
import heap.Person
import heap.Heap

fn pick<T>(a: Heap<T>(?cmp) List<T>, b: Heap<T>(?cmp: cmp2) List<T>, x: T, y: T) [] -> Bool => a, b, x, y {
    return x < y
}
"#,
    );
    assert!(
        errs.iter()
            .any(|e| e.contains("in scope") && e.contains("`cmp`") && e.contains("`cmp2`")),
        "expected the operator to refuse: {errs:?}"
    );
}

/// [deduce-reapply] [cmp-carry] A fn that **establishes** a carried claim
/// publishes the identity the *call* resolved, exactly as a constructor does:
/// `heapify(xs)` under one ordering and under another produce two types, which
/// is what a position demanding one of them can then tell apart.
#[test]
fn an_established_claim_carries_the_identity_the_call_resolved() {
    let errs = errors_with_heap(
        r#"
import heap.Person
import heap.Heap
import heap.by_name
import heap.heapify

fn wants_canonical(h: Heap<Person>(cmp@Person) Mut List<Person>) [] -> Int => h {
    return 0
}

fn run(xs: Mut List<Person>) [] -> Int => xs: Mut {
    heapify(xs, Person {name: "a", age: 1}, cmp = by_name)
    return wants_canonical(xs)
}
"#,
    );
    assert!(
        errs.iter().any(|e| e.contains("Heap<Person>(by_name)")),
        "expected the established ordering named: {errs:?}"
    );
    // …and under the canonical one the same call fits.
    let errs = errors_with_heap(
        r#"
import heap.Person
import heap.Heap
import heap.heapify

fn wants_canonical(h: Heap<Person>(cmp@Person) Mut List<Person>) [] -> Int => h {
    return 0
}

fn run(xs: Mut List<Person>) [] -> Int => xs: Mut {
    heapify(xs, Person {name: "a", age: 1})
    return wants_canonical(xs)
}
"#,
    );
    assert!(errs.is_empty(), "unexpected errors: {errs:?}");
}

// ===== the `Sorted` claim [col-sorted-list] =====

/// The `Sorted` tests run against **std's own** declarations rather than a
/// mirror of them: what is under test is the shape `std.core.list` gives
/// `sort`/`add_sorted`/`binary_search`, which a hand-written prelude could only
/// restate. (The precedent for loading std in a `salvo-core` test is
/// `diag_tests`' `check_errors`.)
fn std_errors(src: &str) -> Vec<String> {
    std_errors_in(&[("main.sv", src)])
}

/// [`std_errors`] over several program files.
fn std_errors_in(files: &[(&str, &str)]) -> Vec<String> {
    let std_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../lib");
    let mut sources = SourceSet::default();
    let io_errors = sources.add_dir(&std_dir, "rs", true, false);
    assert!(io_errors.is_empty(), "failed to read std: {io_errors:?}");
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
        let errors: Vec<_> = diagnostics.iter().filter(|d| d.is_error()).collect();
        assert!(
            errors.is_empty(),
            "parse errors in {}: {errors:?}",
            file.name
        );
        modules.push(ast);
    }
    // [comptime-instantiate] The `by` sites stamp before resolution.
    let _expansion = salvo_core::expand(&sources.files, &mut modules);
    let program = Program {
        files: sources.files,
        modules: Program::share(modules),
        companions: Vec::new(),
    };
    let symbols = Symbols::collect(&program);
    let resolution = resolve(&program);
    let checked = check_program(&program, &resolution, &symbols);
    let mut all: Vec<_> = resolution.errors.iter().chain(checked.errors.iter()).filter(|d| d.is_error()).cloned().collect();
    // [fate-poison] As the gates do, the IR analyses run on every
    // declaration the checker's errors leave clean [ir-analyze]: a poisoned
    // read is the IR flow pass's since R8 piece
    // 3c [flow-use-after-move].
    all.extend(salvo_core::ir::analyze::analyze(&program, &symbols, &resolution, &checked).into_iter().filter(|d| d.is_error()));
    all.into_iter().map(|d| d.message).collect()
}

/// The ordering a program of its own: `sort` publishes it into the claim, and
/// the two operations that read the claim capture it [cmp-binder] — so the
/// search and the insert are computed with the ordering the list actually
/// carries rather than with whatever is visible at the call.
#[test]
fn a_sorted_list_carries_the_ordering_it_was_sorted_by() {
    let errs = std_errors(
        r#"
fn by_len(a: Str, b: Str) [] -> Int => a, b {
    return cmp(size(a), size(b))
}

fn run() [] -> Int? {
    let ordered = sort(List("pear", "fig"), cmp = by_len)
    let growing = mut_sort(List("pear", "fig"), cmp = by_len)
    add_sorted(growing, "durian")
    return binary_search(ordered, "kiwi")
}
"#,
    );
    assert!(errs.is_empty(), "unexpected errors: {errs:?}");
}

/// [cmp-carry] Two orderings are two types here too: a list sorted by `by_len`
/// does not fit a position demanding the canonical `cmp`, and the diagnostic
/// reads the ordering out of the type — which is the whole reason the claim
/// carries it. Before step 5 the claim said only "sorted", so this call was
/// accepted and the search ran under the wrong ordering.
#[test]
fn a_list_sorted_by_one_ordering_does_not_fit_another() {
    let errs = std_errors(
        r#"
fn by_len(a: Str, b: Str) [] -> Int => a, b {
    return cmp(size(a), size(b))
}

fn needs_canonical(xs: Sorted<Str>(cmp) List<Str>) [] -> Int? => xs {
    return binary_search(xs, "x")
}

fn run() [] -> Int? {
    let by = sort(List("pear"), cmp = by_len)
    return needs_canonical(by)
}
"#,
    );
    assert!(
        errs.iter().any(|e| e.contains("Sorted<Str>(by_len)")),
        "expected the carried ordering named: {errs:?}"
    );
}

/// [cmp-binder] One binder twice is one binding, so two sorted lists in one
/// signature must have been sorted the same way.
#[test]
fn two_sorted_lists_under_one_binder_must_agree() {
    let src = r#"
fn by_len(a: Str, b: Str) [] -> Int => a, b {
    return cmp(size(a), size(b))
}

fn merge<T>(a: Sorted<T>(?cmp) List<T>, b: Sorted<T>(?cmp) List<T>, probe: T) [] -> Int?
=> a, b, probe {
    return binary_search(a, probe)
}

fn run() [] -> Int? {
    let one = sort(List("pear"))
    let other = sort(List("fig"), cmp = REPLACE)
    return merge(one, other, "x")
}
"#;
    let mixed = std_errors(&src.replace("REPLACE", "by_len"));
    assert!(
        mixed
            .iter()
            .any(|e| e.contains("Sorted<Str>(cmp)") && e.contains("Sorted<Str>(by_len)")),
        "expected both orderings named: {mixed:?}"
    );
    // The same source with both lists sorted the same way needs no annotation
    // anywhere: the binder is filled from the argument types.
    let agreeing = std_errors(&src.replace("REPLACE", "cmp"));
    assert!(agreeing.is_empty(), "unexpected errors: {agreeing:?}");
}

/// [cmp-carry] A **written type is a pattern for its slots**, an annotation
/// included: `Mut Sorted List<Int>` names the claim without naming its
/// ordering, and the value keeps carrying the one `mut_sort` published — which
/// is what lets `add_sorted` capture it. An annotation that *does* name an
/// ordering is a demand like any other, and the wrong one is refused.
#[test]
fn an_annotation_keeps_an_ordering_it_does_not_name() {
    let errs = std_errors(
        r#"
fn run() [] -> None {
    let live: Mut Sorted List<Int> = mut_sort(List(10, 40))
    add_sorted(live, 20)
}
"#,
    );
    assert!(errs.is_empty(), "unexpected errors: {errs:?}");

    let errs = std_errors(
        r#"
fn by_size(a: Int, b: Int) [] -> Int => a, b {
    return cmp(b, a)
}

fn run() [] -> None {
    let live: Mut Sorted<Int>(by_size) List<Int> = mut_sort(List(10, 40))
    add_sorted(live, 20)
}
"#,
    );
    assert!(
        errs.iter()
            .any(|e| e.contains("Sorted<Int>(by_size)") && e.contains("Sorted<Int>(cmp)")),
        "expected both orderings named: {errs:?}"
    );
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
    let h: Heap<Person>(mine) Mut List<Person> = Mut List()
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

fn run(h: Heap<Person>(nowhere) Mut List<Person>) [] -> Int => h {
    return 0
}
"#,
    );
    assert!(
        errs.iter().any(|e| e.contains("nowhere")),
        "expected the unknown-function error: {errs:?}"
    );
}

/// `cmp@Person` names the canonical [fn-attached]; `cmp@Nothing` names
/// nowhere, and the error says where one would live.
#[test]
fn a_selector_must_name_a_declaration() {
    let errs = errors_with_heap(
        r#"
import heap.Person
import heap.Heap

fn run(h: Heap<Person>(cmp@Nothing) Mut List<Person>) [] -> Int => h {
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
qualifier Bad<T>(?cmp: Int) of List<T>
"#,
    );
    assert!(
        errs.iter().any(|e| e.contains("function slot")),
        "expected the slot-type error: {errs:?}"
    );
}

/// Only a qualifier or an `intrinsic type` may declare one: on a fn the
/// binder binds bare in the signature instead (the ordering round's decision 11).
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

/// [qual-value-arg] Type generics are all-or-none at a use site: a partial
/// list is an error naming the rule and the block.
#[test]
fn type_arguments_are_all_or_none() {
    let errs = errors_with_heap(
        r#"
import heap
struct Person { age: Int }
fn by_age(a: Person, b: Person) [] -> Int => a, b {
    return a.age - b.age
}
fn f(h: Ranked<Person, Int>(by_age) Mut List<Person>) [] -> Int => h {
    return 0
}
"#,
    );
    assert!(
        errs.iter().any(|e| e.contains("all-or-none")),
        "expected the all-or-none error: {errs:?}"
    );
}

// ===== the written opaque lend [proj-infer] =====

/// [proj-infer] What `filter`'s written lend **buys**, over std's own
/// declaration: `-> Mut List<proj T> holds proj(it)` names the pass as the
/// only source, so a later mutation of something the *predicate* captured
/// leaves the filtered view standing. Without the annotation the
/// conservative fallback links the result to every kept argument — the
/// callback included — and this program is refused.
///
/// The spelling is the 2026-09-25 respelling (`T holds proj(x)` after the
/// type, reading "is a `T`, holds a borrow of `x`"); the fact it pins is why
/// the annotation exists at all, which is what made the respelling worth
/// doing rather than deleting the annotation.
#[test]
fn a_written_lend_narrows_which_arguments_the_result_holds() {
    let errs = std_errors(
        "fn main() [use] -> None {\n    \
         use StdConsole()\n    \
         let words: List<Str> = List(\"a\", \"bb\")\n    \
         let limits: Mut List<Int> = Mut List(1)\n    \
         let p = iter(words)\n    \
         let long = filter(p, (s: Str) -> { return size(s) > (first(limits) ?: unreachable!()) })\n    \
         add(limits, 2)\n    \
         println(\"${size(long)}\")\n}\n",
    );
    assert!(
        errs.is_empty(),
        "the lend names `it`, so the captured `limits` is not a source: {errs:?}"
    );
}

/// [proj-infer] …and the lend it *does* name still links: moving the pass's
/// source poisons the view, which is the half no annotation could remove.
#[test]
fn a_written_lend_still_links_the_source_it_names() {
    let errs = std_errors(
        "fn eat(words: List<Str>) [] -> None => !words {\n    \
         return None\n}\n\
         fn main() [use] -> None {\n    \
         use StdConsole()\n    \
         let words: List<Str> = List(\"a\", \"bb\")\n    \
         let p = iter(words)\n    \
         let long = filter(p, (s: Str) -> { return size(s) > 1 })\n    \
         eat(words)\n    \
         println(\"${size(long)}\")\n}\n",
    );
    assert!(
        errs.iter().any(|e| e.contains("shares its fate")),
        "moving the source must poison the view: {errs:?}"
    );
}

// ===== [col-no-literal] a bare literal determines a type argument =====

/// [col-no-literal] A bare collection constructor as an argument determines the
/// callee's type parameter (defect closed 2026-09-25). The cause was subtle
/// enough to pin: substituting an **unbound** variable yields `Unknown`, so the
/// parameter pattern `List<T>` arrived at the literal looking concrete
/// (`List<Unknown>`), the literal adopted `Unknown` as its element type and
/// discarded what its own elements said — and the call then had nothing to
/// infer `T` from. Run against std's own `to_set`/`to_list`, which is where it
/// was found.
#[test]
fn a_bare_literal_argument_determines_a_type_argument() {
    let errs = std_errors(
        "fn main() [use] -> None {\n    \
         use StdConsole()\n    \
         let a = to_set(List(1, 2))\n    \
         let b = to_list(Set(3, 4))\n    \
         let c = to_set(List(\"x\", \"y\"))\n    \
         println(\"${size(a)} ${size(b)} ${size(c)}\")\n}\n",
    );
    assert!(errs.is_empty(), "expected a clean check, got {errs:?}");
}

/// …and the diagnostic still fires where nothing *can* determine the argument
/// (an empty `List()`),
/// which is the rule the fix had to leave standing [col-no-literal].
#[test]
fn an_empty_literal_argument_still_needs_its_type() {
    let errs = std_errors(
        "fn main() [use] -> None {\n    \
         use StdConsole()\n    \
         let a = to_set(List())\n    \
         println(\"${size(a)}\")\n}\n",
    );
    assert!(
        errs.iter().any(|e| e.contains("element type") || e.contains("cannot infer")),
        "an empty literal determines nothing: {errs:?}"
    );
}

/// [struct-slot] [cmp-carry] A **struct** declares fn slots as a type
/// declaration does: the identity is in the type, so two differently ordered
/// values refuse to mix, and a fn over the struct captures the binder from its
/// parameter [cmp-binder].
const RANKED: &str = r#"
struct Ranked<T>(?cmp: (T, T) -> Int) canbe Mut {
    items: List<T> canbe Mut
}

fn ranked<T>(?cmp: (T, T) -> Int) [] -> Mut Ranked<T>(?cmp) {
    return Mut Ranked<T>(?cmp) { items: Mut List<T>() }
}

fn better<T>(r: Ranked<T>(?cmp), a: T, b: T) [] -> Bool => r, a, b {
    return cmp(a, b) < 0
}

fn backwards(a: Int, b: Int) [] -> Int {
    return cmp(b, a)
}
"#;

#[test]
fn a_struct_slot_carries_its_identity_in_the_type() {
    let ok = format!(
        "{RANKED}\nfn use_it() [] -> Bool {{\n    let up = ranked<Int>()\n    \
         let down = ranked<Int>(cmp = backwards)\n    return better(up, 1, 2) && better(down, 1, 2)\n}}\n"
    );
    let errs = errors(&ok);
    assert!(errs.is_empty(), "{errs:?}");
    let mixed = format!(
        "{RANKED}\nfn mix() [] -> None {{\n    let down = ranked<Int>(cmp = backwards)\n    \
         let up: Ranked<Int>(cmp) = down\n}}\n"
    );
    let errs = errors(&mixed);
    assert!(
        errs.iter().any(|e| e.contains("expected `Ranked<Int>(cmp)`, found `Mut Ranked<Int>(backwards)`")),
        "{errs:?}"
    );
}

/// [implicit-resolve-body] An implicit joins resolution by argument types
/// (user decision 2026-10-05, option A): `cmp(a, a)` at `T` goes through it,
/// `cmp(1, 2)` at `Int` reaches the visible `cmp(Int, Int)`.
#[test]
fn an_implicit_joins_resolution_by_argument_types() {
    let errs = errors(
        "fn f<T>(a: T, ?cmp: (T, T) -> Int) [] -> Int => a {\n    \
         return cmp(a, a) + cmp(1, 2)\n}\n",
    );
    assert!(errs.is_empty(), "{errs:?}");
}

/// [call-resolve] A call through a fn-typed local checks its arguments: an
/// ordinary local has no other declarations to fall through to.
#[test]
fn a_call_through_a_local_checks_its_arguments() {
    let errs = errors("fn g(f: (Str) -> Int) [] -> Int => f {\n    return f(1)\n}\n");
    assert!(
        errs.iter().any(|e| e.contains("`f` takes `Str` here, found `Int`")),
        "{errs:?}"
    );
    let errs = errors("fn g(f: (Str) -> Int) [] -> Int => f {\n    return f(\"a\", \"b\")\n}\n");
    assert!(errs.iter().any(|e| e.contains("`f` takes 1 argument(s), found 2")), "{errs:?}");
}

/// [call-resolve] [fn-value] A **computed** callee (`adder()(1)`, an element
/// of a list of fns) is checked like a call through a local (B1 census D19,
/// 2026-10-27): before, each argument was only typed against its parameter,
/// so a wrong argument or a wrong count passed the checker and rustc and
/// kotlinc refused the generated code.
#[test]
fn a_computed_callee_checks_its_arguments() {
    let adder = "fn adder() [] -> (n: Int) -> Int {\n    return (n: Int) -> n + 1\n}\n";
    let ok = errors(&format!("{adder}fn g() [] -> Int {{\n    return adder()(1)\n}}\n"));
    assert!(ok.is_empty(), "{ok:?}");
    let errs = errors(&format!("{adder}fn g() [] -> Int {{\n    return adder()(\"x\")\n}}\n"));
    assert!(
        errs.iter().any(|e| e.contains("this function value takes `Int` here, found `Str`")),
        "{errs:?}"
    );
    let errs = errors(&format!("{adder}fn g() [] -> Int {{\n    return adder()(1, 2)\n}}\n"));
    assert!(
        errs.iter().any(|e| e.contains("this function value takes 1 argument(s), found 2")),
        "{errs:?}"
    );
}

/// [call-resolve] A call through a fn-typed local checks the
/// **claims** its parameter demands, as a declared call does (B1 census D20,
/// 2026-10-27): `f(ys)` with `f: (xs: NonEmpty List<Int>) -> Int` and `ys:
/// List<Int>` was accepted, and an empty list reached a `NonEmpty`
/// parameter. A claimed argument still fits, and so does one whose claim the
/// parameter does not ask for.
#[test]
fn a_call_through_a_local_checks_the_claims_its_parameter_demands() {
    let errs = std_errors(
        "fn through(f: (xs: NonEmpty List<Int>) -> Int, ys: List<Int>) [] -> Int => f, ys {\n    \
         return f(ys)\n}\n",
    );
    assert!(
        errs.iter().any(|e| e.contains("`f` takes `NonEmpty List<Int>` here, found `List<Int>`")
            && e.contains("does not carry what the parameter claims")),
        "{errs:?}"
    );
    let ok = std_errors(
        "fn through(f: (xs: NonEmpty List<Int>) -> Int, ys: List<Int>) [] -> Int => f, ys {\n    \
         if ys is NonEmpty {\n        return f(ys)\n    }\n    return 0\n}\n\n\
         fn loose(f: (xs: List<Int>) -> Int, ys: NonEmpty List<Int>) [] -> Int => f, ys {\n    \
         return f(ys)\n}\n",
    );
    assert!(ok.is_empty(), "{ok:?}");
    // [qual-depend] A claim about another parameter is about this call's
    // argument there, as at a declared call.
    let f = "f: (xs: List<Int>, i: Idx(xs) Int) -> Int";
    let errs = std_errors(&format!(
        "fn other({f}, ys: List<Int>, zs: List<Int>, j: Idx(ys) Int) [] -> Int => f, ys, zs, j {{\n    \
         return f(zs, j)\n}}\n"
    ));
    assert!(
        errs.iter().any(|e| e.contains("found `Idx(ys) Int`") && e.contains("does not carry what the parameter claims")),
        "{errs:?}"
    );
    let ok = std_errors(&format!(
        "fn same({f}, ys: List<Int>, j: Idx(ys) Int) [] -> Int => f, ys, j {{\n    return f(ys, j)\n}}\n"
    ));
    assert!(ok.is_empty(), "{ok:?}");
}

/// [fn-type-named] Rule (c): a type-named fn must return the named base with
/// at least the named qualifiers. `fn Mut List<T> -> Mut List<T>` is fine;
/// returning a plain `List<T>` (missing `Mut`) is refused, and a wrong base
/// is refused.
#[test]
fn type_named_fn_return_is_checked() {
    // Correct: returns exactly the named type.
    let ok = errors(
        "fn Mut List<T>(first: T) -> Mut List<T> => !first {\n    return Mut List(first)\n}\n",
    );
    assert!(ok.is_empty(), "a matching return is accepted: {ok:?}");

    // Missing the named `Mut` qualifier on the return.
    let miss = errors(
        "fn Mut List<T>(first: T) -> List<T> => !first {\n    return Mut List(first)\n}\n",
    );
    assert!(
        miss.iter().any(|e| e.contains("must carry the `Mut` qualifier")),
        "a missing named qualifier is refused: {miss:?}"
    );

    // Extra qualifiers beyond the named ones are allowed (named ⊆ return).
    let extra = errors(
        "fn List<T>(first: T) -> Mut List<T> => !first {\n    return Mut List(first)\n}\n",
    );
    assert!(extra.is_empty(), "extra qualifiers on the return are allowed: {extra:?}");
}

#[test]
fn provenance_type_named_fn_is_checked() {
    // [fn-type-named] The provenance form: the name *is* a provenance
    // qualifier, no base type. A `-> +Ok T` establishment lifts into the
    // constructor clause, so the named qualifier is carried there.
    let ok = errors(
        "provenance qualifier Ok<T> of T\n\
         fn Ok<T canbe linear>(value: T) [] -> +Ok T => !value {\n    return value\n}\n",
    );
    assert!(ok.is_empty(), "a provenance type-fn that establishes its tag is accepted: {ok:?}");

    // The return carries a *different* provenance tag than the name demands.
    let wrong = errors(
        "provenance qualifier Ok<T> of T\n\
         provenance qualifier Err<T> of T\n\
         fn Ok<T canbe linear>(value: T) [] -> +Err T => !value {\n    return value\n}\n",
    );
    assert!(
        wrong.iter().any(|e| e.contains("must carry it") || e.contains("[fn-type-named]")),
        "a provenance type-fn whose return establishes the wrong tag is refused: {wrong:?}"
    );
}

#[test]
fn provenance_arm_is_inferred_from_a_bare_value() {
    // [arm-infer] A bare value that is the payload of exactly one
    // provenance-tagged arm is wrapped into it: `return n` for an
    // `Ok Int | Err Str` return is implicitly `Ok(n)`.
    let prelude = "provenance qualifier Ok<T> of T\n\
         provenance qualifier Err<T> of T\n\
         fn Ok<T canbe linear>(v: T) [] -> +Ok T => !v {\n    return v\n}\n\
         fn Err<T canbe linear>(v: T) [] -> +Err T => !v {\n    return v\n}\n";

    let ok = errors(&format!(
        "{prelude}fn classify(n: Int) -> Ok Int | Err Str {{\n    \
         if n < 0 {{\n        return \"neg\"\n    }}\n    return n\n}}\n"
    ));
    assert!(ok.is_empty(), "bare values infer their provenance arm: {ok:?}");

    // Ambiguous: two provenance arms accept an `Int`, so the bare value is
    // refused and the explicit constructor is required.
    let ambig = errors(&format!(
        "{prelude}fn two(n: Int) -> Ok Int | Err Int {{\n    return n\n}}\n"
    ));
    assert!(
        !ambig.is_empty(),
        "a bare value matching two provenance arms is refused: {ambig:?}"
    );

    // [arm-infer] Not for a literal union with an open provenance arm: a
    // plain value must be wrapped explicitly with the open constructor.
    let open = errors(
        "provenance qualifier Other<T> of T\n\
         fn other<T>(v: T) [] -> +Other T => !v {\n    return v\n}\n\
         type Class = \"A\" | \"B\" | Other Str\n\
         fn f(s: Str) -> Class => s {\n    return s\n}\n",
    );
    assert!(
        !open.is_empty(),
        "a plain value is not auto-wrapped into an open literal-union arm: {open:?}"
    );
}

#[test]
fn conditional_response_narrows_and_is_proof_checked() {
    // [deduce-conditional] A `-> list is NonEmpty` fn returns Bool; its body
    // must prove the claim on the `true` path.
    let prelude = "export qualifier NonEmpty<T> of List<T> {\n    \
         fn qualifies(list: List<T>) -> Bool {\n        return size(list) > 0\n    }\n}\n";

    let ok = errors(&format!(
        "{prelude}fn non_empty<T>(list: List<T>) -> list is NonEmpty => list {{\n    \
         return list is NonEmpty\n}}\n"
    ));
    assert!(ok.is_empty(), "a proven conditional response is accepted: {ok:?}");

    // A `return true` that proves nothing is refused.
    let lies = errors(&format!(
        "{prelude}fn non_empty<T>(list: List<T>) -> list is NonEmpty => list {{\n    \
         return true\n}}\n"
    ));
    assert!(
        lies.iter().any(|e| e.contains("[deduce-conditional]")),
        "an unproven `return true` is refused: {lies:?}"
    );

    // The subject must be a kept parameter.
    let moved = errors(&format!(
        "{prelude}fn non_empty<T>(list: List<T>) -> list is NonEmpty => !list {{\n    \
         return list is NonEmpty\n}}\n"
    ));
    assert!(
        moved.iter().any(|e| e.contains("moves it")),
        "a moved subject is refused: {moved:?}"
    );
}

// ===== [col-key-eligible] where a key's own identity is required =====

/// [col-key-eligible] [cmp-carry] A key needs its **own** `hash`/`eq` only
/// where the identity is resolved by name. Written identities, binders and a
/// `let` pattern the initializer fills all say where the identity comes from,
/// so a struct with none of its own is a key there — the annotation and the
/// call it types agree (found 2026-10-20: the call was accepted and the same
/// value's annotation refused).
#[test]
fn a_written_identity_makes_any_struct_a_key() {
    let errs = std_errors(
        r#"
struct P { x: Int }

fn my_hash(v: P) [] -> Long => v { return hash(v.x) }
fn my_eq(a: P, b: P) [] -> Bool => a, b { return a.x == b.x }

fn count(s: Set<P>(?hash, ?eq)) [] -> Int => s { return size(s) }

fn run() [] -> Int {
    let s = Mut Set<P>(hash = my_hash, eq = my_eq)
    let t: Mut Set<P> = Mut Set(hash = my_hash, eq = my_eq)
    let u: Mut Set<P>(my_hash, my_eq) = Mut Set(hash = my_hash, eq = my_eq)
    return count(s) + count(t) + count(u)
}
"#,
    );
    assert!(errs.is_empty(), "{errs:?}");
    // Where nothing is written the key's own pair is still required, and the
    // refusal names the clause that stamps one.
    let errs = std_errors("struct P { x: Int }\n\nfn count(s: Set<P>) [] -> Int => s { return size(s) }\n");
    assert!(
        errs.iter().any(|e| e.contains("`P` cannot be a key") && e.contains("Hashed<self> by auto")),
        "{errs:?}"
    );
}

/// [col-key-eligible] What no identity fixes is refused however the identity
/// is given, and once: a float key with a hand-written pair, and `Set(1.5)`
/// whose constructor would otherwise also report the `hash` it cannot find.
#[test]
fn an_intrinsically_ineligible_key_is_refused_once() {
    let errs = std_errors(
        r#"
fn dh(d: Double) [] -> Long => d { return 1L }
fn de(a: Double, b: Double) [] -> Bool => a, b { return a == b }

fn run() [] -> Int {
    let t = Mut Set<Double>(hash = dh, eq = de)
    return size(t)
}
"#,
    );
    assert!(errs.iter().any(|e| e.contains("`Double` cannot be a key")), "{errs:?}");
    let errs = std_errors("fn run() [] -> Int {\n    let s = Set(1.5)\n    return size(s)\n}\n");
    assert_eq!(errs.len(), 1, "one mistake, one diagnostic: {errs:?}");
    assert!(errs[0].contains("`Double` cannot be a key"), "{errs:?}");
}

// ===== [cmp-carry] an identity is a declaration =====

/// The program of ROADMAP §0n's aliasing defect: two modules each declare a
/// `my_hash` (and `my_eq`) for one `P` and build sets with them.
const TWO_HASHES: [(&str, &str); 3] = [
    ("p.sv", "export struct P : Hashed<self> by auto { x: Int }\n"),
    (
        "a.sv",
        "import p.P\n\
         export fn my_hash(v: P) [] -> Long => v { return hash(v.x) }\n\
         export fn my_eq(l: P, r: P) [] -> Bool => l, r { return l.x == r.x }\n\
         export fn make_a() [] -> Mut Set<P>(my_hash, my_eq) {\n    return Mut Set<P>(hash = my_hash, eq = my_eq)\n}\n",
    ),
    (
        "b.sv",
        "import p.P\n\
         fn my_hash(v: P) [] -> Long => v { return 7L }\n\
         fn my_eq(l: P, r: P) [] -> Bool => l, r { return true }\n\
         export fn make_b() [] -> Mut Set<P>(my_hash, my_eq) {\n    return Mut Set<P>(hash = my_hash, eq = my_eq)\n}\n",
    ),
];

/// [cmp-carry] [cmp-binder] Two modules' same-named fns are **two
/// identities**: one binder cannot hold both, and the types name each by its
/// module (found 2026-10-20 — they compared equal by name, and the binder was
/// filled with the reading module's `my_hash`, a silently wrong program).
#[test]
fn same_named_identities_of_two_modules_are_two() {
    let main = "import p.P\nimport a.make_a\nimport a.my_hash\nimport b.make_b\n\n\
                fn same(x: Mut Set<P>(?hash, ?eq), y: Mut Set<P>(?hash, ?eq)) [] -> None => x, y {}\n\n\
                fn run() [] -> None {\n    same(make_a(), make_b())\n}\n";
    let mut files = TWO_HASHES.to_vec();
    files.push(("main.sv", main));
    let errs = std_errors_in(&files);
    assert!(
        errs.iter().any(|e| e.contains("Mut Set<P>(my_hash@a, my_eq@a)") && e.contains("Mut Set<P>(my_hash@b, my_eq@b)")),
        "expected the two identities refused, each named by its module: {errs:?}"
    );
}

/// [cmp-binder] A binder filled from a value's identity calls **that**
/// declaration, wherever the value is read: `b`'s private `my_hash` reaches a
/// file that imports `a`'s under the same name, and one that imports neither.
#[test]
fn an_identity_resolves_in_its_declaring_module() {
    for imports in ["import a.my_hash\nimport a.my_eq\n", ""] {
        let main = format!(
            "import p.P\nimport b.make_b\n{imports}\n\
             fn has(x: Mut Set<P>(?hash, ?eq), v: P) [] -> Bool => x, v {{ return contains(x, v) }}\n\n\
             fn run() [] -> Bool {{\n    return has(make_b(), P {{ x: 1 }})\n}}\n"
        );
        let mut files = TWO_HASHES.to_vec();
        files.push(("main.sv", &main));
        let errs = std_errors_in(&files);
        assert!(errs.is_empty(), "with `{imports}`: {errs:?}");
    }
}

/// [cmp-carry] [fn-overload-ambiguous] A written identity has no rung
/// fallback (R10 D23): `SortedSet<Str>(by_len)` beside an imported `by_len` of
/// the same signature is ambiguous, an error naming `by_len@main` and
/// `by_len@lib` — once, though the type is lowered at every use. Before, the
/// most specific rung's (this module's) was taken silently, while the call
/// `by_len(a, b)` was refused.
#[test]
fn a_written_identity_two_modules_fit_is_ambiguous() {
    let lib = "export fn by_len(a: Str, b: Str) [] -> Int => a, b { return size(b) - size(a) }\n";
    let main = "import lib.by_len\n\n\
                fn by_len(a: Str, b: Str) [] -> Int => a, b { return size(a) - size(b) }\n\n\
                fn run() [] -> Int {\n    \
                let m: Mut SortedSet<Str>(by_len) = Mut SortedSet(\"a\", cmp = by_len@main)\n    \
                return size(m)\n}\n";
    let errs = std_errors_in(&[("lib.sv", lib), ("main.sv", main)]);
    let ambiguous: Vec<&String> = errs.iter().filter(|e| e.contains("the identity `by_len` is ambiguous")).collect();
    assert!(
        ambiguous.len() == 1 && ambiguous[0].contains("`by_len@lib` or `by_len@main`"),
        "expected one ambiguity naming both modules: {errs:?}"
    );
    // `by_len@main` settles it.
    let settled = main.replace("SortedSet<Str>(by_len)", "SortedSet<Str>(by_len@main)");
    let errs = std_errors_in(&[("lib.sv", lib), ("main.sv", &settled)]);
    assert!(errs.is_empty(), "{errs:?}");
}
