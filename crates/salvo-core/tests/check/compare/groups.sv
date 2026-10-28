// [cmp-groups] Comparison, equality and hashing as **fns groups**: `Ordered`,
// `Eq` and `Hashed` are ordinary `fns` groups in `std.core.compare`, so a
// capability is "a fn of this shape is in scope" and nothing more. These cases
// are about the machinery the language already has — [implicit-resolve]
// picking the canonical overload for the type a call instantiates,
// [implicit-forward] colouring a generic fn that needs one, and
// [group-obligation] checking a promise at a struct — applied to the three
// groups.

// ===== the capability resolves at a concrete type =====

// A direct call picks the canonical overload for its argument types, like any
// other overload [fn-overload] — the groups add nothing to that path.
fn describe(a: Str, b: Str) [] -> Int => a, b {
    if eq(a, b) {
        return 0
    }
    return cmp(a, b)
}

fn bucket(n: Int) [] -> Long => n {
    return hash(n)
}

// Dot notation reaches it too — `a.cmp(b)` *is* `cmp(a, b)` [fn-dot], which
// is what makes a capability read like a method without being one.
fn compare(a: Int, b: Int) [] -> Int => a, b {
    return a.cmp(b)
}

// ===== the capability at a generic, through a group spread =====

// [implicit-group] [implicit-resolve] A `?Ordered<T>` spread makes the
// ordering a parameter, and the *call site* resolves it: at `Int` and at
// `Str` the canonical overloads fit, and the generic body needs no bound.
fn min_of<T>(a: T, b: T, ?Ordered<T>) [] -> T
=> !a, !b {
    if cmp(a, b) <= 0 {
        return a
    }
    return b
}

fn pick() [] -> Int {
    let s = "b"
    let t = "ab"
    let _smaller = min_of(s, t)
    return min_of(2, 3)
}

// All three groups in one signature, which is what a keyed collection's
// operations ask for (`?Hashed<K>` plus `?Eq<K>`): the members are implicit
// parameters in their own right, so two spreads compose.
fn keyed<T>(a: T, b: T, ?Eq<T>, ?Hashed<T>) [] -> Bool => a, b {
    if eq(a, b) {
        return hash(a) == hash(b)
    }
    return true
}

fn use_it() [] -> Bool {
    return keyed(1, 2)
}

// [implicit-forward] A generic fn that asks for nothing cannot compare:
// nothing about an opaque `T` is knowable [call-resolve], so there is no
// default to resolve and the diagnostic names the two remedies.
fn min_unasked<T>(a: T, b: T) [] -> T
=> !a, !b {
    //~ error[call-resolve:no-overload]
    if cmp(a, b) <= 0 {
        return a
    }
    return b
}

// The capability forwards by name and type, not by how it was declared
// [implicit-forward]: a `?Ordered<T>` spread here fills an individually
// declared `?cmp` there.
fn ranked<T>(a: T, b: T, ?cmp: (T, T) -> Int) [] -> Bool => a, b {
    return cmp(a, b) < 0
}

fn sorted_pair<T>(a: T, b: T, ?Ordered<T>) [] -> Bool => a, b {
    return ranked(a, b)
}

fn go() [] -> Bool {
    return sorted_pair(1, 2)
}

// [cmp-auto] [implicit-group] `Hashed<T>` declares the **pair** a hash
// container needs — `hash` to bucket, `eq` to confirm the bucket hit — and two
// spreads asking for the same position ask for one parameter (user decisions
// 2026-09-22). `?Eq<T>` beside `?Hashed<T>` is one `eq`, not a collision.
fn digest_agrees<T>(a: T, b: T, ?Eq<T>, ?Hashed<T>) [] -> Bool => a, b {
    if eq(a, b) {
        return hash(a) == hash(b)
    }
    return true
}

fn probe_digest(x: Int, y: Int) [] -> Bool => x, y {
    return digest_agrees(x, y)
}

// [implicit-same-name] …while two of one name at *different* types are two
// parameters (user decision 2026-09-28), and a call whose arguments fit both
// is the ambiguity, naming both types.
fns Weird<T> {
    fn eq(a: T, b: T) -> Int
}

fn probe_weird<T>(a: T, b: T, ?Eq<T>, ?Weird<T>) [] -> Bool => a, b {
    //~ error[implicit-same-name:ambiguous]
    return eq(a, b)
}

// ===== a type of your own =====

// A struct joins in by declaring the function: no trait, no `impl`, and the
// overload is reached by the same resolution the primitives use.
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

fn younger(a: Person, b: Person) [] -> Person => !a, !b {
    return min_of(a, b)
}

// [group-obligation] The obligation form works on the three groups as it does
// on `Next`: the promise is checked **at the struct**, so a missing `cmp` is
// an error where the promise is written rather than at some distant call.
//~ error[group-obligation:no-member]
struct Unmet : Ordered<self> {
    x: Int,
    y: Int
}

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

// [cmp-auto] A spread resolves **every** member, used or not (decision 19), so
// `?Hashed<T>` at a type with a `hash` and no `eq` is the ordinary
// missing-implicit error naming `eq`. Asking for less is a narrower spread, or
// the members written individually.
struct Key {
    x: Int

    fn hash(value: Key) [] -> Long => value by auto
}

fn bucket_all<T>(v: T, ?Hashed<T>) [] -> Long => v {
    return hash(v)
}

fn probe_key(k: Key) [] -> Long => k {
    //~ error[implicit-resolve:no-fit]
    return bucket_all(k)
}

// Declaring the member it actually needs asks for less, and works.
fn bucket_hash<T>(v: T, ?hash: (T) -> Long) [] -> Long => v {
    return hash(v)
}

fn probe_key_hash(k: Key) [] -> Long => k {
    return bucket_hash(k)
}

// ===== what has no canonical implementation =====

// `Double` has an `eq` and no `cmp`: `NaN` ties with nothing, so no total
// order exists — the same reason a sorted collection of floats stays refused
// [col-sorted]. The refusal is the ordinary "no overload" one.
fn same_double(a: Double, b: Double) [] -> Bool => a, b {
    return eq(a, b)
}

fn ranked_double(a: Double, b: Double) [] -> Int => a, b {
    //~ error[call-resolve:no-overload]
    return cmp(a, b)
}

// A capability a type does not have is refused at the *call*, naming the fn
// — which is what makes "declare one" the obvious remedy.
struct Opaque {
    at: Int
}

fn ranked_opaque(a: Opaque, b: Opaque) [] -> Int => a, b {
    //~ error[call-resolve:no-overload]
    return cmp(a, b)
}
