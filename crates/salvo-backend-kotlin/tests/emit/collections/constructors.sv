// Collections: what the std constructors render to.

//--- list_constructors_carry_their_element_type
// The std list constructors use this: `Mut List()` must not emit a bare
// `mutableListOf()`, which kotlinc cannot infer: an empty list constructor
// renders its element type from the call's resolved type arguments
// (`mutableListOf<Int>()`), since Kotlin cannot infer a type argument that the
// source never wrote.
export fn main() [use] -> None {
    use StdConsole
    //~ emits: mutableListOf<Int>()
    let xs: Mut List<Int> = Mut List()
    //~ emits: listOf<String>()
    let ys = List<Str>()
    add(xs, 1)
    println("${size(xs)} ${size(ys)}")
}

//--- array_std_functions_lower
// [type-array] Arrays get the `std.core.list` function surface minus
// construction and mutation: `size`, `get`, `first`, `iter` (user
// decision 2026-09-02 — the docs/language/ `CyclicRandom` example calls
// `values.size()` on a `T[]`). (`ARRAY_STD_DEMO` in codegen_tests.rs.)
effect Random<T> {
    fn next_random() -> T
}

handler CyclicRandom<T>(values: T[]) of Random<T> {
    state {
        i: Int = 0
    }

    fn next_random() -> T {
        //~ emits: values.size
        i = (i + 1) % values.size()
        return values[i]
    }
}

fn main() [use] -> None {
    use StdConsole
    use CyclicRandom(array_of(1, 2, 3, 4))
    let nums: Int[] = array_of(3, 4, 5)
    //~ emits: nums.size
    //~ emits: nums.getOrNull(2)
    //~ emits: nums.firstOrNull()
    println("size ${nums.size()} get ${nums.get(2) ?: unreachable!()} first ${nums.first() ?: unreachable!()}")
    //~ emits: = salvo.std.core.array.iter(nums)
    for n in nums.iter() {
        println("iter ${n}")
    }
    println("random ${next_random()} ${next_random()}")
}
