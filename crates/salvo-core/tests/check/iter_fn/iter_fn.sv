// [iter-fn] The `iter fn` form (user decisions 2026-09-09, 2026-09-26): a
// minter under any name, with any parameters, whose body is the step and whose
// **iterator struct is generated**.
//
// The way to be iterable with the least to declare: the parameters stay
// ordinary data, the `state` block names the iterator struct and declares its
// own fields, and the compiler writes the struct plus its `next`. The author's
// body *is* the `next`, so what this file checks is mostly that the generated
// declarations behave like the hand-written ones they stand for — plus the
// `Iter` group [iter-group] and the step-call `for` [iter-step-call].
//
// The expansion happens in `salvo_syntax::desugar`, before resolution, so
// every case here goes through the ordinary checker. The cases' subjects are
// named apart (`S1`, `Bag2`, …) so they share one program.

struct Countdown {
    from: Int
}

iter fn iter(c: Countdown) -> Emitted Int | Finished {
    state CountdownIter {
        at: Int = c.from
    }
    if at <= 0 {
        return finished()
    }
    at = at - 1
    return emitted(at + 1)
}

// --- what the form buys -----------------------------------------------------

// One declaration makes the subject iterable: no pass struct, no constructor.
fn go_iterable() -> None {
    let c = Countdown { from: 3 }
    for _n in c {}
}

// Driving does not consume the subject: the pass holds a *copy*, so a second
// `for` starts over. This is the property that separates a subject from a
// pass [iter-mint].
fn go_twice() -> None {
    let c = Countdown { from: 3 }
    for _n in c {}
    for _n in c {}
}

// The generated `iter` is an ordinary function, so a pass can be *held* — the
// capability a `yield fn` origin does not have, since each `for` re-mints.
fn go_held() -> None {
    let c = Countdown { from: 3 }
    let p = iter(c)
    let _step = next(p)
}

// A `state` field is an ordinary struct field of the pass, so the ordinary
// type rules apply to it.
struct S1 { n: Int }
iter fn iter(s: S1) -> Emitted Int | Finished {
    state S1Iter {
        //~ error[struct-decl:field-mismatch]
        at: Int = "not a number"
    }
    return finished()
}

// [handler-state-infer] [decl-interface-pass] An inferred state field's type is the
// iterator struct's, an interface: a fn declared *above* the `iter fn`
// reads it as one declared below does. It was read off the minter's body
// walk, so the fn above saw `Unknown` and returned an `Int` field from a
// `-> Str` unopposed (found 2026-10-20).
fn peek_above(c: Counts) -> Str {
    //~ error[fn-syntax:return-mismatch]
    return c.at
}

iter fn counts(n: Int) -> Emitted Int | Finished {
    state Counts { at = 0 }
    if at >= n { return finished() }
    at = at + 1
    return emitted(at)
}

fn peek_below(c: Counts) -> Str {
    //~ error[fn-syntax:return-mismatch]
    return c.at
}

// The subject is **read-only** (user decision 2026-09-09): it is a field of
// the pass typed as the subject, so writing through it is the standing
// [struct-mut] refusal rather than a rule of its own.
struct S2 canbe Mut { n: Int }
iter fn iter(s: S2) -> Emitted Int | Finished {
    state S2Iter {
        at: Int = 0
    }
    //~ error[struct-mut:immutable-field-assign]
    s.n = 5
    return finished()
}

// A `state` initializer runs when the pass is minted, and the mint is an
// effect-free call — so an effectful initializer is refused where it is
// written, with no rule of its own [iter-fn].
struct S3 { n: Int }
fn noisy() [Console] -> Int {
    println("hi")
    return 1
}
iter fn iter(s: S3) -> Emitted Int | Finished {
    state S3Iter {
        //~ error[effect-available:callee-no-handler]
        at: Int = noisy()
    }
    return finished()
}

// [iter-fn] Any name: the declaration is the minter, so `countdown(3)` mints
// an iterator and `for n in countdown(3)` drives it (user decision
// 2026-09-26). A parameter that is a Copy scalar is held by value.
iter fn countdown(from: Int) -> Emitted Int | Finished {
    state CountdownState {
        at: Int = from
    }
    if at <= 0 {
        return finished()
    }
    at = at - 1
    return emitted(at + 1)
}
fn go_named() -> Int {
    let sum = 0
    for n in countdown(3) {
        sum = sum + n
    }
    let p = countdown(2)
    for n in p {
        sum = sum + n
    }
    return sum
}

// [iter-fn] Several parameters, of which the non-scalar ones are borrowed by
// the iterator struct and the scalars owned.
iter fn take(xs: List<Int>, count: Int) -> Emitted Int | Finished {
    state Take {
        at: Int = 0
    }
    if at >= count {
        return finished()
    }
    let e = get(xs, at)
    if e is None {
        return finished()
    }
    at = at + 1
    return emitted(copy(e))
}
fn go_take() -> Int {
    let xs = List(1, 2, 3)
    let sum = 0
    for n in take(xs, 2) {
        sum = sum + n
    }
    return sum + size(xs)
}

// An empty block is a struct holding the parameters only.
struct S9 { n: Int }
iter fn iter(s: S9) -> Emitted Int | Finished {
    state S9Iter {
    }
    return finished()
}

// --- reading the subject ------------------------------------------------------

// [iter-fn] Reading a subject field on every turn reads it through the
// iterator's borrow of the subject, typed by the field's declaration.
struct S10 { limit: Int }
iter fn iter(s: S10) -> Emitted Int | Finished {
    state S10Iter {
        at: Int = 0
    }
    if at >= s.limit {
        return finished()
    }
    at = at + 1
    return emitted(copy(at))
}

// A field the subject does not have is reported against the subject's own
// type, not against the generated iterator struct.
struct S11 { limit: Int }
iter fn iter(s: S11) -> Emitted Int | Finished {
    state S11Iter {
        at: Int = 0
    }
    //~ error[struct-field-resolve:no-field]
    if at >= s.limmit {
        return finished()
    }
    return finished()
}

// The body may hand the subject on as a value.
struct S12 { n: Int }
fn describe(s: S12) -> Int => s {
    return s.n
}
iter fn iter(s: S12) -> Emitted Int | Finished {
    state S12Iter {
        left: Int = s.n
    }
    if left <= 0 {
        return finished()
    }
    left = left - 1
    return emitted(describe(s))
}

// A `state` field named like a subject field the body also reads: the
// subject is held whole, so the two never meet.
struct S13 { at: Int, limit: Int }
iter fn iter(s: S13) -> Emitted Int | Finished {
    state S13Iter {
        at: Int = s.at
    }
    if at >= s.limit {
        return finished()
    }
    at = at + 1
    return emitted(copy(at))
}

// --- a generic function over *containers* ------------------------------------

// [implicit-infer] The container-shaped combinator (user decision 2026-09-10):
// a generic fn takes the container and asks for its `iter` as an implicit, so
// the pass type `It` is decided by *which `iter` fills it* — and then the
// `?Next<It, T>` beside it resolves against what that taught.
//
// Before the learning sweep ran after the arguments were typed, `next` was
// reported as ambiguous with `It` still `?`. (The `?iter` said `holds
// proj(c)`, which is inferred now: `proj-infer-fn-type:redundant-holds`.)
fn total<C, It>(c: C, ?iter: (c: C) -> Mut It, ?Next<It, Int>) -> Int =>[iter] c => c {
    let sum = 0
    let p = iter(c)
    for n in p {
        sum = sum + n
    }
    return sum
}

// The pass is the one an `iter fn` generated.
fn go_total() -> Int {
    let c = Countdown { from: 3 }
    return total(c)
}

// The same function over a container with a *written* `iter`.
struct Bag1 { items: List<Int> }
struct BagNext : Next<self, Int> canbe Mut { items: proj List<Int>, at: Int }
fn iter(bag: Bag1) -> Mut BagNext => bag {
    return Mut BagNext { items: bag.items, at: 0 }
}
fn next(p: Mut BagNext) -> Emitted Int | Finished => p: Mut {
    let e = get(p.items, p.at)
    if e is None {
        return finished()
    }
    p.at = p.at + 1
    return emitted(e)
}
fn go_total_written() -> Int {
    let b = Bag1 { items: List(4, 5) }
    return total(b)
}

// [implicit-infer] A written `iter = iter@main` teaches `It` as the bare
// `iter = iter` does, among the fns its selector names, so the `next` written
// beside it resolves at `Mut BagNext` (found by the round trip, PIPELINE B4:
// the written-out form left `It` unknown and `next@main` took `Countdown`'s).
fn go_total_scoped() -> Int {
    let b = Bag1 { items: List(4, 5) }
    return total(b, iter = iter@main, next = next@main)
}

// And the ambiguity the user accepted as the price (2026-09-10): when nothing
// determines the container type either, several `iter`s match and the choice
// would be a guess. The remedies are the ordinary ones — a `rename`, or passing
// the member by name.
fn forward<D>(d: D) -> Int => !d {
    //~ error[implicit-resolve:no-fit]
    return total(d)
}

// --- [iter-group] the `Iter` group -------------------------------------------

// `: Iter<self, It, T>` is satisfied by an `iter fn iter` over the type whose
// state is `It`.
struct Bag2 : Iter<self, Bag2Iter, Int> { items: List<Int> }
iter fn iter(bag: Bag2) -> Emitted Int | Finished {
    state Bag2Iter {
        at: Int = 0
    }
    let e = get(bag.items, at)
    if e is None {
        return finished()
    }
    at = at + 1
    return emitted(copy(e))
}
fn go_bag2() -> Int {
    let b = Bag2 { items: List(1, 2) }
    let sum = 0
    for n in b {
        sum = sum + n
    }
    return sum
}

// …or by a hand-written `iter` returning the named iterator struct.
struct Bag3 : Iter<self, Bag3Iter, Int> { items: List<Int> }
struct Bag3Iter : Next<self, Int> canbe Mut { items: proj List<Int>, at: Int }
fn iter(bag: Bag3) -> Mut Bag3Iter => bag {
    return Mut Bag3Iter { items: bag.items, at: 0 }
}
fn next(p: Mut Bag3Iter) -> Emitted Int | Finished => p: Mut {
    let e = get(p.items, p.at)
    if e is None {
        return finished()
    }
    p.at = p.at + 1
    return emitted(copy(e))
}

// An obligation with no minter behind it is an error at the struct.
//~ error[group-obligation:no-member]
struct Bag4 : Iter<self, Bag4Iter, Int> { items: List<Int> }
struct Bag4Iter : Next<self, Int> canbe Mut { at: Int }
fn next(p: Mut Bag4Iter) -> Emitted Int | Finished => p: Mut {
    return finished()
}

// A type is a source or an iterator struct, never both: `for x in s` would
// have two answers.
//~ error[iter-group:source-and-iterator]
struct Both : Iter<self, BothIter, Int>, Next<self, Int> canbe Mut { at: Int }
iter fn iter(b: Both) -> Emitted Int | Finished {
    state BothIter {}
    return finished()
}
fn next(b: Mut Both) -> Emitted Int | Finished => b: Mut {
    return finished()
}

// `?Iter<C, It, T>`: `for x in c` mints with the `iter` implicit and drives
// with the `next` beside it, and a call fills the pair from whatever `C`
// turns out to be — an `iter fn`'s minter, a named struct's, or std's own.
fn total_spread<C, It>(c: C, ?Iter<C, It, Int>) -> Int => c {
    let sum = 0
    for n in c {
        sum = sum + n
    }
    return sum
}
fn go_spread() -> Int {
    let c = Countdown { from: 3 }
    return total_spread(c)
}

// Inside the spreading fn, `iter(c)` yields the `It`, which a
// `?Next` combinator accepts with the paired `next` forwarded.
fn count_it<It>(it: Mut It, ?Next<It, Int>) -> Int => it: Mut {
    let n = 0
    for _x in it {
        n = n + 1
    }
    return n
}
fn count<C, It>(c: C, ?Iter<C, It, Int>) -> Int => c {
    let p = iter(c)
    return count_it(p)
}
fn go_count() -> Int {
    return count(Countdown { from: 3 })
}

// --- [iter-step-call] `for` over a step call ---------------------------------

// `for i in next(p)` re-invokes the call each turn — which is what lets a step
// under another name be driven at all.
struct Down : Next<self, Int> canbe Mut { at: Int }
fn next(d: Mut Down) -> Emitted Int | Finished => d: Mut {
    if d.at <= 0 {
        return finished()
    }
    d.at = d.at - 1
    return emitted(d.at + 1)
}
fn skip2(d: Mut Down) -> Emitted Int | Finished => d: Mut {
    if d.at <= 1 {
        return finished()
    }
    d.at = d.at - 2
    return emitted(d.at + 2)
}
fn go_step() -> Int {
    let d = Mut Down { at: 6 }
    let sum = 0
    for n in skip2(d) {
        sum = sum + n
    }
    for n in next(d) {
        sum = sum + n
    }
    return sum
}

// Each argument is re-evaluated per turn, so it has to be a place or a
// literal.
fn mk() -> Mut Down {
    return Mut Down { at: 3 }
}
fn go_step_temp() -> None {
    //~ error[iter-step-call:argument-not-place]
    for _n in next(mk()) {}
}

// A step's *result* held in a variable is a union, not a loop.
fn go_step_result() -> None {
    let d = Mut Down { at: 3 }
    let r = next(d)
    //~ error[iter-step-call:step-value]
    for _n in r {}
}

// --- [iter-fn] the named state struct, and every parameter held -----------

// The struct is named by the `state` block, so a fn handing an iterator out
// writes its type (what `-> iter T` used to fill from the body).
fn from(c: Countdown) -> Mut CountdownIter => c {
    return iter(c)
}
fn go_from() -> Int {
    let c = Countdown { from: 3 }
    let sum = 0
    for x in from(c) {
        sum = sum + x
    }
    let p: Mut CountdownIter = from(c)
    let _s = next(p)
    return sum
}

// `iter T` is gone: what wrote it writes a generic with its spread. (The
// old case also deduced `=> it: Mut`, which `iter T` no longer implies.)
//~ error[name-resolve:unknown-type]
fn first(it: iter Int) -> Int {
    return 0
}

//--- duplicate state names
// The name is declared beside the fn, so it collides with a type of the same
// name there. (The type was `Range`, which std declares.) The second half,
// two `iter fn`s with one state name, stays in iter_fn_tests.rs: it also
// reports the generated `next` as a duplicate fn.
struct Interval { a: Int }
iter fn upto(n: Int) -> Emitted Int | Finished {
    //~ error[mod-collision:duplicate]
    state Interval {
        at: Int = 0
    }
    return finished()
}

// --- the refusals -----------------------------------------------------------
// Reported by the desugaring, inside the parser: each its own program.

//--- a variadic parameter
// [iter-fn] A variadic parameter has no field to be held in.
//~ error[iter-fn:variadic-param]
iter fn each(...xs: Int[]) -> Emitted Int | Finished {
    state Each {}
    return finished()
}

//--- a Mut subject
// `Mut` on the subject would promise that advancing writes through it, which
// is the opposite of what a mint does.
struct S canbe Mut { n: Int }
//~ error[iter-fn:qualified-param]
iter fn iter(s: Mut S) -> Emitted Int | Finished {
    state SIter {}
    return finished()
}

//--- the result shape
// The result shape is the obligation's own, and the element type is read out
// of it — so anything else has no element type to declare.
struct S { n: Int }
//~ error[iter-fn:return-type]
iter fn iter(s: S) -> Int {
    state SIter {}
    return 1
}

//--- a binding shadowing a state field
// A binding of a field's name would silently mean something else, so it is
// reported rather than renamed [iter-fn].
struct S { n: Int }
iter fn iter(s: S) -> Emitted Int | Finished {
    state SIter {
        at: Int = 0
    }
    //~ error[iter-fn:shadowing-binding]
    let at = 3
    return finished()
}

//--- a binding shadowing the subject
struct S { n: Int }
iter fn iter(s: S) -> Emitted Int | Finished {
    state SIter {
        at: Int = 0
    }
    //~ error[iter-fn:shadowing-binding]
    for s in List(1, 2) {}
    return finished()
}

//--- a state field named like the subject
// A `state` field named like a parameter is the same collision, caught at the
// declaration rather than at a use.
struct S { n: Int }
iter fn iter(s: S) -> Emitted Int | Finished {
    state SIter {
        //~ error[iter-fn:state-shadows-param]
        s: Int = 0
    }
    return finished()
}

//--- a structural subject
// The generated struct is named after the subject, so the subject needs a
// name: an array, tuple or union has to be wrapped.
//~ error[iter-fn:unnamed-param-type]
iter fn iter(items: Int[]) -> Emitted Int | Finished {
    state ItemsIter {}
    return finished()
}

//--- a state field without an initializer
// Every `state` field is a declaration with a type and an initializer — the
// block is the pass's shape, not code that runs (user decision 2026-09-09).
struct S { n: Int }
iter fn iter(s: S) -> Emitted Int | Finished {
    state SIter {
        //~ error[iter-fn:state-missing-initializer]
        at: Int
    }
    return finished()
}

//--- no state block
// Every `iter fn` names its struct: no `state` block is an error…
iter fn ones(n: Int) -> Emitted Int | Finished {
    //~ error[iter-fn:missing-state]
    return finished()
}

//--- a state block without a name
// …and so is a `state` block without a name.
iter fn ones(n: Int) -> Emitted Int | Finished {
    //~ error[iter-fn:unnamed-state]
    state {}
    return finished()
}
