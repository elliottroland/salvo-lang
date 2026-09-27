// A range of `Int`s from [start] (inclusive) to [end] (exclusive), stepping
// by [step], in either direction. Written as an `iter fn` [iter-fn]: the
// declaration is the minter and its body is the step, so `range(0, 10)` mints
// an iterator (`iter Int` [iter-type]) and `for i in range(0, 10)` drives it.
// There is no `Range` struct — the three parameters *are* the state, and the
// iterator struct the compiler writes is unnameable, as every `iter fn`'s is.
//
// [end] is **exclusive** in both directions, and a zero step is the empty
// range.
export iter fn range(start: Int, end: Int, step: Int) -> Emitted Int | Finished {
    state {
        i: Int = start
    }
    let next = i.copy()
    return when {
        step == 0 {
            finished()
        }
        step > 0 && i >= end {
            finished()
        }
        step < 0 && i <= end {
            finished()
        }
        else {
            i += step
            emitted(next)
        }
    }
}

// Returns a range which starts at [start] (inclusive), ends before [end]
// (exclusive), and steps each iteration by 1 (if start < end) or -1 (if end <
// start). In the case where [start] == [end], the step is set to 0 and the
// iteration is empty.
//
// [iter-type] `-> iter Int` on a fn with a body is a **pattern**: the fn
// returns whichever concrete iterator struct its body mints — here the one
// `range(start, end, step)` does — and callers see that type.
export fn range(start: Int, end: Int) -> iter Int {
    let step = when {
        start < end { 1 }
        start > end { -1 }
        else { 0 }
    }
    return range(start, end, step)
}

// Returns a range which starts a 0 (inclusive), ends before [end] (exclusive),
// and steps each iteration by 1 (if end > 0) or -1 (if end < 0). In the case
// where [end] == 0, the step is set to 0 and the iteration is empty.
export fn range(end: Int) -> iter Int {
    return range(0, end)
}


// [qual-const] The claim that an `Int` lies in a **constant range**, both
// ends inclusive: `InRange(0, 65535) Int` is a port, `InRange(0, 100) Int`
// a percentage — one declaration, a distinct type per pair of bounds
// (constants agree exactly or not at all). The third slot kind, after fn
// identities [cmp-carry] and places [qual-depend]: compile-time known, so
// nothing tracks it and nothing can invalidate it.
export qualifier InRange(lo: Int, hi: Int) of Int {
    fn qualifies(n: Int, lo: Int, hi: Int) -> Bool {
        return n >= lo && n <= hi
    }
}
