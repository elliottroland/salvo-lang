// [op-safe-nav] [type-none-unit] A `?.` call whose result is `None` joins the
// call's unit with the absent arm, which is the unit too: the program runs
// on both backends (ROADMAP 0ar, from B3.4: rustc refused the `if` whose
// branches were `()` and `None`). A branch value `None` beside a `None` call
// is the same join.
//
// Compile-and-run data ([test-run-data]).

//--- the_absent_arm_is_the_unit
fn shout(n: Int) [Console] -> None {
    println("shout ${n}")
}

fn pick(loud: Bool) [Console] -> None {
    return if loud { shout(9) } else { None }
}

fn main() [use] {
    use StdConsole()
    let m: Int? = 3
    m?.shout()
    let k: Int? = None
    k?.shout()
    pick(true)
    pick(false)
    println("done")
}
//= stdout
//= shout 3
//= shout 9
//= done
