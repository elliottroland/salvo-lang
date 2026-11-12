// Ranges of integers to loop over, and the claim that an integer lies within
// constant bounds.
//
// [range] answers an iterator over evenly spaced `Int`s. The end is
// always **exclusive**, in either direction, and the step is taken from the
// bounds unless it is given:
//
// ```
// for i in range(3) {
//     println("${i}")                                  // 0, 1, 2
// }
// for i in range(5, 0) {
//     println("${i}")                                  // 5, 4, 3, 2, 1
// }
// for i in range(0, 10, 4) {
//     println("${i}")                                  // 0, 4, 8
// }
// ```
//
// A range computes its values as it is driven and stores none of them, so a
// long range costs no memory. To loop over the positions of a list, prefer
// `indices(xs)` ([std.core.index]): its values carry the proof that they
// are in range, so reading the list needs no check.
//
// **Constant bounds as a type.** [InRange] is a claim about an `Int`: that it
// lies between two constants, both inclusive. `InRange(1, 65535) Int` is a
// port number and `InRange(0, 100) Int` a percentage. A function that asks
// for one makes its caller test the value first, with `is`, once:
//
// ```
// fn listen(port: InRange(1, 65535) Int) [Console] -> None => port {
//     println("listening on ${port}")
// }
//
// fn start(n: Int) [Console] -> None => n {
//     if n is InRange(1, 65535) {
//         listen(n)
//     } else {
//         println("not a port: ${n}")
//     }
// }
// ```

// The iterator a [range] call answers: the bound, the step, and the value
// the next turn emits. It is an ordinary struct with a `next`, like every
// iterator, so `for` drives it and the functions of [std.core.seq] accept
// it.
// @compilerrules [iter-protocol]
export struct RangeIter : Next<self, Int> canbe Mut {
    // Where the range stops, exclusive.
    end: Int
    // What each turn adds; zero is the empty range.
    step: Int
    // The value the next turn emits.
    i: Int
}

// The `Int`s from [start] (inclusive) towards [end] (exclusive), [step]
// apart.
//
// A positive step counts up and stops before reaching [end]; a negative one
// counts down and stops before reaching it from above. A step of zero, or a
// step pointing away from [end], is the empty range.
//
// ```
// for i in range(10, 0, -3) {
//     println("${i}")                                  // 10, 7, 4, 1
// }
// let none = count(range(0, 10, -1))                   // 0: the step points away
// ```
export fn range(start: Int, end: Int, step: Int) [] -> Mut RangeIter {
    return Mut RangeIter { end: end, step: step, i: start }
}

// Advances a range: its current value, or the end once it has reached or
// passed [RangeIter]'s `end`.
export fn next(p: Mut RangeIter) [] -> Emitted Int | Finished => p: Mut {
    let value = p.i.copy()
    if p.step == 0 || (p.step > 0 && p.i >= p.end) || (p.step < 0 && p.i <= p.end) {
        return finished()
    }
    p.i = p.i + p.step
    return emitted(value)
}

// The `Int`s from [start] (inclusive) to [end] (exclusive), one apart:
// counting up when `start < end`, down when `start > end`, and empty when
// they are equal.
//
// ```
// for i in range(4, 1) {
//     println("${i}")                                  // 4, 3, 2
// }
// ```
export fn range(start: Int, end: Int) -> Mut RangeIter {
    let step = when {
        start < end {
            1
        }
        start > end {
            -1
        }
        else {
            0
        }
    }
    return range(start, end, step)
}

// The `Int`s from `0` (inclusive) to [end] (exclusive), one apart: counting
// up for a positive [end], down for a negative one, and empty for `0`.
//
// ```
// for i in range(-3) {
//     println("${i}")                                  // 0, -1, -2
// }
// ```
export fn range(end: Int) -> Mut RangeIter {
    return range(0, end)
}

// The claim that an `Int` lies between two constants, **both inclusive**:
// `InRange(0, 65535) Int` is a port, `InRange(0, 100) Int` a percentage.
//
// The bounds are part of the type, and two claims are the same only when
// their bounds are equal: an `InRange(10, 20) Int` does not fill an
// `InRange(0, 100) Int` parameter, even though every such value is in both,
// until it is tested again. The bounds must be integer literals (not yet
// negative ones). Because they are known at compile time and an `Int` cannot
// be changed in place, nothing can make the claim stale once a value has it.
//
// A value gets the claim from a test, `n is InRange(1, 65535)`, or from
// `assert!(n is InRange(1, 65535))`, which stops the program with
// "70000 is not in 1..65535" when it fails. Even a literal is tested:
// `listen(8080)` does not prove itself. `whynot?(n is InRange(1, 65535))`
// answers the failure message, or `None` when the value is in range. A
// field or parameter default of such a type is tested each time it is used.
//
// ```
// let n = 8080
// if n is InRange(1, 65535) {
//     println("${n} is a port")
// }
// println(whynot?(70000 is InRange(1, 65535)) ?: "in range")   // 70000 is not in 1..65535
// ```
// @compilerrules [qual-const, cmp-carry, qual-depend]
export qualifier InRange(lo: Int, hi: Int) for Int {
    // Whether [n] lies in `lo..hi`, both ends included.
    //
    // @whynot [n] is not in [lo]..[hi]
    // @compilerrules [qual-whynot]
    fn qualifies(n: Int, lo: Int, hi: Int) -> Bool {
        return n >= lo && n <= hi
    }
}
