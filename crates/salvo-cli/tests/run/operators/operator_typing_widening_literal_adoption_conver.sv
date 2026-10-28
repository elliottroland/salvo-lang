// operator typing: widening, literal adoption, conversions
//
// Compile-and-run data ([test-run-data]), moved from the codegen tests.

//--- operators

fn main() [use] -> None {
    use StdConsole
    let x: Long = 1
    let n = 5
    let y = x + n * 2
    let big = 4000000000L
    let scaled = big * 2 + n
    let d: Double = 3
    let g = d + to_double(n) + 0.25
    let f: Float = 0.5
    let h = f * 1.5f
    let cmp = n < x
    let back = to_int(big)
    let neg: Long = -7
    println("${y} ${scaled} ${g} ${h} ${cmp} ${back} ${neg}")
}
//= stdout
//= 11 8000000005 8.25 0.75 false -294967296 -7

//--- the_safe_pipe_operator

fn maybe(present: Bool) -> Int? {
    if present {
        return 42
    }
    return None
}

fn double(x: Int) -> Int {
    return x * 2
}

fn half_if_even(x: Int) -> Int? {
    if x % 2 == 0 {
        return x / 2
    }
    return None
}

fn show(label: Str, v: Int?) [Console] -> None {
    when v {
        is Int { println("${label} present ${v}") }
        is None { println("${label} absent") }
    }
}

fn main() [use] {
    use StdConsole()
    show("present", maybe(true) ?> double(_))
    show("absent", maybe(false) ?> double(_))
    // right side already optional: result is Int?, not Int??
    show("flat", maybe(true) ?> half_if_even(_))
    show("flat-none", maybe(true) ?> half_if_even(_ + 1))
    // chained: 42 -> 84 -> 168
    show("chain", maybe(true) ?> double(_) ?> double(_))
}
//= stdout
//= present present 84
//= absent absent
//= flat present 21
//= flat-none absent
//= chain present 168
