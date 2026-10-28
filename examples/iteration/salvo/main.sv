// Iteration, in every form Salvo has.
//
// There is no iterator type and no iteration protocol built into the
// compiler. An **iterator struct** is an ordinary struct that declares
// `: Next<self, T>` and has a `next`; `for` is sugar for calling that `next`
// until it answers `Finished`. Everything below is that one rule, seen from
// seven angles.

// ===== 1. a container, driven natively =====
//
// `for` straight over a `List`, an array or a `Str` is the shape the backends
// keep a native loop for: no iterator is allocated and the container is not
// consumed, so `xs` is still readable after the loop.
fn describe_container(xs: List<Int>) [Console] -> None => xs {
    let sum = 0
    for n in xs {
        sum = sum + n
    }
    println("1. list of ${size(xs)} sums to ${sum}")

    let letters = Mut Str()
    for c in "salvo" {
        append(letters, "${c}.")
    }
    println("1. string: ${letters}")

    let arr = array_of(10, 20, 30)
    let from_array = 0
    for n in arr {
        from_array = from_array + n
    }
    println("1. array sums to ${from_array}")
}

// ===== 2. an iterator struct of your own, written out =====
//
// The manual form: the position is a field, `next` is an ordinary function,
// and the struct says what it yields. This is what reads two sources at once
// or wants explicit control of its state — and it is also what `iter(xs)`
// hands back for a `List`. Write it out like this when the iterator needs a
// *name*: to store it in a field, to hand it to a function, to zip two of
// them.
struct Countdown : Next<self, Int> canbe Mut {
    // The next value to emit; the iterator is finished when it reaches zero.
    at: Int
}

fn countdown(from: Int) -> Mut Countdown {
    return Mut Countdown { at: from }
}

// Advancing is a mutation of the position, which is why the parameter is
// `Mut` and is handed back with `=> p: Mut`.
fn next(p: Mut Countdown) -> Emitted Int | Finished => p: Mut {
    if p.at <= 0 {
        return finished()
    }
    let now = copy(p.at)
    p.at = p.at - 1
    return emitted(now)
}

// A second step over the same struct, under another name. Only `next` is the
// canonical one `for p in …` drives; this one is driven by naming the call:
// `for n in skip(p)`.
fn skip(p: Mut Countdown) -> Emitted Int | Finished => p: Mut {
    if p.at <= 1 {
        return finished()
    }
    let now = copy(p.at)
    p.at = p.at - 2
    return emitted(now)
}

// An iterator is a value, so it can be driven in stages: this loop leaves the
// iterator where the `break` found it, and the caller carries on from there.
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

// ===== 2b. the same thing, with the struct generated: `iter fn` =====
//
// An `iter fn` is the same hand-written `next` with the boilerplate removed:
// the declaration is the *minter*, under any name and with any parameters, the
// `state Halving { … }` block names the iterator struct and declares its own
// fields, and the body is its `next`. The compiler writes the struct, which
// holds the parameters beside the state.
//
// Nothing suspends, so there is no state machine: the body *is* the `next`.
iter fn halving(start: Int) -> Emitted Int | Finished {
    state Halving {
        // Evaluated once, when the iterator is minted, and readable and
        // writable for the rest of its life. The parameters are read-only.
        at: Int = start
    }
    if at <= 0 {
        return finished()
    }
    let now = copy(at)
    at = at / 2
    return emitted(now)
}

// The struct is an ordinary type, so a fn handing one out names it.
fn halving_from_ten() -> Mut Halving {
    return halving(10)
}

// ===== 2c. a source: `Iter` =====
//
// A type becomes a *source* — something `for` mints an iterator from — by
// declaring `: Iter<self, It, T>`, naming its iterator struct, and an `iter`
// over it. The `iter fn` named `iter` is the canonical minter the obligation
// asks for, and its `state` names the struct.
struct Bag : Iter<self, BagIter, Int> {
    items: List<Int>
}

iter fn iter(bag: Bag) -> Emitted Int | Finished {
    state BagIter {
        at: Int = 0
    }
    let e = get(bag.items, at)
    if e is None {
        return finished()
    }
    at = at + 1
    return emitted(copy(e))
}

// ===== 3. an effectful iterator, and an unbounded one =====
//
// A `next` is an ordinary function, so effects are ordinary effects: declare
// them and the `for` that drives supplies the handler, once per turn.
iter fn fibs(count: Int) [Console] -> Emitted Int | Finished {
    state Fibs {
        a: Int = 0,
        b: Int = 1,
        made: Int = 0
    }
    if made >= count {
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

// Nothing runs until a consumer pulls, so an unbounded iterator is an ordinary
// thing to write: the `break` below is what ends it.
iter fn naturals(from: Int) -> Emitted Int | Finished {
    state Naturals {
        at: Int = from
    }
    let now = copy(at)
    at = at + 1
    return emitted(now)
}

// ===== 4. combinators of your own =====
//
// `?Next<It, T>` is a spread of implicit parameters, not a bound and not a
// trait: it asks for the `next` that fits the subject, and the call site fills
// it. That spread is also the declaration `for` reads, so a generic iterator
// is driven exactly like a named one.
fn sum_of<It>(it: Mut It, ?Next<It, Int>) -> Int => it: Mut {
    let total = 0
    for n in it {
        total = total + n
    }
    return total
}

// `?Iter<C, It, T>` is the same over a *source*: it brings in the `iter` that
// mints an `It` and the `next` that drives it, so a combinator can take the
// container itself — a `Bag`, a `List`, anything with an `iter`.
fn total<C, It>(c: C, ?Iter<C, It, Int>) -> Int => c {
    let total = 0
    for n in c {
        total = total + n
    }
    return total
}

// Any iterator struct emitting `Int`: a generic with its `?Next` spread, so a
// hand-written struct and an `iter fn`'s both fit.
fn first<It>(it: Mut It, ?Next<It, Int>) -> Int => it: Mut {
    for n in it {
        return n
    }
    return -1
}

fn main() [use] {
    use StdConsole()

    let xs = List(1, 2, 3, 4)
    describe_container(xs)

    // 2. a hand-written iterator, driven in two stages, then through a step
    // under another name: `for n in skip(p)` re-invokes the call each turn.
    let p = countdown(5)
    take(p, 2)
    println("2. rest sums to ${sum_of(p)}")
    let q = countdown(6)
    for n in skip(q) {
        println("2. skip ${n}")
    }

    // 2b. the generated iterator: one declaration, and `halving(20)` mints one
    // — driven straight away, or held and driven by anything that takes one.
    for n in halving(20) {
        println("2b. halving ${n}")
    }
    let hp = halving(20)
    println("2b. summed from a held iterator: ${sum_of(hp)}")
    println("2b. first from a fn returning one: ${first(halving_from_ten())}")

    // 2c. a source: `for` over it mints with its `iter`, and so does a
    // combinator over any source.
    let bag = Bag { items: List(7, 8) }
    for n in bag {
        println("2c. bag ${n}")
    }
    println("2c. total of a bag ${total(bag)}, of a list ${total(xs)}")

    // 3. an effectful iterator: `for` supplies the handler per turn. It cannot
    // fill a *pure* `?Next` position, so a combinator over it would have to
    // declare `[Console]` too — the ordinary effect rule.
    for n in fibs(6) {
        println("3. fib ${n}")
    }

    // An unbounded one, stopped by the consumer.
    for n in naturals(10) {
        if n > 12 {
            break
        }
        println("3. natural ${n}")
    }

    // ===== 5. the sequence functions =====
    //
    // `map`/`filter`/`reduce` are eager and take an *iterator*: a container is
    // iterated by writing its `iter`. A `List` has a fast path under the same
    // name, which is why `map(xs, f)` still reads well.
    let doubled = map(xs, n -> n * 2)
    let odd = filter(xs, n -> n % 2 == 1)
    let total = reduce(xs, 0, (acc, n) -> acc + n)
    println("5. list: ${size(doubled)} doubled, ${size(odd)} odd, total ${total}")

    let words = List("ann", "bo", "carol")
    let lengths = map(iter(words), w -> size(w))
    println("5. lengths: ${reduce(iter(lengths), 0, (acc, n) -> acc + n)}")

    // `filter` returns a *view* of the elements it keeps, so what it walks
    // has to outlive the result: a literal would die at the end of the line.
    let word = "iteration"
    let vowels = filter(iter(word), c -> c == 'i' || c == 'o')
    println("5. vowels: ${size(vowels)}")

    // An `iter fn`'s iterator composes with them exactly like a container's.
    println("5. halving total ${reduce(halving(20), 0, (acc, n) -> acc + n)}")

    // ===== 6. mapping into a collection you provide =====
    //
    // `map_to` maps into a collection you provide and hands it back, reached
    // through an `add` the call site resolves — so the destination need not be
    // a `List`.
    let collected = map_to(Mut List<Int>(), countdown(3), (n: Int) -> n * 10)
    println("6. collected ${size(collected)}")

    // ===== 7. ranges =====
    //
    // std's `range` is an `iter fn` with three parameters and two delegating
    // overloads returning its `Range`.
    let evens = Mut Str()
    for i in range(0, 10, 2) {
        append(evens, "${i} ")
    }
    println("7. evens ${evens}")
}
