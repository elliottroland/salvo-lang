// M7: reachability, packages/imports, companions
//
// Compile-and-run data ([test-run-data]), moved from the codegen tests.

//--- mangled_alias

import lib.shout as holler
import lib.loud

fn main() [use] -> None {
    use StdConsole
    println(holler("hi"))
    let l = loud("hey")
    println(holler(l))
}
// FILE: lib.sv

export qualifier Loud of Str

export fn loud(s: Str) -> +Loud Str
=> !s {
    return s
}

export fn shout(s: Str) -> Str
=> !s {
    return s
}

export fn shout(s: Loud Str) -> Str {
    return "${s}!"
}
//= stdout
//= hi
//= hey!
