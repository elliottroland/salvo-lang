struct Person canbe Mut {
    name: Str,
    surname: Str? = None,
    age: Int
}

struct Pair<S, T> {
    first: S,
    second: T
}

fn build() -> Person {
    let person: Person = Person {name: "Roland", age: 36}
    let person2 = Person {...person, surname: "Elliott", age: 25}
    let mutable_person = Mut Person {...person}
    let person3: Person = {name: "Roland", age: 36}
    let {name, age: their_age} = person
    let (a, b, c) = ("String", -1, true)
    let pair = (1, ("two", 3))
    let one = pair.0
    let two = pair.1.0
    let surname: Str? = person.surname
    return person2
}

fn mutate(person: Mut Person) {
    person.name = "Someone else"
}

fn interpolate(person: Person) -> Str {
    return "Surname is ${person.surname!}"
}
