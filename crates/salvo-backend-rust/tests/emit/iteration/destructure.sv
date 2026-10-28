// Destructuring a loop element.

//--- loop_destructuring_binds_the_parts_of_a_temporary
// [var-let-destructure] A `for` binds a **pattern**, not just a name: the header
// binds the element to a temporary and the body opens with the pattern's
// bindings. Expected output is verbatim the Kotlin backend's.
// [var-let-destructure] [rs-borrow-locals] What it lowers to: the element in a
// temporary, then one binding per name — **by reference**, which is what makes
// one shape serve an owned element and a borrowed one (a pattern with `mut`
// bindings cannot move out of a pass's projection, `E0507`). A name the body
// assigns to takes an owned copy instead, since a reference cannot be
// reassigned.
struct Row { name: Str, score: Int }

fn main() [use] -> None {
    use StdConsole
    // [var-let-destructure] A loop element destructures like a `let`: a tuple by
    // position, a struct by field (renamed where the pattern says so).
    let pairs: List<(Str, Int)> = List(("a", 1), ("b", 2))
    // The tuple parts bound by reference (a scalar copied).
    //~ emits: let mut k = &__elem_6.0;
    //~ emits: let mut v: i32 = __elem_6.1;
    for (k, v) in iter(pairs) {
        println("${k}=${v}")
    }
    let rows: List<Row> = List(Row { name: "x", score: 7 }, Row { name: "y", score: 8 })
    // The struct fields bound by reference, renamed.
    //~ emits: let mut who = &__elem_10.name;
    //~ emits: let mut score: i32 = __elem_10.score;
    for {name: who, score} in iter(rows) {
        println("${who} ${score}")
    }
    // A binding the body assigns to is the local's own, not the element's.
    let nums: List<(Int, Int)> = List((1, 2), (3, 4))
    //~ emits: let mut a: i32 = __elem_14.0;
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
