effect Random<T> {
    fn next_random() -> T
}

effect Console {
    fn println(message: Str) -> None => message
}

handler CyclicRandom<T>(values: T[]) of Random<T> {
    i: Int = 0

    fn next_random() -> T {
        i = (i + 1) % values.size()
        return values[i]
    }
}

handler StdOutConsole of Console {
    fn println(message: Str) -> None {
    }
}

fn main() [use] -> None {
    use CyclicRandom([1, 2, 3, 4])
    let num = next_random()
}

fn age_prediction(person: Surname Person) [Random<Int>, Console] -> None => person {
    let years: Int = next_random()
    println("In ${years} years, ${full_name(person)} will be ${person.age + years}")
}

// [throw] A fn that may leave early declares the effect and keeps its own
// return type; `throw` returns `Nothing`, so nothing after it runs.
fn parse_length(line: Str) [Throw<Str>] -> Int {
    if size(line) == 0 {
        throw("empty line")
    }
    return size(line)
}

// [try] The delimiter is an intrinsic expression whose value is an ordinary
// union: `Ok Int | Thrown Str`.
fn describe_length(line: Str) [Console] -> None {
    let outcome = try {
        parse_length(line)
    }
    when outcome {
        is Ok {
            println("length ${outcome}")
        }
        is Thrown {
            println("could not parse: ${outcome}")
        }
    }
}
