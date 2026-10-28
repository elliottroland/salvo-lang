// destructuring a loop element [var-let-destructure]
//
// Compile-and-run data ([test-run-data]), moved from the codegen tests.

//--- loop_destructuring

struct Row { name: Str, score: Int }

fn main() [use] -> None {
    use StdConsole
    // [var-let-destructure] A loop element destructures like a `let`: a tuple by
    // position, a struct by field (renamed where the pattern says so).
    let pairs: List<(Str, Int)> = List(("a", 1), ("b", 2))
    for (k, v) in iter(pairs) {
        println("${k}=${v}")
    }
    let rows: List<Row> = List(Row { name: "x", score: 7 }, Row { name: "y", score: 8 })
    for {name: who, score} in iter(rows) {
        println("${who} ${score}")
    }
    // A binding the body assigns to is the local's own, not the element's.
    let nums: List<(Int, Int)> = List((1, 2), (3, 4))
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
//= stdout
//= a=1
//= b=2
//= x 7
//= y 8
//= 3
//= 7
//= sum 3
