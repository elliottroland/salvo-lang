// a `try` body is ordinary code to the name/mutability censuses
//
// Compile-and-run data ([test-run-data]), moved from the codegen tests.

//--- try_mutation

import std.throw

fn risky(n: Int) [Throw<Str>] -> Int {
    if n < 0 {
        throw("negative")
    }
    return n
}

fn main() [use] -> None {
    use StdConsole
    let counter = 0
    let outcome = try {
        counter = counter + 1
        let step = risky(3)
        step
    }
    println("counter ${counter}")
}
//= stdout
//= counter 1
