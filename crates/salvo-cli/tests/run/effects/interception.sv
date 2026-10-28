// interception [effect-intercept]
//
// Compile-and-run data ([test-run-data]), moved from the codegen tests.

//--- interception

effect Greeter {
    fn greet(name: Str) -> Str => name
}

effect Store<T> {
    fn keep(value: T) -> Str => value
}

handler Plain of Greeter {
    fn greet(name: Str) -> Str => name {
        return "hello ${name}"
    }
}

handler Formal of Greeter {
    fn greet(name: Str) -> Str => name {
        return "Good day, ${name}"
    }
}

handler Loud [Greeter] of Greeter {
    fn greet(name: Str) -> Str => name {
        return "${greet(name)}!"
    }
}

handler Counting [Greeter] of Greeter {
    state {
        count: Int = 0
    }
    fn greet(name: Str) -> Str => name {
        count = count + 1
        return "${greet(name)} (${count})"
    }
}

handler MemStore<T> of Store<T> {
    fn keep(value: T) -> Str => value {
        return "kept"
    }
}

handler Twice [Store<Int>] of Store<Int> {
    fn keep(value: Int) -> Str => value {
        return "${keep(value)} ${keep(value)}"
    }
}

fn shout(name: Str) [Greeter, Console] -> None => name {
    println(greet(name))
}

fn main() [use] -> None {
    use StdConsole
    use Plain
    shout("a")
    use Formal
    shout("b")
    use Counting
    shout("c")
    if true {
        use Loud
        shout("d")
    }
    shout("e")
    use MemStore<Int>()
    use MemStore<Str>()
    println(keep(1))
    println(keep("x"))
    use Twice
    println(keep(2))
    println(keep("y"))
}
//= stdout
//= hello a
//= Good day, b
//= Good day, c (1)
//= Good day, d (2)!
//= Good day, e (3)
//= kept
//= kept
//= kept kept
//= kept
