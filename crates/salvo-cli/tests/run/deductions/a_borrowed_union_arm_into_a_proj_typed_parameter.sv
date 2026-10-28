// [proj-type] a borrowed union arm into a `proj`-typed parameter
//
// Compile-and-run data ([test-run-data]), moved from the codegen tests.

//--- a_borrowed_union_arm_into_a_proj_parameter

fn show(step: Emitted (proj Str) | Finished) [Console] -> None {
    when step {
        is Emitted { println(step) }
        is Finished { println("done") }
    }
}

fn main() [use] {
    use StdConsole()
    let words = List("ann", "bo")
    let p = iter(words)
    show(next(p))
    show(next(p))
    show(next(p))
    let e = get(words, 0) ?: unreachable!()
    println(e)
    let owned: Str = copy(e)
    println(owned)
}
//= stdout
//= ann
//= bo
//= done
//= ann
//= ann
