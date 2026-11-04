effect Random<T> {
    fn next_random() -> T
}

effect Console {
    fn println(message: Str) -> None => message
}

// [effect-alias] An effect-group alias: shorthand for its member list.
effects Chatty = [Random<Int>, Console]

handler CyclicRandom<T>(values: T[]) for Random<T> {
    state {
        i: Int = 0
    }

    fn next_random() -> T {
        i = (i + 1) % values.size()
        return values[i]
    }
}

handler StdConsole for Console {
    fn println(message: Str) -> None {
    }
}

fn main() [use] -> None {
    use CyclicRandom(array_of(1, 2, 3, 4))
    let num = next_random()
}

fn age_prediction(person: Surname Person) [Random<Int>, Console] -> None => person {
    let years: Int = next_random()
    println("In ${years} years, ${full_name(person)} will be ${person.age + years}")
}

// [throw] A fn that may leave early declares the effect and keeps its own
// return type; `throw` returns `Never`, so nothing after it runs.
fn parse_length(line: Str) [Throw<Str>] -> Int {
    if size(line) == 0 {
        throw("empty line")
    }
    return size(line)
}

// [throw-try] The delimiter is an intrinsic expression whose value is an ordinary
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

// [effect-var] An effect variable: a generic ranging over a set of effects,
// named in fn types' effect lists and as a handle; `eff[]` handles none.
fn each<T, [E]>(xs: List<T>, f: (x: T) [E] -> None) => xs, f {
    for x in xs {
        f(x)
    }
}

fn later<[E]>(fx: eff[E], f: () [E, Console] -> None) [use] -> eff[] => fx, f {
    use fx
    return eff[]
}

// [effect-var] A struct's effect variable, and the list that fills it.
struct Later<[E]> {
    run: () [E] -> None
}

fn fire(a: Later<[Console, Logger]>, b: Later<[]>) => a, b {
}
