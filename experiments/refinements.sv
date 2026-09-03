import core.list as init_list
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

// Predicate qualifiers support constructor functions like constructive
// qualifiers do; the same restriction applies: constructors must live in
// the same file as the qualifier definition.
fn list<T>(first: T, ...rest: T[]) -> List<T> as NonEmpty {
    return init_list(first, ...rest)
}

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
fn remove_first<T>(list: NonEmpty Mut List<T>) -> [list: Mut] T {
    return list.remove_at(0)
}

handler NonRandom of Random {
    fn random() -> Double {
        return 1.0
    }
}

fn main() [use] {
    use StdOutConsole
    use DefaultRandom

    let n = try_get_number()
    if n is Ok {

    }

    let strings = mutable_list("name", "surname", "something")
    if strings is NonEmpty {
        println("First element length: ${strings.first().size()}")
        give_back(strings)
        // TODO: the empty deduction from `give_back` should make `strings` unusable here?
        let s = strings.remove_first()
    }

    // println("First element length: ${strings.first().size()}")

    let (a, b) = (1, 2)
    let c = a.add(b)
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

fn try_get_number() [Random] -> Ok Int | Err Str | None {
    if random() < 0 {
        return ok(1)
    } else {
        return err("it was negative")
    }
}