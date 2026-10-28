// [col-no-literal] [fn-type-named] [col-by] [col-convert] The collection
// constructors — `List(…)`, `Mut Set(…)`, `Map("k": v)` — and the generated
// constructors and converters, as they type over std.

// [col-no-literal] [fn-type-named] The three constructor forms type as their
// collections, and `Mut` on the type name builds the mutable one.
fn the_constructors_type_as_their_collections() -> Int {
    let xs: List<Int> = List(1, 2)
    let s: Set<Str> = Set("a", "b")
    let m: Map<Str, Int> = Map("a": 1)
    let ms: Mut Set<Int> = Mut Set(1)
    let mm: Mut Map<Str, Int> = Mut Map("a": 1)
    let mxs: Mut List<Int> = Mut List(3)
    return size(xs) + size(s) + size(m) + size(ms) + size(mm) + size(mxs)
}

// [col-no-literal] A brace whose first entry is `identifier:` is still a bare
// **struct** literal — which is what makes a map key an expression rather
// than a name.
struct Point {
    x: Int,
    y: Int
}

fn an_identifier_key_is_still_a_struct_literal() -> Int {
    let p: Point = {x: 1, y: 2}
    return p.x
}

// [col-no-literal] A *named* fieldless struct literal keeps working: only a
// **bare** `{}` is an empty collection.
struct Done {}

fn a_named_fieldless_struct_literal() -> Done {
    return Done {}
}

// [col-no-literal] [fn-type-named] An empty constructor takes its element
// type from its type arguments or from the position — and says so when there
// is none. From the constructor's own type arguments, either kind:
fn from_type_arguments() -> Int {
    let s: Set<Int> = Set<Int>()
    let m: Map<Str, Int> = Map<Str, Int>()
    let xs: List<Int> = List<Int>()
    return size(s) + size(m) + size(xs)
}

// From a parameter type.
fn takes(s: Set<Int>) -> Int => s {
    return size(s)
}

fn from_a_parameter() -> Int {
    return takes(Set())
}

// With nothing to say what it is, an error naming the remedies.
fn bare_list() -> Int {
    //~ error[call-type-args:cannot-infer]
    let _x = List()
    return 0
}

fn bare_set() -> Int {
    //~ error[call-type-args:cannot-infer]
    let _x = Set()
    return 0
}

// [col-by] [col-convert] The generated constructors and the converters type
// as their collections, and the two `to_map` forms are told apart by arity.
fn generated_and_converted() -> Int {
    let xs = list_by(3, i -> i + 1)
    let s = set_by(3, i -> i)
    let m = map_by(2, i -> (i, i))
    let u = to_set(xs)
    let pairs = list_by(1, _i -> (1, 2))
    let m2 = to_map(pairs)
    let m3 = to_map(xs, x -> (x, x))
    return size(xs) + size(s) + size(m) + size(u) + size(m2) + size(m3)
}

// FILE: ineligible.sv
// [col-key-eligible] A constructor's key type is checked like any other:
// with no `hash` for `Double`, `Set(1.5)` and `Map(1.5: 1)` are refused — as
// a key, once: the key rule is checked before the constructor's implicits,
// and a key refused as such silences the `hash` it then cannot find
// (2026-10-20; until then the refusal was the implicit's "no `hash` fits").
// (A file of their own: a type refused as a key is reported once per file.)
fn set_of_doubles() -> Int {
    //~ error[col-key-eligible:key]
    let s = Set(1.5)
    return size(s)
}

// FILE: ineligible_map.sv
fn map_of_doubles() -> Int {
    //~ error[col-key-eligible:key]
    let m = Map(1.5: 1)
    return size(m)
}

//--- a_map_literal_wants_a_value_for_every_entry
// [col-pairs] A `Map(…)` call's entries all need values: pair arguments are
// all-or-nothing, so a bare key among them is a parse error.
fn probe() -> Int {
    //~ error[col-pairs:positional-after-pair]
    let _m = Map("a": 1, "b")
    return 0
}
