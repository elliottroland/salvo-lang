// Random values for property tests: a reproducible source of randomness,
// generators of every std type, and the means to build generators of your
// own.
//
// A property test runs its body many times, each time with parameters drawn
// at random, and when one run fails, it looks for the simplest parameters
// that still fail. A **generator**, a [Gen], is how a parameter's type says
// what its values look like. std declares one for each of its own types
// and qualifiers, each named `gen` and told apart by the type it makes, and
// a test file declares the rest:
//
// ```
// struct Point {
//     x: Int,
//     y: Int
// }
//
// // Written by `by random` from the fields' own generators.
// fn gen() -> Gen<Point> by random
//
// // Or by hand, from std's generators and combinators.
// fn small_points() -> Gen<Point> {
//     let coord: Gen<InRange(0, 9) Int> = gen(0, 9)
//     return map(coord, (x) -> Point { x: x, y: 0 })
// }
// ```
//
// **Everything comes from an [Rng].** A generator draws from the `Rng` it is
// handed and from nothing else: it cannot read the clock or a [Random]
// effect, since a generator's fn type declares no effects. So the same
// draws always make the same value, which is what lets a failing run be
// replayed, and shrunk by replaying it over smaller draws.
//
// **Smaller draws make simpler values.** Every draw here maps zero to the
// simplest answer: `0`, `false`, the first arm of a union, an empty list.
// A generator built from these shrinks toward simple values by itself.
// One built from `long` directly should keep to the same rule.
//
// **Where it lives.** This module is visible in every test annex without an
// import, as [std.test] is. When the file also imports `std.random`, write
// `by test.random`, since `random` then names two modules.
// @compilerrules [prop-gen, prop-rng]

// ===== the source of randomness

// The randomness a generator draws from: a seeded generator, or a stream
// being replayed.
//
// Live, it is splitmix64 over its seed, the same numbers on every backend.
// Replaying, it hands back the given draws in order, and zeros once they run
// out. Either way it records every draw, which is what a failing run is
// shrunk from. A property test makes one for each run; a program has no
// reason to.
// @compilerrules [prop-rng]
export struct Rng canbe Mut {
    bits: Long,
    live: Bool,
    stream: List<Long>,
    pos: Int,
    log: Mut List<Long>,
    depth: Int
}

// A live source, seeded with [seed].
// @compilerrules [prop-rng]
export fn live_rng(seed: Long) [] -> Mut Rng {
    return Mut Rng { bits: seed, live: true, stream: List<Long>(), pos: 0, log: Mut List<Long>(), depth: 0 }
}

// A source replaying [stream], then answering zeros.
// @compilerrules [prop-rng]
export fn replay_rng(stream: List<Long>) [] -> Mut Rng => !stream {
    return Mut Rng { bits: 0L, live: false, stream: stream, pos: 0, log: Mut List<Long>(), depth: 0 }
}

// Every draw [rng] has answered so far, in order.
// @compilerrules [prop-rng]
export fn draws_of(rng: Rng) [] -> List<Long> => rng {
    let out: Mut List<Long> = Mut List<Long>()
    for d in rng.log {
        add(out, d)
    }
    return out
}

// The next 64 random bits: every other draw is made of these.
// @compilerrules [prop-rng]
export fn long(rng: Mut Rng) [] -> Long => rng: Mut {
    let d = 0L
    if rng.live {
        // splitmix64.
        rng.bits = rng.bits + -7046029254386353131L
        let z = rng.bits
        z = bit_xor(z, bit_ushr(z, 30)) * -4658895280553007687L
        z = bit_xor(z, bit_ushr(z, 27)) * -7723592293110705685L
        d = bit_xor(z, bit_ushr(z, 31))
    } else {
        d = get(rng.stream, rng.pos) ?: 0L
        rng.pos = rng.pos + 1
    }
    add(rng.log, d)
    return d
}

// A draw in `0` to `n - 1`, for an [n] of at least 1; `0` for a zero draw.
// @compilerrules [prop-rng]
export fn below(rng: Mut Rng, n: Long) [] -> Long => rng: Mut {
    let d = bit_and(long(rng), 9223372036854775807L)
    if n <= 1L {
        return 0L
    }
    return d % n
}

// A draw in `0` to `n - 1`: which of [n] choices to take.
// @compilerrules [prop-rng]
export fn choose(rng: Mut Rng, n: Int) [] -> Int => rng: Mut {
    return to_int(below(rng, to_long(n)))
}

// A draw in [lo] to [hi], both included; [lo] for a zero draw.
// @compilerrules [prop-rng]
export fn int_in(rng: Mut Rng, lo: Int, hi: Int) [] -> Int => rng: Mut {
    if hi <= lo {
        return lo
    }
    return to_int(to_long(lo) + below(rng, to_long(hi) - to_long(lo) + 1L))
}

// A coin: `false` for a zero draw.
// @compilerrules [prop-rng]
export fn bool(rng: Mut Rng) [] -> Bool => rng: Mut {
    return bit_and(long(rng), 1L) == 1L
}

// A double in `[0, 1)`: `0.0` for a zero draw.
// @compilerrules [prop-rng]
export fn double(rng: Mut Rng) [] -> Double => rng: Mut {
    return to_double(bit_ushr(long(rng), 11)) / 9007199254740992.0
}

// Whether a collection being drawn should take one more element: four times
// in five at the top, less often the deeper the value is nested, never past
// 64 elements; `false` for a zero draw, so a list shrinks by getting shorter.
fn more(rng: Mut Rng, count: Int) [] -> Bool => rng: Mut {
    if count >= 64 {
        return false
    }
    let deeper = rng.depth - 1
    return choose(rng, 10) >= 2 + 2 * deeper
}

// ===== generators

// How to make random values of a type: a function from an [Rng] to a
// value, drawing from it alone.
//
// A generator is found by the type it makes: std's are all named `gen`, and
// `let g: Gen<Str> = gen()` is the one for `Str`. Run it with `g.run(rng)`,
// or let [draw] find it from the type a value is wanted at.
//
// ```
// let g: Gen<List<Int>> = gen()
// let rng = live_rng(7L)
// let xs = g.run(rng)
// ```
// @compilerrules [prop-gen]
export struct Gen<T> {
    run: (rng: Mut Rng) -> T
}

// A value of the type wanted where it is called, from that type's
// generator: `let n: Int = draw(rng)`.
//
// @param gen  found at the call site: the generator of the wanted type
// @compilerrules [prop-gen]
export fn draw<T>(rng: Mut Rng, ?gen: () -> Gen<T>) [] -> T => rng: Mut {
    let g = gen()
    return g.run(rng)
}

// A value of [g], the generator of the type wanted: `draw_with<Int>(rng,
// gen())`. What a property test's generated draws call, with the type
// written out so the generator is found by it.
// @compilerrules [prop-gen]
export fn draw_with<T>(rng: Mut Rng, g: Gen<T>) [] -> T => rng: Mut, g {
    return g.run(rng)
}

// The values of [g], each passed through [f].
//
// ```
// let evens: Gen<Int> = map(gen(0, 50), (n) -> n * 2)
// ```
// @compilerrules [prop-gen]
export fn map<T, U>(g: Gen<T>, f: (value: T) -> U) [] -> Gen<U> => !g, !f {
    return Gen<U> { run: (rng: Mut Rng) -> f(g.run(rng)) }
}

// The values of [g] that [f] turns into something, turned: a draw [f]
// answers `None` for is drawn again.
//
// For a value a constructor makes, such as a constructive qualifier: `f`
// is the constructor. A generator [f] refuses 100 times in a row traps,
// since it cannot make values; draw closer to what [f] accepts.
// @compilerrules [prop-gen]
export fn map_not_null<T, U>(g: Gen<T>, f: (value: T) -> U?) [] -> Gen<U> => !g, !f {
    return Gen<U> { run: (rng: Mut Rng) -> {
        let tries = 0
        while tries < 100 {
            let made = f(g.run(rng))
            if made !is None {
                return made
            }
            tries = tries + 1
        }
        return unreachable!("a generator's filter refused 100 draws in a row")
    } }
}

// One of [gs], chosen evenly, then drawn from: the first for a zero draw.
//
// ```
// let low: Gen<InRange(0, 9) Int> = gen(0, 9)
// let high: Gen<InRange(100, 109) Int> = gen(100, 109)
// let g: Gen<Int> = one_of(map(low, (n) -> n + 0), map(high, (n) -> n + 0))
// ```
// @compilerrules [prop-gen]
export fn one_of<T>(...gs: Gen<T>[]) [] -> Gen<T> => !gs {
    let n = size(gs)
    return Gen<T> { run: (rng: Mut Rng) -> gs[choose(rng, n)].run(rng) }
}

// One of [gs], chosen by weight, then drawn from: `weighted((9, common),
// (1, rare))` takes `common` nine times in ten. The first for a zero draw.
// @compilerrules [prop-gen]
export fn weighted<T>(...gs: (Int, Gen<T>)[]) [] -> Gen<T> => !gs {
    let total = 0
    let i = 0
    while i < size(gs) {
        total = total + gs[i].0
        i = i + 1
    }
    return Gen<T> { run: (rng: Mut Rng) -> {
        let k = choose(rng, total)
        let j = 0
        while j < size(gs) {
            if k < gs[j].0 {
                return gs[j].1.run(rng)
            }
            k = k - gs[j].0
            j = j + 1
        }
        return unreachable!("weighted needs a positive weight")
    } }
}

// Lists of [g]'s values: empty for a zero draw, mostly short, at most 64
// elements, shorter the deeper they are nested.
// @compilerrules [prop-gen]
export fn list_of<T>(g: Gen<T>) [] -> Gen<List<T>> => !g {
    return Gen<List<T>> { run: (rng: Mut Rng) -> {
        let out: Mut List<T> = Mut List<T>()
        rng.depth = rng.depth + 1
        while more(rng, size(out)) {
            add(out, g.run(rng))
        }
        rng.depth = rng.depth - 1
        return out
    } }
}

// ===== std's generators

// `Int`s: small ones mostly, `0`, `1`, `-1` and both extremes one draw in
// eight; `0` for a zero draw. The extremes are a draw away from the small
// numbers, so a failure shrinks to the smallest number that fails rather
// than to an extreme.
// @compilerrules [prop-gen]
export fn gen() [] -> Gen<Int> {
    return Gen<Int> { run: draw_int }
}

fn draw_int(rng: Mut Rng) [] -> Int => rng: Mut {
    if choose(rng, 8) == 7 {
        let e = choose(rng, 5)
        if e == 0 {
            return 0
        } elif e == 1 {
            return 1
        } elif e == 2 {
            return -1
        } elif e == 3 {
            return 2147483647
        }
        return -2147483647 - 1
    }
    // Small magnitudes are likelier than large ones: a bit count, itself
    // drawn below a drawn bound.
    let bits = choose(rng, choose(rng, 32) + 1)
    let mag = to_int(bit_and(long(rng), bit_shl(1L, bits) - 1L))
    if bool(rng) {
        return 0 - mag
    }
    return mag
}

// `Long`s: as the `Int`s are, over 64 bits.
// @compilerrules [prop-gen]
export fn gen() [] -> Gen<Long> {
    return Gen<Long> { run: draw_long }
}

fn draw_long(rng: Mut Rng) [] -> Long => rng: Mut {
    if choose(rng, 8) == 7 {
        let e = choose(rng, 5)
        if e == 0 {
            return 0L
        } elif e == 1 {
            return 1L
        } elif e == 2 {
            return -1L
        } elif e == 3 {
            return 9223372036854775807L
        }
        return -9223372036854775807L - 1L
    }
    let bits = choose(rng, choose(rng, 64) + 1)
    let mag = bit_and(long(rng), bit_shl(1L, bits) - 1L)
    if bool(rng) {
        return 0L - mag
    }
    return mag
}

// `Double`s: a whole part as the `Int`s are plus a fraction, and one draw in
// eight `0.0`, `-0.0`, `1.0`, `-1.0`, `NaN`, `Infinity` or `-Infinity`.
// @compilerrules [prop-gen]
export fn gen() [] -> Gen<Double> {
    return Gen<Double> { run: draw_double }
}

fn draw_double(rng: Mut Rng) [] -> Double => rng: Mut {
    if choose(rng, 8) == 7 {
        let e = choose(rng, 7)
        if e == 0 {
            return 0.0
        } elif e == 1 {
            return -0.0
        } elif e == 2 {
            return 1.0
        } elif e == 3 {
            return -1.0
        } elif e == 4 {
            return 0.0 / 0.0
        } elif e == 5 {
            return 1.0 / 0.0
        }
        return -1.0 / 0.0
    }
    let whole = to_double(draw_int(rng))
    return whole + double(rng)
}

// `Bool`s: `false` for a zero draw.
// @compilerrules [prop-gen]
export fn gen() [] -> Gen<Bool> {
    return Gen<Bool> { run: (rng: Mut Rng) -> bool(rng) }
}

// `Byte`s, `0` to `255`.
// @compilerrules [prop-gen]
export fn gen() [] -> Gen<Byte> {
    return Gen<Byte> { run: (rng: Mut Rng) -> to_byte(choose(rng, 256)) }
}

// `Char`s: printable ASCII nine draws in ten (a space for a zero draw), and
// otherwise a tab, a newline or a letter beyond ASCII.
// @compilerrules [prop-gen]
export fn gen() [] -> Gen<Char> {
    return Gen<Char> { run: draw_char }
}

fn draw_char(rng: Mut Rng) [] -> Char => rng: Mut {
    if choose(rng, 10) < 9 {
        let ascii = " !\"#\$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`abcdefghijklmnopqrstuvwxyz{|}~"
        return char_at(ascii, choose(rng, 95)) ?: ' '
    }
    let other = List('\t', '\n', 'é', 'ß', 'Ω', '中')
    return get(other, choose(rng, 6)) ?: ' '
}

// `Str`s: mostly short and printable ASCII, sometimes with a tab, a newline
// or text beyond ASCII (an emoji included); empty for a zero draw.
// @compilerrules [prop-gen]
export fn gen() [] -> Gen<Str> {
    return Gen<Str> { run: (rng: Mut Rng) -> {
        let out: Mut Str = Mut Str("")
        let n = 0
        rng.depth = rng.depth + 1
        while more(rng, n) {
            if choose(rng, 10) < 9 {
                append(out, to_str(draw_char(rng)))
            } else {
                let k = choose(rng, 5)
                let piece = if k == 0 { "\t" } elif k == 1 { "\n" } elif k == 2 { "é" } elif k == 3 { "中" } else { "😀" }
                append(out, piece)
            }
            n = n + 1
        }
        rng.depth = rng.depth - 1
        return out
    } }
}

// `Bytes`: mostly short; empty for a zero draw.
// @compilerrules [prop-gen]
export fn gen() [] -> Gen<Bytes> {
    return Gen<Bytes> { run: (rng: Mut Rng) -> {
        let out: Mut Bytes = mut_bytes()
        let n = 0
        rng.depth = rng.depth + 1
        while more(rng, n) {
            add(out, to_byte(choose(rng, 256)))
            n = n + 1
        }
        rng.depth = rng.depth - 1
        return out
    } }
}

// Lists of the element type's values: empty for a zero draw, mostly short.
//
// @param gen  found at the call site: the element type's generator
// @compilerrules [prop-gen]
export fn gen<T>(?gen: () -> Gen<T>) [] -> Gen<List<T>> {
    return list_of(gen())
}

// Mutable lists of the element type's values, as [gen] for `List<T>` makes
// them: for a property that changes its list.
//
// @param gen  found at the call site: the element type's generator
// @compilerrules [prop-gen]
export fn gen<T>(?gen: () -> Gen<T>) [] -> Gen<Mut List<T>> {
    let g = gen()
    return Gen<Mut List<T>> { run: (rng: Mut Rng) -> {
        let out: Mut List<T> = Mut List<T>()
        rng.depth = rng.depth + 1
        while more(rng, size(out)) {
            add(out, g.run(rng))
        }
        rng.depth = rng.depth - 1
        return out
    } }
}

// Optional values: `None` one draw in four, and for a zero draw.
//
// @param gen  found at the call site: the value type's generator
// @compilerrules [prop-gen]
export fn gen<T>(?gen: () -> Gen<T>) [] -> Gen<T?> {
    let g = gen()
    return Gen<T?> { run: (rng: Mut Rng) -> {
        if choose(rng, 4) == 0 {
            return None
        }
        return g.run(rng)
    } }
}

// Lists with at least one element: one for a zero draw.
//
// @param gen  found at the call site: the element type's generator
// @compilerrules [prop-gen]
export fn gen<T>(?gen: () -> Gen<T>) [] -> Gen<NonEmpty List<T>> {
    let g = gen()
    return Gen<NonEmpty List<T>> { run: (rng: Mut Rng) -> {
        let out: Mut List<T> = Mut List<T>()
        rng.depth = rng.depth + 1
        add(out, g.run(rng))
        while more(rng, size(out)) {
            add(out, g.run(rng))
        }
        rng.depth = rng.depth - 1
        if out is NonEmpty {
            return out
        }
        return unreachable!()
    } }
}

// `Int`s in [lo] to [hi], both included: either end one draw in four, [lo]
// for a zero draw. Called with the constants of the type it makes:
// `let g: Gen<InRange(1, 6) Int> = gen(1, 6)`.
// @compilerrules [prop-gen, qual-const]
export fn gen(lo: Int, hi: Int) [] -> Gen<InRange(lo, hi) Int> {
    return Gen<InRange(lo, hi) Int> { run: (rng: Mut Rng) -> {
        let e = choose(rng, 8)
        let n = if e == 0 { lo } elif e == 1 { hi } else { int_in(rng, lo, hi) }
        if n is InRange(lo, hi) {
            return n
        }
        return unreachable!("an empty range: ${lo} to ${hi}")
    } }
}

// Valid indices of [list]: `0` for a zero draw. Called with the list the
// indices are of: `let g: Gen<Idx(xs) Int> = gen(xs)`.
// @compilerrules [prop-gen, qual-depend]
export fn gen<T>(list: NonEmpty List<T>) [] -> Gen<Idx(list) Int> => list {
    return Gen<Idx(list) Int> { run: (rng: Mut Rng) -> {
        let i = choose(rng, size(list))
        if i is Idx(list) {
            return i
        }
        return unreachable!()
    } }
}

// ===== derived generators

// The generator of a struct, from its fields' own: `fn gen() -> Gen<Point>
// by random`. Each field is drawn by [draw] at its type, in declaration
// order, so every field's type needs a `gen` of its own.
// @compilerrules [prop-derive, fn-by]
export comptime fn gen<T is Struct>() [] -> Gen<T> {
    return Gen<T> { run: (rng: Mut Rng) -> {
        return T {
            [for field in T.fields] {
                [field]: draw@random(rng)
            }
        }
    } }
}

// The generator of a named union, from its arms' own: an arm chosen evenly,
// then drawn by [draw] at its type. `None` for a `None` arm. The first arm
// for a zero draw, and more and more often the deeper the value is nested,
// so a recursive type whose first arm is its base case stays finite.
// @compilerrules [prop-derive, fn-by]
export comptime fn gen<T is Union>() [] -> Gen<T> {
    return Gen<T> { run: (rng: Mut Rng) -> {
        let n = 0
        [for arm in T.arms] {
            n = n + 1
        }
        rng.depth = rng.depth + 1
        let k = if rng.depth >= 8 || choose@random(rng, 8) < rng.depth { 0 } else { choose@random(rng, n) }
        let i = 0
        [for arm in T.arms] {
            if k == i {
                [if arm.type is None] {
                    rng.depth = rng.depth - 1
                    return None
                } else {
                    let v: [arm.type] = draw@random(rng)
                    rng.depth = rng.depth - 1
                    return v
                }
            }
            i = i + 1
        }
        return unreachable!()
    } }
}
