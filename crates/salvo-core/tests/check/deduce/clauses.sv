// [deduce-syntax] [deduce-default] [deduce-infer] The `=>` clause checked
// against the body: what a written entry or a default promises the caller,
// and the diagnostics when the body does something else. What the clauses
// *record* (kept, surviving qualifiers, the effect's form) is read from the
// checker's tables in `deduce_tests.rs`.

qualifier A<T> of List<T>
qualifier B<T> of List<T>

fn drop_a<T>(list: A B List<T>) [] -> None => list: B {}
fn consume<T>(list: List<T>) [] -> None => !list {}

// [deduce-default] A `Mut` parameter defaults to the exhaustive `: Mut` form
// — only `Mut` survives, since a mutable parameter the body may change cannot
// promise other qualifiers back (user decision 2026-10-11). A written
// `=> list` (keep-all) that promises more than `Mut` on a mutated parameter
// is then an error: the body may remove the extra qualifier.
fn mutate<T>(list: Mut List<T>) [] -> None => list: Mut {}

//~ error[deduce-gained:qualifier-removed]
fn keeps_all<T>(list: Mut A List<T>) -> None => list {
    mutate(list)
}

// [deduce-default] A move must now be **declared**: a plain fn that returns
// its parameter, or passes it to a consuming call, with no clause defaults to
// keep and so is an error naming the `=> !p` remedy (user decision
// 2026-10-11, replacing body-inference of moves). [fate-move-mode] S2 used to
// claim such a move for an unwritten parameter; now the default keep is
// checked against the body and rejects it.
//~ error[deduce-default:kept-by-default-moved]
fn pass_back<T>(list: List<T>) -> List<T> {
    return list
}

//~ error[deduce-default:kept-by-default-moved]
fn consumes<T>(list: List<T>) -> None {
    consume(list)
}

// [deduce-syntax] A written list promising a parameter back that the body
// moves is an error; so is promising a qualifier the body may remove.
//~ error[deduce-default:promised-back-moved]
fn bad_move<T>(list: List<T>) -> None => list {
    consume(list)
}

//~ error[deduce-gained:qualifier-removed]
fn bad_qual<T>(list: A B List<T>) -> None => list: A B {
    drop_a(list)
}

// [deduce-syntax] Written-list shape errors: unknown parameter, duplicate
// entry, and keeping a qualifier the parameter does not declare.
//~ error[deduce-syntax:unknown-param]
fn unknown_param(x: Int) -> None => y {
}

//~ error[deduce-syntax:duplicate-param]
fn duplicate(x: Int) -> None => x, x {
}

//~ error[deduce-gained:qualifier-not-established]
fn undeclared_qual<T>(list: List<T>) -> None => list: A {
}

// [deduce-default] Every parameter's contract — a written entry or a default
// — is checked against the body. A written `=> list` that the body moves is
// an error, and so is a defaulted `other` the body moves (both must be
// declared `!`).
//~ error[deduce-default:promised-back-moved]
//~ error[deduce-default:kept-by-default-moved]
fn bad<T>(list: List<T>, other: List<T>) -> None => list {
    consume(list)
    consume(other)
}

// A bodiless declaration must mention every parameter except Copy scalars.
effect Sink {
    //~ error[deduce-syntax:bodiless-clause]
    fn eat<T>(list: List<T>, n: Int) -> None
    fn fine<T>(list: List<T>, n: Int) -> None => !list
}

// [deduce-default] A lambda that captures and **mutates** a kept parameter is
// an error — the caller keeps the parameter, so the closure cannot own it;
// the remedy is `copy` or declaring the move (`=> !p`). A **read**-only
// capture keeps the parameter and is clean. (Before 2026-10-11 a mutating
// capture of an unwritten parameter *claimed* it as moved; there is no such
// inference now, so a kept parameter — default or written — behaves the
// same.)
fn runs(f: () -> None) -> None {
    f()
}

fn bump<T>(store: Mut List<T>) [] -> None => store: Mut {}

fn store_size<T>(store: List<T>) [] -> Int => store { return 0 }

fn mutates<T>(store: Mut List<T>) -> None {
    //~ error[fate-lambda:captures-kept]
    let g = () -> { bump(store) }
    runs(g)
}

fn moved<T>(store: Mut List<T>) -> None => !store {
    let g = () -> { bump(store) }
    runs(g)
}

fn reads<T>(store: Mut List<T>) -> None {
    let g = () -> { let _n = store_size(store) }
    runs(g)
}

// [deduce-fixpoint] L3: checking iterates to a fixpoint (capped) — a
// move-mode candidate first discovered in round two (when inferred
// deductions are first enforced) is applied by a third round, so the
// diagnostic lands at the true problem site instead of the raw derived-move
// error.
// [deduce-default] A move through a binding chain is reported as a fate
// conflict when a later read reaches the same root: `ys` (bound from `seed`
// via `xs`) is moved into the consuming `steal`, so `xs`/`seed` cannot be
// read afterwards. The move is declared on `steal` (`=> !list`); `late`
// itself moves `seed`, so it declares `=> !seed`.
fn steal<T>(list: List<T>) -> List<T> => !list {
    return list
}

fn list_size<T>(list: List<T>) [] -> Int => list { return 0 }

fn late<T>(seed: List<T>) -> Int => !seed {
    let xs = seed
    let ys = xs
    let _zs = steal(ys)
    //~ error[fate-move-mode:bound-away]
    return list_size(xs)
}

// `=>[f] !t` scopes a consumption to a fn-typed parameter (its parameter
// named in the type), and `proj(a, b)` joins two sources: either branch is a
// valid source. `f(list)` consumes `list`, so `apply` declares that too.
fn apply<T>(list: List<T>, f: (t: List<T>) -> Int) -> Int =>[f] !t => !list {
    return f(list)
}

fn either<T>(a: List<T>, b: List<T>, flag: Bool) -> proj(a, b) List<T> {
    if flag {
        return a
    }
    return b
}

// [fate-poison] The result of a `proj(a, b)` call shares fate with both:
// moving `a` poisons it.
fn first_of<T>(a: List<T>, b: List<T>) -> proj(a, b) List<T> {
    return a
}

fn linked<T>(a: List<T>, b: List<T>) -> None => !a {
    let v = first_of(a, b)
    consume(a)
    //~ error[fate-poison:shared-root]
    let _n = v
}

// [proj-readonly-return] [proj-anywhere] A derived return checks the
// *returned expression*, not the first argument of a one-argument call:
// dot-notation (`list.pick(index)`) leaves exactly one written argument, and
// the constructor-operand unwrap that a `proj`-arm union return needs must
// not apply to a plain `proj(p) T` return — it used to, so the checker asked
// the *index* where it came from and rejected the call, while accepting a
// one-argument call that borrows nothing. One error, and it is `leak`'s:
// `forward` returns the borrow `pick` handed back, however the call is
// spelled. (The error's span starting at the call is `deduce_tests.rs`'s.)
fn pick<T>(list: List<T>, index: Int) -> proj(list) List<T> {
    return list
}

fn owned<T>(list: List<T>) -> List<T> {
    return copy(list)
}

fn forward<T>(list: List<T>, index: Int) -> proj(list) List<T> {
    return list.pick(index)
}

fn leak<T>(list: List<T>) -> proj(list) List<T> {
    //~ error[proj-readonly-return:not-derived]
    return owned(list)
}

// [deduce-std-explicit] An ordinary (non-std) tree is not held to total
// clauses: an exported project fn with no clause is not refused for it (its
// move is the ordinary default-keep diagnostic). The `std = true` side is
// `deduce_tests.rs`'s.
//~ error[deduce-default:kept-by-default-moved]
export fn keep(s: List<Int>) [] -> List<Int> {
    return s
}
