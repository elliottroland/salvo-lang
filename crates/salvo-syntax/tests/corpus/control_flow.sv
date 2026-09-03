fn full_name(person: Person) -> [person] Str {
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

fn last_number(numbers: Int[]) -> Int {
    let i = 0
    let last = while i++ < numbers.size() {
        numbers[i]
    } else {
        -1
    }
    return last
}

fn count(person: Person) [Console] -> [person] None {
    for i in range(0, person.age) {
        println("Person is older than ${i}...")
    }
}

fn full_name_expr(person: Person) -> [person] Str? {
    let full_name = if person.surname is Str {
        "${person.name} ${person.surname}"
    }
    return full_name
}
