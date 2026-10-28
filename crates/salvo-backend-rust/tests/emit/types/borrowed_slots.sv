// Borrowed optionals and unions of borrows in kept owned slots.

//--- borrowed_optionals_in_owned_optional_slots
// [proj-opt-slot] [rs-opt-borrow] A borrowed optional — `get`'s answer, an
// optional handle, a `Mut Str` element — fits a kept owned `T?` parameter,
// of a fn or an effect member; owned optionals, literals and `None` still
// do; `copy` of one is an owned optional. On Rust the parameter is
// `Option<&T>`, so the body's lets, narrowings and `?:` read the
// borrow it holds.
struct Entity canbe Mut { name: Str, hp: Int }

// A kept optional is the optional of a borrow.
//~ emits: pub fn inner(mut s: Option<&String>) -> String
fn inner(s: Str?) -> Str {
    return s ?: "-"
}

fn outer(s: Str?) -> Str {
    let t = s
    let u = inner(t)
    let n = if s is Str v { to_upper(v) } else { "?" }
    let c = copy(s)
    let none = if s is None { "absent" } else { "present" }
    return "${u} ${n} ${inner(c)} ${none}"
}

fn named(e: Entity?) -> Str {
    if e is Entity f {
        return "${f.name}:${f.hp}"
    }
    return "nobody"
}

fn view(s: Str?) -> (proj(s) Str)? => s {
    return s
}

effect Shower {
    // An effect member's too.
    //~ emits: fn show(&self, s: Option<&String>);
    fn show(s: Str?) -> None => s
}

handler PrintShower [Console] of Shower {
    fn show(s: Str?) -> None => s {
        println("shown ${inner(s)}")
    }
}

fn main() [use] {
    use StdConsole()
    use PrintShower()
    let xs: List<Str> = List("abc", "de")
    println(outer(get(xs, 0)))
    println(outer(get(xs, 9)))
    let o: Str? = "own"
    // An owned optional lends with `.as_ref()`.
    //~ emits: (&o).as_ref()
    println(outer(o))
    println(outer("lit"))
    println(outer(None))
    let ws: List<Mut Str> = List(Mut Str("w"))
    println(outer(get(ws, 0)))
    println(outer(at(ws, 0)))
    let cw = copy(get(ws, 0))
    println(outer(cw))
    let es: List<Mut Entity> = List(Mut Entity { name: "e", hp: 3 })
    println("${named(get(es, 0))} ${named(at(es, 0))} ${named(get(es, 4))} ${named(None)}")
    let h = at(es, 0)
    println(named(h))
    let ce = copy(get(es, 0))
    let hh = h ?: unreachable!()
    hh.hp = 40
    println("${named(ce)} ${named(get(es, 0))}")
    println(inner(view(get(xs, 1))))
    show(get(xs, 0))
    show(o)
}

//--- unions_of_borrows
// [proj-opt-slot] [rs-opt-borrow] Unions of borrows: a `proj Mut A | B`
// value, an owned union and an optional `get` all reach kept `A | B` and
// `(A | B)?` parameters, which bind, narrow and pass the union on; a `Mut A
// | Mut B` parameter is mutated through its arm; `copy` of a borrowed union
// is an owned one.
struct A canbe Mut { n: Int }
struct B canbe Mut { s: Str }

//~ emits: pub fn describe(mut x: crate::unions::Union2<&crate::A, &crate::B>) -> String
fn describe(x: A | B) -> Str => x {
    if x is A {
        return "a${x.n}"
    }
    if x is B {
        return "b${x.s}"
    }
    return "?"
}

fn relay(x: A | B) -> Str => x {
    let y = x
    return "${describe(y)}/${describe(x)}"
}

fn maybe(x: (A | B)?) -> Str => x {
    if x is None {
        return "none"
    }
    if x is A {
        return "A${x.n}"
    }
    return "other"
}

//~ emits: x: crate::unions::Union2<&mut crate::A, &mut crate::B>
fn bump(x: Mut A | Mut B) -> None => x: None {
    if x is A {
        x.n = x.n + 1
    }
    if x is B {
        x.s = "${x.s}+"
    }
}

fn pick(flag: Bool, xs: List<Mut A>, b: B) -> Str => xs, b {
    let v = if flag { get(xs, 0) ?: unreachable!() } else { b }
    return relay(v)
}

fn main() [use] {
    use StdConsole()
    let xs: List<Mut A> = List(Mut A { n: 1 })
    let b = B { s: "x" }
    println("${pick(true, xs, b)} ${pick(false, xs, b)}")
    let own: A | B = A { n: 7 }
    println("${describe(own)} ${relay(own)} ${maybe(own)} ${maybe(None)} ${maybe(get(xs, 0))} ${maybe(get(xs, 3))}")
    let m: Mut A | Mut B = Mut B { s: "q" }
    bump(m)
    bump(m)
    let h = at(xs, 0) ?: unreachable!()
    let w: Mut A | Mut B = Mut A { n: 0 }
    bump(w)
    println("${describe(m)} ${describe(w)} ${(get(xs, 0) ?: unreachable!()).n}")
    let c = copy(if true { get(xs, 0) ?: unreachable!() } else { b })
    println(describe(c))
    let _ = h
}

//--- mut_optional_slots
// [proj-opt-slot] [rs-opt-borrow] A `Mut T?` parameter takes an optional
// handle, a bound one more than once, an owned optional and `None`; it is
// handed on to another `Mut T?` parameter. On Rust it is `Option<&mut T>`.
struct Entity canbe Mut { hp: Int }

//~ emits: pub fn heal(mut e: Option<&mut crate::Entity>) -> i32
fn heal(e: Mut Entity?) -> Int => e: Mut {
    if e is Entity {
        e.hp = e.hp + 1
        return e.hp
    }
    return -1
}

fn heal_twice(e: Mut Entity?) -> None => e: Mut {
    heal(e)
    heal(e)
}

fn shout(s: Mut Str?) -> None => s: Mut {
    if s is Mut Str {
        append(s, "!")
    }
}

fn main() [use] {
    use StdConsole()
    let es: List<Mut Entity> = List(Mut Entity { hp: 3 }, Mut Entity { hp: 10 })
    println("${heal(at(es, 0))} ${heal(at(es, 9))}")
    let h = at(es, 1)
    heal(h)
    heal_twice(h)
    heal_twice(at(es, 0))
    let own: Mut Entity? = Mut Entity { hp: 100 }
    heal(own)
    heal(None)
    let ws: List<Mut Str> = List(Mut Str("hey"))
    shout(at(ws, 0))
    shout(at(ws, 0))
    println("${(get(es, 0) ?: unreachable!()).hp} ${(get(es, 1) ?: unreachable!()).hp} ${(own ?: unreachable!()).hp} ${get(ws, 0) ?: unreachable!()}")
}
