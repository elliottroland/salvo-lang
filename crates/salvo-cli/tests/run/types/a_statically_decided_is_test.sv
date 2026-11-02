// [is-narrowing] An `is` the checker decides statically (the subject's type
// already is the arm) is `true`, and still binds and still evaluates its
// subject (ROADMAP 0p: the binding was lowered to nothing, and a program the
// checker accepted did not build).
//
// Compile-and-run data ([test-run-data]).

//--- the_binding_binds_and_the_subject_runs
struct Person {
    name: Str
    age: Int
}

fn first_age() -> Int {
    let people = Mut List(Person { name: "Ada", age: 36 })
    // `first` of a non-empty list answers the element itself.
    let head = first(people)
    if head is Person h {
        return h.age
    }
    return 0
}

fn tick() [Console] -> Int {
    println("tick")
    return 1
}

fn main() [use] {
    use StdConsole()
    println("${first_age()}")
    if tick() is Int n {
        println("bound ${n}")
    }
    if tick() is Int {
        println("no binding")
    }
    let early = false
    if early && tick() is Int {
        println("never")
    }
}
//= stdout
//= 36
//= tick
//= bound 1
//= tick
//= no binding
