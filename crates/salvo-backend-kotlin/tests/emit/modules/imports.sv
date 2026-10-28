// Modules: foreign references are fully qualified, so an import, aliased or
// not, leaves no trace in the importing module's code.

//--- modules_get_packages_and_generated_imports
// [kt-package] [kt-imports]
// A multi-module program: `main` uses `geometry` (imported) but not
// `unused`. (Its packages and import lines stay in codegen_tests.rs.)
import geometry.area

fn main() [use] -> None {
    use StdConsole
    // [kt-ir] A foreign reference is fully qualified, so a module imports
    // only the root runtime package.
    //~ emits(main.kt): salvo.std.core.console.println(
    //~ emits(main.kt): salvo.geometry.area(
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

//--- aliased_imports_emit_kotlin_alias_imports
// [kt-imports] An aliased Salvo import leaves no trace: the call names the
// declaration, fully qualified.
import geometry.area as rect_area

fn main() [use] -> None {
    use StdConsole
    // [kt-ir] An aliased import names the declaration: the reference is
    // the declaration's own, fully qualified, and the alias is gone.
    //~ emits(main.kt): salvo.geometry.area(3, 4)
    //~ !emits: rectArea
    println("area: ${rect_area(3, 4)}")
}
// FILE: geometry.sv
export fn area(w: Int, h: Int) -> Int {
    return w * h
}

//--- aliased_import_of_mangled_overload_keeps_suffix
// [kt-imports] [fn-emit-name] An aliased import of a fn with a mangled
// qualified overload: each call names its overload's mangled name, fully
// qualified.
import lib.shout as holler
import lib.loud

fn main() [use] -> None {
    use StdConsole
    // [kt-ir] [fn-emit-name] The aliased calls name the mangled overloads
    // of the declaring module, fully qualified.
    //~ emits(main.kt): salvo.lib.shout__Str("hi")
    println(holler("hi"))
    let l = loud("hey")
    //~ emits(main.kt): salvo.lib.shout__LoudStr(l)
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
