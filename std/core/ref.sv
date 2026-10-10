// [ref-notsame] `core.ref`: what is known about `ref` handles beyond their
// type — the proof that two handles name different elements. `core.*` is
// visible everywhere, so nothing imports it.

// [ref-notsame] Whether [a] and [b] are the **same storage**: two handles of
// one element, or one value seen twice. Not expressible in Salvo, which has
// no identity comparison: each backend answers it its own way — Kotlin by
// reference (`===`), Rust by address (`std::ptr::eq`), both exact for a
// handle, whose element is never a scalar (`Mut` does not apply to one).
// Private: `is NotSame` is the surface, and a name this plain is the user's.
intrinsic fn same<T>(a: T, b: T) [] -> Bool => a, b

// [ref-notsame] The claim that a handle **names a different element from
// one particular other handle**: `b is NotSame(a)`, bound to `a`'s identity.
// With it the two may be live at once — a write through one leaves the other
// standing, and one call may take both [elem-distinct] — which on Rust is a
// split borrow where their paths diverge [rs-path]. Rebinding either side
// strips it [qual-depend], like any dependent claim.
export qualifier NotSame<T>(a: T) of T {
    fn qualifies(b: T, a: T) -> Bool {
        return !same(a, b)
    }
}
