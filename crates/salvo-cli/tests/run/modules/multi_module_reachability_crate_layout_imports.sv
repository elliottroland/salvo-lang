// multi-module: reachability, crate layout, imports
//
// Compile-and-run data ([test-run-data]), moved from the codegen tests.

//--- multi_module
// BACKENDS: kotlin

import geometry.area

fn main() [use] -> None {
    use StdConsole
    println("area: ${area(3, 4)}")
}
// FILE: geometry.sv

export fn area(w: Int, h: Int) -> Int {
    return w * h
}
// FILE: unused.sv

export fn never_called() -> Int {
    return 42
}
// FILE: geometry_helpers.kt
package salvo.geometry

fun helper(): Int = 1
// FILE: unused_helpers.kt
package salvo.unused
//= stdout
//= area: 12
