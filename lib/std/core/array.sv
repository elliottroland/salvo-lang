// Arrays: fixed-size sequences, written `T[]`.
//
// An array is built once, with [array_of] or [array_by], and never grows or
// shrinks; its elements are read with `arr[i]` (from `0`) and walked with
// `for`, both built into the language. Arrays exist mainly for variadic parameters: a
// function declared `fn sum(...xs: Int[])` receives its arguments as an
// array, and `sum(...arr)` passes one on. For an ordinary sequence, use a
// `List`, which also has the larger library.
//
// ```
// let squares = array_by(4, (i: Int) -> i * i)        // [0, 1, 4, 9]
// let names = array_of("ann", "bob")
// println("${size(names)} names, the first is ${names[0]}")
// for n in names {
//     println(n)                                       // ann, then bob
// }
// let third: Int? = get(squares, 2)                    // 4
// let tenth: Int? = get(squares, 9)                    // None
// ```
//
// **No literal.** There is no array literal: `List(1, 2)` builds a list,
// not an array, so [array_of] is the constructor.
//
// **Arrays of linear values.** An array may hold linear values (values that
// must be used exactly once, such as open files), and then the array owes
// them. It can be measured and walked, but [get] and [first] are closed to
// it, since taking an element out would leave the array owing a value it no
// longer holds; [drain] is how such an array ends.
// @compilerrules [type-array, col-no-literal, linear-generics]

// Builds an array from its arguments, in order.
//
// The elements are moved in, as into any constructor, so
// `array_of(...xs)` spends `xs`. The variadic parameter already *is* an
// array, so nothing is copied: the call answers it.
//
// ```
// let primes = array_of(2, 3, 5, 7)
// ```
// @compilerrules [fn-variadic, col-no-literal]
export intrinsic fn array_of<T canbe linear>(...elems: T[]) [] -> T[] => !elems

// Builds an array of [size] elements, each made from its index by a
// function, which is called once per index, in order from `0`.
//
// ```
// let doubled = array_by(3, (i: Int) -> i * 2)        // [0, 2, 4]
// ```
//
// @param size  how many elements
// @param init  makes the element at an index; its effects are the call's
// @compilerrules [col-by]
export intrinsic fn array_by<T, [E]>(size: Int, init: (Int) [E] -> T) [E] -> T[] => size, init

// The number of elements in an array.
export intrinsic fn size<T canbe linear>(array: T[]) [] -> Int => array

// The element at [index], or `None` when the index is out of range
// (negative, or not less than the size).
//
// The element is borrowed from the array, not copied.
export intrinsic fn get<T>(array: T[], index: Int) [] -> (proj(array) T)? => array, index

// The first element of an array, borrowed from it, or `None` when it is
// empty.
export intrinsic fn first<T>(array: T[]) [] -> proj(array) T? => array

// A new iterator over an array's elements, front to back. The array is
// borrowed, not moved, and each element is borrowed from it.
//
// A `for` over an array needs no `iter`; pass `iter(arr)` where a function
// asks for an iterator, such as those in [std.core.seq].
//
// ```
// let xs = array_of(1, 2, 3, 4)
// let big = count(filtering(iter(xs), (x) -> x > 2))   // 2
// ```
// @compilerrules [iter-mint, proj-field, proj-infer]
export fn iter<T>(array: T[]) [] -> Mut ArrayIter<T> => array {
    return Mut ArrayIter<T> { items: array, at: 0 }
}

// The iterator [iter] answers over an array: the array, borrowed, and the
// position of the next element.
//
// It is the array form of [std.core.list]'s [ListIter].
// @compilerrules [iter-protocol, iter-for-native]
export struct ArrayIter<T> : Next<self, proj T> canbe Mut {
    // The array being walked, borrowed.
    // @compilerrules [proj-field]
    items: proj (T[]),
    // The index of the next element to emit.
    at: Int
}

// Advances the iterator: the element at its position, or the end of the
// array.
export fn next<T>(p: Mut ArrayIter<T>) [] -> Emitted (proj(p) T) | Finished => p: Mut {
    // The backends keep their native loop for a `for` over an array; this
    // shape is what combinators see [iter-for-native].
    let elem = get(p.items, p.at)
    if elem is None {
        return finished()
    }
    p.at = p.at + 1
    return emitted(elem)
}

// Consumes an array and hands each element to [each], in order. This is how
// an array of linear values ends when its elements go separate ways, such as
// closing every file in it.
//
// It is a function rather than a `for` because a loop can stop early, and
// the elements it did not reach would then be owed by nobody: [drain]
// always reaches all of them.
//
// ```
// fn print_all(...xs: Int[]) [Console] -> None => !xs {
//     drain(xs, (x: Int) -> println("${x}"))
// }
// ```
//
// @param items  the array, moved into the call
// @param each   consumes one element; its effects are the call's
// @compilerrules [linear-container, fn-effects]
export fn drain<T canbe linear, [E]>(items: T[], each: (x: T) [E] -> None) -> None
=>[each] !x => !items, each {
    let list = Mut List<T>()
    list.append(items)
    drain(list, each)
}
