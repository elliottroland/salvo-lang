effect Random<T> {
    fn next_random() -> T
}

effect Console {
    fn println(message: Str) -> [message] None
}

handler CyclicRandom<T>(values: T[]) of Random<T> {
    i: Int = 0

    fn next_random() -> T {
        i = (i + 1) % values.size()
        return values[i]
    }
}

external handler StdOutConsole of Console

fn main() [use] -> [] None {
    use CyclicRandom([1, 2, 3, 4])
    let num = next_random()
}

fn age_prediction(person: Surname Person) [Random<Int>, Console] -> [person] None {
    let years: Int = next_random()
    println("In ${years} years, ${full_name(person)} will be ${person.age + years}")
}
