// [effect-state-store] [deduce-infer] Handler storage and member contracts.

qualifier A<T> of List<T>

fn eat(list: List<Int>) -> None => !list { }

// [effect-state-store] [deduce-infer] Storing a value into a handler *state*
// field is a move — the field outlives every member call — so a member that
// promises its parameter back while storing it is rejected. This was a real
// parity divergence: the member declared `[list: Mut]`, Kotlin aliased the
// list into the state (later caller mutations visible) while Rust cloned it
// (invisible), and the same program printed 2 and 1. Handler member bodies
// were never validated against their contracts, because the deduction pass
// only collected top-level fns.
effect Keeper {
    fn keep(list: A List<Int>) -> None => list: A
}

handler KeepBin of Keeper {
    state {
        held: List<Int> = List()
    }

    //~ error[deduce-default:promised-back-moved]
    fn keep(list: A List<Int>) -> None => list: A {
        held = list
    }
}

// [effect-state-store] A member that declares the parameter *moved* may store
// it: that is the honest contract, and callers then give up ownership.
effect Stasher {
    fn stash(list: A List<Int>) -> None => !list
}

handler StashBin of Stasher {
    state {
        held: List<Int> = List()
    }

    fn stash(list: A List<Int>) -> None => !list {
        held = list
    }
}

// [effect-state-store] The **read** direction of the same rule, and the same
// evidence: a member cannot move a value *out* of the handler's storage
// either, because the handler still owns it when the member returns. Rust
// silently `.clone()`d it (so the handler kept a copy and mutable data
// diverged: `eat(items)` then `size(items)` printed 2 on Rust and 3 on
// Kotlin) while Kotlin shared the reference, and where the clone was missing
// — the consuming *intrinsics* — rustc reported a bare E0507 with no Salvo
// diagnostic at all. Found 2026-09-15. Both kinds of storage are covered: a
// `state` field and a constructor parameter.
effect Go {
    fn go() -> None
}

handler StateBin of Go {
    state {
        held: List<Int> = List()
    }

    fn go() -> None {
        //~ error[effect-state-store:moved-out]
        eat(held)
    }
}

handler SeedBin(seed: List<Int>) of Go {
    fn go() -> None {
        //~ error[effect-state-store:moved-out]
        eat(seed)
    }
}

// [effect-state-store] Returning it is the same move, and the shape most
// likely to be written: a getter. `copy` is the remedy there too.
effect Read {
    fn read() -> List<Int>
}

handler ReadBin of Read {
    state {
        held: List<Int> = List()
    }

    fn read() -> List<Int> {
        //~ error[effect-state-store:moved-out]
        return held
    }
}

// [effect-state-store] …and `copy` settles it, on both counts.
effect Both {
    fn read_both() -> List<Int>
    fn go_both() -> None
}

handler CopyBin(seed: List<Int>) of Both {
    state {
        held: List<Int> = List()
    }

    fn read_both() -> List<Int> {
        return copy(held)
    }

    fn go_both() -> None {
        eat(copy(seed))
    }
}

// [copy-scalar-free] A native scalar is exempt: its copy is free and
// indistinguishable from a move, both backends agree, and requiring `copy`
// around every `Int` a member answers with would be noise. This is why the
// first asynchronous test (`sum: Int`, `out.send(sum)`) never met the rule.
fn eat_any<T>(n: T) -> None => !n { }

effect Count {
    fn read_count() -> Int
    fn go_count() -> None
}

handler CountBin of Count {
    state {
        count: Int = 0
    }

    fn read_count() -> Int {
        return count
    }

    fn go_count() -> None {
        eat_any(count)
    }
}

// [fate-move-mode] A handler member's written clause is its own contract, as
// a fn's is: a parameter it consumes is owned in the body, so a binding taken
// from it is move-mode and may be moved on. A kept one stays read-only.
// (Found 2026-09-30: members had no contract in scope, so every derived
// binding in a member was read-only — `let {body} = input` then
// `close(body)` was refused.) [fate-derived-readonly] The S1 error is the IR
// flow pass's since R8 piece 8, which decides the modes.
struct Pair { a: List<Int>, b: List<Int> }

fn sink(l: List<Int>) [] -> None => !l { return None }

effect Take {
    fn take(p: Pair) -> None => !p
    fn look(p: Pair) -> None => p
}

handler Taker() of Take {
    fn take(p: Pair) -> None => !p {
        let {a, b} = p
        sink(a)
        sink(b)
    }

    fn look(p: Pair) -> None => p {
        let {a} = p
        //~ error[fate-derived-readonly:shares-fate]
        sink(a)
    }
}
