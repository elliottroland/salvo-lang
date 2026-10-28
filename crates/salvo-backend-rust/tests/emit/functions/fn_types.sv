// Fn types: `once`, contracts, and how fn values travel.

//--- once_fn_params_emit_fnonce
// A `once` parameter accepts both a capture-consuming lambda (which is
// `once`-typed by construction) and a plain lambda (inverted
// subtyping); the checker guarantees at most one call.
// [fn-once] `once` fn parameters emit `impl FnOnce`.
//~ emits: pub fn run_once(console: &crate::std_core_console::Console, f: impl FnOnce(&crate::std_core_console::Console))
fn run_once(f: once () [Console] -> None) {
    f()
}

fn consume_list(v: List<Int>) [Console] -> None => !v {
    println("consumed ${size(v)} items")
}

fn main() [use] -> None {
    use StdConsole
    let xs = List(1, 2, 3)
    let g = () -> { consume_list(xs) }
    run_once(g)
    let n = 7
    let plain = () -> { println("plain ${n}") }
    run_once(plain)
    println("done")
}

//--- fn_type_contracts_emit_modes
// Contracts on fn types: a keeping contract lets the callee call the
// value repeatedly and the caller keep the argument (Rust: borrowed
// argument types, `&mut impl FnMut`); a consuming contract moves the
// argument; named fns pass through adapter closures (Rust) and
// function references (Kotlin).
// [fn-contract] Keeping contracts borrow, consuming contracts own; fn
// params are `&mut dyn FnMut`; named fns wrap in adapters.
struct Person {
    name: Str,
    age: Int
}

//~ emits: pub fn apply_keeping(f: &mut dyn FnMut(&Vec<crate::Person>) -> i32, data: &Vec<crate::Person>) -> i32
fn apply_keeping(f: (v: List<Person>) -> Int, data: List<Person>) -> Int =>[f] v => data {
    return f(data) + f(data)
}

//~ emits: pub fn apply_consuming(f: &mut dyn FnMut(Vec<crate::Person>) -> i32, mut data: Vec<crate::Person>) -> i32
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
    // An adapter: a path names the module's own `count`, whatever core
    // declares.
    //~ emits: crate::apply_keeping(&mut |__a0: &Vec<crate::Person>| crate::count(__a0), &people)
    let named = apply_keeping(count, people)
    println("named=${named}")
    let eaten = apply_consuming((v: List<Person>) -> { return size(v) }, people)
    println("eaten=${eaten}")
    println("done")
}
