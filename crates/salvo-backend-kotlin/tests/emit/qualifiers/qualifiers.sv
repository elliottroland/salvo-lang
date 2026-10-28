// Qualifiers: predicate qualifiers as `qualifies` functions, the mangled
// names of qualifier overloads, field overrides, and constructor functions.

//--- qualifiers_lower_to_predicates_and_mangled_overloads
// [is-qualifies] [fn-emit-name] [qual-field-override]
// The M4 demo (`QUALIFIERS_DEMO` in codegen_tests.rs): predicate qualifiers
// (`qualifies` calls at runtime), struct-field overrides with casts,
// qualifier-based overloading (with erasure mangling), `while x is T`, and
// qualified union groups.
struct Person {
    name: Str,
    surname: Str? = None,
    age: Int
}

// Predicate qualifiers become top-level `qualifies` functions.
//~ emits: fun Surname_qualifies(person: Person): Boolean
qualifier Surname of Person {
    surname: Str

    fn qualifies(person: Person) -> Bool {
        return person.surname is Str
    }
}

//~ emits: fun Positive_qualifies(int: Int): Boolean
qualifier Positive of Int {
    fn qualifies(int: Int) -> Bool {
        return int > 0
    }
}

fn full_name(person: Person) -> Str {
    return person.name
}

// The qualified overload is mangled; the checker routes the narrowed call to
// it and the unqualified call to the base name.
//~ emits: fun fullName__SurnamePerson(person: Person): String
fn full_name(person: Surname Person) -> Str {
    // Field overrides read through a narrowing binding justified by the
    // claim [qual-field-override].
    //~ emits: : String = person.surname!!
    return "${person.name} ${person.surname}"
}

fn describe(person: Person) [Console] {
    // `is` predicate checks call them.
    //~ emits: if (salvo.main.Surname_qualifies(person))
    if person is Surname {
        //~ emits: println(console, salvo.main.fullName__SurnamePerson(person))
        println(full_name(person))
    } else {
        //~ emits: println(console, salvo.main.fullName__Person(person))
        println(full_name(person))
    }
}

fn check(n: Int) [Console] {
    //~ emits: if (salvo.main.Positive_qualifies(n))
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
    // `while x is T` lowers to a test-inside loop with a per-iteration
    // binding.
    //~ emits: if (!((current != null))) {
    //~ emits: val c: Int = current!!
    while current is Int c {
        println("tick ${c}")
        current = if c > 1 { c - 1 } else { None }
    }

    let inner: Ok Str | Err Int = ok("yes")
    let nested: Ok (Ok Str | Err Int) | Err Bool = ok(inner)
    if nested is Ok {
        // A qualified union group is physically the inner union.
        //~ emits: val back: Union2<String, Int> = nested_
        let back: Ok Str | Err Int = nested
        if back is Ok {
            println("inner ok: ${back}")
        }
    }
}

//--- predicate_qualifiers_may_have_constructors
// [qual-ctor-predicate] Predicate qualifiers may have constructor
// functions (same-file rule still applies): the constructor asserts the
// predicate by construction, qualifiers erase, and overloads on the
// qualified type resolve statically.
export qualifier Positive of Int {
    fn qualifies(int: Int) -> Bool {
        return int > 0
    }
}

// The constructor is a plain fn after erasure; the call site resolves to the
// Positive overload (mangled name).
//~ emits: fun make(): Int
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
    //~ emits: salvo.main.describe__PositiveInt(salvo.main.make())
    println(describe(make()))
}

//--- widening_materializes_the_peel_kotlin
// [qual-lift] Kotlin materializes the same peel with a shadowing `val`
// (the alternative, a fresh name, would need every read rewritten).
// [qual-lift] `^` tests the arm *and* removes the claim, so a branch can
// `when` the union inside a qualified one — the shape `try` outcomes produce.
// (`WIDEN_DEMO` in codegen_tests.rs.)
import std.throw

fn wrapped(n: Int) [Throw<Str>] -> Ok Int | Err Str {
    if n < 0 {
        throw("negative")
    }
    if n == 0 {
        return err("zero")
    }
    return ok(n)
}

fn limit(n: Int) [Throw<Int>] -> Int {
    if n > 4 {
        throw(n)
    }
    return n
}

fn describe(n: Int) [Console] -> None {
    let nested = try { wrapped(n) }
    when nested {
        //~ emits: : Union2<Int, String> = ((nested as Union2.U1<*, *>).value as Union2<Int, String>)
        is ^Ok {
            when nested {
                is Ok {
                    println("value ${nested}")
                }
                is Err {
                    println("error ${nested}")
                }
            }
        }
        is Thrown {
            println("thrown ${nested}")
        }
    }
}

fn main() [use] -> None {
    use StdConsole
    describe(7)
    describe(0)
    describe(-1)
    let mixed = try {
        let a = wrapped(1)
        limit(9)
    }
    when mixed {
        is Ok {
            println("mixed ok ${mixed}")
        }
        is ^Thrown {
            when mixed {
                is Str {
                    println("message text ${mixed}")
                }
                is Int {
                    println("message number ${mixed}")
                }
            }
        }
    }
}
