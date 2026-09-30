// [type-literal] The open arm of a union of literals: `"A" | "B" | Other Str`
// is the listed values *and* any other string, told apart by `when`. A value
// reaches the `Other` arm only through [other] — anything not constructed so
// must be one of the listed literals (user decisions 2026-09-30). At run time
// it collapses with the literals into one arm of their base, so a
// `"A" | "B" | Other Str` is a plain string on both backends
// [union-arm-identity].
//
// An ordinary constructive qualifier, erased like any other [qual-erasure];
// nothing about it is special to the compiler. Own module, as `core.result`
// is, so a program that never names it emits nothing for it [mod-used-only].

// [qual-constructive] A value outside the listed literals of a union.
export provenance qualifier Other<T> of T

// [qual-ctor-fn] Tags [value] as the open arm of a literal union: a value the
// union does not list, such as a service's enum value this client predates.
export fn other<T>(value: T) [] -> +Other T {
    return value
}
