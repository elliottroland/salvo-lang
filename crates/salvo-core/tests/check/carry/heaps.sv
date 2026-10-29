// FILE: heaps.sv
// [cmp-carry] Structures that **hold** an ordering: the identity in the
// type, the fn slot that declares where it goes, and the `?cmp` binder.
//
// The ordering round's last step (user decisions 2026-09-21, the ordering round
// decisions 1, 11 and 12). What these cases pin down is that the identity
// travels *in the type* — two heaps ordered differently are different types
// and refuse to mix — and that a signature's `?cmp` is one binding: filled by
// resolution where the fn declares it (and published by the result type), and
// captured from the argument types where it does not.
//
// The declarations every case shares: a struct with a canonical ordering
// [fn-attached], a second ordering of its own, and the heap qualifier with
// its fn slot. They check clean on their own: a fn slot in a qualifier's
// generics list, a binder in a parameter type, in a constructor's clause, and
// a bare `Heap` where the body never needs the identity. (The module is
// `heaps`, not the old `heap`, beside std's `std.heap`.)

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

// FILE: main.sv
import heaps.Person
import heaps.Heap
import heaps.Ranked
import heaps.by_name
import heaps.empty_heap
import heaps.empty_ranked
import heaps.heap_push
import heaps.heap_size
import heaps.heapify

// The round trip: build under an ordering, push under the same one, read the
// size without naming it. What the caller never writes is the ordering — it
// is in the type from the construction on.
fn round_trip() [] -> Int {
    let h = empty_heap<Person>()
    let h2 = heap_push(h, Person {name: "a", age: 3})
    return heap_size(h2)
}

// [cmp-carry-override] The carried ordering written out by its attached name
// is the one it carries; another ordering is not.
fn written_again() [] -> Int {
    let h = empty_heap<Person>()
    let h2 = heap_push(h, Person {name: "a", age: 3}, cmp = cmp@Person)
    //~ error[cmp-carry:override-disagrees]
    let h3 = heap_push(h2, Person {name: "b", age: 4}, cmp = by_name)
    return heap_size(h3)
}

// Two orderings are two types. A heap built by `by_name` does not fit a
// parameter demanding the canonical one, and the diagnostic reads the
// identity out of the type ("Heap<Person>(by_name)") — which is the whole
// reason it is in the type.
fn wants_canonical(h: Heap<Person>(cmp@Person) Mut List<Person>) [] -> Int => h {
    return 0
}

fn two_orderings() [] -> Int {
    let h = empty_heap<Person>(cmp = by_name)
    //~ error[call-resolve:no-overload]
    return wants_canonical(h)
}

// And the other way round, where an annotation shows both sides
// ("Heap<Person>(cmp@Person)" and "Heap<Person>(by_name)").
fn two_orderings_annotated() [] -> Int {
    //~ error[var-let-infer:annotation-mismatch]
    let _h: Heap<Person>(cmp@Person) Mut List<Person> = empty_heap<Person>(cmp = by_name)
    return 0
}

// [cmp-binder] The same signature's `?cmp` twice is **one** binding, so two
// arguments must carry the same ordering: `heap_merge`'s rule (the ordering
// round's decision 11).
fn merge<T>(a: Heap<T>(?cmp) Mut List<T>, b: Heap<T>(?cmp) Mut List<T>) [] -> Int => a, b {
    return cmp(a.size(), b.size())
}

fn merge_disagreeing() [] -> Int {
    let one = empty_heap<Person>()
    let other = empty_heap<Person>(cmp = by_name)
    //~ error[call-resolve:no-overload]
    return merge(one, other)
}

// [cmp-binder] Two heaps under the *same* ordering merge without the caller
// saying so.
fn merge_agreeing() [] -> Int {
    let one = empty_heap<Person>()
    let other = empty_heap<Person>()
    return merge(one, other)
}

// [cmp-binder] The binder is callable in the body, like any implicit:
// the ordering the *caller's* structure was built with is what the body
// compares with.
fn least<T>(heap: Heap<T>(?cmp) Mut List<T>, a: T, b: T) [] -> Int => heap, a, b {
    return cmp(a, b)
}

fn binder_callable() [] -> Int {
    let h = empty_heap<Person>()
    return least(h, Person {name: "a", age: 1}, Person {name: "b", age: 2})
}

// [cmp-binder] A deduction entry names qualifiers, not their arguments — and
// keeping the qualifier keeps the **identity**, since it lives in the type and
// the fn could not have changed it. The proof is the second call: it demands
// the identity by name, which a claim that had lost its argument could not
// satisfy.
fn touch<T>(r: Heap<T>(?cmp) Mut List<T>) [] -> Int => r: Heap Mut {
    return size(r)
}

fn keeps_the_identity() [] -> Int {
    let h = empty_heap<Person>()
    let n = touch(h)
    return n + wants_canonical(h)
}

// [cmp-carry] A `fns` group spread into a **slot list** declares one slot
// per member (user decision 2026-09-22), so `Ranked<T>(?Ordered<T>)` is the
// `Heap` declaration written the short way — and a use site fills it by naming
// the member's slot.
fn least_ranked<T>(r: Ranked<T>(?cmp) List<T>, a: T, b: T) [] -> Int => r, a, b {
    return cmp(a, b)
}

fn group_spread() [] -> Int {
    let r = empty_ranked<Person>()
    return least_ranked(r, Person {name: "a", age: 1}, Person {name: "b", age: 2})
}

// [cmp-binder] A slot a signature does not mention is **not constrained** — the
// value keeps carrying it — which is what makes a written slot list a pattern
// rather than an exact type, and a bare `Heap` the empty case of one rule.
fn size_only<T>(h: Heap Mut List<T>) [] -> Int => h {
    return size(h)
}

fn unmentioned_slot() [] -> Int {
    let a = empty_heap<Person>()
    let b = empty_heap<Person>(cmp = by_name)
    return size_only(a) + size_only(b)
}

// [cmp-binder] Two heaps in one signature need two names, and the **alias**
// gives the second one: `?cmp: cmp2` fills the `cmp` slot under the name
// `cmp2`.
fn compare_across<T>(a: Heap<T>(?cmp) List<T>, b: Heap<T>(?cmp: cmp2) List<T>, x: T, y: T) [] -> Int => a, b, x, y {
    // Each ordering is reachable by its own name.
    return cmp(x, y) + cmp2(x, y)
}

fn aliased() [] -> Int {
    let one = empty_heap<Person>()
    let other = empty_heap<Person>(cmp = by_name)
    return compare_across(one, other, Person {name: "a", age: 1}, Person {name: "b", age: 2})
}

// [cmp-binder] …and an alias does not make the *operator* unambiguous: two
// orderings in one scope are two candidates for `<` whatever they are called,
// so the body must name the one it means ("in scope", `cmp`, `cmp2`).
// Aliasing is how two orderings get into one scope, not how the choice
// between them is dodged.
fn pick<T>(a: Heap<T>(?cmp) List<T>, b: Heap<T>(?cmp: cmp2) List<T>, x: T, y: T) [] -> Bool => a, b, x, y {
    //~ error[implicit-same-name:operator]
    return x < y
}

// [deduce-reapply] [cmp-carry] A fn that **establishes** a carried claim
// publishes the identity the *call* resolved, exactly as a constructor does:
// `heapify(xs)` under one ordering and under another produce two types, which
// is what a position demanding one of them can then tell apart
// ("Heap<Person>(by_name)").
fn established_by_name(xs: Mut List<Person>) [] -> Int => xs: Mut {
    heapify(xs, Person {name: "a", age: 1}, cmp = by_name)
    //~ error[call-resolve:no-overload]
    return wants_canonical(xs)
}

// …and under the canonical one the same call fits.
fn established_canonical(xs: Mut List<Person>) [] -> Int => xs: Mut {
    heapify(xs, Person {name: "a", age: 1})
    return wants_canonical(xs)
}

// ===== static identity =====

// [cmp-carry] A fn bound into a type must be **named, top-level and
// capture-free** (decision 12): a lambda has no identity a type can carry,
// and the error says so rather than losing the claim silently.
fn lambda_identity() [] -> Int {
    //~ error[cmp-carry:no-identity]
    let _h = empty_heap<Person>(cmp = (a, b) -> 0)
    return 0
}

// Nor does a local of fn type written in the type itself, a slot filled with
// a name nothing declares, or a selector naming nowhere (`cmp@Nothing`); nor
// is a partial type-argument list accepted [qual-value-arg]. Those four stay
// Rust tests, `carry_tests.rs`: over the real std each reports more than its
// one diagnostic.
