// [fn-attached] A function declared *on* a type: written inside its body, or
// the file's fulfilment of an obligation the type declares. It is an ordinary
// overload, named by the type's selector, and it travels with the type.

// A fn written **inside** the struct body is an ordinary overload — bare calls
// and uniform calls reach it [fn-uniform] — and it is declared on the type.
struct Person {
    name: Str,
    age: Int

    fn cmp(a: Person, b: Person) [] -> Int => a, b {
        return cmp(a.age, b.age)
    }
}

fn older(a: Person, b: Person) [] -> Bool => a, b {
    return a.cmp(b) > 0
}

// [fn-attached] The other route: the file's fulfilment of an obligation the
// struct declares. Nothing moves — the fn stays top-level — and it is attached
// all the same.
struct Fulfilled : Ordered<self> {
    name: Str,
    age: Int
}

fn cmp(a: Fulfilled, b: Fulfilled) [] -> Int => a, b {
    return cmp(a.age, b.age)
}

fn older_fulfilled(a: Fulfilled, b: Fulfilled) [] -> Bool => a, b {
    return a.cmp(b) > 0
}

// [fn-attached] The selector still *reads* the attachment: `cmp@Person` names
// the fn declared on `Person`, at a call and as a value, which is what
// `cmp = cmp@Person` needs [implicit-override].
fn min_of<T>(a: T, b: T, ?Ordered<T>) [] -> T
=> !a, !b {
    if cmp(a, b) <= 0 {
        return a
    }
    return b
}

fn pick(a: Person, b: Person) [] -> Person => !a, !b {
    let _explicit = cmp@Person(a, b)
    return min_of(a, b, cmp = cmp@Person)
}

// [fn-attached] [mod-export] A **detached** fulfilment of an exported type's
// obligation has to `export` too: it is declared on a type every importer can
// use, and a private one would be unreachable there.
export struct Hidden : Ordered<self> {
    x: Int
}

//~ error[fn-attached:private-fulfilment]
fn cmp(a: Hidden, b: Hidden) [] -> Int => a, b {
    return cmp(a.x, b.x)
}

export struct Shown : Ordered<self> {
    x: Int
}

export fn cmp(a: Shown, b: Shown) [] -> Int => a, b {
    return cmp(a.x, b.x)
}

// A *private* type needs nothing: there is no importer to hide it from.
struct Private : Ordered<self> {
    x: Int
}

fn cmp(a: Private, b: Private) [] -> Int => a, b {
    return cmp(a.x, b.x)
}

//--- imported with the type, declared inside
// FILE: people.sv
// [fn-attached] It **travels with the type**: a module that imports `Person`
// and nothing else can still compare two, which is what closes
// [implicit-resolve]'s per-call-site visibility hole. Both routes travel.
export struct Person {
    name: Str,
    age: Int

    fn cmp(a: Person, b: Person) [] -> Int => a, b {
        return cmp(a.age, b.age)
    }
}
// FILE: main.sv
import people.Person

fn min_of<T>(a: T, b: T, ?Ordered<T>) [] -> T
=> !a, !b {
    if cmp(a, b) <= 0 {
        return a
    }
    return b
}

fn go(a: Person, b: Person) [] -> Person => !a, !b {
    let _direct = cmp(a, b)
    return min_of(a, b)
}

//--- imported with the type, a fulfilment
// FILE: people.sv
export struct Person : Ordered<self> {
    name: Str,
    age: Int
}

export fn cmp(a: Person, b: Person) [] -> Int => a, b {
    return cmp(a.age, b.age)
}
// FILE: main.sv
import people.Person

fn min_of<T>(a: T, b: T, ?Ordered<T>) [] -> T
=> !a, !b {
    if cmp(a, b) <= 0 {
        return a
    }
    return b
}

fn go(a: Person, b: Person) [] -> Person => !a, !b {
    let _direct = cmp(a, b)
    return min_of(a, b)
}

//--- a second canonical in another module
// FILE: people.sv
// [op-unique-shape] A second `cmp` for `Person` in another module is refused
// where it is declared (user decision O1-D, 2026-10-27): `cmp` is what `<`
// means on `Person`, and a function of that shape is declared only in
// `Person`'s own file.
export struct Person {
    name: Str,
    age: Int

    fn cmp(a: Person, b: Person) [] -> Int => a, b {
        return cmp(a.age, b.age)
    }
}
// FILE: main.sv
import people.Person

// This module's own `cmp` for the same shape.
//~ error[op-unique-shape:outside-home]
fn cmp(a: Person, b: Person) [] -> Int => a, b {
    return cmp(a.name, b.name)
}

//--- a second ordering, named and passed
// FILE: people.sv
// [op-unique-shape] [implicit-override] The remedy the refusal names: a second
// ordering is named for what it does and passed explicitly, and `cmp@Person`
// still names the one declared on the type [fn-attached]. And the canonical
// is the **default selection** even where a nearer rung has a fitting
// candidate of a *different* shape: only candidates that fit compete, so an
// unrelated `cmp` in this module is not an ambiguity.
export struct Person {
    name: Str,
    age: Int

    fn cmp(a: Person, b: Person) [] -> Int => a, b {
        return cmp(a.age, b.age)
    }
}
// FILE: main.sv
import people.Person

fn by_name(a: Person, b: Person) [] -> Int => a, b {
    return cmp(a.name, b.name)
}

fn min_of<T>(a: T, b: T, ?Ordered<T>) [] -> T
=> !a, !b {
    if cmp(a, b) <= 0 {
        return a
    }
    return b
}

fn go(a: Person, b: Person) [] -> Person => !a, !b {
    let _by_age = cmp@Person(a, b)
    let _named = by_name(a, b)
    let _by_default = cmp(a, b)
    return min_of(a, b, cmp = by_name)
}

struct Box {
    at: Int
}

fn cmp(a: Box, b: Box) [] -> Int => a, b {
    return cmp(a.at, b.at)
}

fn unrelated(a: Person, b: Person) [] -> Int => a, b {
    return cmp(a, b)
}

//--- a declaration takes no selector
// [fn-attached] A `@` on a *declaration* is refused outright: attachment is
// declaring the function on the type, not selecting from it. The diagnostic
// names both routes. (A parse error.)
struct Person {
    name: Str
}

//~ error[fn-attached:at-selector]
fn cmp@Person(a: Person, b: Person) -> Int => a, b {
    return 0
}

//--- a hand-written member returning self
// FILE: shapes.sv
// [obligation-return] [fn-attached] A hand-written fulfilment of a member that
// only returns the type is declared on it too: it is imported with the type.
export fns Default<T> {
    fn default() -> T
}

export struct Point : Default<self> {
    x: Int
}

export fn default() [] -> Point {
    return Point { x: 0 }
}
// FILE: main.sv
import shapes.Point

fn probe() [] -> Int {
    let p: Point = default()
    return p.x
}

//--- a clause stamps the group its file sees
// FILE: a.sv
// [type-identity] [obligation-by] The group a clause names is the one its
// file sees: two modules each declare `fns Shown<T>` with different members,
// and the clause stamps the imported one's (PIPELINE.md step 6).
export fns Shown<T> {
    fn cmp(x: T, y: T) -> Int
}
// FILE: b.sv
export fns Shown<T> {
    fn eq(x: T, y: T) -> Bool
}
// FILE: main.sv
import b.Shown

struct P : Shown<self> by auto {
    a: Int
}

fn same(p: P, q: P) [] -> Bool => p, q {
    return eq(p, q)
}
