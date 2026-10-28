// Reachability follows resolution, and what a program uses is emitted
// [mod-used-only] [interp-to-str].

//--- reachability_iterating_emits_what_it_uses
// [mod-used-only] Reachability follows **resolution** for functions, not
// names (user decision 2026-09-25, closing a defect open since 2026-09-23).
// A name resolves to every module declaring it, so a program that merely
// iterates — using the name `next` — used to emit `std.core.range` as well,
// because `std.core.range` declares a `next` too. The checker knows which `next`
// the loop drives, so the edge follows that. (That `std.core.range`, `fs` and
// `std.core.bytes` are *not* emitted is `reachability_follows_resolution_not_names`
// in `codegen_tests.rs`: a file's absence is not an annotation.)
// What it does use is still there.
//~ emits(std/core/list.rs): pub fn iter<T: Clone>(list: &Vec<T>)
//~ emits(std/core/console.rs): pub fn println(console: 
//~ emits(std/core/seq.rs): pub fn reduce__It_A_Fn<
fn main() [use] -> None {
    use StdConsole()
    let xs: List<Int> = List(1, 2, 3)
    for x in xs {
        println("${x}")
    }
    let total = reduce(iter(xs), 0, (a: Int, b: Int) -> { return a + b })
    println("${total}")
}

//--- reachability_a_program_that_ranges_gets_range
// …and a program that really does range still gets it, which is the half a
// narrowing could quietly break.
//~ emits(std/core/range.rs): pub fn range__Int_Int(
fn main() [use] -> None {
    use StdConsole()
    for i in range(1, 4) {
        println("${i}")
    }
}

//--- a_to_str_from_another_module
// FILE: pair.sv
// [interp-to-str] [iter-resolve] A `to_str` the checker resolved in **another
// module**, which the file never *names*: name-based reachability left the
// module un-emitted and name-based imports left it un-imported, so the call
// had nothing to bind to. Resolution edges now reach it
// (`reach::resolved_dep_files`). [op-unique-shape] The `to_str` used to be in a
// third module (`fmt`) the file imported whole and never named; it now has to
// be in `Pair`'s own file, so the case is `Pair`'s attached `to_str`,
// imported with `Pair` [fn-attached] and never named: the import half of the
// bug (the reachability half cannot arise for an operator, whose module is
// the type's).
// The module the interpolation resolved into is emitted.
//~ emits(pair.rs): pub struct Pair {
export struct Pair {
    a: Int,
    b: Int

    fn to_str(p: Pair) -> Str => p {
        return "(${p.a}, ${p.b})"
    }
}
// FILE: main.sv
import pair.Pair

fn main() [use] -> None {
    use StdConsole()
    let p = Pair { a: 1, b: 2 }
    println("${p}")
}
