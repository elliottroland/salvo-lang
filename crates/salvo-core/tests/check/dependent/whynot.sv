// [qual-whynot] A predicate qualifier says why a value fails it in a
// `@whynot` tag on `fn qualifies`, whose `[name]`s are `qualifies`'
// parameters — the subject, or a scalar slot. A fn `explain` is refused,
// naming the tag. `Q.qualifies(…)` stays callable directly, with the
// subject then the slot values, and checked.

qualifier Small(hi: Int) of Int {
    // @whynot [n] is not below [hi], [[sic]
    fn qualifies(n: Int, hi: Int) -> Bool {
        return n < hi
    }
}

fn says_why(n: Int) [] -> Str? => n {
    if Small.qualifies(n, 10) {
        return None
    }
    return whynot?(n is Small(10) Int)
}

qualifier Bad of Int {
    // @whynot [m] is wrong
    //~ error[qual-whynot:unknown-slot]
    fn qualifies(n: Int) -> Bool {
        return true
    }
    //~ error[qual-whynot:on-fn]
    fn explain(n: Int) -> Str {
        return "no"
    }
}

qualifier Within(xs: List<Int>) of Int {
    // @whynot [n] is not in [xs]
    //~ error[qual-whynot:slot-type]
    fn qualifies(n: Int, xs: List<Int>) -> Bool {
        return true
    }
}

qualifier Below(hi: Int) of Int {
    fn qualifies(n: Int, hi: Int) -> Bool {
        return n < hi
    }
}

fn calls_qualifies_wrongly(n: Int) [] -> Bool => n {
    //~ error[qual-whynot:call-arg-count]
    //~ error[qual-whynot:call-arg-mismatch]
    return Below.qualifies(n) || Below.qualifies("x", 3)
}
