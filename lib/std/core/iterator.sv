// The iteration protocol: what `for` drives, and how to write an iterator
// of your own.
//
// An **iterator** is a value holding a position in a sequence. Its `next`
// advances it and answers either the next element, tagged [Emitted], or
// [Finished] at the end. `for x in it` calls `next(it)` until it answers
// `Finished`. Nothing about this is built into the compiler: the protocol is
// the two groups declared here, [Next] and [Iter], and std's own lists,
// ranges and adaptors are written with them.
//
// An iterator is an ordinary struct that declares `: Next<self, T>` and has
// a `next` over it:
//
// ```
// struct Countdown : Next<self, Int> canbe Mut {
//     at: Int
// }
//
// fn next(c: Mut Countdown) [] -> Emitted Int | Finished => c: Mut {
//     if c.at <= 0 {
//         return finished()
//     }
//     c.at = c.at - 1
//     return emitted(c.at + 1)
// }
//
// fn main() [use] -> None {
//     use StdConsole()
//     let c = Mut Countdown { at: 3 }
//     for n in c {
//         println("${n}")                              // 3, 2, 1
//     }
// }
// ```
//
// **The short form.** When the iterator's fields are just the function's
// parameters and a position, an `iter fn` writes the struct and its `next`
// for you; its `state` block names the struct and declares its fields:
//
// ```
// iter fn countdown(from: Int) -> Emitted Int | Finished {
//     state Countdown {
//         at: Int = from
//     }
//     if at <= 0 {
//         return finished()
//     }
//     at = at - 1
//     return emitted(at + 1)
// }
// ```
//
// **Sources.** A type `for` can loop over without being an iterator itself,
// such as a list, is a *source*: an `iter` function mints an iterator from
// it, and `for x in xs` means `next(iter(xs))`. A type of your own becomes a
// source by declaring `: Iter<self, It, T>`, naming its iterator struct, and
// having an `iter` that answers one.
//
// **Advanced in place.** An iterator held in a variable keeps the position a
// loop reached, so a function can read part of it and leave the rest for
// someone else. The functions that take iterators, such as `map`, `filter`
// and the lazy `mapping` and `taking`, are in [std.core.seq].
// @compilerrules [iter-protocol, iter-fn, mod-used-only]

// The element arm of a `next` result: `Emitted T` is the element itself,
// tagged so that it cannot be confused with the end. Made by [emitted].
//
// Because the element keeps its own type, a sequence of optionals still has
// a distinguishable end: `Emitted None | Finished` has two arms, where
// `None | None` would have one. At run time the tag is erased.
// @compilerrules [qual-constructive, qual-erasure]
// @compilerknown
export provenance qualifier Emitted<T> for T

// The end arm of a `next` result: the sequence has no more elements. Made
// by [finished].
//
// It is a struct with no fields, rather than `None`, because `None` says
// "absent" where this says "the sequence ended".
// @compilerknown
export struct Finished {}

// Tags a value as the element a `next` answers.
//
// The value is moved into the result. It may be linear (a value that must be
// used exactly once): its obligation travels with it.
// @compilerrules [qual-ctor-fn, linear-generics]
export fn emitted<T canbe linear>(value: T) [] -> +Emitted T => !value {
    return value
}

// The end of a sequence, as a `next` answers it.
export fn finished() [] -> Finished {
    return Finished {}
}

// What an iterator is: a type `It` with a `next` that advances it and
// answers an element of type `T`, or the end.
//
// A struct declares `: Next<self, T>` to promise the `next`, and the promise
// is checked at the struct: a missing or misspelled `next` is an error
// there, not a puzzling "not iterable" at some distant loop. The clause also
// tells a combinator what the iterator yields.
//
// **Borrowed or owned elements.** An iterator that walks existing data,
// such as a list's, emits borrows of it: it declares `: Next<self, proj T>`,
// so nothing is copied as it is read, and whoever stores an element copies
// it. One that computes its elements, such as a range, declares
// `: Next<self, T>` and hands them over. A function asking for
// `?Next<It, T>` accepts either.
//
// **Effects.** `E` is what advancing performs: nothing, written by leaving it
// out (`: Next<self, Int>`), or the effects a `next` needs, such as reading a
// stream (`: Next<self, Str, [Streams]>`). Whoever drives the iterator
// performs them.
//
// **In a signature.** A generic function reaches an iterator's `next` by
// asking for the group, `?Next<It, T>`: the caller's `next` for its own
// iterator type fills it, and the function writes `for x in it`.
//
// ```
// fn sum<It>(it: Mut It, ?Next<It, Int>) [] -> Int => it: Mut {
//     let total = 0
//     for n in it {
//         total = total + n
//     }
//     return total
// }
//
// fn ten() [] -> Int {
//     return sum(range(1, 5))                          // 1 + 2 + 3 + 4
// }
// ```
// @compilerrules [iter-protocol, group-obligation, implicit-group, group-self, iter-yield-proj, copy-opt-in, effect-var]
// @compilerknown
export fns Next<It, T, [E]> {
    // Advances the iterator: the next element, or the end. The iterator is
    // taken as `Mut`, since advancing changes its position.
    fn next(it: Mut It) [E] -> Emitted T | Finished => it: Mut
}

// What a source is: a type `C` that an `iter` mints an iterator `It` from,
// whose `next` answers elements of type `T`.
//
// A type joins by declaring `: Iter<self, It, T>`, naming its iterator
// struct, and either an `iter fn iter(...)` whose `state` block is that
// struct (the compiler writes both members) or an `iter` of its own
// answering one. Declaring both `: Iter` and `: Next` on one type is
// refused, since `for x in s` would then have two meanings.
//
// The two members are one decision: a caller may supply its own `iter`, but
// never a `next` alone, since the step belongs to whatever iterator the
// `iter` answers. Minting performs nothing; `E` is what driving performs, as
// for [Next].
//
// ```
// fn total<C, It>(c: C, ?Iter<C, It, Int>) [] -> Int => c {
//     let sum = 0
//     for n in c {                                     // mints, then drives
//         sum = sum + n
//     }
//     return sum
// }
//
// fn six() [] -> Int {
//     return total(List(1, 2, 3))
// }
// ```
// @compilerrules [iter-group, implicit-with, effect-var]
// @compilerknown
export fns Iter<C, It, T, [E]> => iter with next {
    // Mints a new iterator over the source, at its start.
    fn iter(collection: C) -> Mut It
    // Advances an iterator the source minted.
    fn next(iterator: Mut It) [E] -> Emitted T | Finished => iterator: Mut
}
