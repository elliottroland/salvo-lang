// Values known to hold at least one element. `NonEmpty List<T>` is a list
// the compiler knows is not empty, so asking for its first element answers
// the element itself, with no `None` to handle.
//
// The claim comes from how a value was made or tested: a list built with
// elements (`List(1, 2, 3)`) is non-empty, so is one just added to, and any
// value passes a test with `is NonEmpty`. A function that needs at least one
// element says so in its parameter type, and its callers prove it once:
//
// ```
// fn largest(xs: NonEmpty List<Int>) [] -> Int => xs {
//     let best = first(xs)                             // an Int, not an Int?
//     for x in xs {
//         if x > best {
//             best = x
//         }
//     }
//     return best
// }
//
// fn report(xs: List<Int>) [Console] -> None => xs {
//     if xs is NonEmpty {
//         println("largest: ${largest(xs)}")
//     } else {
//         println("no values")
//     }
// }
// ```
//
// **Removing gives it up.** On a `Mut List`, the claim lasts until a change
// that could empty the list, such as `remove_first`; adding puts it back.
//
// ```
// let xs: Mut List<Int> = Mut List()
// add(xs, 7)
// let seven = first(xs)                                // 7: add made xs NonEmpty
// ```
//
// One `NonEmpty` serves everything with a `size`: `List`, `Set`, `Map`,
// the sorted collections, `Deque` and `Str`, so a function asks for
// `NonEmpty List<T>` or `NonEmpty Str` with the same name. The claim exists
// only at compile time and costs nothing at run time.
// @compilerrules [col-nonempty]

// The claim that a value holds at least one element: `size(value) > 0`.
//
// It works for any type with a `size` function, found where the claim is
// tested, so nothing about it shows in the type beyond the word itself.
// Which operations establish or keep it is said by each type, in its own
// declaration: a list's `add` establishes it, which is how `List(first,
// ...rest)` answers a `NonEmpty List<T>` without a test.
// @compilerrules [col-nonempty, qual-refn-type]
export qualifier NonEmpty<T canbe linear> for T {
    // Whether `size(value) > 0`.
    //
    // @param size  found where the claim is tested: the value's size
    fn qualifies(value: T, ?size: (value: T) -> Int) -> Bool {
        return size(value) > 0
    }
}
