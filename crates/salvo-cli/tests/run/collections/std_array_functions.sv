// std array functions
//
// Compile-and-run data ([test-run-data]), moved from the codegen tests.

//--- array_std

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
    println("size ${nums.size()} get ${nums.get(2) ?: unreachable!()} first ${nums.first() ?: unreachable!()}")
    for n in nums.iter() {
        println("iter ${n}")
    }
    println("random ${next_random()} ${next_random()}")
}
//= stdout
//= size 3 get 5 first 3
//= iter 3
//= iter 4
//= iter 5
//= random 2 3
