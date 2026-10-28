// [op-arith] [op-pow] [op-unique-shape] An operator on a type of your own is
// the function its own file declares — `plus`, `minus`, `times`, `divide`,
// `pow` — resolved as the written call is; a missing one is the call's
// refusal, said of the operator.

struct V {
    x: Int
}

struct Scaled {
    v: V
}

fn plus(a: V, b: V) -> V {
    return V { x: a.x + b.x }
}

fn minus(a: V, b: V) -> V {
    return V { x: a.x - b.x }
}

fn times(a: V, k: Int) -> Scaled {
    return Scaled { v: V { x: a.x * k } }
}

fn divide(a: V, k: Int) -> V {
    return V { x: a.x / k }
}

fn uses_all_four(a: V, b: V) -> Scaled {
    let d = (a + b - a) / 2
    return d * 3
}

fn no_plus_for_v_and_int(a: V) -> V {
    //~ error[call-resolve:no-overload]
    return a + 1
}

struct Person {
    name: Str
    age: Int

    fn pow(p: Person, power: Int) -> Person {
        return Person { name: p.name, age: p.age ** power }
    }
}

fn squared(p: Person) -> Int {
    let q = p ** 2
    return q.age
}

//--- a member and a fn tie
// [effect-member-overload] R10 D3: an available effect's member `times`
// takes `*`; beside a fn of its signature the operator is the call's
// ambiguity, said of the operator, and the written call is too.

struct V {
    x: Int
}

effect Scale {
    fn times(a: V, b: V) -> V => a, b
}

fn times(a: V, b: V) -> V {
    return V { x: a.x * b.x }
}

fn by_operator(a: V, b: V) [Scale] -> V {
    //~ error[effect-available:member-or-fn]
    return a * b
}

fn by_call(a: V, b: V) [Scale] -> V {
    //~ error[effect-available:member-or-fn]
    return times(a, b)
}

//--- a member alone
struct V {
    x: Int
}

effect Scale {
    fn times(a: V, b: V) -> V => a, b
}

fn by_operator(a: V, b: V) [Scale] -> V {
    return a * b
}

//--- ordering needs a cmp
// [op-order] A struct needs its `cmp` like anything else.
struct P {
    x: Int
}

fn ordered(a: P, b: P) -> Bool => a, b {
    //~ error[call-resolve:no-overload]
    return a < b
}
