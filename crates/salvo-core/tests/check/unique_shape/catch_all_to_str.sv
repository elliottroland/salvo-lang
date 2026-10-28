// A catch-all `to_str<T>` beside std's.

//--- a_catch_all_to_str_is_refused_at_its_declaration
// [op-unique-shape] The program above as it was before 2026-10-27, with a
// module's catch-all `to_str<T>` beside std's: refused at the catch-all's
// declaration, used or not — a `to_str` on a type parameter would be
// interpolation's function for every type at once, and a type parameter has
// no file to declare it in. The written override (`to_str = to_str@main`)
// goes with it; the catch-all is given another name and passed explicitly.
//~ error[op-unique-shape:bare-type-param]
fn to_str<T>(x: T) [] -> Str => x { return "?" }

fn main() [use] {
    use StdConsole()
    let xs = List(1, 2)
    //~ error[fn-overload-at:no-fn-in-module]
    println("${to_str(xs, to_str = to_str@main)}")
}
