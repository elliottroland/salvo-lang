// [expr-as-qual] `x as! Q` gives a value a qualifier or traps; `x as? Q`
// answers the value qualified, or `None`. The subject is evaluated once.

//--- as_bang_and_as_question
qualifier NonNegative for Int {
    fn qualifies(int: Int) -> Bool {
        return int >= 0
    }
}

fn double(n: NonNegative Int) -> Int {
    return n + n
}

fn counted(n: Int, hits: Mut List<Int>) -> Int {
    add(hits, n)
    return n
}

fn main() [use] {
    use StdConsole()
    let age = 36 as! NonNegative
    println("${double(age)}")
    let hits: Mut List<Int> = Mut List<Int>()
    let kept = counted(5, hits) as? NonNegative
    let dropped = counted(-5, hits) as? NonNegative
    println(when kept {
        is None { "none" }
        is NonNegative Int { "kept ${kept}" }
    })
    println(when dropped {
        is None { "dropped" }
        is NonNegative Int { "kept" }
    })
    println("evaluated ${size(hits)} times")
}
//= stdout
//= 72
//= kept 5
//= dropped
//= evaluated 2 times

//--- as_bang_traps_on_a_failing_value
qualifier NonNegative for Int {
    fn qualifies(int: Int) -> Bool {
        return int >= 0
    }
}

fn main() [use] {
    use StdConsole()
    let bad = -1 as! NonNegative
    println("unreachable ${bad}")
}
//= trap -1 is not NonNegative at main:9:15
