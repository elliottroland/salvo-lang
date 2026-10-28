// Implicit parameters [implicit-param] [implicit-group]: trailing fn
// parameters, filled at the call site.

//--- implicit_parameters_lower_to_trailing_fn_parameters
// [implicit-param] Implicit parameters are ordinary trailing parameters of
// fn type; a resolved default is passed as a Kotlin function reference, and
// a group leaves no runtime representation [implicit-group].
// The same source and expected stdout as the Rust backend's
// `rustc_compiles_and_runs_implicit_parameters` (`IMPLICIT_DEMO` in
// codegen_tests.rs).
// A `fns` group must leave no runtime representation.
//~ !emits: class Field
fns Field<T> {
    fn add(a: T, b: T) -> T
    fn zero() -> T
}

fn add(a: Int, b: Int) -> Int { return a + b }
fn zero() -> Int { return 0 }
// [op-unique-shape] Not `times`: that is the `*` operator on `Int`, declared
// only in `Int`'s own file; an override is named for what it does.
fn multiply(a: Int, b: Int) -> Int { return a * b }
fn one() -> Int { return 1 }

//~ emits: fun<T> total(xs: List<T>, add: (T, T) -> T, zero: () -> T): T {
fn total<T>(xs: List<T>, ?Field<T>) -> T => xs {
    let acc = zero()
    for x in xs {
        acc = add(acc, x)
    }
    return acc
}

fn total_all<T>(rows: List<List<T>>, ?Field<T>) -> T => rows {
    let acc = zero()
    for row in rows {
        acc = add(acc, total(row))
    }
    return acc
}

fn sum_pair<T>(a: T, b: T, ?add: (T, T) -> T) -> T {
    return add(a, b)
}

fn main() [use] {
    use StdConsole()
    //~ emits: total(listOf<Int>(1, 2, 3), ::add, ::zero)
    println("total=${total(List(1, 2, 3))}")
    // An override written at the call site [implicit-override].
    //~ emits: total(listOf<Int>(2, 3, 4), ::multiply, ::one)
    println("product=${total(List(2, 3, 4), add = multiply, zero = one)}")
    // [col-of-nonempty] Literals rather than constructor calls: the element
    // constructors claim `NonEmpty`, and a `List<NonEmpty List<Int>>` does not
    // fit a `List<List<Int>>` position (type arguments are invariant). A literal
    // claims nothing, which is what a nested plain list needs.
    println("nested=${total_all(List<List<Int>>(List(1, 2), List(3)))}")
    println("pair=${sum_pair(20, 22)}")
    println("lambda=${sum_pair(2, 3, add = (a: Int, b: Int) -> a * b)}")
}

//--- an_effect_member_carries_its_implicits_into_the_interface
// [implicit-param] The same source and stdout as the Rust backend's
// `rustc_compiles_and_runs_effect_member_implicits`: an effect member's
// implicit parameters reach the interface, every handler's override, and the
// call site. (`MEMBER_IMPLICIT_DEMO` in codegen_tests.rs.)
//~ emits: fun show(v: Int, fmt: (Int) -> String): String
effect Show {
    fn show(v: Int, ?fmt: (Int) -> Str) -> Str => v
}

//~ emits: override fun show(v: Int, fmt: (Int) -> String): String {
handler Angle of Show {
    fn show(v: Int, ?fmt: (Int) -> Str) -> Str => v {
        return "<${fmt(v)}>"
    }
}

fn fmt(n: Int) -> Str => n {
    return "n=${n}"
}

fn loud(n: Int) -> Str => n {
    return "N=${n}!"
}

fn main() [use] {
    use StdConsole()
    use Angle()
    //~ emits: .show(7, ::fmt)
    println(show(7))
    //~ emits: .show(7, ::loud)
    println(show(7, fmt = loud))
}
