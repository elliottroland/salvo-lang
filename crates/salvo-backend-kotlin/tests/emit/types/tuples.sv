// Tuples: `Pair`/`Triple` components, and the generated classes past three.

//--- tuple_elements_emit_pair_components
// [kt-tuple-component] Tuple elements map onto `Pair`/`Triple`
// components, and a narrowed one asserts — a stdlib property is not
// smart-cast [kt-narrow-field-assert].
// `t.0` reads a tuple element; a constant index narrows like a field
// [flow-place], and nesting (`t.1.0`) is two projections [expr-tuple-index].
// (`TUPLE_INDEX_DEMO` in codegen_tests.rs.)
fn main() [use] -> None {
    use StdConsole
    let t: (Int, Str, Bool) = (1, "two", true)
    //~ emits: ${t.first} ${t.second} ${t.third}
    println("${t.0} ${t.1} ${t.2}")
    let nested: (Int, (Str, Int)) = (7, ("in", 9))
    //~ emits: ${nested.second.first}
    println("${nested.1.0} ${nested.1.1}")
    let maybe: (Str?, Int) = ("here", 5)
    if maybe.0 is Str {
        //~ emits: : String = maybe.first!!
        println("some ${maybe.0} ${maybe.1 + 1}")
    } else {
        println("none")
    }
}

//--- tuples_past_three_emit_a_generated_class
// [kt-tuple-class] What a tuple past three lowers to: a `SalvoTupleN` literal,
// the `v3`/`v4` component names past `Pair`'s three [kt-tuple-component], and
// a generated `tuples.kt` whose class is a `data class` (structural equality
// and `componentN`, so a `Set` element and a destructuring both work)
// carrying nothing more: its `eq`/`cmp`/`hash` are Salvo's.
// [kt-tuple-class] [type-tuple] Kotlin has `Pair` and `Triple` and nothing
// past them, so a bigger tuple is a **generated data class** — which is what
// makes indexing, destructuring, hashing and ordering work on the same terms
// as a `Pair`'s. Source and expected stdout are **verbatim** the Rust
// backend's `rustc_compiles_and_runs_tuples_past_three`.
// (`BIG_TUPLE_DEMO` in codegen_tests.rs.)
//~ emits(tuples/Tuple5.kt): data class Tuple5<
//~ !emits(tuples/Tuple5.kt): SalvoTuple
//~ emits(tuples/Tuple5.kt): val v3: T4
fn main() [use] -> None {
    use StdConsole
    // [type-tuple] Past three elements Kotlin has no tuple type of its own,
    // so the backend generates one; Rust's are native at any arity.
    //~ emits: Tuple5(1, "two", true, 4, "five")
    //~ emits: Tuple5<Int, String, Boolean, Int, String>
    let q: (Int, Str, Bool, Int, Str) = (1, "two", true, 4, "five")
    //~ emits: ${q.third} ${q.v3} ${q.v4}
    println("${q.0} ${q.1} ${q.2} ${q.3} ${q.4}")
    let (a, b, c, d, e) = q
    println("${a} ${b} ${c} ${d} ${e}")
    // A keyed container over a tuple used to live here, exercising the
    // generated class's structural comparison. It cannot today: a keyed
    // constructor asks for its identity as a capability (user decision
    // 2026-09-26) and std declared `cmp`/`hash` for the intrinsic
    // scalars only, so a tuple has no identity to pass — the same gap that
    // stops `(1, 2) == (1, 2)` and `(1, 2) < (1, 3)` from resolving, which
    // never worked either. ROADMAP's "Recursive implicit resolution" is the
    // lift; until then the arity, the component reads and the destructuring
    // above are what this case is about.
}
