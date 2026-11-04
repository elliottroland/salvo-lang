// The iterator a [range] call mints: the bound, the step and the value the
// next turn emits. A struct with a `next`, like every iterator.
// @compilerrules [iter-protocol]
export struct RangeIter : Next<self, Int> canbe Mut {
    // Where the range stops, exclusive.
    end: Int,
    // What each turn adds; zero is the empty range.
    step: Int,
    // The value the next turn emits.
    i: Int
}

// A range of `Int`s from [start] (inclusive) to [end] (exclusive), stepping
// by [step], in either direction: `range(0, 10, 2)` mints a [RangeIter] and
// `for i in range(0, 10, 2)` drives it.
//
// [end] is **exclusive** in both directions, and a zero step is the empty
// range.
export fn range(start: Int, end: Int, step: Int) [] -> Mut RangeIter {
    return Mut RangeIter { end: end, step: step, i: start }
}

// Advances a range: its current value, or the end once it passes [end].
export fn next(p: Mut RangeIter) [] -> Emitted Int | Finished => p: Mut {
    let value = p.i.copy()
    if p.step == 0 || (p.step > 0 && p.i >= p.end) || (p.step < 0 && p.i <= p.end) {
        return finished()
    }
    p.i = p.i + p.step
    return emitted(value)
}

// Returns a range which starts at [start] (inclusive), ends before [end]
// (exclusive), and steps each iteration by 1 (if start < end) or -1 (if end <
// start). In the case where [start] == [end], the step is set to 0 and the
// iteration is empty.
export fn range(start: Int, end: Int) -> Mut RangeIter {
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
export fn range(end: Int) -> Mut RangeIter {
    return range(0, end)
}


// The claim that an `Int` lies in a **constant range**, both
// ends inclusive: `InRange(0, 65535) Int` is a port, `InRange(0, 100) Int`
// a percentage — one declaration, a distinct type per pair of bounds
// (constants agree exactly or not at all). The third slot kind, after fn
// identities and places: compile-time known, so
// nothing tracks it and nothing can invalidate it.
// @compilerrules [qual-const, cmp-carry, qual-depend]
export qualifier InRange(lo: Int, hi: Int) for Int {
    // Why a value failed: "70000 is not in 1..65535".
    //
    // @whynot [n] is not in [lo]..[hi]
    // @compilerrules [qual-whynot]
    fn qualifies(n: Int, lo: Int, hi: Int) -> Bool {
        return n >= lo && n <= hi
    }
}
