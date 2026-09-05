import random.Random
import random.DefaultRandom

qualifier NonEmpty<T> of List<T> with Mut<T> {
    fn qualifies(list: List<T>) -> Bool {
        return list.size() > 0
    }

    // Refinements allow us to update deductions without having to run the
    // `qualifies` function each time.
    // refn add(list: Mut List<T>, elem: T) -> [list: Mut NonEmpty] None
}

qualifier Positive of Int {
    fn qualifies(int: Int) -> Bool {
        return int > 0
    }
}

provenance qualifier Limited<T> of T

// Represents an individual person uniquely picked out by their [id_number]
struct Person {
    id_number: Str,
    // The first name of the person
    name: Str,
    // The last name of the person, if we know it
    surname: Str? = None,
    // The age of the person
    age: Int
}

// Predicate qualifiers support constructor functions like constructive
// qualifiers do; the same restriction applies: constructors must live in
// the same file as the qualifier definition.
// Because [first] is given, we know that the result will be non-empty.
// fn list<T>(first: T, ...rest: T[]) -> List<T> as NonEmpty {
//     return init_list(first, ...rest)
// }

fn first<T>(list: List<T>) -> T? {
    return list.get(0)
}

fn first<T>(list: NonEmpty List<T>) -> T {
    return list.get(0)!
}

fn give_back<T>(list: NonEmpty List<T>) -> List<T> {
    return copy(list)
}

// Does something
// fn remove_first<T>(list: NonEmpty Mut List<T>) -> [list: Mut] T {
//     return list.remove_at(0)
// }

provenance qualifier T of Int

handler NonRandom of Random {
    fn random() -> Double {
        return 1.0
    }
}

struct FileHandle canbe Linear {
    fd: Int
}

fn main() [use] {
    use StdOutConsole
    use DefaultRandom

    let person = Person { id_number: "1231232", name: "Roland", age: number }
    let name = person.name
    // let person2 = Person { name: person.name, id_number: "34578346584", age: 16 }
    discard(person)
    println(name)

    try {
        let fh = get_fh()
        defer { close(fh) }
        throws()
    }

    let n = try_get_number()
    let message = when n {
        is Ok { "the number: ${n}" }
        is Err { "an error: ${n}" }
        is Bool { "a boolean: ${n}" }
        is None { "something else? " }
    }

    let strings = mutable_list("name", "surname", "something")

    if strings is NonEmpty {
        println("First element length: ${strings.first().size()}")
        give_back(strings)
        let s = strings.remove_first()
    }

    // println("First element length: ${strings.first().size()}")

    let (a, b) = (1, 2)
    let c = a.add(b)
    // close(fh)
}

fn close(fh: FileHandle) -> [] None {
    // Close it
    discard(fh)
}

fn get_fh() -> FileHandle {
    return FileHandle { fd: 10 }
}

fn add(a: Int, b: Int) -> [] Int {
    return a + b
}

fn make_random() [Random, Console] -> Int {
    println("About to generate random number")
    if random() < 0.5 {
        return -1
    }
    return 1
}

fn try_get_number() [Random] -> Ok Int | Err Str | Bool | None {
    if random() < 0 {
        return ok(1)
    } else {
        return err("it was negative")
    }
}

fn may_fail(fh: FileHandle) [Abort<Str>] -> [] None {
    throws()
}

fn throws() [Abort<Str>] {
    abort("Something went wrong")
}