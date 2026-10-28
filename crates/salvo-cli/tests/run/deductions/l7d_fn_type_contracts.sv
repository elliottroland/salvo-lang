// L7d: fn-type contracts [fn-contract]
//
// Compile-and-run data ([test-run-data]), moved from the codegen tests.

//--- fn_contracts

struct Person {
    name: Str,
    age: Int
}

fn apply_keeping(f: (v: List<Person>) -> Int, data: List<Person>) -> Int =>[f] v => data {
    return f(data) + f(data)
}

fn apply_consuming(f: (v: List<Person>) -> Int, data: List<Person>) -> Int =>[f] !v => !data {
    return f(data)
}

fn count(people: List<Person>) -> Int => people {
    return size(people)
}

fn main() [use] -> None {
    use StdConsole
    let people = List(Person {name: "Ada", age: 36}, Person {name: "Grace", age: 45})
    let twice = apply_keeping((v: List<Person>) -> { return size(v) }, people)
    println("twice=${twice}")
    println("still=${size(people)}")
    let named = apply_keeping(count, people)
    println("named=${named}")
    let eaten = apply_consuming((v: List<Person>) -> { return size(v) }, people)
    println("eaten=${eaten}")
    println("done")
}
//= stdout
//= twice=4
//= still=2
//= named=4
//= eaten=2
//= done
