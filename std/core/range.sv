// Represents a range from [start] (inclusive) to [end] (exclusive), which increments by [step].
// The range can go in either direction.
//
// [mod-export] The struct and its `next` are exported, not only the `range`
// functions: driving a pass needs the iteration declaration *in scope*
// [iter-resolve], so a private `Range` made the exported constructors unusable
// anywhere but here (user decision 2026-09-23). The price is paid by every
// program: reachability follows names, and every program that iterates uses the
// name `next`, so `core.range` is emitted whether or not a program ranges over
// anything [mod-used-only] — ROADMAP.md carries that as the thing to fix.
export struct Range {
    start: Int
    end: Int
    step: Int
}

export iter fn next(range: Range) -> Emitted Int | Finished {
    state {
        i: Int = range.start
    }
    let next = i.copy()
    // [end] is **exclusive** in both directions, and a zero step is the empty
    // range. (Until 2026-09-23 the ascending guard read `i > range.end`, so
    // `range(3)` yielded 0..3, and the descending one stepped with `i -=
    // range.step` — which *adds* when the step is negative, so a descending
    // range ran away upward. Neither could be caught before: `Range` was
    // private, so nothing outside this file could iterate one.)
    return when {
        range.step == 0 {
            finished()
        }
        range.step > 0 && i >= range.end {
            finished()
        }
        range.step < 0 && i <= range.end {
            finished()
        }
        else {
            i += range.step
            emitted(next)
        }
    }
}

// Returns a range which starts at [start] (inclusive), ends before [end] (exclusive), and steps
// each iteration by [step].
export fn range(start: Int, end: Int, step: Int) -> Range {
    return Range { start: start, end: end, step: step }
}

// Returns a range which starts at [start] (inclusive), ends before [end] (exclusive), and steps
// each iteration by 1 (if start < end) or -1 (if end < start). In the case where [start] == [end],
// the step is set to 0 and the iteration is empty.
export fn range(start: Int, end: Int) -> Range {
    let step = when {
        start < end { 1 }
        start > end { -1 }
        else { 0 }
    }
    return range(start, end, step)
}

// Returns a range which starts a 0 (inclusive), ends before [end] (exclusive), and steps each
// iteration by 1 (if end > 0) or -1 (if end < start). In the case where [end] == 0, the step
// is set to 0 and the iteration is empty.
export fn range(end: Int) -> Range {
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
