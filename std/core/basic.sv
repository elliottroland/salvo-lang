intrinsic type Byte
intrinsic type Int
intrinsic type Long
intrinsic type Float
intrinsic type Double
intrinsic type Char
intrinsic type Bool
intrinsic type Any
intrinsic type Nothing

type Number = Int | Long | Double | Float

// [op-convert] The explicit numeric conversions the operators point at
// where they refuse to mix classes ([op-arith]: `Int + Double` is an
// error naming `to_double`). Truncating conversions truncate toward zero
// and saturate at the target's bounds, identically on both backends;
// `to_int(Long)` keeps the low 32 bits (both backends' `toInt()`/`as i32`
// semantics).
intrinsic fn to_int(value: Long) [] -> Int => value
intrinsic fn to_int(value: Double) [] -> Int => value
intrinsic fn to_int(value: Float) [] -> Int => value
intrinsic fn to_long(value: Int) [] -> Long => value
intrinsic fn to_long(value: Double) [] -> Long => value
intrinsic fn to_long(value: Float) [] -> Long => value
intrinsic fn to_double(value: Int) [] -> Double => value
intrinsic fn to_double(value: Long) [] -> Double => value
intrinsic fn to_double(value: Float) [] -> Double => value
intrinsic fn to_float(value: Int) [] -> Float => value
intrinsic fn to_float(value: Long) [] -> Float => value
intrinsic fn to_float(value: Double) [] -> Float => value

// [byte-value] A `Byte` is an **unsigned** 0..255 octet on both backends, and
// it is not operator-numeric: these two conversions are the whole of its
// arithmetic surface, so a byte is turned into an `Int`, computed with, and
// turned back. `to_byte` keeps the low 8 bits (300 is 44, -1 is 255), which
// is what both backends' truncation does.
intrinsic fn to_byte(value: Int) [] -> Byte => value
intrinsic fn to_int(value: Byte) [] -> Int => value

// [intrinsic-fn] [copy-fn] Duplicates a value: the argument is kept
// untouched (with all its qualifiers) and the result is a fresh,
// independent value with no fate links to the source. Implemented by
// each backend directly (Kotlin: identity for transitively immutable
// types, a real copy for Mut-capable ones; Rust: `.clone()`).
intrinsic fn copy<T>(value: proj T) [] -> T => value

// [linear-group] [linear-discard] What **linear** means: a type declares
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
intrinsic fn discard<T canbe linear>(value: T) [] -> None => !value

// [interp-to-str] The obligation "this type has a text form". Declaring
// `: ToStr<self>` on a type is a *convenience*: it does not enable
// interpolation — a `to_str` in scope does that, resolved at the
// interpolation site — but it validates at the declaration that one exists,
// which is where the mistake is easier to see (user decision 2026-09-11).
params ToStr<T> {
    fn to_str(value: T) -> Str
}
