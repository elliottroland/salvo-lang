// Comparison, equality and hashing as fns groups [cmp-groups].

//--- the_comparison_groups_lower_to_host_operations
// [cmp-groups] Comparison, equality and hashing as **fns groups** — the
// ordering round's foundation (user decisions 2026-09-21). Every canonical
// implementation for an intrinsic type is exercised directly, then through a
// group spread at a Copy scalar *and* at a `Str`, which is the case that
// makes the position's convention load-bearing: `min_of` compares two values
// and then answers one of them, so a *kept* position must not move what it
// was handed [rs-fn-param-convention].
//
// [cmp-hash-values] No hash **value** appears in the expected output: each
// backend hashes with its host's own algorithm, so the values differ by
// design and only the agreement with `eq` is asserted — the posture `random`
// already has.
//
// Source and expected stdout are **verbatim** the Kotlin backend's
// `kotlinc_compiles_and_runs_the_comparison_groups`. That equality is the
// assertion: `Str` ordering is by code point on both backends, which Kotlin
// has to arrange deliberately [kt-ordered].
//
// The lowerings themselves: each canonical is the host's own operation
// [backend-intrinsic], and `Str` ordering goes through `str`'s byte-wise
// `Ord` — which *is* code-point order.
//
// A group is never a value, so nothing of the three survives as a type.
//~ !emits(main.rs): struct Ordered
//~ !emits(main.rs): struct Hashed
// [cmp-groups] The three capabilities, asked for by name. Nothing about a `T`
// is knowable, so an ordering arrives as an implicit parameter — and the call
// site fills it with the canonical overload for the type it instantiates.
// [rs-fn-param-convention] A kept position borrows: the generic body
// compares `a` and `b` and still owns them afterwards.
//~ emits(main.rs): pub fn min_of<T: Clone>(mut a: T, mut b: T, cmp: &mut dyn FnMut(&T, &T) -> i32) -> T
fn min_of<T>(a: T, b: T, ?Ordered<T>) [] -> T
=> !a, !b {
    if cmp(a, b) <= 0 {
        return a
    }
    return b
}

fn alike<T>(a: T, b: T, ?Eq<T>) [] -> Bool => a, b {
    return eq(a, b)
}

// [cmp-hash-values] A hash value is never printed: it differs between the
// backends by design. What holds on both is that equal values hash equal —
// which is what this answers.
fn digest_agrees<T>(a: T, b: T, ?Eq<T>, ?Hashed<T>) [] -> Bool => a, b {
    if eq(a, b) {
        return hash(a) == hash(b)
    }
    return true
}

fn sign(n: Int) [] -> Str {
    if n < 0 { return "<" }
    if n > 0 { return ">" }
    return "="
}

fn main() [use] {
    use StdConsole()
    let ab = "ab"
    let b = "b"
    let same_ab = "ab"
    // The canonical `cmp` at each intrinsic type, called directly.
    // `Ordering` is a fieldless `#[repr(i8)]` enum whose discriminants are
    // the sign convention `cmp` answers, so the cast is the lowering.
    //~ emits(main.rs): (Ord::cmp(&(1i32), &(2i32)) as i32)
    println("int ${sign(cmp(1, 2))}${sign(cmp(2, 2))}${sign(cmp(3, 2))}")
    println("long ${sign(cmp(to_long(9), to_long(4)))}")
    println("byte ${sign(cmp(to_byte(200), to_byte(3)))}")
    println("char ${sign(cmp('a', 'b'))}")
    println("bool ${sign(cmp(false, true))}")
    // `Str` compares by **code point**, which is what the Kotlin backend has
    // to arrange deliberately: "ab" < "b" because 'a' < 'b'.
    //~ emits(main.rs): (Ord::cmp(&ab[..], &b[..]) as i32)
    println("str ${sign(cmp(ab, b))}${sign(cmp(b, b))}${sign(cmp(b, ab))}")
    // Equality covers the float widths, where no total order exists.
    //~ emits(main.rs): (&ab[..] == &b[..])
    println("eq ${eq(1, 1)} ${eq(1.5, 2.5)} ${eq(to_float(1.0), to_float(1.0))} ${eq(ab, b)}")
    println("same ${alike(7, 7)} ${alike(ab, b)} ${alike(ab, same_ab)}")
    //~ emits(main.rs): std::hash::DefaultHasher::new()
    println("digest ${digest_agrees(ab, same_ab)} ${digest_agrees(3, 3)}")
    // `min_of` answers one of its arguments, so it **moves** them: the
    // deduction is declared `=> !a, !b` [deduce-default], and these two are
    // the last use of their values.
    println("min ${min_of(4, 2)} ${min_of(ab, b)}")
}
