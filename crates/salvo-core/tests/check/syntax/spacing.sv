// [syntax-space] Spacing tells a name's arguments from what a qualifier
// qualifies (user decision 2026-11-12): the `(` of a name's argument list —
// a call's, a declaration's parameters, a qualifier's or a type's value
// arguments — touches the name, and a qualifier is a space away from what it
// qualifies. Keywords are not names: `if (a)` and `return (x)` keep their
// space. Each slip is one error, and the parentheses are read as the rule
// says the author meant, so what follows is still parsed.

//--- the_accepted_forms
import std.heap.Heap

struct Point { x: Int, y: Int }

// A qualifier's arguments touch it, and the qualified type is a space away.
fn ranged(n: InRange(0, 100) Int) -> None {
    //~ type[InRange(0, 100) Int]
    type?(n)
}

fn borrowed(xs: List<Int>, i: Int) -> proj(xs) Int? => xs {
    return get(xs, i)
}

fn heaped<T>(heap: Heap<T>(?cmp) Mut List<T>) -> None {
    //~ type[Heap<T>(?cmp) Mut List<T>]
    type?(heap)
}

// A space after a qualifier: what it qualifies, here the tuple type of the
// literal types `1` and `2`.
fn tupled(p: Ok (1, 2), q: Ok (Int, Str)) -> None {
    //~ type[Ok (1, 2)]
    type?(p)
    //~ type[Ok (Int, Str)]
    type?(q)
}

// Touching, the same parentheses are `Ok`'s arguments, which it does not
// take.
//~ error[qual-depend:no-value-args]
fn constants(n: Ok(1, 2) Int) -> None {
}

// A keyword is not a name: a parenthesized expression may follow it after a
// space.
fn keywords(a: Bool, n: Int) -> Int {
    if (a) {
        return (n + 1)
    }
    while (false) {
    }
    let _in = n is InRange(0, 5)
    return (0)
}

// Calls and declarations touch their parentheses; a struct literal, a
// type-named call and a lambda argument after a comma are as before.
fn calls(xs: List<Int>) [Console] {
    let p = Point { x: 1, y: 2 }
    let ys = Mut List(1, 2)
    add(ys, size(xs))
    let total = reduce(xs, 0, (acc, x) -> acc + x)
    assert!(total >= 0)
    println("${p.x} ${ys.size()}")
}

// A `(` on the line after a name is never its argument list: it starts the
// next statement.
fn next_line(n: Int) -> Int {
    let m = n
    (m + 1)
    //~ type[Int]
    type?(m)
    return m
}

//--- a_space_before_a_call_s_arguments_is_refused
struct Point { x: Int }

effect Log {
    fn log(s: Str) => s
}

handler Quiet for Log {
    fn log(s: Str) {
    }
}

fn twice(n: Int) -> Int {
    return n * 2
}

fn calls(xs: List<Int>) [Console, use] {
    //~ error[syntax-space:before-args]
    let a = twice (1)
    //~ error[syntax-space:before-args]
    let b = xs.size ()
    //~ error[syntax-space:before-args]
    let c = twice<Int> (2)
    //~ error[syntax-space:before-args]
    let ys = Mut List (1, 2)
    //~ error[syntax-space:before-args]
    assert! (a > 0)
    //~ error[syntax-space:before-args]
    use Quiet ()
    //~ error[syntax-space:before-args]
    //~ type[Int]
    type? (a)
    //~ error[syntax-space:before-args]
    type! (a: Int)
    //~ error[syntax-space:before-args]
    let _w = whynot? (a is InRange(0, 5))
    let d = twice(
        twice
        //~ error[syntax-space:before-args]
        (3))
    println("${a} ${b} ${c} ${d} ${ys.size()}")
}

//--- a_space_before_a_declaration_s_parameters_is_refused
//~ error[syntax-space:before-args]
fn spaced (x: Int) -> None {
}

//~ error[syntax-space:before-args]
fn generic<T> (x: T) -> None {
}

effect Log {
    //~ error[syntax-space:before-args]
    fn log (s: Str) => s
}

//~ error[syntax-space:before-args]
handler Prefixed (prefix: Str) for Log {
    fn log(s: Str) {
    }
}

//~ error[syntax-space:before-args]
qualifier Between (lo: Int, hi: Int) for Int

qualifier Tagged<T> for List<T> {
    //~ error[syntax-space:before-args]
    refn add (list: Mut List<T>, elem: T) => list: +Tagged
}

//~ error[syntax-space:before-args]
fn keep_all<It>(it: Mut It) -> Mut List<Int> holds proj (it) => it: Mut {
    return Mut List()
}

//--- a_space_before_a_comptime_form_s_arguments_is_refused
struct P { a: Int }

comptime fn f(p: P) [] -> Int => p {
    [for f in P.fields] {
        // Read as `refuse!(…)` all the same, so it refuses.
        //~ error[syntax-space:before-args]
        //~ error[comptime-refuse:refused]
        refuse! ("${f.name}")
    }
    return 0
}

//--- a_space_before_a_test_s_parameters_is_refused
// FILE: main.test.sv
//~ error[syntax-space:before-args]
test property (runs=2) "x"(n: Int) {
}

//~ error[syntax-space:before-args]
test property "y" (n: Int) {
}

//--- a_space_before_a_qualifier_s_arguments_is_refused
// Only where the parentheses can be nothing but arguments: a fn slot's `?`,
// a value name, constants a type follows, or, in an `is` test, constants.
//~ error[syntax-space:before-args]
fn ranged(n: InRange (0, 100) Int) -> None {
}

fn spaced(xs: List<Int>, n: Int) -> None {
    //~ error[syntax-space:before-args]
    let _p: proj (xs) Int? = get(xs, 0)
    //~ error[syntax-space:before-args]
    let _k = n is InRange (0, 5)
}

//--- a_qualifier_touching_what_it_qualifies_is_refused
//~ error[syntax-space:qualifier-touching]
fn glued(s: Mut(Str)) -> None {
}

//~ error[syntax-space:qualifier-touching]
fn tuple(p: Ok(Int, Str)) -> None {
}

//~ error[syntax-space:qualifier-touching]
fn once_glued(f: once(() -> Int)) -> None => !f {
}

//~ error[syntax-space:qualifier-touching]
fn borrowed(xs: List<Int>, i: Int) -> proj(xs)Int? => xs {
    return get(xs, i)
}

//~ error[syntax-space:qualifier-touching]
fn ranged(n: InRange(0, 100)Int) -> None {
}

//--- touching_parentheses_are_arguments_a_slotless_qualifier_refuses
// Touching, the parentheses are arguments even where the qualifier takes
// none; the checker refuses them, for a declared qualifier and for the
// compiler's own alike.
//~ error[qual-depend:no-value-args]
//~ error[qual-depend:no-value-args]
fn given(s: Mut(1) Str, xs: NonEmpty(1) List<Int>) -> None {
}

//~ error[qual-depend:no-value-args]
fn once_given(f: once(1) () -> Int) -> None => !f {
}
