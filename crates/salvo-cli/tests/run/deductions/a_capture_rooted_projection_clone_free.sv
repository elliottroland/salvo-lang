// [fate-lambda-view] a capture-rooted projection, clone-free
//
// Compile-and-run data ([test-run-data]), moved from the codegen tests.

//--- a_capture_rooted_projection

fn main() [use] {
    use StdConsole()
    let all = List("a", "b", "c", "d")
    let indices = List(1, 3)
    let picked = map(indices, i -> (get(all, i) ?: unreachable!()))
    for w in picked {
        println(w)
    }
    let p = iter(indices)
    let doubled = map(p, i -> i + i)
    for n in doubled {
        println("${n}")
    }
}
//= stdout
//= b
//= d
//= 2
//= 6
