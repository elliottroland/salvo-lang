fn full_name(person: Person) -> Str => person {
    if (person.surname is Str) {
        return "${person.name} ${person.surname}"
    }
    return person.name
}

fn union_moves() -> None {
    let string_or_number: Str | Int = func()
    if string_or_number is Str s {
        println(s)
    }
    while string_or_number is Int i {
        string_or_number = func()
    }
}

fn describe(result: Ok Str | Err Str | Err Bool) -> Str {
    let value = when result {
        is Ok {
            result
        }
        is Err {
            println("Error: ${result}")
            return "error"
        }
    }
    return value
}

fn precise_checks(result: Ok Str | Err Str | Err Bool) -> None {
    if result is Err Str {
        println("string error")
    }
    if result is Err {
        println("any error")
    }
}

// [when-condition] Without a subject, `when` is a condition chain: bare
// boolean branch heads and a mandatory `else`, which is what makes it
// exhaustive (and keeps `None` out of its value, unlike an `if` chain).
fn classify(n: Int) -> Str {
    return when {
        n < 0 { "negative" }
        n == 0 { "zero" }
        else { "positive" }
    }
}

// [when-condition] The heads are ordinary boolean expressions, so `is`
// narrows its branch and the `else` sees the arm removed.
fn describe(value: Str | Int) [Console] -> None {
    when {
        value is Str s { println("str ${s}") }
        else { println("int ${value}") }
    }
}

fn last_number(numbers: Int[]) -> Int {
    let i = 0
    let last = while i++ < numbers.size() {
        numbers[i]
    } else {
        -1
    }
    return last
}

fn count(person: Person) [Console] -> None => person {
    for i in range(0, person.age) {
        println("Person is older than ${i}...")
    }
}

fn full_name_expr(person: Person) -> Str? => person {
    let full_name = if person.surname is Str {
        "${person.name} ${person.surname}"
    }
    return full_name
}

// [linear-obligation] Release on every path is written out: `defer` was
// removed from the language (2026-09-10) because linearity is what makes the
// obligation checked, and repeating the release is the honest cost.
fn read_config(path: Str) [Console] -> Str {
    let file = open(path)
    if is_empty(file) {
        close(file)
        return ""
    }
    for line in lines(file) {
        println(line)
    }
    let out = contents(file)
    close(file)
    return out
}
