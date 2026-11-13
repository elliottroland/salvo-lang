// [syntax-space] Spacing tells a name's arguments from what a qualifier
// qualifies (user decision 2026-11-12): the `(` of a name's argument list —
// a call's, a declaration's parameters, a qualifier's or a type's value
// arguments — touches the name, and a qualifier is a space away from what it
// qualifies. Keywords are not names: `if (a)` and `return (x)` keep their
// space. A type argument list's `<` touches its name as a `(` does, and a
// form's `!` or `?` (`assert!`, `type?`) is part of its name. An operator
// the parser reads from two tokens (`?:`, `?.`, `?>`, `!is`, `as!`, `as?`, a
// type's `[]`) is one operator, with nothing inside it. Each
// slip is one error, and the parentheses are read as the rule says the
// author meant, so what follows is still parsed. A qualifier's arguments
// are counted in a type as an `is` test counts them.

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

//--- a_type_argument_list_touches_its_name
// The `<` of type arguments and type parameters follows the `(` rule: it
// touches the name. A `<` the parser does not read as type arguments is a
// comparison, spaced or not.
import std.heap.Heap

struct Box<T> { value: T }

qualifier Tagged<T> for List<T>

effect Source<T> {
    fn next() -> T
}

fn twice<T>(x: T) -> T => !x {
    return x
}

fn heaped<T>(heap: Heap<T>(?cmp) Mut List<T>) -> None {
}

fn uses(a: Int, b: Int, xs: List<Int>) -> Bool {
    let ys: List<Int> = twice<List<Int>>(List(a))
    let zs = List<Int>(1, 2)
    let _box: Box<Int> = Box { value: twice<Int>(a) }
    let lt = a <b
    let lt2 = a < b
    let lt3 = (a
        < b)
    return lt && lt2 && lt3 && ys.size() + zs.size() > 0
}

//--- a_space_before_type_arguments_is_refused
// Refused and read as the type arguments all the same.
import std.heap.Heap

//~ error[syntax-space:before-type-args]
struct Box <T> { value: T }

//~ error[syntax-space:before-type-args]
qualifier Tagged <T> for List<T>

//~ error[syntax-space:before-type-args]
effect Source <T> {
    fn next() -> T
}

//~ error[syntax-space:before-type-args]
fn twice <T>(x: T) -> T => !x {
    return x
}

//~ error[syntax-space:before-type-args]
fn heaped<T>(heap: Heap <T>(?cmp) Mut List<T>) -> None {
}

//~ error[syntax-space:before-type-args]
fn listed(xs: List <Int>) -> Int {
    return xs.size()
}

fn uses(a: Int, xs: List<Int>) -> Int {
    //~ error[syntax-space:before-type-args]
    let b = twice <Int>(a)
    //~ error[syntax-space:before-type-args]
    let ys = List <Int>(1, 2)
    // A line break outside brackets is refused in a type too.
    let zs: List
        //~ error[syntax-space:before-type-args]
        <Int> = List(1)
    let c = twice(
        twice
        //~ error[syntax-space:before-type-args]
        <Int>(a))
    return b + c + xs.size() + ys.size() + zs.size()
}

//--- a_space_before_a_comptime_fn_s_bound_is_refused
struct P { a: Int }

//~ error[syntax-space:before-type-args]
comptime fn f <T is Struct>(t: T) [] -> Int => t {
    return 0
}

//--- a_form_s_suffix_is_part_of_its_name
// `assert!`, `type?` and the other forms written with `!` or `?` are one
// name: a space inside one is refused as inside any name, and the form read
// all the same.
fn forms(a: Int) -> Int {
    //~ error[syntax-space:inside-name]
    assert !(a > 0)
    //~ error[syntax-space:inside-name]
    //~ type[Int]
    type ?(a)
    //~ error[syntax-space:inside-name]
    type !(a: Int)
    //~ error[syntax-space:inside-name]
    let _w = whynot ?(a is InRange(0, 5))
    if a > 9 {
        //~ error[syntax-space:inside-name]
        //~ warning[assert-fn:todo]
        todo !()
    }
    //~ error[syntax-space:inside-name]
    unreachable !()
}

//--- a_space_inside_refuse_is_refused
struct P { a: Int }

comptime fn f(p: P) [] -> Int => p {
    [for f in P.fields] {
        //~ error[syntax-space:inside-name]
        //~ error[comptime-refuse:refused]
        refuse !("${f.name}")
    }
    return 0
}

//--- a_qualifier_s_arguments_are_counted_in_a_type
// As an `is` test counts them: every value slot filled, and type arguments
// all or none.
import std.heap.Heap

//~ error[qual-depend:unfilled]
fn short(n: InRange(1) Int) -> None {
}

//~ error[qual-depend:unfilled]
fn long(n: InRange(1, 2, 3) Int) -> None {
}

//~ error[qual-depend:unfilled]
fn bare(n: InRange Int) -> None {
}

//~ error[qual-value-arg:partial-type-args]
fn too_many_types(xs: NonEmpty<Int, Str> List<Int>) -> None {
}

//~ error[qual-depend:no-value-args]
fn listed(xs: List<Int>(1)) -> None {
}

qualifier Pos for Int

//~ error[qual-value-arg:no-type-args]
fn positive(n: Pos<Int> Int) -> None {
}

//~ error[qual-depend:no-value-args]
fn generic<T>(t: T(1)) -> None {
}

// An unknown name is reported as that alone.
//~ error[name-resolve:unknown-type]
fn unknown(h: Hep<Int>(1)) -> None {
}

// A fn slot may be left out (it is resolved by its name), but not overfilled.
fn heaped<T>(a: Heap<T> Mut List<T>, b: Heap<T>(?cmp) Mut List<T>) -> None {
}

fn overfilled(
    //~ error[cmp-binder:too-many]
    a: Heap<Int>(cmp, cmp) Mut List<Int>,
    //~ error[cmp-binder:unknown-slot]
    b: Heap<Int>(?other) Mut List<Int>,
) -> None {
}

//--- a_space_inside_a_gated_mint_is_refused
actor effect Counter {
    send fn total(out: Reply<Int>) => !out
}

send fn finish(out: Reply<Int>, total: Int) => !out {
    out.send(total)
}

fn fetch(out: Reply<Int>) [Counter] -> None => !out {
    //~ error[syntax-space:inside-name]
    //~ error[task-mint:gated-free-send]
    total(replyto ! finish(out))
}

//--- an_operator_written_whole
// `?:`, `?.`, `?>`, `!is`, `as!`, `as?` and a type's `[]` are each one
// operator the parser reads from two tokens: written whole they check, with
// any spacing around them, and a tier-1 one may lead a line.
struct P { f: Int }

fn g(x: Int) -> Int {
    return x
}

fn first<T>(x: T) -> T => !x {
    return x
}

fn filled<T>(list: List<T>) -> list is NonEmpty => list {
    return list is NonEmpty
}

fn empty<T>(list: List<T>) -> list !is NonEmpty => list {
    return list !is NonEmpty
}

qualifier Bare<T> for List<T> {
    refn is_empty<T>(list: List<T>) -> list !is Bare
}

comptime fn kinds(p: P) [] -> Int => p {
    [for f in P.fields] {
        [if f.type !is Str] {}
    }
    return 0
}

fn whole(a: Int?, p: P?, n: Int, xs: Int[]) -> Int {
    let b = a ?: 0
    //~ type[Int]
    type?(b)
    let c = a?:0
    let d = p?.f
    //~ type[Int?]
    type?(d)
    let e = a ?> g(_)
    //~ type[Int?]
    type?(e)
    let _t = n !is InRange(0, 5)
    //~ type[Bool]
    type?(_t)
    let _q = n as? InRange(0, 9)
    let h = a
        ?: 0
    let k = a
        ?> g(_)
    return b + c + (d ?: 0) + (e ?: 0) + h + (k ?: 0) + size(xs)
}

// The same tokens apart, where the parser reads no operator from them: a
// type's `?` before a generic list's `>`, a `!` before a group, an effect
// list with nothing in it.
fn apart(n: Int) [ ] -> Bool {
    let _a: List<Int?>? = None
    let _b = first<Int?>(None)
    //~ type[Int?]
    type?(_b)
    return !(n is InRange(0, 5))
}

//--- a_space_inside_an_operator_is_refused
// A space or a line break inside one is refused as inside a name, and the
// operator read all the same, so the slip is reported once.
struct P { f: Int }

fn g(x: Int) -> Int {
    return x
}

//~ error[syntax-space:inside-operator]
fn empty<T>(list: List<T>) -> list ! is NonEmpty => list {
    return list !is NonEmpty
}

qualifier Full<T> for List<T> {
    //~ error[syntax-space:inside-operator]
    refn is_empty<T>(list: List<T>) -> list ! is Full
}

comptime fn kinds(p: P) [] -> Int => p {
    [for f in P.fields] {
        //~ error[syntax-space:inside-operator]
        [if f.type ! is Str] {}
    }
    return 0
}

//~ error[syntax-space:inside-operator]
fn spaced(a: Int?, p: P?, n: Int, _xs: Int[ ]) -> Int {
    //~ error[syntax-space:inside-operator]
    let b = a ? : 0
    //~ type[Int]
    type?(b)
    //~ error[syntax-space:inside-operator]
    let d = p ? .f
    //~ type[Int?]
    type?(d)
    //~ error[syntax-space:inside-operator]
    let e = a ? > g(_)
    //~ type[Int?]
    type?(e)
    //~ error[syntax-space:inside-operator]
    let _t = n ! is InRange(0, 5)
    //~ type[Bool]
    type?(_t)
    //~ error[syntax-space:inside-operator]
    let _r = n as ! InRange(0, 9)
    //~ error[syntax-space:inside-operator]
    let _o = n as ? InRange(0, 9)
    // A line break is refused as a space is, inside brackets and out.
    let c = (a ?
        //~ error[syntax-space:inside-operator]
        : 0)
    let h = a ?
        //~ error[syntax-space:inside-operator]
        : 0
    let i = (p ?
        //~ error[syntax-space:inside-operator]
        .f)
    let j = (a ?
        //~ error[syntax-space:inside-operator]
        > g(_))
    let _f = (n !
        //~ error[syntax-space:inside-operator]
        is InRange(0, 5))
    let _h = (n as
        //~ error[syntax-space:inside-operator]
        ! InRange(0, 9))
    let l = a ?
        //~ error[syntax-space:inside-operator]
        > g(_)
    let m = p ?
        //~ error[syntax-space:inside-operator]
        .f
    let _z = n !
        //~ error[syntax-space:inside-operator]
        is InRange(0, 5)
    // A tier-3 operator leading its line is its own slip.
    let k = n
        //~ error[expr-wrap:tier3-leading]
        //~ error[syntax-space:inside-operator]
        ! is InRange(0, 5)
    return b + c + h + (d ?: 0) + (e ?: 0) + (i ?: 0) + (j ?: 0) + k + (l ?: 0) + (m ?: 0)
}

fn broken(
    _xs: Int[
        //~ error[syntax-space:inside-operator]
        ],
) -> None {
}
