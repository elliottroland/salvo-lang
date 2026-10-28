// [op-unique-shape] Shapes on std's types and on the structural types, from a
// program file: each is refused, and since std's file is not the program's
// to edit, the remedy is another name.

// D-2: the first explicit parameter's *base* type, qualifiers stripped: on
// `List` (module `std.core.list`) and on `Str` (`std.core.string`).
//~ error[op-unique-shape:outside-home]
fn eq<T>(a: NonEmpty List<T>, b: List<T>) [] -> Bool => a, b { return true }
//~ error[op-unique-shape:outside-home]
fn next(s: Mut Str) [] -> Emitted Char | Finished => s: Mut { return finished() }

// D-3: a name with another arity, or `eq`/`cmp`/`next` with another return,
// is an ordinary fn; implicit parameters are not counted. Only the last is
// refused: one explicit parameter, the implicit not counted.
struct Stepped canbe Mut { at: Int }
fn iter(list: List<Str>, step: Int) [] -> Mut Stepped => list, step { return Mut Stepped { at: step } }
fn eq(a: Int, b: Int) [] -> Int => a, b { return 0 }
fn cmp(a: Str, b: Str) [] -> Bool => a, b { return true }
fn next(n: Mut List<Int>) [] -> Int => n: Mut { return 0 }
fn to_str(a: Int, b: Int) [] -> Str => a, b { return "" }
fn plus(a: Int) [] -> Int => a { return a }
effect Counter {
    fn next(step: Int) -> Int => step
}
//~ error[op-unique-shape:outside-home]
fn to_str(x: Long, ?f: (x: Long) -> Str) [] -> Str => x { return f(x) }

// D-5: a bare type parameter has no home, so a shape on one is refused —
// the catch-all `fn to_str<T>(x: T)`.
//~ error[op-unique-shape:bare-type-param]
fn to_str<T>(x: T) [] -> Str => x { return "?" }
//~ error[op-unique-shape:bare-type-param]
fn eq<T>(a: T, b: T) [] -> Bool => a, b { return true }

// D-5: the structural types' homes in std — tuples, fn types and anonymous
// unions in `std.core.basic`, arrays in `std.core.array`, an `eff[E]` handle
// where `Addr` is declared (`std.core.actor`); a literal type is on its base
// (`Str`, `std.core.string`).
//~ error[op-unique-shape:outside-home]
fn eq<A, B>(a: (A, B), b: (A, B)) [] -> Bool => a, b { return true }
//~ error[op-unique-shape:outside-home]
fn to_str(f: (x: Int) -> Int) [] -> Str => f { return "fn" }
//~ error[op-unique-shape:outside-home]
fn to_str(x: Int | Str) [] -> Str => x { return "u" }
//~ error[op-unique-shape:outside-home]
fn to_str(x: Bool?) [] -> Str => x { return "o" }
//~ error[op-unique-shape:outside-home]
fn to_str(xs: Int[]) [] -> Str => xs { return "a" }
//~ error[op-unique-shape:outside-home]
fn to_str(c: eff[Console]) [] -> Str => c { return "h" }
//~ error[op-unique-shape:outside-home]
fn to_str(x: "A") [] -> Str => x { return "A" }

// O4: a declared union is its own home; a non-union alias names the type it
// expands to [type-alias], so its shapes live with that type (`List`).
struct A {}
struct B {}
type Shape = A | B
type Grid = List<Int>
fn to_str(s: Shape) [] -> Str => s { return "shape" }
//~ error[op-unique-shape:outside-home]
fn to_str(g: Grid) [] -> Str => g { return "grid" }
