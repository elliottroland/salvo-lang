import random.Random
import random.DefaultRandom

struct Person {
    first_name: Str
    last_name: Str? = None
    age: Int
}

fn full_name(p: Person) -> Str {
    return "${p.first_name} ${p.last_name ?: "unknown"}"
}

qualifier NE<T> of List<T> {
    fn qualifies(list: List<T>) -> Bool {
        return size(list) >= 5
    }

    refn add<T>(list: NE Mut List<T>, elem: T) => list: +NE
}

handler CyclicRandom(numbers: NE List<Double>) of Random {
    i: Int = 0

    fn random() -> Double {
        assert!(i is Idx(numbers))
        return numbers.get(i++)
    }
}

fn main() [use] {
    use StdOutConsole()
    let random_numbers = [1.4, 6.3, 2.342, 0.5, 0.9]
    assert!(random_numbers is NE)
    use CyclicRandom(random_numbers)

    let roland = Person { first_name: "Roland", age: 35 }
    let numbers: Mut List<Int> = [1, 2, 3, 4, 5]

    if numbers is NE {
        numbers.add@demo(6)
        print_full_name_with(roland, numbers)
    }
}

fn print_full_name(p: Person) [Console, Random] {
    println("${full_name(p)}, your lucky number is: ${random()}")
}

fn print_full_name_with(p: Person, numbers: NE List<Int>) [Console] {
    for i in numbers {
        println("${full_name(p)}, your lucky number is: ${i}")
    }
}