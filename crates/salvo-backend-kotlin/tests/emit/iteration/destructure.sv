// Destructuring a loop element [var-let-destructure].

//--- loop_destructuring_binds_the_parts_of_a_temporary_kotlin
// [var-let-destructure] What it lowers to: the element in a temporary — cast to its
// own type, which is what a pass-driven header needs — then one `val` per name,
// a tuple's by component and a struct's by field. Kotlin *could* destructure a
// `Pair` in the header, but not the pass's payload and not a struct, so one
// shape serves every loop and matches the Rust backend's.
// [var-let-destructure] A `for` binds a **pattern**, not just a name. Source and
// expected stdout are **verbatim** the Rust backend's
// `rustc_compiles_and_runs_loop_destructuring`. (`LOOP_DESTRUCTURE` in
// codegen_tests.rs.)
struct Row { name: Str, score: Int }

fn main() [use] -> None {
    use StdConsole
    // [var-let-destructure] A loop element destructures like a `let`: a tuple by
    // position, a struct by field (renamed where the pattern says so).
    let pairs: List<(Str, Int)> = List(("a", 1), ("b", 2))
    //~ emits: : kotlin.Pair<String, Int> = __emitted_
    //~ emits: val k: String = __elem_6.first
    //~ emits: val v: Int = __elem_6.second
    for (k, v) in iter(pairs) {
        println("${k}=${v}")
    }
    let rows: List<Row> = List(Row { name: "x", score: 7 }, Row { name: "y", score: 8 })
    //~ emits: val who: String = __elem_10.name
    //~ emits: val score: Int = __elem_10.score
    for {name: who, score} in iter(rows) {
        println("${who} ${score}")
    }
    // A binding the body assigns to is the local's own, not the element's.
    let nums: List<(Int, Int)> = List((1, 2), (3, 4))
    //~ emits: var a: Int = __elem_14.first
    for (a, b) in iter(nums) {
        a = a + b
        println("${a}")
    }
    // In value position, and nested — two loops, two element temporaries.
    let sum = for (x, y) in iter(pairs) {
        size(x) + y
    } else {
        0
    }
    println("sum ${sum}")
}
