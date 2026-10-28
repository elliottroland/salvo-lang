// S1: the `copy` intrinsic [platform-intrinsic-fn] [copy-fn] [rs-copy]
//
// Compile-and-run data ([test-run-data]), moved from the codegen tests.

// [is-narrowing] [rs-opt-borrow] A borrowed optional element (`&Option<P>`
// from the total `get` over a `List<P?>`) narrows through `as_ref()`, and an
// optional borrow of a scalar (`Option<&i32>`) is `.copied()` into an owned
// `Int?` — as an argument, an annotated `let` and a `copy`.
//--- borrowed_optional_elements
struct P { a: Int }

fn show(v: Int?) [] -> Str => v {
    if v is Int n {
        return "${n}"
    }
    return "-"
}

fn main() [use] -> None {
    use StdConsole()
    // The first element fixes the constructor's `T`, so it is typed `P?`.
    let first: P? = P { a: 1 }
    let xs: List<P?> = List(first, None, P { a: 3 })
    for i in indices(xs) {
        let s = get(xs, i)
        if s is P p {
            println("${p.a}")
        } else {
            println("none")
        }
    }
    let ns = List(4, 5)
    let v: Int? = get(ns, 1)
    println("${show(get(ns, 0))} ${show(v)} ${show(copy(get(ns, 7)))}")
}
//= stdout
//= 1
//= none
//= 3
//= 4 5 -

// [struct-field-canbe-mut] The Kotlin backend's `canbe Mut` field case, run on
// Rust for parity (`Mut` is erased here; the checker does the work).
//--- canbe_mut_field

struct Bag canbe Mut {
    n: Int,
    items: List<Int> canbe Mut,
    fixed: Mut List<Int>
}

struct Outer canbe Mut {
    bag: Bag canbe Mut,
    tag: Str
}

fn fill(b: Mut Bag) -> None => b: Mut {
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
//= stdout
//= 2 3 0 1 12
//= 0 1 3
//= 1 2
//= 16

//--- copy

struct Person canbe Mut {
    name: Str,
    age: Int
}

fn main() [use] -> None {
    use StdConsole
    let s = "hi"
    let t = copy(s)
    println(t)
    let xs = Mut List(1, 2, 3)
    let ys = copy(xs)
    ys.add(4)
    println("${xs.size()} ${ys.size()}")
    let p = Mut Person {name: "a", age: 1}
    let q = copy(p)
    q.name = "b"
    println("${p.name} ${q.name}")
    let arr = array_of(1, 2)
    let brr = copy(arr)
    brr[0] = 9
    println("${arr[0]} ${brr[0]}")
    let zs = xs
    println("${zs.size()}")
}
//= stdout
//= hi
//= 3 4
//= a b
//= 1 9
//= 3

// [kt-copy] The Kotlin backend's element-wise copy case, run on Rust for
// parity: `clone` is deep, so the copy never aliases (ROADMAP §0j step 1).
//--- copy_elements

struct Counter canbe Mut { n: Int }

fn main() [use] {
    use StdConsole()
    let xs: List<Mut Counter> = List(Mut Counter { n: 1 })
    let ys = copy(xs)
    let e = at(xs, 0) ?: unreachable!()
    e.n = 5
    println("${(get(xs, 0) ?: unreachable!()).n} ${(get(ys, 0) ?: unreachable!()).n}")
    let ls = Mut List(Mut List(1))
    let ms = copy(ls)
    add(at(ls, 0) ?: unreachable!(), 2)
    println("${size(get(ls, 0) ?: unreachable!())} ${size(get(ms, 0) ?: unreachable!())}")
}
//= stdout
//= 5 1
//= 2 1

// [comptime-generic] [implicit-resolve-body] The Kotlin backend's generic
// stamp case, run on Rust: a global shadowed by an implicit of its name is
// called through `crate::`.
//--- generic_stamp

struct Inner : Ordered<self> by auto, Hashed<self> by auto, ToStr<self> by auto {
    k: Int
}

struct Wrapper<T> : Hashed<self> by auto, Ordered<self> by auto, ToStr<self> by auto {
    value: T,
    inner: Inner,
    n: Int
}

fn main() [use] -> None {
    use StdConsole()
    let a = Wrapper<Str> { value: "a", inner: Inner { k: 1 }, n: 1 }
    let b = Wrapper<Str> { value: "a", inner: Inner { k: 2 }, n: 0 }
    println("${cmp(a, b)} ${eq(a, b)} ${hash(a) == hash(copy(a))}")
    println("${a < b} ${a == b} ${a != b} ${a}")
    run_h()
}

fn h<T>(a: T, i: Inner, ?cmp: (T, T) -> Int) [] -> Int => a, i {
    return cmp(1, 2) + cmp(a, a) + cmp(i, i)
}

fn run_h() [use] -> None {
    use StdConsole()
    println("${h("x", Inner { k: 3 })}")
}
//= stdout
//= -1 false true
//= true false true Wrapper { value: a, inner: Inner { k: 1 }, n: 1 }
//= -1

// [op-wrap] [op-bits] Integer arithmetic wraps as on the JVM, and the `bit_`
// functions agree with Kotlin's (ROADMAP §0j step 2).
//--- int_wrap_and_bits

fn main() [use] {
    use StdConsole()
    let big = 2147483647
    let wrapped = big + 1
    let neg = -wrapped
    let i = big
    i++
    let l = 9223372036854775807L
    println("${wrapped} ${neg} ${i} ${l + 1L} ${big * 2}")
    println("${bit_and(12, 10)} ${bit_or(12, 10)} ${bit_xor(12, 10)} ${bit_not(0)}")
    println("${bit_shl(1, 33)} ${bit_shr(-16, 2)} ${bit_ushr(-16, 28)} ${bit_ushr(-1L, 60)}")
    println("${mix_hash(17L, 5L)}")
}
//= stdout
//= -2147483648 -2147483648 -2147483648 -9223372036854775808 -2
//= 8 14 6 -1
//= 2 -4 15 15
//= 532

// [cmp-carry] [fn-type-named] A collection constructor carries the identity
// it is given by `hash =`/`eq =`, empty or not (ROADMAP §0j step 1; a
// literal took it from the position until [col-no-literal]).
//--- literal_identity

fn fold_hash(s: Str) [] -> Long => s {
    return hash(size(s))
}
fn fold_eq(a: Str, b: Str) [] -> Bool => a, b {
    return size(a) == size(b)
}

fn main() [use] {
    use StdConsole()
    let m: Mut Map<Str, Int>(fold_hash, fold_eq) = Mut Map("ab": 1, hash = fold_hash, eq = fold_eq)
    put(m, "cd", 2)
    let e: Mut Map<Str, Int>(fold_hash, fold_eq) = Mut Map<Str, Int>(hash = fold_hash, eq = fold_eq)
    put(e, "ab", 1)
    put(e, "cd", 2)
    let s: Mut Set<Str>(fold_hash, fold_eq) = Mut Set("ab", hash = fold_hash, eq = fold_eq)
    add(s, "cd")
    let t: Mut Set<Str>(fold_hash, fold_eq) = Mut Set<Str>(hash = fold_hash, eq = fold_eq)
    add(t, "ab")
    add(t, "cd")
    println("${size(m)} ${size(e)} ${size(s)} ${size(t)}")
}
//= stdout
//= 1 1 1 1

// [obligation-by] [rs-shadowed-call] The Kotlin backend's mixed-members
// case, run on Rust: the stamped `to_str` calls `crate::std_core_list::to_str`,
// since its own implicit `to_str` shadows the name.
//--- mixed_members

struct Bag<T> : Eq<self>, ToStr<self> by auto {
    items: List<T>,
    n: Int
}

// Order-blind equality, by hand, beside the stamped `to_str`.
fn eq<T>(a: Bag<T>, b: Bag<T>, ?Eq<T>) [] -> Bool => a, b {
    if size(a.items) != size(b.items) {
        return false
    }
    for x in a.items {
        if !contains_eq(b.items, x) {
            return false
        }
    }
    return true
}

fn contains_eq<T>(xs: proj List<T>, x: proj T, ?Eq<T>) [] -> Bool => xs, x {
    for y in xs {
        if eq(x, y) {
            return true
        }
    }
    return false
}

struct Point : Hashed<self> by auto, ToStr<self> {
    x: Int,
    y: Int
}

fn to_str(p: Point) [] -> Str => p {
    return "(${p.x}, ${p.y})"
}

fn main() [use] -> None {
    use StdConsole()
    let a = Bag<Int> { items: List(1, 2), n: 0 }
    let b = Bag<Int> { items: List(2, 1), n: 5 }
    println("${a == b} ${a}")
    let p = Point { x: 1, y: 2 }
    let s = Set(p, Point { x: 1, y: 2 })
    println("${p} ${size(s)}")
}
//= stdout
//= true Bag { items: [1, 2], n: 0 }
//= (1, 2) 1

// [struct-opaque] The Kotlin backend's opaque-struct case, run on Rust.
//--- opaque_struct
// FILE: bag.sv

export opaque struct Bag : Eq<self> by auto, ToStr<self> by auto canbe Mut {
    items: List<Int> canbe Mut,
    total: Int
}

export fn bag() [] -> Mut Bag {
    return Mut Bag { items: Mut List<Int>(), total: 0 }
}

export fn put(b: Mut Bag, x: Int) [] -> None => b: Mut, !x {
    b.total = b.total + x
    add(b.items, x)
}

export fn total(b: Bag) [] -> Int => b {
    return b.total
}
// FILE: main.sv

import bag.Bag
import bag.bag
import bag.put
import bag.total

struct Shelf : Eq<self> by auto, ToStr<self> by auto {
    b: Bag
}

fn main() [use] -> None {
    use StdConsole()
    let b = bag()
    put(b, 3)
    put(b, 4)
    let c = copy(b)
    put(c, 1)
    let s = Shelf { b: copy(c) }
    println("${total(b)} ${total(c)} ${b == c} ${s}")
}
//= stdout
//= 7 8 false Shelf { b: Bag { items: [3, 4, 1], total: 8 } }

// [struct-slot] [cmp-binder] A struct with a fn slot, run on Rust: a
// captured binder is a trailing reference parameter, which counts against
// lifetime elision for `top`'s `proj(r)` return [proj-readonly-return].
//--- struct_slot

struct Ranked<T>(?cmp: (T, T) -> Int) canbe Mut {
    items: List<T> canbe Mut
}

fn ranked<T>(?cmp: (T, T) -> Int) [] -> Mut Ranked<T>(?cmp) {
    return Mut Ranked<T>(?cmp) { items: Mut List<T>() }
}

fn push<T>(r: Mut Ranked<T>(?cmp), x: T) [] -> None => r: Mut, !x {
    let i = 0
    while i < size(r.items) {
        let y = get(r.items, i) ?: unreachable!()
        if x < y {
            break
        }
        i = i + 1
    }
    insert_at(r.items, i, x)
}

fn top<T>(r: Ranked<T>(?cmp)) [] -> (proj(r) T)? => r {
    return get(r.items, 0)
}

fn backwards(a: Int, b: Int) [] -> Int {
    return cmp(b, a)
}

fn main() [use] -> None {
    use StdConsole()
    let up = ranked<Int>()
    push(up, 3)
    push(up, 1)
    push(up, 2)
    let down = ranked<Int>(cmp = backwards)
    push(down, 3)
    push(down, 1)
    push(down, 2)
    println("${up.items} ${down.items} ${top(up) ?: unreachable!()} ${top(down) ?: unreachable!()}")
}
//= stdout
//= [1, 2, 3] [3, 2, 1] 1 3
