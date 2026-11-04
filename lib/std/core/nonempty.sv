// [col-nonempty] `std.core.nonempty`: the claim that a value has at least one
// element. Its own module (user decision 2026-10-30), since one `NonEmpty`
// serves everything with a `size` — `List`, `Set`, `Map`, the sorted pair,
// `Deque`, `Str` — so a function asks for `NonEmpty List<T>` or
// `NonEmpty Str` with the same name. `std.core.*` is visible everywhere, so
// nothing imports it.

// The claim that a value holds at least one element, and the reason the
// qualifier machinery is worth having over a container: with it, `first`
// answers with an element instead of an optional.
//
// Generic over its subject: `qualifies` takes the `size` as an implicit,
// resolved where the claim is tested at the concrete type — `xs is NonEmpty`
// over a `List<Int>` passes `size(List<T>)`, over a `Str` the string's — so
// nothing about it prints in the type.
//
// What establishes it is said by each type, in its own body: `refn add(…) =>
// list: +NonEmpty` in `List`'s. A generic claim belongs to no one subject, so
// the file that declares a type is trusted to establish it about that type —
// which is how `List(first, ...rest)` returns `+NonEmpty List<T>`.
// @compilerrules [col-nonempty, qual-refn-type]
export qualifier NonEmpty<T canbe linear> for T {
    fn qualifies(value: T, ?size: (value: T) -> Int) -> Bool {
        return size(value) > 0
    }
}
