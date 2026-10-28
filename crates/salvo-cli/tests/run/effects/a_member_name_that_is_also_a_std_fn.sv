// [effect-available] a member name that is also a std fn
//
// Compile-and-run data ([test-run-data]), moved from the codegen tests.

//--- a_member_named_like_a_std_fn

effect Tally {
    fn add(n: Int) -> None => !n
    fn read() -> Int
}

handler Summing() of Tally {
    state {
        sum: Int = 0
    }

    fn add(n: Int) -> None {
        sum = sum + n
    }

    fn read() -> Int {
        return sum
    }
}

effect Shout {
    fn to_upper(s: Str) -> Str => !s
}

handler Excited() of Shout {
    fn to_upper(s: Str) -> Str {
        return "${s}!"
    }
}

fn double(x: Int) [] -> Int {
    return x * 2
}

fn main() [use] {
    use StdConsole()
    use Summing()
    use Excited()
    // The member: an instance is in scope, so the bare name is the member.
    add(4)
    add(5)
    println("tally ${read()}")
    // std's own `add`, reached by hand — and std's internals, which call
    // `add(out, x)` inside `map`, reached without the program saying anything.
    let xs: Mut List<Int> = Mut List()
    add@core.list(xs, 7)
    let mapped = map(iter(List(1, 2, 3)), double)
    println("list ${size(xs)} mapped ${size(mapped)}")
    // Both spellings of a name owned by an effect *and* by std.
    let mine = to_upper@Shout("hi")
    let theirs = to_upper@core.string("hi")
    println("member ${mine} std ${theirs}")
}
//= stdout
//= tally 9
//= list 1 mapped 3
//= member hi! std HI

//--- a_member_named_like_a_std_fn_2

effect Tally {
    fn add(n: Int) -> None => !n
    fn read() -> Int
}

handler Summing() of Tally {
    state {
        sum: Int = 0
    }

    fn add(n: Int) -> None {
        sum = sum + n
    }

    fn read() -> Int {
        return sum
    }
}

effect Shout {
    fn to_upper(s: Str) -> Str => !s
}

handler Excited() of Shout {
    fn to_upper(s: Str) -> Str {
        return "${s}!"
    }
}

fn double(x: Int) [] -> Int {
    return x * 2
}

fn main() [use] {
    use StdConsole()
    use Summing()
    use Excited()
    add(4)
    add(5)
    println("tally ${read()}")
    let xs: Mut List<Int> = Mut List()
    add@core.list(xs, 7)
    let mapped = map(iter(List(1, 2, 3)), double)
    println("list ${size(xs)} mapped ${size(mapped)}")
    let mine = to_upper@Shout("hi")
    let theirs = to_upper@core.string("hi")
    println("member ${mine} std ${theirs}")
}
//= stdout
//= tally 9
//= list 1 mapped 3
//= member hi! std HI
