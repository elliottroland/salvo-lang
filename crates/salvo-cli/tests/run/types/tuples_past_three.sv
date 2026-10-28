// tuples past three [type-tuple]
//
// Compile-and-run data ([test-run-data]), moved from the codegen tests.

//--- tuples_past_three

fn main() [use] -> None {
    use StdConsole
    // [type-tuple] Past three elements Kotlin has no tuple type of its own,
    // so the backend generates one; Rust's are native at any arity.
    let q: (Int, Str, Bool, Int, Str) = (1, "two", true, 4, "five")
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
//= stdout
//= 1 two true 4 five
//= 1 two true 4 five
