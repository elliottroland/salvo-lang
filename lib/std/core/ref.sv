// The proof that two mutable handles name different elements.
//
// A **handle** is a `Mut` view of an element inside a container, such as the
// one a list's `at(xs, i)` answers: a write through it changes the element
// in place. Two handles into one container might name the same element, so
// the compiler refuses to keep two of them live at once, or to pass both to
// one call, until it knows they do not. Testing `d is NotSame(a)` tells it:
//
// ```
// struct Entity canbe Mut { hp: Int, energy: Int }
//
// fn duel(a: Mut Entity, d: Mut Entity) -> None => a: Mut, d: Mut {
//     a.energy = a.energy - 1
//     d.hp = d.hp - 2
// }
//
// fn fight(squad: List<Mut Entity>, i: Int, j: Int) -> None => squad {
//     let a = at(squad, i) ?: unreachable!()
//     let d = at(squad, j) ?: unreachable!()
//     if d is NotSame(a) {
//         duel(a, d)                                   // both handles at once
//     }
// }
// ```
//
// The test runs at run time and compares the handles' identity, not their
// contents. A write *through* either handle keeps the claim, since it is
// still the same element; binding either name to another handle strips it.
// Handles made inside one expression have no name to prove anything about,
// so bind them with `let` first. The Mutable handles topic has the whole
// story.
// @compilerrules [ref-notsame]

// Whether [a] and [b] are the **same storage**: two handles of
// one element, or one value seen twice. Not expressible in Salvo, which has
// no identity comparison, so each backend answers it its own way: Kotlin by
// reference (`===`), Rust by address (`std::ptr::eq`), both exact for a
// handle, whose element is never a scalar (`Mut` does not apply to one).
// Private: `is NotSame` is the surface, and a name this plain is the user's.
// @compilerrules [ref-notsame]
intrinsic fn same<T>(a: T, b: T) [] -> Bool => a, b

// The claim that a handle names a **different element** from one
// particular other handle: `d is NotSame(a)`, bound to `a`.
//
// With it the two handles may be live at once: a write through one leaves
// the other usable, and one call may take both. Two handles of the same
// element fail the test. The claim is about the handles themselves, so it
// holds however they were found: an index into a list, a key into a map, a
// search through a struct of your own.
// @compilerrules [ref-notsame, fate-elem-distinct, rs-path, qual-depend]
// @compilerknown
export qualifier NotSame<T>(a: T) for T {
    // Whether [b] is not the same storage as [a].
    fn qualifies(b: T, a: T) -> Bool {
        return !same(a, b)
    }
}
