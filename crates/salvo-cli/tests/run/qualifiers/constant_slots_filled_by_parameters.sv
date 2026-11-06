// [qual-const] A value slot a signature fills with a parameter: the call's
// literal fills it (`clamp(n, 1, 65535)` is an `InRange(1, 65535) Int`), a
// negative constant included; a place argument makes it a claim about that
// place, as any dependent slot is [qual-depend].

//--- constant_slots_filled_by_parameters
fn clamp(n: Int, lo: Int, hi: Int) -> InRange(lo, hi) Int {
    let m = if n < lo { lo } elif n > hi { hi } else { n }
    if m is InRange(lo, hi) {
        return m
    }
    return unreachable!()
}

fn port(p: InRange(1, 65535) Int) -> Int => !p {
    return p
}

fn signed(n: InRange(-5, 5) Int) -> Int => !n {
    return n
}

fn main() [use] -> None {
    use StdConsole()
    println("${port(clamp(70000, 1, 65535))}")
    println("${signed(clamp(9, -5, 5))}")
    let lo = 0
    let hi = 9
    let d = clamp(12, lo, hi)
    if d is InRange(lo, hi) {
        println("${d} is in range")
    }
}
//= stdout
//= 65535
//= 5
//= 9 is in range
