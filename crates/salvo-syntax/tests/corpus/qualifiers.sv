qualifier Surname of Person with Old

qualifier Old of Person with Surname

qualifier Ok<T> of T
qualifier Err<T> of T

qualifier Something<S, T> of Pair<S, T>
qualifier FirstStr<T> of Pair<Str, T>
qualifier Ints of Pair<Int, Int>

type Result<S, T> = Ok S | Err T

qualifier Positive of Int {
    fn qualifies(int: Int) -> Bool {
        return int > 0
    }
}

qualifier NonNegative of Int with Positive {
    fn qualifies(int: Int) -> Bool {
        return int >= 0
    }
}

qualifier HasSurname of Person {
    surname: Str

    fn qualifies(person: Person) -> Bool {
        return person.surname is Str
    }
}

qualifier RandomPositive of Int

fn random_positive_int() [Random<Int>, as] -> RandomPositive Int {
    let num = next_random()
    while num <= 0 {
        num = next_random()
    }
    return num as RandomPositive
}

fn checks(person: Person) -> None {
    let p: Old Surname Person | None = check_old_surname(person)
    if p is Surname {
        println("has surname")
    } elif p is Person {
        println("plain person")
    } else {
        println("nothing")
    }
    let nested: Ok (Ok Str | Err Int) | Err Bool = some_function()
    let pair = (person, check_surname(person))
}
