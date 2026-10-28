// Qualifiers lower to predicate fns and mangled overloads.

//--- qualifiers_lower_to_predicates_and_mangled_fns
// [is-qualifies] [rs-fn-mangling] [qual-field-override]
struct Person {
    name: Str,
    surname: Str? = None,
    age: Int
}

// Predicate qualifiers become top-level fns taking borrows, named with
// their subject [fn-emit-name].
//~ emits: pub fn Surname__Person_qualifies(person: &crate::Person) -> bool
qualifier Surname of Person {
    surname: Str

    fn qualifies(person: Person) -> Bool {
        return person.surname is Str
    }
}

//~ emits: pub fn Positive__Int_qualifies(mut int: i32) -> bool
qualifier Positive of Int {
    fn qualifies(int: Int) -> Bool {
        return int > 0
    }
}

fn full_name(person: Person) -> Str {
    return person.name
}

// The overloads are named by their parameter types, the qualifier
// included where it is what tells them apart.
//~ emits: pub fn full_name__SurnamePerson(person: &crate::Person) -> String
fn full_name(person: Surname Person) -> Str {
    // Field overrides read out of the declared representation, by reference.
    //~ emits: let mut __claimed_1 = person.surname.as_ref().unwrap();
    return "${person.name} ${person.surname}"
}

fn describe(person: Person) [Console] {
    //~ emits: if crate::Surname__Person_qualifies(person)
    if person is Surname {
        println(full_name(person))
    } else {
        println(full_name(person))
    }
}

fn check(n: Int) [Console] {
    //~ emits: if crate::Positive__Int_qualifies(n)
    if n is Positive {
        println("${n} is positive")
    } else {
        println("${n} is not positive")
    }
}

fn main() [use] {
    use StdConsole
    describe(Person {name: "Roland", surname: "Elliott", age: 36})
    describe(Person {name: "Anon", age: 3})
    check(5)
    check(-2)

    let current: Int? = 3
    // `while x is T` re-binds per iteration; `T?` narrows physically.
    //~ emits: if !(current.is_some()) {
    //~ emits: let mut c = current.unwrap();
    while current is Int c {
        println("tick ${c}")
        current = if c > 1 { c - 1 } else { None }
    }

    let inner: Ok Str | Err Int = ok("yes")
    let nested: Ok (Ok Str | Err Int) | Err Bool = ok(inner)
    if nested is Ok {
        let back: Ok Str | Err Int = nested
        if back is Ok {
            println("inner ok: ${back}")
        }
    }
}

//--- predicate_qualifier_constructors_emit_plain_fns
// [qual-ctor-predicate] Predicate-qualifier constructors emit as plain
// fns after erasure; overloads on the qualified type resolve statically
// (mangled name).
export qualifier Positive of Int {
    fn qualifies(int: Int) -> Bool {
        return int > 0
    }
}

//~ emits: fn make() -> i32
export fn make() -> +Positive Int {
    return 1
}

export fn describe(x: Positive Int) -> Str {
    return "positive"
}

export fn describe(x: Int) -> Str {
    return "unknown"
}

export fn main() [use] -> None {
    use StdConsole
    // The Positive overload wins at the call site (a Copy scalar travels
    // by value [rs-copy-by-value]).
    //~ emits: crate::describe__PositiveInt(crate::make())
    println(describe(make()))
}
