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

// A fn type is grouped in every arm of a union. First and in the middle the
// pair is written (without it the return type takes the arms after it) and
// kept, not doubled; last it parses bare and the pair is added, so `A | () ->
// B` does not read as `(A | ()) -> B`.
platform fn fn_arm_first(a: (() -> A) | B, b: (once (x: Int) -> A) | B) -> None

platform fn fn_arm_middle(a: A | ((x: Int) -> B) | C, b: A | (once () [Console] -> B) | C) -> None

platform fn fn_arm_last(a: A | (() -> B), b: A | B | ((x: Int) [Console] -> C), c: A | (once () -> B)) -> A | (() -> B)

// The same in a type argument, after a qualifier, inside a union under `?`
// and `[]`, and in an effect's type argument; an arm `(() -> A)?` is a
// nullable, whose fn type is grouped once, for the `?`.
platform fn fn_arm_nested(
    a: List<A | (() -> B)>,
    b: Ok (A | (() -> B)),
    c: (A | (() -> B))?,
    d: (A | (() -> B))[],
    e: B | (() -> A)?,
    f: () [Throws<A | (() -> B)>] -> None
) -> None

// A spread's fn type is grouped too, before another arm, and a spread under `?`.
platform fn spreads(a: ...(() -> R) | B, b: (...T)?) -> None

// A qualifier is a space away from what it qualifies, so a tuple after one
// needs no pair of its own: `Ok (1, 2)` is a tuple of literal types, and
// `Ok(1, 2)`, touching, is `Ok` with two constants, qualifying the type after
// it. Written pairs are kept.
platform fn tuples(a: Ok (A, B), b: Ok ((A, B)), c: InRange(0, 5) (A, B)) -> None

platform fn literals(d: Ok (1, 2), e: Ok(1, 2) Int, f: Ok ((1, 2))) -> None

// After `ref(c)` a qualifier still reads value arguments touching it; a fn type
// whose first parameter is a literal type stands after the space as it is.
platform fn value_args(a: ref(c) (1, 2), b: Ok (1) -> R, c: proj (A, B), d: Heap<T>(?cmp) Mut List<T>) -> None

platform fn unions(a: Ok (Ok Str | Err Int), b: List<Ok A | Err B>, c: Map<Str, Int | Str>) -> Ok (A, B) | Errs E
