// L7c: derived returns [proj-readonly-return]
//
// Compile-and-run data ([test-run-data]), moved from the codegen tests.

//--- derived_returns

struct Person {
    name: Str,
    age: Int
}

fn find_adult(persons: List<Person>) -> proj(persons) Person? => persons {
    for person in persons {
        if person.age >= 18 {
            return person
        }
    }
    return None
}

fn head_of(persons: List<Person>, tag: Str) -> proj(persons) Person? => persons, tag {
    return first(persons)
}

fn main() [use] -> None {
    use StdConsole
    let people = List(Person {name: "Kid", age: 9}, Person {name: "Grace", age: 45})
    let adult = find_adult(people)
    if adult is Person a {
        println("adult: ${a.name}")
    }
    let head = head_of(people, "x")
    if head is Person h {
        println("head: ${h.name}")
    }
    println("done")
}
//= stdout
//= adult: Grace
//= head: Kid
//= done
