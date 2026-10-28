// Lambdas share fate with what they capture (L4).

//--- l4_lambda_captures
// [fate-lambda] L4: lambdas are ordinary values with shared fate.
// Immutable captures are free; a transitively-mutable read capture
// fate-links the closure to the variable (mutating it poisons the
// closure); a mutated capture is consumed at lambda creation (claimed
// when the enclosing fn's contract is inferable, an error when it is a
// written-kept parameter); and a lambda can never consume a capture
// (it may run any number of times) — `copy` is the remedy everywhere.
//
// NOTE (deduction-defaults change): `infer_claims`'s consumption of `xs`
// used to be inferred purely from the lambda capturing-and-mutating it;
// deductions are no longer inferred from the body, so the claim now has
// to be written (`=> !xs`). The test's point (the claim, once established,
// propagates to the caller's argument) still holds.
fn apply(f: (Int) -> Int, v: Int) -> Int {
    return f(v)
}

fn run(f: () -> None) {
    f()
}

fn consume_list(v: List<Int>) -> None => !v {
}

// The positive matrix (immutable captures, pre-mutation use, `copy`
// remedies) is clean.
fn immutable_free() -> Int {
    let base = 10
    let text = "hi"
    let f = (n: Int) -> { return n + base + size(text) }
    let r = apply(f, 1)
    return r + base + size(text)
}

fn used_before_mutation() -> Int {
    let xs = Mut List(1, 2)
    let f = (n: Int) -> { return n + size(xs) }
    let r = apply(f, 1)
    add(xs, 9)
    return r + size(xs)
}

fn mutate_capture_remedy() -> Int {
    let xs = Mut List(1, 2)
    let snapshot = copy(xs)
    let g = () -> { add(snapshot, 1) }
    run(g)
    return size(xs)
}

// A capture-consuming lambda is legal since L7b but `once`-typed
// [fn-once]: passing it where a plain fn is expected fails at the
// boundary.
fn move_capture_rejected() {
    let xs = List(1, 2)
    let h = () -> { consume_list(xs) }
    //~ error[call-resolve:no-overload]
    run(h)
}

fn move_capture_remedy() {
    let xs = List(1, 2)
    let h = () -> { consume_list(copy(xs)) }
    run(h)
}

// A written-kept parameter cannot be captured-and-mutated.
fn kept_mutates(xs: Mut List<Int>) -> None => xs: Mut {
    //~ error[fate-lambda:captures-kept]
    let g = () -> { add(xs, 1) }
    run(g)
}

fn infer_claims(xs: Mut List<Int>)
=> !xs {
    let g = () -> { add(xs, 1) }
    run(g)
}

//--- l4_mutating_a_read_capture_poisons_the_closure
// Mutating a mutable read-capture's root poisons the closure: the IR
// flow pass's diagnostic since R8 piece 3c.
fn apply(f: (Int) -> Int, v: Int) -> Int {
    return f(v)
}

fn poisoned_after_mutation() -> Int {
    let xs = Mut List(1, 2)
    let f = (n: Int) -> { return n + size(xs) }
    let before = apply(f, 1)
    add(xs, 9)
    //~ error[fate-poison:shared-root]
    return before + apply(f, 1)
}

//--- l4_a_mutated_capture_is_consumed_at_creation
// [flow-use-after-move] The uses after a move are the IR flow pass's.
fn run(f: once () -> None) {
    f()
}

// A mutated capture is consumed at creation (the lambda's line).
fn mutate_capture_consumes() -> Int {
    let xs = Mut List(1, 2)
    let g = () -> { add(xs, 1) }
    run(g)
    //~ error[flow-use-after-move:consumed]
    return size(xs)
}

fn infer_claims(xs: Mut List<Int>)
=> !xs {
    let g = () -> { add(xs, 1) }
    run(g)
}

// The capture claim propagates: the caller's argument is consumed.
fn claim_reaches_caller() -> Int {
    let xs = Mut List(1)
    infer_claims(xs)
    //~ error[flow-use-after-move:consumed]
    return size(xs)
}

//--- a_capturing_lambda_is_a_view_of_its_captures
// [fate-lambda-view] A capturing lambda is a *view*: it holds borrows of its
// non-Copy read captures, so binding it links it to the captured roots,
// a call result linked to the lambda reaches them transitively, and
// consuming a capture while the closure lives poisons it. A capture-free
// lambda holds nothing — `map(p, w -> w)` needs no ceremony. (User
// decision 2026-09-12; closed the hole where naming the lambda evaded
// the discipline.)
// Scenarios 1 and 2 are clean: exactly the one violation.
fn eat(xs: List<Str>) -> None => !xs {}

fn main() [use] {
    use StdConsole()
    // 1. capture-free: the one-liner binds with no links
    let words = List("ann", "bo")
    let p = iter(words)
    let kept = map(p, w -> w)
    println("${size(kept)}")
    // 2. a capture-rooted projection: legal, linked to the capture
    let all = List("a", "b", "c", "d")
    let indices = List(1, 3)
    let picked = map(iter(indices), i -> (get(all, i) ?: unreachable!()))
    println("${size(picked)}")
    // 3. the named-lambda form is linked the same way: consuming
    // the capture poisons the result derived through the closure
    let all2 = List("a", "b")
    let f = i -> (get(all2, i) ?: unreachable!())
    let picked2 = map(iter(indices), f)
    eat(all2)
    //~ error[fate-poison:shared-root]
    println("${size(picked2)}")
}
