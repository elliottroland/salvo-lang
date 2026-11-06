// [qual-imply] A qualifier's `for` type may carry qualifiers, and a value
// with the qualifier carries them too: `Positive Int` is `Positive NonZero
// Int`. `is Positive` tests `NonZero` first, so `qualifies` answers only
// about a `NonZero Int`; dropping `NonZero` drops `Positive`.

qualifier NonZero for Int {
    fn qualifies(x: Int) -> Bool { return x != 0 }
}

qualifier Positive for NonZero Int {
    fn qualifies(x: NonZero Int) -> Bool { return x > 0 }
}

fn inverse(x: NonZero Int) [] -> Int => x { return 100 / x }

// The short form: `Positive Int` holds `NonZero`.
fn half(x: Positive Int) [] -> Int => x { return inverse(x) }

fn tested(a: Int) [] -> Int => a {
    if a is Positive {
        return half(a)
    }
    return 0
}

// A slot reference in the `for` type takes the claim's argument.
qualifier AtMost(n: Int) for Int {
    fn qualifies(x: Int, n: Int) -> Bool { return x <= n }
}

qualifier Small(n: Int) for AtMost(n) Int {
    fn qualifies(x: AtMost(n) Int, n: Int) -> Bool { return x >= 0 }
}

fn bounded(x: AtMost(10) Int) [] -> Int => !x { return x }

fn small(x: Small(10) Int) [] -> Int => !x { return bounded(x) }

fn smaller(x: Small(5) Int) [] -> Int => !x {
    //~ error[call-resolve:no-overload]
    return bounded(x)
}

// Dropping what a qualifier implies drops the qualifier.
fn forget(x: Int) [] -> Int => x: -NonZero { return x }

fn dropped(x: Positive Int) [] -> Int => x {
    let y: Positive Int = x
    let _z = forget(y)
    //~ error[call-resolve:no-overload]
    return half(y)
}

// Provenance implies provenance: an `Ordered` binding is `ShardKeyOrdered`.
actor effect Counter {
    send fn add(n: Int)
}

handler Counting() for Ordered Counter {
    mailbox { capacity: 4 }
    send fn add(n: Int) {}
}

fn by_key() [ShardKeyOrdered Counter] -> None { add(1) }

fn ordered() [use] {
    use Counting()
    by_key()
}

//--- refusals
// [qual-imply] Implication runs one way and within a family.
qualifier A for Int {
    fn qualifies(x: Int) -> Bool { return true }
}

//~ error[qual-imply:cycle]
qualifier B for C Int {
    fn qualifies(x: Int) -> Bool { return true }
}

//~ error[qual-imply:cycle]
qualifier C for B Int {
    fn qualifies(x: Int) -> Bool { return true }
}

provenance qualifier Trusted for Str

//~ error[qual-imply:family]
qualifier Clean for Trusted Str {
    fn qualifies(x: Str) -> Bool { return true }
}

//~ error[qual-imply:family]
provenance qualifier Signed for A Int
