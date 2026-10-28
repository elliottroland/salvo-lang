// std array functions.

//--- array_std_functions_lower
// [type-array] Arrays get the `std.core.list` function surface minus
// construction and mutation: `size`, `get`, `first`, `iter` (user
// decision 2026-09-02). `T[]` and `List<T>` share the `Vec<T>`
// rendering, so their lowerings mirror each other.
effect Random<T> {
    fn next_random() -> T
}

handler CyclicRandom<T>(values: T[]) of Random<T> {
    state {
        i: Int = 0
    }

    fn next_random() -> T {
        i = (i + 1) % values.size()
        return values[i]
    }
}

fn main() [use] -> None {
    use StdConsole
    use CyclicRandom(array_of(1, 2, 3, 4))
    let nums: Int[] = array_of(3, 4, 5)
    //~ emits: (nums.len() as i32)
    //~ emits: nums.get((2i32) as i64 as usize).copied()
    //~ emits: nums.first().copied()
    println("size ${nums.size()} get ${nums.get(2) ?: unreachable!()} first ${nums.first() ?: unreachable!()}")
    for n in nums.iter() {
        println("iter ${n}")
    }
    println("random ${next_random()} ${next_random()}")
}
