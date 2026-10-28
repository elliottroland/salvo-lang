// S3: borrow emission [rs-borrow-locals]
//
// Compile-and-run data ([test-run-data]), moved from the codegen tests.

//--- borrows

struct Person {
    name: Str,
    age: Int
}

fn count_long(persons: List<Person>) -> Int => persons {
    let total = 0
    for person in persons {
        let n = person.name
        if size(n) > 3 {
            total = total + 1
        }
    }
    return total
}

fn poison_guards_the_borrow() -> Int {
    let xs = Mut List(1, 2)
    let ys = xs
    let n = size(ys)
    add(xs, 9)
    return n + size(xs)
}

fn main() [use] -> None {
    use StdConsole
    let people = List(Person {name: "Ada", age: 36}, Person {name: "Grace", age: 45})
    println("${count_long(people)}")
    println("${poison_guards_the_borrow()}")
}
//= stdout
//= 1
//= 5
