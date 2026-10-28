// Deductions decide how parameters and bindings travel in Rust: borrowed,
// borrowed mutably, moved, or cloned.

//--- parameter_modes
// [rs-borrows] Deductions decide parameter modes: kept params borrow,
// `Mut` params borrow mutably, omitted params move.
// Kept -> `&`; kept+Mut -> `&mut`; omitted -> owned (move).
//~ emits: pub fn read(list: &Vec<i32>) -> i32
export fn read(list: List<Int>) -> Int => list {
    return list.size()
}

//~ emits: pub fn fill(list: &mut Vec<i32>, mut n: i32)
export fn fill(list: Mut List<Int>, n: Int) -> None => list: Mut {
    add(list, n)
}

//~ emits: pub fn consume(mut list: Vec<i32>) -> i32
export fn consume(list: List<Int>) -> Int => !list {
    return list.size()
}

export fn main() [use] -> None {
    use StdConsole
    let items: Mut List<Int> = Mut List(1, 2)
    // Call sites render the matching argument shapes; the moved arg
    // passes by value.
    //~ emits: crate::fill(&mut items, 3i32)
    fill(items, 3)
    //~ emits: crate::read(&items)
    println("${read(items)}")
    //~ emits: crate::consume(items)
    println("${consume(items)}")
}

//--- copy_lowers_to_clone
// The same shape as the Kotlin copy demo: identity/clone lowering per
// type, plus a fate-linked alias (`let zs = xs`) whose source must stay
// physically valid ([fate-link]: linked bindings clone, not move).
// [platform-intrinsic-fn] [rs-copy] `copy` lowers to
// `.clone()` on the argument's place; a fate-linked `let` from a bare
// identifier clones instead of moving [fate-link].
struct Person canbe Mut {
    name: Str,
    age: Int
}

fn main() [use] -> None {
    use StdConsole
    let s = "hi"
    // `copy` clones the place, whatever the type.
    //~ emits: let mut t: String = (s).clone();
    let t = copy(s)
    println(t)
    let xs = Mut List(1, 2, 3)
    //~ emits: let mut ys: Vec<i32> = (xs).clone();
    let ys = copy(xs)
    ys.add(4)
    println("${xs.size()} ${ys.size()}")
    let p = Mut Person {name: "a", age: 1}
    //~ emits: let mut q: crate::Person = (p).clone();
    let q = copy(p)
    q.name = "b"
    println("${p.name} ${q.name}")
    let arr = array_of(1, 2)
    //~ emits: let mut brr: Vec<i32> = (arr).clone();
    let brr = copy(arr)
    brr[0] = 9
    println("${arr[0]} ${brr[0]}")
    // The fate-linked alias is a real borrow since S3
    // [rs-borrow-locals]: both `zs` and `xs` stay usable, no clone.
    //~ emits: let mut zs = &xs;
    let zs = xs
    println("${zs.size()}")
}

//--- move_mode_bindings
// The flagship zero-clone consuming pipeline: `persons` is inferred
// moved (the `name` binding takes ownership through the loop binding),
// the loop iterates by value, the field moves out, and the result moves
// up — no clones anywhere. Plus a mutation-driven move-mode binding
// (`ys` from `xs`) and a per-iteration consumed loop binding.
// [fate-move-mode] Move-mode bindings and loops emit real moves: the
// claimed parameter is taken by value, the loop iterates by value, the
// field projection partial-moves, and the binding chain never clones.
struct Person {
    name: Str,
    age: Int
}

// The claimed parameter is owned (moved in).
//~ emits: pub fn longest_name(mut persons: Vec<crate::Person>) -> String
// The whole pipeline is clone-free (the whole program is).
//~ !emits: .clone()
fn longest_name(persons: List<Person>) -> Str
=> !persons {
    let longest = ""
    // Move-mode loop: by value, no clone.
    //~ emits: for mut person in persons.into_iter() {
    for person in persons {
        // Move-mode bindings: a real partial move and a real move.
        //~ emits: let mut name: String = person.name;
        let name = person.name
        if size(name) > size(longest) {
            longest = name
        }
    }
    return longest
}

fn consume(text: Str) -> None => !text {
}

fn main() [use] -> None {
    use StdConsole
    let people = List(Person {name: "Ada", age: 36}, Person {name: "Grace", age: 45})
    println(longest_name(people))
    for s in List("x", "y") {
        consume(s)
    }
    let xs = Mut List(1, 2)
    //~ emits: let mut ys: Vec<i32> = xs;
    let ys = xs
    ys.add(3)
    println("${ys.size()}")
}

//--- borrow_mode_bindings
// Borrow-mode bindings and loops emit real borrows: reading through a
// kept parameter never clones — the collection iterates by reference,
// the field binding holds `&T`, and reads thread through the
// reference-binding rendering.
// [rs-borrow-locals] Borrow-mode bindings from pure places emit `&T`
// locals; borrow-mode loops over concrete non-union elements iterate by
// reference; the read-only pipeline is clone-free.
struct Person {
    name: Str,
    age: Int
}

//~ emits: pub fn count_long(persons: &Vec<crate::Person>) -> i32
// The read-only pipeline is clone-free (the whole program is).
//~ !emits: .clone()
fn count_long(persons: List<Person>) -> Int => persons {
    let total = 0
    // By-reference iteration over the borrowed parameter.
    //~ emits: for mut person in persons.iter() {
    for person in persons {
        //~ emits: let mut n = &person.name;
        let n = person.name
        if size(n) > 3 {
            total = total + 1
        }
    }
    return total
}

fn poison_guards_the_borrow() -> Int {
    let xs = Mut List(1, 2)
    // Borrow-mode alias of an owned local.
    //~ emits: let mut ys = &xs;
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

//--- derived_returns
// Zero-copy accessors across fn boundaries: a derived return borrows
// the kept parameter (elided lifetime with one reference parameter, a
// generated `'a` with more), std's `first` is clone-free, and the
// caller's narrowing works on the borrowed result.
// [proj-readonly-return] Derived returns emit borrows: elided lifetime for a
// single reference parameter, a generated `'a` when there are more; the
// std `first` is clone-free.
struct Person {
    name: Str,
    age: Int
}

//~ emits: pub fn find_adult(persons: &Vec<crate::Person>) -> Option<&crate::Person>
fn find_adult(persons: List<Person>) -> proj(persons) Person? => persons {
    for person in persons {
        if person.age >= 18 {
            //~ emits: return Some(person);
            return person
        }
    }
    return None
}

//~ emits: pub fn head_of<'a>(persons: &'a Vec<crate::Person>, tag: &String) -> Option<&'a crate::Person>
fn head_of(persons: List<Person>, tag: Str) -> proj(persons) Person? => persons, tag {
    //~ emits: return crate::std_core_list::first_platform::<crate::Person>(persons);
    return first(persons)
}

fn main() [use] -> None {
    use StdConsole
    let people = List(Person {name: "Kid", age: 9}, Person {name: "Grace", age: 45})
    let adult = find_adult(people)
    if adult is Person a {
        println("adult: ${a.name}")
    }
    let head = head_of(people, "x")
    if head is Person h {
        println("head: ${h.name}")
    }
    println("done")
}
