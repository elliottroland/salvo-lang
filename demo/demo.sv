import random.Random
import random.DefaultRandom

struct Person {
    first_name: Str
    last_name: Str? = None
    age: Int
}

fn full_name(p: Person) -> Str {
    if p.last_name !is None {
        return "${p.first_name} ${p.last_name}"
    }
    return p.first_name
}

qualifier NE<T> of List<T> with NonEmpty {
    fn qualifies(list: List<T>) -> Bool {
        return size(list) >= 5
    }

    refn add<T>(list: NE Mut List<T>, elem: T) => list: +NE
}

handler CyclicRandom(numbers: NE List<Double>) of Random {
    i: Int = 0

    fn random() -> Double {
        assert!(i is Idx(numbers))
        return numbers.get(i++).copy()
    }
}

fn main() [use] {
    use StdOutConsole()
    use CyclicRandom([1.4, 6.3, 2.342])

    let roland = Person { first_name: "Roland", age: 35 }
    let numbers: Mut List<Int> = [1, 2, 3, 4, 5]

    if numbers is NE {
        numbers.add(6)
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