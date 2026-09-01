import core.list as init_list
import random.Random
import random.DefaultRandom

qualifier NonEmpty<T> of List<T> with Mut<T> {
    fn qualifies(list: List<T>) -> Bool {
        return list.size > 0
    }

    // Refinements allow us to update deductions without having to run the
    // `qualifies` function each time.
    // refn add(list: Mut List<T>, elem: T) -> [list: Mut NonEmpty] None
}

// TODO: We should support qualifer constructors even for predicate qualifiers, so that the following is valid
//       the same restriction still applies, though, namely that the constructors must be in the same file as the
//       qualifier definition.
// fn list<T>(first: T, ...rest: T[]) -> List<T> as NonEmpty {
//     return init_list(first, ...rest)
// }

fn first<T>(list: List<T>) -> T? {
    return list.get(0)
}

fn first<T>(list: NonEmpty List<T>) -> T {
    return list.get(0)!
}

fn main() [use] {
    use StdOutConsole
    use DefaultRandom

    let strings = list("name", "surname", "something")
    if strings is NonEmpty {
        println("First element length: ${strings.first().size()}")
    }

    // This line should fail
    // println("First element length: ${strings.first().size()}")

    let (a, b) = (1, 2)
    let c = a.add(b)

    // TODO: Because `add` deduces an empty result, `a` shouldn't be accessible here -- this should be a compile time error
    println("${a}")
}

fn add(a: Int, b: Int) -> [] Int {
    return a + b
}

fn random() [Random] -> Int {
    if random() < 0.5 {
        return -1
    }
    // TODO: Compiler should complain that not all branches return something
}