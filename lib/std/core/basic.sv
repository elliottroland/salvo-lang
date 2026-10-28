export intrinsic type Byte : Ordered<self>, Eq<self>, Hashed<self>, ToStr<self>
export intrinsic type Int : Ordered<self>, Eq<self>, Hashed<self>, ToStr<self>
export intrinsic type Long : Ordered<self>, Eq<self>, Hashed<self>, ToStr<self>
export intrinsic type Float : Eq<self>, ToStr<self>
export intrinsic type Double : Eq<self>, ToStr<self>
export intrinsic type Char : Ordered<self>, Eq<self>, Hashed<self>, ToStr<self>
export intrinsic type Bool : Ordered<self>, Eq<self>, Hashed<self>, ToStr<self>
export intrinsic type Any
export intrinsic type Never

export type Number = Int | Long | Double | Float

// The explicit numeric conversions the operators point at
// where they refuse to mix classes (: `Int + Double` is an
// error naming `to_double`). Truncating conversions truncate toward zero
// and saturate at the target's bounds, identically on both backends;
// `to_int(Long)` keeps the low 32 bits (both backends' `toInt()`/`as i32`
// semantics).
// @compilerrules [op-convert, op-arith]
export intrinsic fn to_int(value: Long) [] -> Int => value
export intrinsic fn to_int(value: Double) [] -> Int => value
export intrinsic fn to_int(value: Float) [] -> Int => value
export intrinsic fn to_long(value: Int) [] -> Long => value
export intrinsic fn to_long(value: Double) [] -> Long => value
export intrinsic fn to_long(value: Float) [] -> Long => value
export intrinsic fn to_double(value: Int) [] -> Double => value
export intrinsic fn to_double(value: Long) [] -> Double => value
export intrinsic fn to_double(value: Float) [] -> Double => value
export intrinsic fn to_float(value: Int) [] -> Float => value
export intrinsic fn to_float(value: Long) [] -> Float => value
export intrinsic fn to_float(value: Double) [] -> Float => value

// Bitwise operations on the integer types, as functions rather
// than operators (user decision 2026-10-05: `&`, `|`, `^` and `>>` already
// mean other things in the grammar). A shift count is taken modulo the width
// (0..31 for an `Int`, 0..63 for a `Long`), as the JVM does, so both
// backends agree for every count; `bit_shr` keeps the sign, `bit_ushr` fills
// with zeros.
// @compilerrules [op-bits]
export intrinsic fn bit_and(a: Int, b: Int) [] -> Int => a, b
export intrinsic fn bit_and(a: Long, b: Long) [] -> Long => a, b
export intrinsic fn bit_or(a: Int, b: Int) [] -> Int => a, b
export intrinsic fn bit_or(a: Long, b: Long) [] -> Long => a, b
export intrinsic fn bit_xor(a: Int, b: Int) [] -> Int => a, b
export intrinsic fn bit_xor(a: Long, b: Long) [] -> Long => a, b
export intrinsic fn bit_not(a: Int) [] -> Int => a
export intrinsic fn bit_not(a: Long) [] -> Long => a
export intrinsic fn bit_shl(value: Int, count: Int) [] -> Int => value, count
export intrinsic fn bit_shl(value: Long, count: Int) [] -> Long => value, count
export intrinsic fn bit_shr(value: Int, count: Int) [] -> Int => value, count
export intrinsic fn bit_shr(value: Long, count: Int) [] -> Long => value, count
export intrinsic fn bit_ushr(value: Int, count: Int) [] -> Int => value, count
export intrinsic fn bit_ushr(value: Long, count: Int) [] -> Long => value, count

// A `Byte` is an **unsigned** 0..255 octet on both backends, and
// it is not operator-numeric: these two conversions are the whole of its
// arithmetic surface, so a byte is turned into an `Int`, computed with, and
// turned back. `to_byte` keeps the low 8 bits (300 is 44, -1 is 255), which
// is what both backends' truncation does.
// @compilerrules [type-byte]
export intrinsic fn to_byte(value: Int) [] -> Byte => value
export intrinsic fn to_int(value: Byte) [] -> Int => value

// Duplicates a value: the argument is kept
// untouched (with all its qualifiers) and the result is a fresh,
// independent value with no fate links to the source. Implemented by
// each backend directly (Kotlin: identity for transitively immutable
// types, a real copy for Mut-capable ones; Rust: `.clone()`).
// @compilerrules [platform-intrinsic-fn, copy-fn]
export intrinsic fn copy<T>(value: proj T) [] -> T => value

// What **linear** means: a type declares
// itself with the `linear struct` modifier, and its obligation — use the
// value exactly once, ending in a discharge — is discharged by any fn
// declared in the *type's own file* that consumes a parameter of it (its
// **dischargers**: `close` for a file, `stop`/`join` for a thread,
// `remove(cache, handle)` for a pooled handle). A `linear struct` with no
// discharger is an error at the struct: the obligation would have no
// legal death.
//
// `discard` is the obligation's **terminal**: inside a discharger — and
// only there — `discard(value)` ends the obligation. Everywhere else,
// dropping a handle is exactly the leak linearity exists to prevent, and
// a discharger's own body must terminate the obligation on every path
// (a `discard`, or a forward into another consuming fn).
// @compilerrules [linear-group, linear-discard]
export intrinsic fn discard<T canbe linear>(value: T) [] -> None => !value

// The obligation "this type has a text form". Declaring
// `: ToStr<self>` on a type is a *convenience*: it does not enable
// interpolation — a `to_str` in scope does that, resolved at the
// interpolation site — but it validates at the declaration that one exists,
// which is where the mistake is easier to see (user decision 2026-09-11).
// @compilerrules [interp-to-str]
export fns ToStr<T> {
    fn to_str(value: T) -> Str
}

// The text form of the scalars, as *functions*.
//
// Interpolation renders these natively and needs none of them; a **`?ToStr<T>`
// implicit** does, because resolution looks for a function
// — so without these, `expect_eq(count, 3)` could not
// render an `Int` (added 2026-09-23 with the test surface, user decision: std
// grows what the framework needs).
// @compilerrules [interp-to-str, implicit-resolve]
export intrinsic fn to_str(value: Int) [] -> Str => value
export intrinsic fn to_str(value: Long) [] -> Str => value
export intrinsic fn to_str(value: Byte) [] -> Str => value
export intrinsic fn to_str(value: Char) [] -> Str => value
export intrinsic fn to_str(value: Bool) [] -> Str => value
// A float's text is Salvo's rule, the shortest digits that round-trip with a
// decimal point always present.
// @compilerrules [interp-float]
export intrinsic fn to_str(value: Double) [] -> Str => value
export intrinsic fn to_str(value: Float) [] -> Str => value

// A tuple's text form: `(1, 2.5)`, each
// part by its own `to_str` (ROADMAP §0j step 8).
// @compilerrules [interp-to-str, implicit-recursive]
export fn to_str<A, B>(value: (A, B), ?to_str: (x: A) -> Str, ?to_str: (x: B) -> Str) [] -> Str => value {
    return "(${to_str(value.0)}, ${to_str(value.1)})"
}

export fn to_str<A, B, C>(value: (A, B, C), ?to_str: (x: A) -> Str, ?to_str: (x: B) -> Str, ?to_str: (x: C) -> Str) [] -> Str => value {
    return "(${to_str(value.0)}, ${to_str(value.1)}, ${to_str(value.2)})"
}

// ===== ordering and equality: the operators `<` and `==` =====
//
// What `cmp` and `eq` at a scalar resolve to — so
// what `a < b` (`cmp(a, b) < 0`) and `a == b` (`eq(a, b)`) mean on one —
// declared here, beside the types, because an operator on a type is declared
// in the type's own file (user decision O1-D, 2026-10-27; `std.core.compare`
// declares the groups). `intrinsic` for the same reason every other primitive
// operation is: the comparison is the host's, and each backend lowers it.
// `Str`'s are in `std.core.string`, `List`'s in
// `std.core.list`.
// @compilerrules [op-unique-shape, cmp-groups, backend-intrinsic]
export intrinsic fn cmp(a: Int, b: Int) [] -> Int => a, b
export intrinsic fn cmp(a: Long, b: Long) [] -> Int => a, b
export intrinsic fn cmp(a: Byte, b: Byte) [] -> Int => a, b
export intrinsic fn cmp(a: Char, b: Char) [] -> Int => a, b
export intrinsic fn cmp(a: Bool, b: Bool) [] -> Int => a, b

// Equality covers the float widths where ordering cannot: comparing two
// `Double`s is meaningful (and both backends agree on IEEE, `NaN` equalling
// nothing), ordering a set of them is not.
export intrinsic fn eq(a: Int, b: Int) [] -> Bool => a, b
export intrinsic fn eq(a: Long, b: Long) [] -> Bool => a, b
export intrinsic fn eq(a: Double, b: Double) [] -> Bool => a, b
export intrinsic fn eq(a: Float, b: Float) [] -> Bool => a, b
export intrinsic fn eq(a: Byte, b: Byte) [] -> Bool => a, b
export intrinsic fn eq(a: Char, b: Char) [] -> Bool => a, b
export intrinsic fn eq(a: Bool, b: Bool) [] -> Bool => a, b

// ===== arithmetic: the operators `+ - * /` and `**` =====
//
// What `a + b` (`plus(a, b)`), `a - b` (`minus`),
// `a * b` (`times`) and `a / b` (`divide`) mean on a number, declared here,
// beside the types, as the comparisons above are (OPERATOR_OVERLOAD.md O1).
// The backends write their own operator for these: integer
// arithmetic wraps, `Int / Int` divides toward zero, and division
// by zero is the host's trap. Mixed widths widen before the call,
// so only the same-width overloads exist. `Byte` has none: it
// is an octet, not a number. `%` names no function.
// @compilerrules [op-arith, op-unique-shape, backend-core-layers, op-wrap, op-promote, type-byte]
export intrinsic fn plus(a: Int, b: Int) [] -> Int => a, b
export intrinsic fn plus(a: Long, b: Long) [] -> Long => a, b
export intrinsic fn plus(a: Float, b: Float) [] -> Float => a, b
export intrinsic fn plus(a: Double, b: Double) [] -> Double => a, b
export intrinsic fn minus(a: Int, b: Int) [] -> Int => a, b
export intrinsic fn minus(a: Long, b: Long) [] -> Long => a, b
export intrinsic fn minus(a: Float, b: Float) [] -> Float => a, b
export intrinsic fn minus(a: Double, b: Double) [] -> Double => a, b
export intrinsic fn times(a: Int, b: Int) [] -> Int => a, b
export intrinsic fn times(a: Long, b: Long) [] -> Long => a, b
export intrinsic fn times(a: Float, b: Float) [] -> Float => a, b
export intrinsic fn times(a: Double, b: Double) [] -> Double => a, b
export intrinsic fn divide(a: Int, b: Int) [] -> Int => a, b
export intrinsic fn divide(a: Long, b: Long) [] -> Long => a, b
export intrinsic fn divide(a: Float, b: Float) [] -> Float => a, b
export intrinsic fn divide(a: Double, b: Double) [] -> Double => a, b

// `a ** b` is `pow(a, b)`, at the written types (no widening: an
// exponent is not a base). An integer power wraps like the other integer
// arithmetic, and a **negative exponent traps** (the host's
// arithmetic exception, as a division by zero does; PIPELINE.md §II.10 P17).
// A `Double` to an `Int` power is repeated squaring, the same multiplications
// in the same order on both backends, so the same bits (a negative exponent
// is the reciprocal); a `Double` to a `Double` power is the host's `pow`.
// `Float` has none.
// @compilerrules [op-pow, op-wrap]
export intrinsic fn pow(base: Int, exponent: Int) [] -> Int => base, exponent
export intrinsic fn pow(base: Long, exponent: Int) [] -> Long => base, exponent
export intrinsic fn pow(base: Double, exponent: Int) [] -> Double => base, exponent
export intrinsic fn pow(base: Double, exponent: Double) [] -> Double => base, exponent

// A tuple compares and orders
// **element by element, through the elements' own identities**: a
// `(Person, Int)` compares by `Person`'s declared `eq`. Ordinary Salvo over the
// implicits of the element types (ROADMAP §0j step 8). Tuples are a
// structural type, and this file is their home: their `to_str`
// above, their `eq` and `cmp` here (their `hash`, not an operator, is in
// `std.core.compare`).
// @compilerrules [col-hashed-ordered, implicit-recursive, op-unique-shape]
export fn eq<A, B>(a: (A, B), b: (A, B), ?eq: (x: A, y: A) -> Bool, ?eq: (x: B, y: B) -> Bool) [] -> Bool => a, b {
    return eq(a.0, b.0) && eq(a.1, b.1)
}

export fn cmp<A, B>(a: (A, B), b: (A, B), ?cmp: (x: A, y: A) -> Int, ?cmp: (x: B, y: B) -> Int) [] -> Int => a, b {
    let c = cmp(a.0, b.0)
    if c != 0 {
        return c
    }
    return cmp(a.1, b.1)
}

export fn eq<A, B, C>(a: (A, B, C), b: (A, B, C), ?eq: (x: A, y: A) -> Bool, ?eq: (x: B, y: B) -> Bool, ?eq: (x: C, y: C) -> Bool) [] -> Bool => a, b {
    return eq(a.0, b.0) && eq(a.1, b.1) && eq(a.2, b.2)
}

export fn cmp<A, B, C>(a: (A, B, C), b: (A, B, C), ?cmp: (x: A, y: A) -> Int, ?cmp: (x: B, y: B) -> Int, ?cmp: (x: C, y: C) -> Int) [] -> Int => a, b {
    let c = cmp(a.0, b.0)
    if c != 0 {
        return c
    }
    let d = cmp(a.1, b.1)
    if d != 0 {
        return d
    }
    return cmp(a.2, b.2)
}
