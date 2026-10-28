// [col-key-eligible] What may be a **key**: a `Set` element, or a `Map` key.
//
// Hashing and equality are what a hash container needs. The two refusals
// worth testing are the ones a user will hit: `Double`/`Float`, which Rust's
// `f64` cannot hash or compare for equality at all (so accepting it would
// diverge between backends [backend-parity]), and a struct with no `hash`,
// whose route in is `: Hashed<self> by auto`.
//
// The rule is enforced in two places, because a key can arrive two ways:
// *written* (`Map<Double, Str>` in a signature, caught by the declaration
// walk) and *inferred* (`Set(1.5)`, where the key exists only in the
// substitution the call resolved).

// The intrinsic key types are keys, in both containers.
fn intrinsic_keys(a: Set<Str>, b: Set<Int>, c: Map<Long, Str>, d: Map<Char, Int>, e: Map<Bool, Str>) -> Int => a, b, c, d, e {
    return size(a)
}

// [col-key-eligible] `Double` is refused as a key, and the message says why
// rather than just refusing — the reason (backend divergence) is not
// guessable.
//~ error[col-key-eligible:key]
fn double_set(s: Set<Double>) -> Int => s {
    return size(s)
}

//~ error[col-key-eligible:key]
fn double_map(m: Map<Double, Str>) -> Int => m {
    return size(m)
}

// A map's **value** side is unrestricted: only the key has to hash.
fn float_value(m: Map<Str, Double>) -> Int => m {
    return size(m)
}

// [col-key-eligible] [obligation-by] A struct with no `hash` is not a key,
// and the diagnostic names the clause that makes it one.
struct Point {
    x: Int,
    y: Int
}

//~ error[col-key-eligible:key]
fn struct_key(m: Map<Point, Str>) -> Int => m {
    return size(m)
}

// The written rule reaches every declaration site the type walk visits, not
// just parameters: a return type, a struct field, a `let` annotation, and
// inside another container.
//~ error[col-key-eligible:key]
fn in_a_return() -> Set<Double> {
    return Set()
}

struct Holder {
    //~ error[col-key-eligible:key]
    seen: Set<Double>
}

fn in_a_let() -> Int {
    //~ error[col-key-eligible:key]
    let s: Set<Double> = Set()
    return size(s)
}

//~ error[col-key-eligible:key]
fn nested(xs: List<Map<Double, Int>>) -> Int => xs {
    return 0
}

// [col-key-eligible] [cmp-auto] A struct with the structural pair *is* a
// valid key — the whole point of asking for it.
struct Spot : Hashed<self> by auto {
    x: Int,
    y: Int
}

fn hashed_key(s: Set<Spot>, m: Map<Spot, Str>) -> Int => s, m {
    return size(s) + size(m)
}

// A generic parameter is *not* refused at the declaration: like
// [linear-generics], the instantiation is where it is checked, so generic
// code over keyed collections stays writable.
fn count<K>(m: Map<K, Str>) -> Int => m {
    return size(m)
}

// [col-key-eligible] The **inferred** half: `Set(1.5)` writes no type
// anywhere, so the ineligible key exists only in the resolved substitution.
// A type refused as a key is reported once per file (one mistake, one
// diagnostic, when an annotation and the call it types both reach it), so
// each inferred case is a file of its own.
// FILE: inferred_set.sv
fn inferred_set() -> Int {
    //~ error[col-key-eligible:key]
    let s = Set(1.5)
    return size(s)
}

// FILE: inferred_map.sv
fn inferred_map() -> Int {
    //~ error[col-key-eligible:key]
    let _m = Map((1.5, 2))
    return 0
}

// FILE: instantiated.sv
// The instantiation of a generic key is where the refusal lands.
fn count_keys<K>(m: Map<K, Str>) -> Int => m {
    return size(m)
}

fn instantiated() -> Int {
    //~ error[col-key-eligible:key]
    let m = Map((1.5, "x"))
    return count_keys(m)
}
