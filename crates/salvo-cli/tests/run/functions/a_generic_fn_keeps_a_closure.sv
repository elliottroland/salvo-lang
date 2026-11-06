// [qual-depend] A generic fn whose result keeps a closure over a value of
// its type parameter's type, and a generic caller reaching it with one of
// its own: both compile on every backend (the Rust one bounds the parameters
// the kept closure captures). The dependent claim travels through the
// struct's type argument to the caller's list.

//--- a_generic_fn_keeps_a_closure
struct Pick {
    seed: Int
}

struct Picker<T> {
    run: (p: Pick) -> T
}

fn index_into<T>(list: NonEmpty List<T>) -> Picker<Idx(list) Int> {
    return Picker<Idx(list) Int> { run: (p: Pick) -> {
        let i = p.seed % size(list)
        if i is Idx(list) {
            return i
        }
        return unreachable!()
    } }
}

fn first_index<T>(list: NonEmpty List<T>) -> Picker<Idx(list) Int> {
    return index_into(list)
}

fn main() [use] -> None {
    use StdConsole()
    let xs = List("a", "b", "c")
    if xs is NonEmpty {
        let picker = first_index(xs)
        let i = picker.run(Pick { seed: 4 })
        println(get(xs, i))
    }
}
//= stdout
//= b
