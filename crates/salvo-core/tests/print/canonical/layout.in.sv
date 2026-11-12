// [fmt-blank-lines] [fmt-comments] What the printer changes of a layout:
// blank lines written in a run print as one, and none at a body's start or
// end; consecutive one-line declarations stay together, and any other two
// get one blank line; a comment between a branch's `}` and its `elif`
// closes the branch.

import a
import b
type A = Int
fn f() {}
type B = Int


type C = Int
fn g(x: Int) -> Int {


    let a = 1



    let b = 2

    // a free-standing comment


    return a

}
fn h(x: Int) -> Int {
    if x > 0 {
        return 1
    }
    // before the elif
    elif x < 0 {
        return 2
    }
    return 3
}
