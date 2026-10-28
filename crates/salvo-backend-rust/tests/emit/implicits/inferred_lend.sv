// A fn-typed slot with an opaque return lends its kept positions without
// saying so [proj-infer-fn-type].

//--- inferred_lend
// [proj-infer-fn-type] [rs-proj-lends] A fn-typed slot whose **return is
// opaque** — a bare type parameter, which every pass-minting slot is — lends
// its kept positions without saying so (user decision 2026-09-26). Before
// this, the most ordinary generic-iteration signature in the language emitted
// a closure type with an anonymous input lifetime while the pass it returns
// holds a borrow of the input, and rustc said "lifetime may not live long
// enough" about code nobody wrote. Kotlin ran it, so it was an accept/reject
// divergence [backend-parity].
//
// Two parameters, because one shared lifetime is not merely the simple choice:
// the opaque return's hidden lifetime has to *be* one of them, and separate
// lifetimes would leave rustc no way to know which.
//
// [proj-infer-fn-type] The signature it produces: **one** lifetime, shared by
// the container and by the slot's matching parameter, so the pass the slot
// mints can hold it.
struct Stepped : Next<self, proj Str> canbe Mut {
    items: proj List<Str>,
    at: Int,
    step: Int
}

fn next(p: Mut Stepped) [] -> Emitted (proj(p) Str) | Finished => p: Mut {
    let elem = get(p.items, p.at)
    if elem is None {
        return finished()
    }
    p.at = p.at + p.step
    return emitted(elem)
}

fn iter(list: List<Str>, step: Int) [] -> Mut Stepped => list, step {
    return Mut Stepped { items: list, at: 0, step: copy(step) }
}

// No `holds proj(...)` written on either slot.
// Each lent parameter names its lifetime, and the slot takes both.
//~ emits(main.rs): pub fn count<'k, 'c, C: Clone, K: Clone, It: Clone, T: Clone>(c: &'c C, k: &'k K, iter: &mut dyn FnMut(&'c C, &'k K) -> It
fn count<C, K, It, T>(c: C, k: K, ?iter: (c: C, k: K) -> Mut It, ?Next<It, T>) -> Int
=> c, k {
    let n = 0
    for _ in iter(c, k) {
        n = n + 1
    }
    return n
}

// The one-parameter case ties the same way.
//~ emits(main.rs): pub fn total<'c, C: Clone, It: Clone, T: Clone>(c: &'c C, iter: &mut dyn FnMut(&'c C) -> It
fn total<C, It, T>(c: C, ?iter: (c: C) -> Mut It, ?Next<It, T>) -> Int => c {
    let n = 0
    for _ in iter(c) {
        n = n + 1
    }
    return n
}

fn main() [use] -> None {
    use StdConsole()
    println("${count(List("a", "b", "c", "d", "e", "f"), 2)}")
    println("${total(List("x", "y", "z"))}")
    // The owning pass still fits the same slot: the lifetime it does not use
    // costs it nothing, which is why inferring the lend is free.
    println("${total(Set("p", "q"))}")
}
