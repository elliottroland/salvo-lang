// [group-obligation] [group-self] [group-not-a-value] Obligation groups as
// a mechanism (roadmap R1, user decisions 2026-09-08): `: Group<Args>` on a
// struct declaration promises the group's members, checked at the *struct*;
// `Self` inside a `fns` group stands for the declaring type; and no value
// may ever have a group as its type — the single restriction that keeps the
// mechanism a where-clause rather than a trait.
//
// Nothing is designated here: the groups under test are declared here,
// which is the point — R1 is the mechanism, `Next<T>`/`Linear` get
// designated in R2/R4.
//
// A group shaped like the iteration protocol, declared here rather than
// designated by the compiler. The state is an ordinary parameter and comes
// first; a declaring type writes itself as `self` at the obligation
// [group-self].
fns Step<It, T> {
    fn advance(it: Mut It) -> Emitted T | Finished => it: Mut
}

// --- satisfaction [group-obligation] ----------------------------------------

struct Countdown : Step<self, Int> canbe Mut {
    at: Int
}

fn advance(c: Mut Countdown) -> Emitted Int | Finished => c: Mut {
    if c.at <= 0 {
        return finished()
    }
    let v = copy(c.at)
    c.at = c.at - 1
    return emitted(v)
}

// The declaration is the error site, and the message names the missing
// signature with `Self` substituted ("`Unstepped` declares `: Step`",
// "fn advance(Mut Unstepped) -> Emitted Int | Finished"). (`Unstepped` was a
// second `Countdown`.)
//~ error[group-obligation:no-member]
struct Unstepped : Step<self, Int> canbe Mut {
    at: Int
}

// Right name, wrong shape — the state is not `Mut` — is still missing ("no
// visible `advance` matches"). (`Frozen` was a third `Countdown`.)
//~ error[group-obligation:no-member]
struct Frozen : Step<self, Int> canbe Mut {
    at: Int
}

fn advance(c: Frozen) -> Emitted Int | Finished => c {
    return finished()
}

// A generic struct satisfies a group through its own type parameters: the
// candidate's variables are matched against the struct's by renaming.
// (Over the real std `get` lends its element, so the pair is of copies.)
struct Zip<A, B> : Step<self, (A, B)> canbe Mut {
    left: List<A>,
    right: List<B>,
    at: Int
}

fn advance<A, B>(z: Mut Zip<A, B>) -> Emitted (A, B) | Finished => z: Mut {
    let l = get(z.left, z.at)
    let r = get(z.right, z.at)
    if l is A && r is B {
        z.at = z.at + 1
        return emitted((copy(l), copy(r)))
    }
    return finished()
}

// Variable matching is bijective: an `advance` over `Twin<A, A>` does not
// satisfy an obligation whose expected signature is over `Twin<A, B>` ("no
// visible `advance` matches").
//~ error[group-obligation:no-member]
struct Twin<A, B> : Step<self, (A, B)> canbe Mut {
    xs: List<A>
}

fn advance<A>(t: Mut Twin<A, A>) -> Emitted (A, A) | Finished => t: Mut {
    return finished()
}

// Parameter *names* are the group's own; an implementation picks its own.
// (The satisfied cases already use `c`/`z` against the group's `it` — this
// pins that a same-named parameter is not *required* by using a different
// name for every parameter of a two-parameter member.) (`Join` was `Pair`,
// which names the group of the last section here.)
fns Join<S, T> {
    fn combine(a: S, b: S) -> T
}

struct Point : Join<self, Int> {
    x: Int
}

fn combine(first: Point, second: Point) -> Int {
    return first.x + second.x
}

// --- the clause itself -------------------------------------------------------

// "unknown `fns` group `Missing`"; "`Int` is a type; an obligation names a
// `fns` group".
//~ error[group-obligation:unknown-group]
struct Unknown : Missing {
    x: Int
}

//~ error[group-obligation:unknown-group]
struct NotAGroup : Int {
    x: Int
}

// "`Step` takes 2 type argument(s), found 3"; "obligation `Step` is declared
// more than once". A missing-member error for the arity-failed struct would
// be noise: the obligation could not even be read. One error for `Arity`,
// one for `Dup`'s duplicate.
//~ error[group-obligation:type-arg-count]
struct Arity : Step<self, Int, Str> canbe Mut {
    x: Int
}

//~ error[group-obligation:duplicate]
struct Dup : Step<self, Int>, Step<self, Int> canbe Mut {
    x: Int
}

fn advance(b: Mut Dup) -> Emitted Int | Finished => b: Mut {
    return finished()
}

// --- `self` at the obligation [group-self] -----------------------------------

// The point of writing the declaring type as an **argument** rather than as a
// magic `Self` inside the group (user decision 2026-09-08): the group stays
// ordinary, so *one* declaration serves both the obligation and the `?`
// spread. This is what composition needs — a generic combinator reaches its
// source's `next` through an implicit.
fn drive<It>(source: Mut It, ?Step<It, Int>) -> Int => source: Mut {
    let _step = advance(source)
    return 0
}

// [implicit-group] [fn-contract] A spread member's **deduction list is part
// of the position**: `fn advance(it: Mut It) -> …` is filled by an => it: Mut
// implementation that mutates its parameter, which is the whole point of a
// pass. The member's fn type used to be built without its contract, so
// every parameter read as kept-and-immutable and no real `advance` could
// fill the position ("`advance` mutates `it`, which this position does not
// permit") — found when R5's combinator surface was probed.
fn drain<It>(source: Mut It, ?Step<It, Int>) -> Int => source: Mut {
    let n = 0
    let going = true
    while going {
        let step = advance(source)
        when step {
            is Emitted {
                n = n + 1
            }
            is Finished {
                going = false
            }
        }
    }
    return n
}

fn go() -> Int {
    return drain(Mut Countdown { at: 3 })
}

// `self` is the *obligation's* shorthand and nothing else: in a spread it has
// no declaration to refer to ("unknown type `self`").
//~ error[name-resolve:unknown-type]
fn drive_self<It>(source: Mut It, ?Step<self, Int>) -> Int {
    return 0
}

// A group with no obligation involved still spreads exactly as before — the
// mechanism is additive. (`Additive` was `Field`, which `std.core.comptime`
// exports as a struct.)
fns Additive<T> {
    fn add(a: T, b: T) -> T
}

fn total(x: Int, y: Int, ?Additive<Int>) -> Int {
    return add(x, y)
}

fn add(a: Int, b: Int) -> Int {
    return a + b
}

// --- no value has a group type [group-not-a-value] ---------------------------

// The rule that keeps the mechanism from being a trait: a group name is
// refused in *every* value-type position ("`Pair` is a `fns` group, not a
// type"), with one diagnostic (the unknown-type error is suppressed, not
// doubled): a parameter, a return, a `let` annotation, a field, a type
// argument, a union arm.
fns Pair<T> {
    fn combine(a: T, b: T) -> T
}

//~ error[group-not-a-value:data-position]
fn in_a_parameter(p: Pair<Int>) -> Int {
    return 0
}

//~ error[group-not-a-value:data-position]
fn in_a_return() -> Pair<Int> {
    return 0
}

fn in_a_let() -> Int {
    //~ error[group-not-a-value:data-position]
    let _x: Pair<Int> = 0
    return 0
}

struct InAField {
    //~ error[group-not-a-value:data-position]
    p: Pair<Int>
}

//~ error[group-not-a-value:data-position]
fn in_a_type_argument(xs: List<Pair<Int>>) -> Int {
    return 0
}

//~ error[group-not-a-value:data-position]
fn in_a_union_arm(u: Pair<Int> | Str) -> Int {
    return 0
}
