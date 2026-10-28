// D5: qualifier subjects [qual-subject]
//
// Compile-and-run data ([test-run-data]), moved from the codegen tests.

//--- provenance_survives_mutation_where_state_does_not

qualifier Checked of List<Int>
provenance qualifier Trusted of List<Int>

fn check(l: Mut List<Int>) -> +Checked Mut List<Int>
=> !l {
    return l
}

fn trust(l: Mut List<Int>) -> +Trusted Mut List<Int>
=> !l {
    return l
}

fn describe(l: List<Int>) -> Str {
    return "plain ${l.size()}"
}

fn describe(l: Checked List<Int>) -> Str {
    return "checked ${l.size()}"
}

fn describe(l: Trusted List<Int>) -> Str {
    return "trusted ${l.size()}"
}

fn main() [use] -> None {
    use StdConsole()
    let t = trust(Mut List(1, 2))
    t.add(3)
    println(describe(t))
    let c = check(Mut List(1, 2))
    c.add(3)
    println(describe(c))
    let c2 = check(Mut List(4, 5))
    println(describe(c2))
}
//= stdout
//= trusted 3
//= plain 3
//= checked 2
