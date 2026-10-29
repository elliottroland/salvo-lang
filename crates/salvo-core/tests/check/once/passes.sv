// [iter-protocol] [fn-once] Hand-written passes over std's step protocol
// (`Next`, `Emitted`, `Finished`): what is left of `once`'s generalization
// after the reduction to `next` (R5) is that **driving a pass consumes it**,
// which holds for a hand-written pass because that is the only kind there is.

// [iter-protocol] [fn-once] A type with a `next` is driven directly rather
// than through `iter`: `for x in c` is `next(c)` when a `next` takes `c`
// [iter-for] (R10 D10; the `: Next<self, T>` clause is a promise checked at
// the struct [group-obligation], not the loop's gate). The whole point: a
// hand-written pass drives a `for` loop, which is what makes `zip`/`merge` —
// the iterators `yield` cannot express — writable at all.
struct Countdown : Next<self, Int> canbe Mut {
    at: Int
}

fn next(c: Mut Countdown) -> Emitted Int | Finished => c: Mut {
    if c.at <= 0 {
        return finished()
    }
    let v = copy(c.at)
    c.at = c.at - 1
    return emitted(v)
}

fn build(from: Int) -> Countdown { return Countdown { at: from } }

fn go() -> None { for _n in build(3) {} }

// A builder may hand the pass over as `Mut` — the qualifier a mutating `next`
// wants — and the loop drives it the same way.
fn build_mut(from: Int) -> Mut Countdown { return Mut Countdown { at: from } }

fn go_mut() -> None { for _n in build_mut(3) {} }

// [iter-drive-in-place] A named pass is driven where it lives (user decision
// 2026-09-12 — a `for` never consumes a variable, which is what lets a linear
// pass be bound, driven and explicitly discharged). Driving an exhausted pass
// again is meaningful: `next` keeps answering `Finished`, so the second loop
// runs zero times.
fn go_twice() -> None {
    let p = build(3)
    for _n in p {}
    for _n in p {}
}

// [iter-for] [iter-protocol] R10 D10 (B1 census D10): `for x in c` is the
// call `next(c)` when a `next` takes `c`, so a type declaring no `: Next<self,
// T>` is driven as the call reaches it — the declaration is a promise checked
// where it is written [group-obligation], not the loop's gate.
struct Undeclared canbe Mut {
    at: Int
}

fn next(c: Mut Undeclared) -> Emitted Int | Finished => c: Mut {
    return finished()
}

fn build_undeclared(from: Int) -> Undeclared { return Undeclared { at: from } }

fn go_undeclared() -> None { for _n in build_undeclared(3) {} }

// [iter-for] [implicit-resolve-body] [implicit-same-name] R10 D11: the
// enclosing fn's own implicit `?next` drives the loop when it takes the
// subject, as it takes the call `next(c)` (no declared `next` takes a
// `Walked`).
struct Walked canbe Mut {
    n: Int
}

fn walk(c: Mut Walked, ?next: (c: Mut Walked) -> Emitted Int | Finished) -> None => c: Mut {
    for _n in c {}
}

// …and two that both fit with none more specific are the call's tie.
fn walk_tied(c: Mut Walked, ?next: (c: Walked | Int) -> Emitted Int | Finished, ?next: (c: Walked | Str) -> Emitted Int | Finished) -> None => c: Mut {
    //~ error[implicit-same-name:ambiguous]
    for _n in c {}
}

// [iter-for] [iter-protocol] A `next` of another result shape is an ordinary
// fn (D-3) and does not make its argument an iterator; with no `iter` either,
// the value is not iterable, and the error says which calls `for` names.
struct Plain canbe Mut {
    n: Int
}

fn next(c: Mut Plain) -> Int => c: Mut { return 1 }

fn go_plain(c: Mut Plain) -> None => c: Mut {
    //~ error[iter-resolve:not-iterable]
    for _n in c {}
}

// [iter-for] [iter-group] A type with both a `next` and an `iter` and no
// obligations is driven by its `next` — "a `next` fits" decides (DECISION
// P15: whether to refuse such a type); declaring both obligations stays
// refused at the type.
struct Other canbe Mut {
    n: Int
}

fn next(o: Mut Other) -> Emitted Str | Finished => o: Mut { return finished() }

struct Both canbe Mut {
    n: Int
}

fn next(c: Mut Both) -> Emitted Int | Finished => c: Mut { return finished() }

fn iter(c: Both) -> Mut Other => c { return Mut Other { n: 0 } }

fn go_both(c: Mut Both) -> Int => c: Mut {
    let total = 0
    for n in c { total = n }
    return total
}

//--- a renamed next, undeclared
// [iter-for] [fn-rename] R10 / B1 census D9: a renamed `next` answers to its
// new name only — in a call, so in the `for` that names one…
struct Countdown canbe Mut {
    at: Int
}

fn next(c: Mut Countdown) -> Emitted Int | Finished => c: Mut {
    return finished()
}

rename fn advance = next(c: Mut Countdown)

fn go(c: Mut Countdown) -> None => c: Mut {
    //~ error[iter-resolve:not-iterable]
    for _n in c {}
}

//--- a renamed next, declared
// …and in the `: Next` obligation it used to satisfy; the step call through
// the new name drives.
//~ error[group-obligation:no-member]
struct Countdown : Next<self, Int> canbe Mut {
    at: Int
}

fn next(c: Mut Countdown) -> Emitted Int | Finished => c: Mut {
    return finished()
}

rename fn advance = next(c: Mut Countdown)

fn go(c: Mut Countdown) -> None => c: Mut { for _n in advance(c) {} }

//--- a member and a fn next tie
// [iter-for] [effect-member-overload] [fn-overload-ambiguous] R10 D3: an
// available effect's member `next` competes with the fn as at the call, and a
// tie is the call's ambiguity, said of the `for`.
struct Ctr canbe Mut {
    n: Int
}

effect Gen {
    fn next(c: Mut Ctr) -> Emitted Int | Finished => c: Mut
}

fn next(c: Mut Ctr) -> Emitted Int | Finished => c: Mut {
    return finished()
}

fn go(c: Mut Ctr) [Gen] -> None => c: Mut {
    //~ error[fn-overload-ambiguous:call]
    for _n in c {}
}
