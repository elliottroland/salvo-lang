// A prefix qualifier binds tighter than `?`, `[]` and `|`, so none of these
// needs parentheses: `(NonEmpty List<Int>)?`, `(Mut Str)?`, `(Q T)[]`,
// `(Mut Str) | Int`, `(proj(xs) T)?`.
platform fn qualified(a: NonEmpty List<Int>?, b: Mut Str?, c: Q T[], d: Mut Str | Int) -> proj(xs) T? => xs

platform fn nested(a: Mut NonEmpty List<T>?, b: ref(c) Mut T?, c: Heap<T>(?cmp) Mut List<T>[], d: T?[]) -> None

// Written parentheses are kept: here they change the reading.
platform fn grouped(a: NonEmpty (List<Int>?), b: (T[])?, c: (Mut Str)?) -> (proj(xs) T)?

// A fn type after a qualifier needs none; under `?` and in a union it does.
platform fn fn_types(f: once () -> Q, g: once (t: T) [Console] -> None, h: (() -> Q)?, i: A | (() -> B) | C) -> None

platform fn fn_types_written(f: once (() -> Q), g: (once () -> Q)?) -> None

// A spread's fn type is grouped too, before another arm, and a spread under `?`.
platform fn spreads(a: ...(() -> R) | B, b: (...T)?) -> None

// A qualifier over a tuple: `Ok(A,B)` is the tuple `(A, B)`, as `Ok (A, B)`
// is. A tuple of literal types needs its own pair, or it reads as the
// qualifier's value arguments: `Ok ((1, 2))` against `Ok(1, 2)`.
platform fn tuples(a: Ok(A,B), b: Ok ((A, B)), c: InRange(0, 5) (A, B), d: Ok ((1, 2)), e: Ok(1, 2)) -> None

// After `ref(c)` a qualifier still reads value arguments; a fn type whose first
// parameter is a literal type needs the pair as a tuple does.
platform fn value_args(a: ref(c) ((1, 2)), b: Ok ((1) -> R), c: proj (A, B)) -> None

platform fn unions(a: Ok (Ok Str | Err Int), b: List<Ok A | Err B>, c: Map<Str, Int | Str>) -> Ok (A, B) | Errs E
