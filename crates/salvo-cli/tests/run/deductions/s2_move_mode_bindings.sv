// S2: move-mode bindings [fate-move-mode]
//
// Compile-and-run data ([test-run-data]), moved from the codegen tests.

//--- move_modes

struct Person {
    name: Str,
    age: Int
}

fn longest_name(persons: List<Person>) -> Str
=> !persons {
    let longest = ""
    for person in persons {
        let name = person.name
        if size(name) > size(longest) {
            longest = name
        }
    }
    return longest
}

fn consume(text: Str) -> None => !text {
}

fn main() [use] -> None {
    use StdConsole
    let people = List(Person {name: "Ada", age: 36}, Person {name: "Grace", age: 45})
    println(longest_name(people))
    for s in List("x", "y") {
        consume(s)
    }
    let xs = Mut List(1, 2)
    let ys = xs
    ys.add(3)
    println("${ys.size()}")
}
//= stdout
//= Grace
//= 3
