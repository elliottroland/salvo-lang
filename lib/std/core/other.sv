// The open arm of a union of literals. `"A" | "B" | Other Str` is the two
// listed strings *and* any other string, told apart with `is` or `when`. This
// is how a program models a set of values that may grow without it: the
// codes a service may add, the regions a cloud may open.
//
// ```
// type Region = "us-east-1" | "eu-west-1" | Other Str
//
// fn region(s: Str) [] -> Region => !s {
//     if s is "us-east-1" | "eu-west-1" {
//         return s
//     }
//     return other(s)
// }
//
// fn city(r: Region) [] -> Str => r {
//     when r {
//         is "us-east-1" { return "Virginia" }
//         is "eu-west-1" { return "Dublin" }
//         is Other { return "somewhere else (${r})" }
//     }
// }
// ```
//
// **Only [other] reaches the open arm.** A literal the union does not list
// is still a compile-time error, so a typo in `"us-esat-1"` is caught where
// it is written; a value is in the `Other` arm only when it was built with
// [other], which says in the source that it is deliberately unlisted.
//
// **At run time it is just the base type.** `Other` is erased, and its
// value collapses with the listed literals into one arm: a
// `"A" | "B" | Other Str` is a plain string on both backends, so the open
// arm costs nothing.
// @compilerrules [type-literal, type-union-arm-identity, qual-erasure, mod-used-only]

// A value outside the listed literals of a union: `Other Str` in
// `"A" | "B" | Other Str`. Made by [other].
// @compilerrules [qual-constructive]
export provenance qualifier Other<T> for T

// Tags a value as the open arm of a literal union: a value the union does
// not list, such as a code a service added after this program was written.
// The value is moved into the result.
//
// ```
// type Answer = "yes" | "no" | Other Str
//
// fn answer(s: Str) [] -> Answer => !s {
//     if s is "yes" | "no" {
//         return s
//     }
//     return other(s)
// }
// ```
// @compilerrules [qual-ctor-fn]
export fn other<T>(value: T) [] -> +Other T => !value {
    return value
}
