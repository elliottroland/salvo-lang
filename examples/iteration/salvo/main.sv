// Iteration, in every form Salvo has.
//
// There is no iterator type and no iteration protocol built into the
// compiler. A **pass** is an ordinary struct that declares `: Yield<self, T>`
// and has a `next`; `for` is sugar for calling that `next` until it answers
// `Finished`. Everything below is that one rule, seen from six angles.

// ===== 1. a container, driven natively =====
//
// `for` straight over a `List`, an array or a `Str` is the shape the backends
// keep a native loop for: no pass is allocated and the container is not
// consumed, so `xs` is still readable after the loop.
fn describe_container(xs: List<Int>) [Console] -> None => xs {
    let sum = 0
    for n in xs {
        sum = sum + n
    }
    println("1. list of ${size(xs)} sums to ${sum}")

    let letters = mutable_str()
    for c in "salvo" {
        append(letters, "${c}.")
    }
    println("1. string: ${letters}")

    let arr = [10, 20, 30]
    let from_array = 0
    for n in arr {
        from_array = from_array + n
    }
    println("1. array sums to ${from_array}")
}

// ===== 2. a pass of your own, written out =====
//
// The manual form: the position is a field, `next` is an ordinary function,
// and the struct says what it yields. This is what `yield` cannot express —
// two sources at once, explicit control of the state — and it is also what
// `iter(xs)` hands back for a `List`. Write it out like this when the pass
// needs a *name*: to store it in a field, to hand it to a function, to zip
// two of them.
struct Countdown : Yield<self, Int> canbe Mut {
    // The next value to emit; the pass is finished when it reaches zero.
    at: Int
}

fn countdown(from: Int) -> Mut Countdown {
    return Mut Countdown { at: from }
}

// Advancing is a mutation of the position, which is why the parameter is
// `Mut` and is handed back with `[p: Mut]`.
fn next(p: Mut Countdown) -> Emitted Int | Finished => p: Mut {
    if p.at <= 0 {
        return finished()
    }
    let now = copy(p.at)
    p.at = p.at - 1
    return emitted(now)
}

// A pass is a value, so it can be driven in stages: this loop leaves the pass
// where the `break` found it, and the caller carries on from there.
fn take(p: Mut Countdown, count: Int) [Console] -> None => p: Mut {
    let seen = 0
    for n in p {
        println("2. got ${n}")
        seen = seen + 1
        if seen == count {
            break
        }
    }
}

// ===== 2b. the same pass, with the struct generated: `iter fn` =====
//
// Most passes need no name — they are only ever driven by a `for`. An `iter fn`
// is the same hand-written `next` with the boilerplate removed: the subject
// stays ordinary data, the `state { … }` block declares the pass's own fields,
// and the compiler writes the struct and the `iter` that mints one.
//
// Nothing suspends, so there is no state machine: the body *is* the `next`.
struct Halving {
    // The number to start from; halved on every turn.
    start: Int
}

iter fn next(h: Halving) -> Emitted Int | Finished {
    state {
        // Evaluated once, when the pass is minted, and readable and writable
        // for the rest of its life. The subject's own fields are read-only.
        at: Int = h.start
    }
    if at <= 0 {
        return finished()
    }
    let now = copy(at)
    at = at / 2
    return emitted(now)
}

// ===== 3. an effectful pass, and an unbounded one =====
//
// A `next` is an ordinary function, so effects are ordinary effects: declare
// them and the `for` that drives supplies the handler, once per turn.
struct Fibs {
    // How many numbers to produce before the sequence ends.
    count: Int
}

fn fibs(count: Int) -> Fibs {
    return Fibs { count: count }
}

iter fn next(f: Fibs) [Console] -> Emitted Int | Finished {
    state {
        a: Int = 0,
        b: Int = 1,
        made: Int = 0
    }
    if made >= f.count {
        println("3. finished")
        return finished()
    }
    let now = copy(a)
    let sum = a + b
    a = copy(b)
    b = copy(sum)
    made = made + 1
    return emitted(now)
}

// Nothing runs until a consumer pulls, so an unbounded pass is an ordinary
// thing to write: the `break` below is what ends it.
struct Naturals {
    // The first number to emit.
    from: Int
}

fn naturals(from: Int) -> Naturals {
    return Naturals { from: from }
}

iter fn next(n: Naturals) -> Emitted Int | Finished {
    state {
        at: Int = n.from
    }
    let now = copy(at)
    at = at + 1
    return emitted(now)
}

// ===== 4. a combinator of your own =====
//
// `?Yield<It, T>` is a spread of implicit parameters, not a bound and not a
// trait: it asks for the `next` that fits the subject, and the call site fills
// it. That spread is also the declaration `for` reads, so a generic pass is
// driven exactly like a named one.
fn sum_of<It>(it: Mut It, ?Yield<It, Int>) -> Int => it: Mut {
    let total = 0
    for n in it {
        total = total + n
    }
    return total
}

// Anything that yields an `Int` fits, whether it is a container's pass, a
// hand-written one or a generator's origin.
fn main() [use] {
    use StdOutConsole()

    let xs = list(1, 2, 3, 4)
    describe_container(xs)

    // 2. a hand-written pass, driven in two stages.
    let p = countdown(5)
    take(p, 2)
    println("2. rest sums to ${sum_of(p)}")

    // 2b. the generated pass: one declaration, and the subject stays ordinary
    // data — so it replays, and `iter` hands back a pass you can hold.
    let h = Halving { start: 20 }
    for n in h {
        println("2b. halving ${n}")
    }
    // `iter(h)` mints a pass and hands it over, so it can be held in a local
    // and driven by anything that takes a pass.
    let hp = iter(h)
    println("2b. summed from a held pass: ${sum_of(hp)}")

    // 3. a generator, and the same origin driven twice — each `for` builds a
    // fresh machine, so it starts over.
    for n in fibs(6) {
        println("3. fib ${n}")
    }
    // An effectful pass is driven by `for`, which supplies the handler per
    // turn. It cannot fill a *pure* `?Yield` position, so a combinator over it
    // would have to declare `[Console]` too — the ordinary effect rule.

    // An unbounded one, stopped by the consumer.
    for n in naturals(10) {
        if n > 12 {
            break
        }
        println("3. natural ${n}")
    }

    // ===== 5. the sequence functions =====
    //
    // `map`/`filter`/`reduce` are eager and take a *pass*: a container is
    // iterated by writing its `iter`. A `List` has a fast path under the same
    // name, which is why `map(xs, f)` still reads well.
    let doubled = map(xs, n -> n * 2)
    let odd = filter(xs, n -> n % 2 == 1)
    let total = reduce(xs, 0, (acc, n) -> acc + n)
    println("5. list: ${size(doubled)} doubled, ${size(odd)} odd, total ${total}")

    let words = list("ann", "bo", "carol")
    let lengths = map(iter(words), w -> size(w))
    println("5. lengths: ${reduce(iter(lengths), 0, (acc, n) -> acc + n)}")

    // `filter` returns a *view* of the elements it keeps, so what it walks
    // has to outlive the result: a literal would die at the end of the line.
    let word = "iteration"
    let vowels = filter(iter(word), c -> c == 'i' || c == 'o')
    println("5. vowels: ${size(vowels)}")

    // A generator is a pass like any other, so it composes with them too.
    // A pass of your own composes with them exactly like a container's.
    println("5. halving total ${reduce(iter(Halving { start: 20 }), 0, (acc, n) -> acc + n)}")

    // ===== 6. mapping into a collection you provide =====
    //
    // `map_to` maps into a collection you provide and hands it back, reached
    // through an `add` the call site resolves — so the destination need not be
    // a `List`.
    let collected = map_to(mutable_list<Int>(), countdown(3), (n: Int) -> n * 10)
    println("6. collected ${size(collected)}")
}
