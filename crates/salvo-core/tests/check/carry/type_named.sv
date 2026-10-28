// [fn-type-named] Rule (c): a type-named fn must return the named base with
// at least the named qualifiers. `fn Mut List<T> -> Mut List<T>` is fine;
// returning a plain `List<T>` (missing `Mut`) is refused. Each a program of
// its own: they declare `List`'s constructors beside std's.
//
// Correct: returns exactly the named type.
fn Mut List<T>(first: T) -> Mut List<T> => !first {
    return Mut List(first)
}

//--- a missing named qualifier
// Missing the named `Mut` qualifier on the return ("must carry the `Mut`
// qualifier").
//~ error[fn-type-named:missing-qualifier]
fn Mut List<T>(first: T) -> List<T> => !first {
    return Mut List(first)
}

//--- extra qualifiers
// Extra qualifiers beyond the named ones are allowed (named ⊆ return).
fn List<T>(first: T) -> Mut List<T> => !first {
    return Mut List(first)
}

//--- provenance
// [fn-type-named] The provenance form: the name *is* a provenance
// qualifier, no base type. A `-> +Good T` establishment lifts into the
// constructor clause, so the named qualifier is carried there. (`Good`/`Bad`
// were `Ok`/`Err`, which std declares.)
provenance qualifier Good<T> of T
provenance qualifier Bad<T> of T

fn Good<T canbe linear>(value: T) [] -> +Good T => !value {
    return value
}

// The return carries a *different* provenance tag than the name demands.
//~ error[fn-type-named:missing-provenance]
fn Bad<T canbe linear>(value: T) [] -> +Good T => !value {
    return value
}

// [qual-arm-infer] A bare value that is the payload of exactly one
// provenance-tagged arm is wrapped into it: `return n` for an
// `Ok Int | Err Str` return is implicitly `Ok(n)`. Over std's own `Ok`/`Err`.
fn classify(n: Int) -> Ok Int | Err Str {
    if n < 0 {
        return "neg"
    }
    return n
}

// Ambiguous: two provenance arms accept an `Int`, so the bare value is
// refused and the explicit constructor is required. That case stays a Rust
// test, `carry_tests.rs`: over the real std it is refused as `type-union:no-arm`
// ("no arm … accepts a value of type `Int`", when two do) beside the return
// mismatch.

// [qual-arm-infer] Not for a literal union with an open provenance arm: a
// plain value must be wrapped explicitly with the open constructor. Over
// std's own `Other`. (Reported twice, the narrowing advice and the return
// mismatch, as `type-union:no-arm` is — ROADMAP §0ar.)
type Class = "A" | "B" | Other Str

fn open_arm(s: Str) -> Class => !s {
    //~ error[type-literal:needs-narrowing]
    //~ error[fn-syntax:return-mismatch]
    return s
}

// [deduce-conditional] A `-> list is NonEmpty` fn returns Bool; its body
// must prove the claim on the `true` path. Over std's own `NonEmpty`.
fn non_empty<T>(list: List<T>) -> list is NonEmpty => list {
    return list is NonEmpty
}

// A `return true` that proves nothing is refused ("claims `list is
// NonEmpty`, but nothing on this path proved it").
fn non_empty_lies<T>(list: List<T>) -> list is NonEmpty => list {
    //~ error[deduce-conditional:unproven]
    return true
}

// The subject must be a kept parameter ("moves it").
//~ error[deduce-conditional:moved]
fn non_empty_moved<T>(list: List<T>) -> list is NonEmpty => !list {
    return list is NonEmpty
}
