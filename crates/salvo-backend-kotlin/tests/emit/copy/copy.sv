// `copy`: the Kotlin lowering of each shape a copied value can have, and the
// shapes Kotlin cannot copy yet.

//--- copy_lowers_type_directedly
// [platform-intrinsic-fn] [kt-copy] `copy` lowers type-directedly from the
// checker's resolved argument type.
// Exercises every Kotlin `copy` lowering shape (`COPY_DEMO` in
// codegen_tests.rs): identity for immutable data, `.toMutableList()` for `Mut
// List`, `.copy()` for a `Mut` struct with immutable fields, and `.copyOf()`
// for arrays — plus a fate-linked alias (`let zs = xs`) that must stay
// readable.
struct Person canbe Mut {
    name: Str,
    age: Int
}

fn main() [use] -> None {
    use StdConsole
    let s = "hi"
    // Identity for a transitively immutable type (Str).
    //~ emits: val t: String = s
    //~|
    let t = copy(s)
    println(t)
    let xs = Mut List(1, 2, 3)
    // A real copy for `Mut List<Int>`.
    //~ emits: val ys: salvo.platform.std.core.list.MutList<Int> = xs.toMutableList()
    let ys = copy(xs)
    ys.add(4)
    println("${xs.size()} ${ys.size()}")
    let p = Mut Person {name: "a", age: 1}
    // The data-class shallow copy for a `Mut` struct with immutable fields.
    //~ emits: : Person = p.copy()
    let q = copy(p)
    q.name = "b"
    println("${p.name} ${q.name}")
    let arr = array_of(1, 2)
    // Arrays are index-assignable without `Mut`, so they copy for real.
    //~ emits: : Array<Int> = arr.copyOf()
    let brr = copy(arr)
    brr[0] = 9
    println("${arr[0]} ${brr[0]}")
    let zs = xs
    println("${zs.size()}")
}

//--- copy_of_nested_mutable_type_is_an_error
// [kt-copy] [backend-never-wrong] Nested mutability Kotlin cannot copy
// correctly yet — a map's mutable values — is a codegen error, never a
// diverging copy. (A list of mutable lists used to be the example; it
// copies element-wise since 2026-10-05.)
fn main() [use] -> None {
    use StdConsole
    let m = Mut Map(("a", Mut List(1)))
    //~ refused[kt-copy:no-plan]
    let n = copy(m)
    println("${size(n)}")
}

//--- canbe_mut_field_reads_cast_and_structs_copy_deeply
// [struct-field-canbe-mut] [kt-field-canbe-mut] [kt-copy] A `canbe Mut` field is
// `Mut` exactly when its struct value is, and `copy` of a struct is deep:
// the copy's lists, nested structs included, are its own.
// (`CANBE_MUT_FIELD_DEMO` in codegen_tests.rs.)
struct Bag canbe Mut {
    n: Int,
    // Declared at the plain type, read as `Mut` through a cast.
    //~ emits: var items: List<Int>
    items: List<Int> canbe Mut,
    fixed: Mut List<Int>
}

struct Outer canbe Mut {
    bag: Bag canbe Mut,
    tag: Str
}

fn fill(b: Mut Bag) -> None => b: Mut {
    //~ emits: (b.items as salvo.platform.std.core.list.MutList<Int>)
    add(b.items, 7)
    b.n = b.n + 1
}

fn total(b: Bag) -> Int => b {
    let t = b.n
    for x in b.items {
        t = t + x
    }
    return t
}

fn main() [use] -> None {
    use StdConsole()
    let b = Mut Bag { n: 1, items: Mut List<Int>(), fixed: Mut List<Int>() }
    add(b.items, 3)
    fill(b)
    // A deep copy replaces every mutable field.
    //~ emits: .copy(items = 
    let c = copy(b)
    add(c.items, 4)
    add(c.fixed, 9)
    println("${size(b.items)} ${size(c.items)} ${size(b.fixed)} ${size(c.fixed)} ${total(b)}")
    let p = Bag { n: 0, items: List(1, 2), fixed: Mut List<Int>() }
    let q = copy(p)
    add(q.fixed, 1)
    println("${size(p.fixed)} ${size(q.fixed)} ${total(p)}")
    let o = Mut Outer { bag: Mut Bag { n: 0, items: Mut List<Int>(), fixed: Mut List<Int>() }, tag: "t" }
    add(o.bag.items, 5)
    let o2 = copy(o)
    add(o2.bag.items, 6)
    println("${size(o.bag.items)} ${size(o2.bag.items)}")
    let frozen: Bag = c
    println("${total(frozen)}")
}

//--- canbe_mut_str_field_is_a_kotlin_error
// [kt-field-canbe-mut] A `Str canbe Mut` field would hold a `String` or a
// `StringBuilder` under one property: refused on Kotlin.
//~ refused[kt-field-canbe-mut:mut-not-subtype]
struct Doc canbe Mut { text: Str canbe Mut }

fn main() [use] -> None {
    use StdConsole()
    let d = Doc { text: "a" }
    println(d.text)
}

//--- copy_of_mutable_elements_is_element_wise
// [kt-copy] A list whose elements are mutable copies **element-wise**, as
// Rust's `clone` does: mutating an element of the original leaves the copy
// alone (ROADMAP §0j step 1; it used to alias on Kotlin).
// (`COPY_ELEMENTS_DEMO` in codegen_tests.rs.)
struct Counter canbe Mut { n: Int }

fn main() [use] {
    use StdConsole()
    let xs: List<Mut Counter> = List(Mut Counter { n: 1 })
    //~ emits: .map { __c
    let ys = copy(xs)
    let e = at(xs, 0) ?: unreachable!()
    e.n = 5
    println("${(get(xs, 0) ?: unreachable!()).n} ${(get(ys, 0) ?: unreachable!()).n}")
    let ls = Mut List(Mut List(1))
    let ms = copy(ls)
    add(at(ls, 0) ?: unreachable!(), 2)
    println("${size(get(ls, 0) ?: unreachable!())} ${size(get(ms, 0) ?: unreachable!())}")
}
