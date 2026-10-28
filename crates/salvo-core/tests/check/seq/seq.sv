// [seq-iterator] [implicit-group] [implicit-infer] [fn-overload-rank] The
// sequence functions — `map`, `filter`, `reduce` — and what makes them work
// over *any pass*.
//
// There is no `Iterable` trait, and since R5 there is no `Iterable` group
// either: a combinator's subject **is** the pass, and its `next` arrives
// through a `?Next<It, T>` spread [iter-protocol] that each call fills with
// whichever `next` fits. A container is iterated by writing its `iter`
// (`map(iter(xs), f)`), which is what keeps the inference ordinary. Two facts
// carry the design:
//
// * `It` is bound by an ordinary argument, so the spread resolves with the
//   plain implicit machinery — and `T` comes from **which `next` filled the
//   implicit**, which is what lets a bare lambda be typed [call-type-args];
// * expected types have to reach a lambda even though the name is
//   *overloaded* (each function also has a `List` fast path), which is what
//   the specificity ranking's *lead candidate* is for — re-narrowed per
//   argument, so the subject decides.
//
// Which overload a call went to is `seq_tests.rs`'s (a table).

// ===== the generic path: any pass =====

// [implicit-infer] A list, iterated by writing its `iter`: `It` from the
// argument, `T` from the `next` that fills the implicit, `U` from the
// lambda's body.
fn a_pass_subject_infers_everything() -> None {
    let xs = List(1, 2)
    let _ys: Mut List<Int> = map(iter(xs), n -> n * 2)
}

// A `Str` pass yields *characters*, so the lambda's parameter is a `Char` —
// which is only knowable from the `next` that fills the implicit. If the
// `List` fast path had still been leading [fn-overload-rank], the lambda
// would have been typed against `T` from `List<T>` — unbound — and `U` would
// not have been inferable.
fn a_str_pass_iterates_characters() -> None {
    let _ys: Mut List<Bool> = map(iter("ab"), c -> c == 'a')
}

// ... and the element really is a `Char`, not a `Str`.
fn a_char_is_not_a_str() -> None {
    //~ error[call-resolve:no-overload]
    let _ys: Mut List<Int> = map(iter("ab"), c -> size(c))
}

// Chains compose, because `map`'s result is a `Mut List` and that has an
// `iter` like any other list — one call more than the old `?Iterable`
// surface, and no cross-implicit inference (user decision 2026-09-09).
fn chains_compose_through_iter() -> None {
    let xs = List(1, 2)
    let _zs: Mut List<Int> = map(iter(map(xs, n -> n * 2)), n -> n + 1)
}

// A type of your own joins in by declaring an `iter` that answers a pass —
// nothing else, and no trait.
struct Bag {
    items: List<Int>
}

fn iter(bag: Bag) -> Mut ListNext<Int> => bag {
    return iter(bag.items)
}

fn a_user_type_becomes_iterable() -> None {
    let b = Bag {items: List(1, 2)}
    let _total: Int = reduce(iter(b), 0, (a, x) -> a + x)
}

// A subject that is not a pass is an error. It surfaces as *no matching
// overload* rather than as the missing implicit: the pass position is
// `Mut It`, and an `Int` cannot be `Mut`, so the candidate is out before its
// spread is ever resolved.
fn a_subject_that_is_not_a_pass() -> None {
    //~ error[call-resolve:no-overload]
    let _ys = map(1, n -> n)
}

// ===== [iter-generic-drive] a generic pass is driven by `for` =====

// The rule that lets std's own combinators read like ordinary Salvo (user
// decision 2026-09-09): when a body has a `?Next<It, T>` spread, the position
// *is* the declaration `for` needs — it says "this call supplies a `next` for
// `It`" — so the loop drives by calling that implicit parameter, and the
// element type comes off its result.
fn total<It>(it: Mut It, ?Next<It, Int>) [] -> Int => it: Mut {
    let sum = 0
    for n in it {
        sum = sum + n
    }
    return sum
}

// The element is the pass's to give: `next` returns `Emitted T` **by value**,
// so the loop binding is an owned value and not a projection of the pass —
// which is what lets a combinator move each element into its output.
fn keep<It, T>(it: Mut It, ?Next<It, T>) [] -> Mut List<T> => it: Mut {
    let out = Mut List<T>()
    for x in it {
        add(out, x)
    }
    return out
}

// Without the spread there is nothing to drive with, and the diagnostic is
// the ordinary not-iterable one [iter-resolve]: a bare type parameter says
// nothing.
fn no_spread<It>(it: Mut It) [] -> Int => it: Mut {
    let sum = 0
    //~ error[iter-resolve:not-iterable]
    for _n in it {
        sum = sum + 1
    }
    return sum
}

// The spread's position has to take its state as `Mut`, for the same reason
// a declared `next` does: advancing a pass mutates its position
// [iter-protocol].
fn not_mut<It, T>(it: Mut It, ?next: (It) -> Emitted T | Finished) [] -> Int => it: Mut {
    let sum = 0
    //~ error[iter-protocol:next-not-mut]
    for _n in it {
        sum = sum + 1
    }
    return sum
}

//--- own_pass_named_like_stds
// [implicit-resolve] [fn-overload-scope] A program declaring its own pass
// under a name std also uses — its own `ListNext` plus a `next` for it — used
// to make every drive of that name ambiguous: implicit resolution pooled
// core's `next` and the module's own without the scope ladder every named
// call walks. The most specific rung wins now, so the module's own `next`
// fills the spread and the program checks. (Its own program: its `ListNext`
// would shadow std's for the cases above.)
// The shape is std's own (`std.core.list`): the pass borrows its list, and
// `next` emits projections of it.
struct ListNext<T> : Next<self, proj T> canbe Mut {
    items: proj List<T>,
    at: Int
}

fn mine<T>(items: List<T>) [] -> Mut ListNext<T> => items {
    return Mut ListNext<T> { items: items, at: 0 }
}

fn next<T>(p: Mut ListNext<T>) [] -> Emitted (proj(p) T) | Finished => p: Mut {
    let elem = get(p.items, p.at)
    if elem is None {
        return finished()
    }
    p.at = p.at + 1
    return emitted(elem)
}

fn main() [] -> None {
    let xs = List(1, 2)
    let p = mine(xs)
    for x in p {
        let _y = x
    }
    let ys = List(3, 4)
    let q = mine(ys)
    let _doubled: Mut List<Int> = map(q, (n: Int) -> { return n + n })
    return None
}
