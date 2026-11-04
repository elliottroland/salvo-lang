// Effect variables [effect-var]: a generic written `[E]` ranges over a *set*
// of effects. A call binds it to what the fn value it is handed performs
// beyond the effects the parameter's type names; inside the body it is
// opaque — passed on, never performed.

import std.throw.Throw

effect Say {
    fn say(s: Str) -> None => s
}

fn each<T, [E]>(xs: List<T>, f: (x: T) [E] -> None) => xs, f {
    for x in xs {
        f(x)
    }
}

// ===== declarations =====

// Nothing a call is handed could bind it.
//~ error[effect-var:undetermined]
fn not_bound<[E]>(n: Int) [E] {
}

// A plain type parameter is a type, not an effect.
//~ error[effect-var:not-declared]
fn plain_in_list<T>(f: () [T] -> None) => f {
}

// An effect variable is not a type of values: its handle is `eff[E]`.
//~ error[effect-var:as-type]
fn as_type<[E]>(x: E, f: () [E] -> None) => x, f {
}

// Each effect it is bound to keeps its own identity strength.
//~ error[effect-var:any]
fn weak<[E]>(f: () [E] -> None) [any E] => f {
}

// One variable per fn type: a call could not split the value's effects.
//~ error[effect-var:two-in-fn-type]
fn two<[E], [F]>(f: () [E, F] -> None) => f {
}

// A written list is the whole set: `[]` does not have `E`, so the value
// cannot be called.
fn cannot_call<[E]>(f: () [E] -> None) [] => f {
    //~ error[effect-available:fn-value]
    f()
}

// ===== calls =====

fn thrower(x: Int) [Throw<Str>] {
    throw("no")
}

// A parameter standing for one effect takes no set of them.
fn one<E>(h: eff[E]) => h {
}

fn calls() [Say] {
    // A throw is control flow, not a handle.
    //~ error[effect-var:throw]
    each(List(1), thrower)
    // Fine: the lambda performs `Say`, which binds `E`.
    each(List(1), (x) -> say("x"))
}

handler Speaker() for Say {
    fn say(s: Str) -> None => s {}
}

effect Other {
    fn other() -> None
}

handler Quiet() for Other {
    fn other() -> None {}
}

fn handles() [use] {
    let s = use Speaker()
    let o = use Quiet()
    //~ error[effect-var:row-for-type]
    one(eff[s, o])
}

//--- an_effect_variable_on_an_effect
// A function, a struct or a `fns` group declares one; nothing else does.
//~ error[effect-var:misplaced]
effect Later<[E]> {
    fn later() -> None
}

//--- struct_effect_variables
// A struct's effect variable is named by a field and given as a list.
struct Later<[E]> {
    n: Int,
    run: (x: Int) [E] -> Int
}

// No field names it, so no value could bind it.
//~ error[effect-var:undetermined]
struct Idle<[E]> {
    n: Int
}

// A list fills an effect variable, and nothing else.
//~ error[effect-var:row-misplaced]
fn wrong_list(xs: List<[Console]>) => xs {
}

// An effect variable takes a list.
//~ error[effect-var:row-expected]
fn wrong_arg(l: Later<Int>) => l {
}

fn fine(a: Later<[]>, b: Later<[Console]>) => a, b {
}

// A trailing effect variable left out is `[]`.
fn left_out(a: Later) -> Later<[]> => !a {
    return a
}

//--- group_effect_variables
// [implicit-group] A `fns` group declares one: a spread or an obligation
// fills it with a list, or leaves the trailing one out for `[]`.
fns Gen<S, [E]> {
    fn gen(s: S) [E] -> Int
}

// A list fills an effect variable, and nothing else.
//~ error[effect-var:row-misplaced]
fn wrong_list<S>(s: S, ?Gen<[Console], [Console]>) => s {
}

// An effect variable takes a list.
//~ error[effect-var:row-expected]
fn wrong_arg<S>(s: S, ?Gen<S, Int>) => s {
}

// A member performing what its obligation's row does not name.
//~ error[group-obligation:member-effects]
struct Quiet : Gen<self> {
    n: Int
}

fn gen(q: Quiet) [Console] -> Int {
    println("quiet")
    return q.n
}

struct Loud : Gen<self, [Console]> {
    n: Int
}

fn gen(l: Loud) [Console] -> Int {
    println("loud")
    return l.n
}

// The spread names the fn's own variable, which a fill binds.
fn shared<S, [E]>(s: S, ?Gen<S, [E]>) -> Int => s {
    return gen(s)
}

// Left out, the spread is pure.
fn pure_spread<S>(s: S, ?Gen<S>) -> Int => s {
    return gen(s)
}

fn calls() [Console] {
    let _a = shared(Loud { n: 1 })
    //~ error[implicit-resolve:no-fit]
    let _b = pure_spread(Loud { n: 2 })
}

//--- an_actor_effect_through_a_handle
// An actor's handle is not its instance, which is what a pack holds: one
// passed in as `eff[E]` is made the instance, but none is made back into an
// address yet — a returned `eff[E]`, or one read out of a struct's field.
actor effect Ping {
    send fn ping(n: Int)
}

handler Pinger() for Ping {
    mailbox { capacity: 4 }
    send fn ping(n: Int) {}
}

fn later<[E]>(fx: eff[E], f: () [E] -> None) [use] => fx, f {
    use fx
    f()
}

fn give<[E]>(fx: eff[E], f: () [E] -> None) [] -> eff[E] => !fx, f {
    return fx
}

struct Held<[E]> {
    fx: eff[E]
}

fn main() [use, spawn] {
    let p = spawn Pinger() on pool(1)
    let q = copy(p)
    use q
    // Going in is fine.
    later(copy(p), () -> ping(1))
    //~ error[effect-var:actor-handle]
    let _back = give(copy(p), () -> ping(2))
    let h = Held { fx: copy(p) }
    //~ error[effect-var:actor-handle]
    let _out = h.fx
}
