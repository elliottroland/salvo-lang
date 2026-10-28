// Implicit parameters [implicit-param] [implicit-group].

//--- an_implicit_argument_does_not_depend_on_declaration_order
// [implicit-param] A call fills the **callee's** implicit parameters, read out
// of a side table — so the answer must not depend on where the callee is
// declared. It did until 2026-09-22: a callee written *below* its caller (or in
// a file sorted after it) looked like a fn with no implicits, so the call was
// accepted with none filled and the emitted call was short an argument, on both
// backends. The checker could not see it — it accepted the call either way —
// which is why the test lives here, where the symptom was.

fn caller(a: Str, b: Str) -> Int => a, b {
    // …and the call from *above* it fills the position rather than omitting it.
    //~ emits(main.rs): crate::bigger::<String>(a, b, &mut |__a0: &String, __a1: &String|
    return bigger(a, b)
}

// Declared *after* its caller, and takes an implicit.
// The callee takes the position…
//~ emits(main.rs): pub fn bigger<T: Clone>(a: &T, b: &T, cmp: &mut dyn FnMut(&T, &T) -> i32)
fn bigger<T>(a: T, b: T, ?Ordered<T>) -> Int => a, b {
    if cmp(a, b) > 0 {
        return 1
    }
    return 0
}

fn main() [use] {
    use StdConsole()
    let n = caller("bb", "a")
    println("${n}")
}

//--- implicit_parameters_lower_to_trailing_fn_arguments
// Every path through the feature in one program: a `fns` group spread
// with no binder, its members resolved from the visible `Int` overloads, a
// call overriding *one* member by name, forwarding through a generic fn
// whose `T` is opaque, and an individually declared `?add` reached by the
// same name. The Kotlin backend asserts the same stdout for the same
// source.
//
// [implicit-param] An implicit parameter lowers to an ordinary trailing
// parameter of fn type, and the call site passes what resolution found —
// so nothing about the feature survives into Rust. A group leaves no trace
// at all: it was never a value [implicit-group].
//~ !emits(main.rs): struct Field
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

// The group's members, expanded in declaration order.
// `dyn`, uniformly: an effect member's implicits land in an
// object-safe trait, and forwarding has to compose in every
// direction, so one convention serves both.
// [rs-fn-param-convention] A member's parameters are *kept* (no
// deduction clause keeps everything [fn-contract]), so a non-Copy
// position borrows — `&T` at a type variable, which is never known to
// be Copy. By value it was a move, and a generic fn that compared two
// values and then used one of them did not compile (2026-09-21).
//~ emits(main.rs): pub fn total<T: Clone>(xs: &Vec<T>, add: &mut dyn FnMut(&T, &T) -> T, zero: &mut dyn FnMut() -> T)
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
        // Forwarding reborrows the enclosing fn's own parameter.
        //~ emits(main.rs): &mut *add
        acc = add(acc, total(row))
    }
    return acc
}

fn sum_pair<T>(a: T, b: T, ?add: (T, T) -> T) -> T {
    return add(a, b)
}

fn main() [use] {
    use StdConsole()
    // A resolved default is passed as an adapter closure over the fn,
    // which bridges the position's convention to the callee's own: `add`
    // takes its `Int`s by value, so the adapter derefs the borrows.
    //~ emits(main.rs): &mut |__a0: &i32, __a1: &i32| crate::add(*__a0, *__a1)
    println("total=${total(List(1, 2, 3))}")
    // The same bridging for an override written at the call site
    // [implicit-override].
    //~ emits(main.rs): &mut |__a0: &i32, __a1: &i32| crate::multiply(*__a0, *__a1)
    println("product=${total(List(2, 3, 4), add = multiply, zero = one)}")
    // [col-of-nonempty] Literals rather than constructor calls: the element
    // constructors claim `NonEmpty`, and a `List<NonEmpty List<Int>>` does not
    // fit a `List<List<Int>>` position (type arguments are invariant). A literal
    // claims nothing, which is what a nested plain list needs.
    println("nested=${total_all(List<List<Int>>(List(1, 2), List(3)))}")
    println("pair=${sum_pair(20, 22)}")
    println("lambda=${sum_pair(2, 3, add = (a: Int, b: Int) -> a * b)}")
}
