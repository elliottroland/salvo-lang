// [op-order] [op-equality] The operators through the groups: `a < b` is
// `cmp(a, b) < 0` and `a == b` is `eq(a, b)`, so both work wherever the
// function is — including at a `Str`: `cmp(Str, Str)` is code-point order on
// both backends.
struct Point : Ordered<self> by auto, Eq<self> by auto {
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

// [op-order] Comparing an **unconstrained generic** is the ordinary
// missing-capability error, and the remedy it names is the signature
// (`?Ordered<T>`).
fn largest<T>(a: T, b: T) [] -> Bool => a, b {
    //~ error[call-resolve:no-overload]
    return a < b
}

fn largest_asked<T>(a: T, b: T, ?Ordered<T>) [] -> Bool => a, b {
    return a < b
}

fn go() [] -> Bool {
    return largest_asked(1, 2)
}

// [op-equality] The same for equality, which is the half that makes it
// *opt-in* — and the same colouring at a generic (naming `?Eq<T>`).
struct Note {
    text: Str
}

fn same_note(a: Note, b: Note) [] -> Bool => a, b {
    //~ error[call-resolve:no-overload]
    return a == b
}

fn same_generic<T>(a: T, b: T) [] -> Bool => a, b {
    //~ error[call-resolve:no-overload]
    return a == b
}

// [op-no-none] [op-equality] A possibly-`None` operand is refused once: its
// call would only repeat the refusal (one mistake, one diagnostic).
fn optional(x: Int?) [] -> Bool => x {
    //~ error[op-no-none:optional-operand]
    return x < 1
}

// [op-equality] The operator reads what its call returns: an `eq` returning
// anything but `Bool` is not of `==`'s shape [op-unique-shape], and a call that
// reaches one is refused at the operator.
struct P {
    v: Int
}

fn eq(a: P, b: P) [] -> Int => a, b {
    return 0
}

fn same_p(a: P, b: P) [] -> Bool => a, b {
    //~ error[op-order:return-type]
    return a == b
}

// [op-order] Ordering `Double`s keeps the host's IEEE order where no `cmp` is
// in reach (std declares none: a sorted container of them has no total
// order), and an enclosing `?cmp` takes it as it takes the call
// [implicit-resolve-body] (PIPELINE.md §II.10 P12).
fn plain(a: Double, b: Double) [] -> Bool => a, b {
    return a < b
}

fn through(a: Double, b: Double, ?cmp: (a: Double, b: Double) -> Int) [] -> Bool => a, b {
    return a < b
}

// [op-equality] No same-type rule (R10 D6): `a == b` on two types is the
// call `eq(a, b)`, so it is refused only when no `eq` takes the pair — saying
// so of the operator — and accepted once one is declared, with `B`'s operand
// the second argument.
struct A : Eq<self> by auto {
    v: Int
}

struct B : Eq<self> by auto {
    v: Int
}

struct C : Eq<self> by auto {
    v: Int
}

fn probe_ab(a: A, b: B) [] -> Bool => a, b {
    //~ error[call-resolve:no-overload]
    return a == b
}

fn eq(a: A, b: C) [] -> Bool => a, b {
    return a.v == b.v
}

fn probe_ac(a: A, c: C) [] -> Bool => a, c {
    return a == c
}

//--- a member beside a fn of one signature
// [op-equality] [effect-member-overload] An available effect's member `eq`
// is a candidate for `==` as for the call `eq(a, b)` (R10 D3), so beside a fn
// of the same signature the operator is the call's ordinary ambiguity, said of
// the operator.
struct P {
    v: Int
}

fn eq(a: P, b: P) [] -> Bool => a, b {
    return true
}

effect Same {
    fn eq(a: P, b: P) -> Bool => a, b
}

fn probe(a: P, b: P) [Same] -> Bool => a, b {
    //~ error[fn-overload-ambiguous:call]
    return a == b
}

fn call(a: P, b: P) [Same] -> Bool => a, b {
    //~ error[fn-overload-ambiguous:call]
    return eq(a, b)
}
