// A range of `Int`s from [start] (inclusive) to [end] (exclusive), stepping
// by [step], in either direction. Written as an `iter fn`: the
// declaration is the minter and its body is the step, so `range(0, 10)` mints
// a `Range` (the struct its `state` block names) and `for i in range(0, 10)`
// drives it. The struct holds the three parameters and the position.
//
// [end] is **exclusive** in both directions, and a zero step is the empty
// range.
// @compilerrules [iter-fn]
export iter fn range(start: Int, end: Int, step: Int) -> Emitted Int | Finished {
    state Range {
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
export fn range(start: Int, end: Int) -> Mut Range {
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
export fn range(end: Int) -> Mut Range {
    return range(0, end)
}


// The claim that an `Int` lies in a **constant range**, both
// ends inclusive: `InRange(0, 65535) Int` is a port, `InRange(0, 100) Int`
// a percentage — one declaration, a distinct type per pair of bounds
// (constants agree exactly or not at all). The third slot kind, after fn
// identities and places: compile-time known, so
// nothing tracks it and nothing can invalidate it.
// @compilerrules [qual-const, cmp-carry, qual-depend]
export qualifier InRange(lo: Int, hi: Int) of Int {
    // Why a value failed: "70000 is not in 1..65535".
    //
    // @whynot [n] is not in [lo]..[hi]
    // @compilerrules [qual-whynot]
    fn qualifies(n: Int, lo: Int, hi: Int) -> Bool {
        return n >= lo && n <= hi
    }
}
