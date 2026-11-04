// The basic types (`Int`, `Long`, `Double`, `Float`, `Byte`, `Char` and
// `Bool`) and what can be done with them: arithmetic and comparison, which
// the operators call, the explicit conversions between them, the bitwise
// operations, and their text. Also `copy`, which duplicates any value, and
// the comparison and text of tuples.
//
// ```
// let count = 42                                       // Int
// let total = count + 8L                               // Long: 50
// let ratio = to_double(count) / 8.0                   // 5.25
// let half = 7 / 2                                     // 3: integer division
// let low = bit_and(255, 300)                          // 44
// println("${ratio} ${half} ${count > 40}")            // 5.25 3 true
// ```
//
// **Operators are calls.** `a + b` is `plus(a, b)`, `a - b` is `minus`,
// `a * b` `times`, `a / b` `divide`, `a % b` `rem`, `-a` `neg`, `a ** b`
// `pow`, `a < b` is `cmp(a, b) < 0` and `a == b` is `eq(a, b)`. The functions
// below are what those calls find for the basic types, and the backends turn
// them into the host's own operators, so they cost nothing. A type of your
// own gets an operator by declaring the function in its own file (see the
// Data and types topic).
//
// **Integer arithmetic wraps** on overflow, the same on both backends:
// `2147483647 + 1` is `-2147483648`. `/` on integers divides toward zero
// (`7 / -2` is `-3`), `%` keeps the sign of the dividend (`-7 % 2` is `-1`),
// and an integer division by zero stops the program.
//
// **Widths widen, classes do not mix.** An `Int` and a `Long` combine at
// `Long`, and a `Float` and a `Double` at `Double`, without a word. An
// integer and a float never combine implicitly: `1 + 2.5` is a compile-time
// error that names the conversion to write, [to_double] or [to_int].
//
// **A `Byte` is not a number.** It is an unsigned octet, 0 to 255, with no
// arithmetic of its own: convert it with [to_int], compute, and convert back
// with [to_byte].
// @compilerrules [op-arith, op-convert, op-wrap, op-promote, type-byte]

// A byte: an unsigned 8-bit value, 0 to 255. It has no arithmetic operators;
// [to_int] and [to_byte] convert it.
export intrinsic type Byte : Ordered<self>, Eq<self>, Hashed<self>, ToStr<self>
// A 32-bit signed integer, the type of a whole-number literal such as `42`.
export intrinsic type Int : Ordered<self>, Eq<self>, Hashed<self>, ToStr<self>
// A 64-bit signed integer, written with an `L` suffix: `42L`.
export intrinsic type Long : Ordered<self>, Eq<self>, Hashed<self>, ToStr<self>
// A 32-bit floating-point number, written with an `f` suffix: `1.5f`. It has
// no `hash`, so it cannot be a set element or a map key.
export intrinsic type Float : Ordered<self>, Eq<self>, ToStr<self>
// A 64-bit floating-point number, the type of a literal with a decimal point
// such as `1.5`. It has no `hash`, so it cannot be a set element or a map
// key.
export intrinsic type Double : Ordered<self>, Eq<self>, ToStr<self>
// A character: `'S'`.
export intrinsic type Char : Ordered<self>, Eq<self>, Hashed<self>, ToStr<self>
// `true` or `false`. `&&`, `||` and `!` take only `Bool`s.
export intrinsic type Bool : Ordered<self>, Eq<self>, Hashed<self>, ToStr<self>
// The top type: every value is an `Any`.
// @compilerrules [type-any-never]
export intrinsic type Any
// The bottom type, which has no values: the type of an expression that
// never finishes, such as `return`, `throw(…)` or `exit(…)`. It fits
// wherever a value is expected, and the code after it does not run.
// @compilerrules [type-any-never]
export intrinsic type Never

// Any of the four numeric types.
export type Number = Int | Long | Double | Float

// Converts a number to an `Int`. This and [to_long], [to_double] and
// [to_float] are the explicit conversions the operators ask for where they
// refuse to mix an integer with a float (`Int + Double` is an error naming
// `to_double`).
//
// From a float, the value is truncated toward zero and saturates at the
// `Int` bounds: `to_int(2.9)` is `2`, `to_int(-2.9)` is `-2`, and a `Double`
// too large for an `Int` gives `2147483647`. From a `Long`, it keeps the low
// 32 bits: `to_int(3000000000L)` is `-1294967296`. Every conversion gives
// the same answer on both backends.
//
// ```
// let n = 7
// let avg = to_double(n) / 2.0                         // 3.5
// let whole = to_int(avg)                              // 3
// let big = to_long(n) * 1000000000L                   // 7000000000
// ```
// @compilerrules [op-convert, op-arith]
export intrinsic fn to_int(value: Long) [] -> Int => value
export intrinsic fn to_int(value: Double) [] -> Int => value
export intrinsic fn to_int(value: Float) [] -> Int => value
// Converts a number to a `Long`. From a float, the value is truncated toward
// zero and saturates at the `Long` bounds, as for [to_int].
export intrinsic fn to_long(value: Int) [] -> Long => value
export intrinsic fn to_long(value: Double) [] -> Long => value
export intrinsic fn to_long(value: Float) [] -> Long => value
// Converts a number to a `Double`.
export intrinsic fn to_double(value: Int) [] -> Double => value
export intrinsic fn to_double(value: Long) [] -> Double => value
export intrinsic fn to_double(value: Float) [] -> Double => value
// Converts a number to a `Float`.
export intrinsic fn to_float(value: Int) [] -> Float => value
export intrinsic fn to_float(value: Long) [] -> Float => value
export intrinsic fn to_float(value: Double) [] -> Float => value

// The bits set in both operands: bitwise and.
//
// The bitwise operations on `Int` and `Long` are functions, since `&`, `|`,
// `^` and `>>` already mean other things in Salvo's grammar: [bit_and],
// [bit_or], [bit_xor], [bit_not], and the shifts [bit_shl], [bit_shr] and
// [bit_ushr].
//
// ```
// let flags = bit_or(1, 4)                             // 5
// let has_four = bit_and(flags, 4) != 0                // true
// let cleared = bit_and(flags, bit_not(4))             // 1
// let shifted = bit_shl(1, 10)                         // 1024
// let sign = bit_shr(-8, 1)                            // -4
// let zeros = bit_ushr(-8, 28)                         // 15
// ```
// @compilerrules [op-bits]
export intrinsic fn bit_and(a: Int, b: Int) [] -> Int => a, b
export intrinsic fn bit_and(a: Long, b: Long) [] -> Long => a, b
// The bits set in either operand: bitwise or.
export intrinsic fn bit_or(a: Int, b: Int) [] -> Int => a, b
export intrinsic fn bit_or(a: Long, b: Long) [] -> Long => a, b
// The bits set in exactly one operand: bitwise exclusive or.
export intrinsic fn bit_xor(a: Int, b: Int) [] -> Int => a, b
export intrinsic fn bit_xor(a: Long, b: Long) [] -> Long => a, b
// Every bit flipped: bitwise complement.
export intrinsic fn bit_not(a: Int) [] -> Int => a
export intrinsic fn bit_not(a: Long) [] -> Long => a
// Shifts the bits left by [count] places, filling with zeros.
//
// The count is taken modulo the width (0 to 31 for an `Int`, 0 to 63 for a
// `Long`), so `bit_shl(1, 33)` is `2`, the same on both backends; so for
// [bit_shr] and [bit_ushr].
export intrinsic fn bit_shl(value: Int, count: Int) [] -> Int => value, count
export intrinsic fn bit_shl(value: Long, count: Int) [] -> Long => value, count
// Shifts the bits right by [count] places, keeping the sign: the top bit is
// copied into the places vacated, so `bit_shr(-8, 1)` is `-4`.
export intrinsic fn bit_shr(value: Int, count: Int) [] -> Int => value, count
export intrinsic fn bit_shr(value: Long, count: Int) [] -> Long => value, count
// Shifts the bits right by [count] places, filling with zeros, so a negative
// value becomes a positive one: `bit_ushr(-8, 28)` is `15`.
export intrinsic fn bit_ushr(value: Int, count: Int) [] -> Int => value, count
export intrinsic fn bit_ushr(value: Long, count: Int) [] -> Long => value, count

// Converts an `Int` to a `Byte`, keeping its low 8 bits: `to_byte(300)` is
// `44` and `to_byte(-1)` is `255`.
//
// This and `to_int` of a byte are all of a byte's arithmetic: turn it into
// an `Int`, compute, and turn it back.
//
// ```
// let b = to_byte(200)
// let next = to_byte(to_int(b) + 1)                    // 201
// let wrapped = to_int(to_byte(to_int(b) + 100))       // 44
// ```
// @compilerrules [type-byte]
export intrinsic fn to_byte(value: Int) [] -> Byte => value
// [to_int] of a `Byte`: its value, 0 to 255.
export intrinsic fn to_int(value: Byte) [] -> Int => value

// Duplicates a value. The argument is left as it was, and the answer is a
// new, independent value: changing one never changes the other.
//
// Salvo moves values rather than sharing them, so `copy` is how a program
// keeps a value and also hands it on, or stores an element it was only
// lent. A linear value (one that must be used exactly once) cannot be
// copied.
//
// ```
// let a = Mut List(1, 2)
// let b = copy(a)
// add(b, 3)
// println("${a} ${b}")                                 // [1, 2] [1, 2, 3]
// ```
// @compilerrules [platform-intrinsic-fn, copy-fn]
export intrinsic fn copy<T>(value: proj T) [] -> T => value

// What linear means, for the declarations that use it: a type declares
// itself with the `linear struct` modifier, and its obligation (use the
// value exactly once, ending in a discharge) is discharged by any fn
// declared in the type's own file that consumes a parameter of it (its
// dischargers: `close` for a file, `stop`/`join` for a thread,
// `remove(cache, handle)` for a pooled handle). A `linear struct` with no
// discharger is an error at the struct: the obligation would have no
// legal death.

// The promise that a type has a text form: a `to_str` taking it.
//
// Declaring `: ToStr<self>` on a type checks, at the declaration, that a
// `to_str` for it exists, so a missing one is reported there rather than
// where a value is first printed. It does not enable interpolation:
// `"${value}"` calls whatever `to_str` is in scope, declared or not.
// `: ToStr<self> by auto` writes one that prints a struct in its literal's
// shape.
//
// A generic function that needs to print a `T` asks for `?ToStr<T>`.
// @compilerrules [interp-to-str]
// @compilerknown
export fns ToStr<T> {
    // The value's text.
    fn to_str(value: T) -> Str
}

// The text of a basic value, as interpolation writes it: `to_str(42)` is
// `"42"`, `to_str(true)` is `"true"`, `to_str('a')` is `"a"`.
//
// Interpolation renders the basic types itself and does not call these; they
// exist so that a function asking for a `to_str` (a `?ToStr<T>` or a `?to_str`
// parameter) can be given one for an `Int`, as a test's `expect_eq(count, 3)`
// is when it prints a mismatch.
// @compilerrules [interp-to-str, implicit-resolve]
export intrinsic fn to_str(value: Int) [] -> Str => value
export intrinsic fn to_str(value: Long) [] -> Str => value
export intrinsic fn to_str(value: Byte) [] -> Str => value
export intrinsic fn to_str(value: Char) [] -> Str => value
export intrinsic fn to_str(value: Bool) [] -> Str => value
// The text of a float, the same on both backends: the shortest digits that
// read back as the same number, always with a decimal point, and
// scientific notation outside `[0.001, 10000000)`. So `2.0` is `"2.0"`,
// `0.0001` is `"1.0E-4"`, and the special values are `NaN`, `Infinity` and
// `-Infinity`.
// @compilerrules [interp-float]
export intrinsic fn to_str(value: Double) [] -> Str => value
export intrinsic fn to_str(value: Float) [] -> Str => value

// The text of a pair, each part by its own `to_str`: `(1, 2.5)`.
//
// @param to_str  found at the call site: the text of each part
// @compilerrules [interp-to-str, implicit-recursive]
export fn to_str<A, B>(value: (A, B), ?to_str: (x: A) -> Str, ?to_str: (x: B) -> Str) [] -> Str => value {
    return "(${to_str(value.0)}, ${to_str(value.1)})"
}

// The text of a triple, each part by its own `to_str`: `(1, "a", true)` is
// `(1, a, true)`.
export fn to_str<A, B, C>(value: (A, B, C), ?to_str: (x: A) -> Str, ?to_str: (x: B) -> Str, ?to_str: (x: C) -> Str) [] -> Str => value {
    return "(${to_str(value.0)}, ${to_str(value.1)}, ${to_str(value.2)})"
}

// ===== ordering and equality: the operators `<` and `==` =====
//
// What `a < b` (`cmp(a, b) < 0`) and `a == b` (`eq(a, b)`) call on the basic
// types. An operator on a type is declared in the type's own file, so these
// are here, beside the types; [std.core.compare] declares the groups and the
// hashes, `Str`'s are in [std.core.string], and `List`'s in [std.core.list].
// @compilerrules [op-unique-shape, cmp-groups, backend-intrinsic]

// Compares two basic values: negative when `a` is less, zero when they are
// equal, positive when `a` is greater. `false` sorts before `true`, and
// characters by their code point.
//
// ```
// let order = cmp(3, 5)                                // negative
// let same = cmp('a', 'a')                             // 0
// ```
export intrinsic fn cmp(a: Int, b: Int) [] -> Int => a, b
export intrinsic fn cmp(a: Long, b: Long) [] -> Int => a, b
export intrinsic fn cmp(a: Byte, b: Byte) [] -> Int => a, b
export intrinsic fn cmp(a: Char, b: Char) [] -> Int => a, b
export intrinsic fn cmp(a: Bool, b: Bool) [] -> Int => a, b

// Compares two floats by IEEE comparison: `-1` when `a < b`, `1` when
// `a > b`, and `0` otherwise, so a `NaN` ties with everything.
//
// That is not a total order, which is why a float cannot be the element of a
// sorted collection.
// @compilerrules [op-order, col-sorted]
export intrinsic fn cmp(a: Float, b: Float) [] -> Int => a, b
export intrinsic fn cmp(a: Double, b: Double) [] -> Int => a, b

// Whether two basic values are equal. For the floats this is IEEE equality,
// the same on both backends: `NaN` equals nothing, itself included.
export intrinsic fn eq(a: Int, b: Int) [] -> Bool => a, b
export intrinsic fn eq(a: Long, b: Long) [] -> Bool => a, b
export intrinsic fn eq(a: Double, b: Double) [] -> Bool => a, b
export intrinsic fn eq(a: Float, b: Float) [] -> Bool => a, b
export intrinsic fn eq(a: Byte, b: Byte) [] -> Bool => a, b
export intrinsic fn eq(a: Char, b: Char) [] -> Bool => a, b
export intrinsic fn eq(a: Bool, b: Bool) [] -> Bool => a, b

// ===== arithmetic: the operators `+ - * / %`, unary `-`, and `**` =====
//
// What `a + b` (`plus(a, b)`), `a - b` (`minus`), `a * b` (`times`),
// `a / b` (`divide`), `a % b` (`rem`) and `-a` (`neg`) call on a number.
// @compilerrules [op-arith, op-unique-shape, backend-core-layers, op-wrap, op-promote, type-byte]

// Adds two numbers: what `a + b` calls.
//
// `plus`, [minus], [times], [divide], [rem] and [neg] are the arithmetic of
// the numeric types. Integer arithmetic wraps on overflow, `Int / Int`
// divides toward zero, `%` keeps the dividend's sign, and an integer
// division or remainder by zero stops the program. Operands of different
// widths are widened before the call, so only same-width overloads exist.
// `Byte` has none: it is an octet, not a number.
//
// ```
// let wrapped = 2147483647 + 1                         // -2147483648
// let q = -7 / 2                                       // -3
// let r = -7 % 2                                       // -1
// let x = 1.5 * 4.0                                    // 6.0
// ```
export intrinsic fn plus(a: Int, b: Int) [] -> Int => a, b
export intrinsic fn plus(a: Long, b: Long) [] -> Long => a, b
export intrinsic fn plus(a: Float, b: Float) [] -> Float => a, b
export intrinsic fn plus(a: Double, b: Double) [] -> Double => a, b
// Subtracts [b] from [a]: what `a - b` calls. Integers wrap on overflow.
export intrinsic fn minus(a: Int, b: Int) [] -> Int => a, b
export intrinsic fn minus(a: Long, b: Long) [] -> Long => a, b
export intrinsic fn minus(a: Float, b: Float) [] -> Float => a, b
export intrinsic fn minus(a: Double, b: Double) [] -> Double => a, b
// Multiplies two numbers: what `a * b` calls. Integers wrap on overflow.
export intrinsic fn times(a: Int, b: Int) [] -> Int => a, b
export intrinsic fn times(a: Long, b: Long) [] -> Long => a, b
export intrinsic fn times(a: Float, b: Float) [] -> Float => a, b
export intrinsic fn times(a: Double, b: Double) [] -> Double => a, b
// Divides [a] by [b]: what `a / b` calls. Integer division truncates toward
// zero (`7 / -2` is `-3`), and an integer division by zero stops the
// program.
export intrinsic fn divide(a: Int, b: Int) [] -> Int => a, b
export intrinsic fn divide(a: Long, b: Long) [] -> Long => a, b
export intrinsic fn divide(a: Float, b: Float) [] -> Float => a, b
export intrinsic fn divide(a: Double, b: Double) [] -> Double => a, b
// The remainder of dividing [a] by [b]: what `a % b` calls. It has the sign
// of [a] (`-7 % 2` is `-1`), and an integer remainder by zero stops the
// program.
export intrinsic fn rem(a: Int, b: Int) [] -> Int => a, b
export intrinsic fn rem(a: Long, b: Long) [] -> Long => a, b
export intrinsic fn rem(a: Float, b: Float) [] -> Float => a, b
export intrinsic fn rem(a: Double, b: Double) [] -> Double => a, b
// The negation of a number: what unary `-a` calls. Integers wrap, so the
// negation of the smallest `Int` is itself.
export intrinsic fn neg(a: Int) [] -> Int => a
export intrinsic fn neg(a: Long) [] -> Long => a
export intrinsic fn neg(a: Float) [] -> Float => a
export intrinsic fn neg(a: Double) [] -> Double => a

// Raises a number to a power: what `a ** b` calls.
//
// The operands keep the types they are written with: an exponent is not
// widened to match its base. An integer power wraps like the rest of integer
// arithmetic, and a **negative integer exponent stops the program**, as a
// division by zero does. `Double ** Int` is computed by repeated
// multiplication, the same multiplications in the same order on both
// backends, so it gives the same bits everywhere (a negative exponent gives
// the reciprocal); `Double ** Double` is the host's `pow`. `Float` has no
// power.
//
// ```
// let k = 2 ** 10                                      // 1024
// let half = 2.0 ** -1                                 // 0.5
// let root = 2.0 ** 0.5                                // 1.4142135623730951
// ```
// @compilerrules [op-pow, op-wrap]
export intrinsic fn pow(base: Int, exponent: Int) [] -> Int => base, exponent
export intrinsic fn pow(base: Long, exponent: Int) [] -> Long => base, exponent
export intrinsic fn pow(base: Double, exponent: Int) [] -> Double => base, exponent
export intrinsic fn pow(base: Double, exponent: Double) [] -> Double => base, exponent

// Whether two pairs are equal: both parts equal, each by its own type's
// `eq`, so a `(Person, Int)` compares by `Person`'s declared `eq`.
//
// Tuples compare, order and hash **part by part, through the parts' own
// functions**. A tuple has no declaration of its own, so its `to_str`, `eq`
// and `cmp` are here; its `hash` is in [std.core.compare].
//
// ```
// let same = (1, "a") == (1, "a")                      // true
// let less = (1, "b") < (2, "a")                       // true: the first part decides
// ```
//
// @param eq  found at the call site: the equality of each part
// @compilerrules [col-hashed-ordered, implicit-recursive, op-unique-shape]
export fn eq<A, B>(a: (A, B), b: (A, B), ?eq: (x: A, y: A) -> Bool, ?eq: (x: B, y: B) -> Bool) [] -> Bool => a, b {
    return eq(a.0, b.0) && eq(a.1, b.1)
}

// Orders two pairs lexicographically: by the first part, and by the second
// when the first parts tie, each by its own type's `cmp`.
export fn cmp<A, B>(a: (A, B), b: (A, B), ?cmp: (x: A, y: A) -> Int, ?cmp: (x: B, y: B) -> Int) [] -> Int => a, b {
    let c = cmp(a.0, b.0)
    if c != 0 {
        return c
    }
    return cmp(a.1, b.1)
}

// Whether two triples are equal: every part equal, each by its own type's
// `eq`.
export fn eq<A, B, C>(a: (A, B, C), b: (A, B, C), ?eq: (x: A, y: A) -> Bool, ?eq: (x: B, y: B) -> Bool, ?eq: (x: C, y: C) -> Bool) [] -> Bool => a, b {
    return eq(a.0, b.0) && eq(a.1, b.1) && eq(a.2, b.2)
}

// Orders two triples lexicographically, part by part, each by its own
// type's `cmp`.
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
