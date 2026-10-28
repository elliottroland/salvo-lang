// one name, several declarations: type identity across modules
//
// Compile-and-run data ([test-run-data]), moved from the CLI's run tests.

// [type-identity] [net-wire-format] Two modules declare `Token`: `alpha`'s,
// which holds the plain key, can cross the wire, and `beta`'s cannot.
// `beta`'s `Holder { t: Token }` must be judged by `beta`'s own `Token`, so
// it gets no codec. Before 2026-10-20 the wire predicate read the field by
// its written name, found `alpha`'s, and gave `Holder` a codec that asked
// for one `beta.Token` never had: no such `__Codec_Token` on Kotlin, no
// `__Wire` impl on Rust (ROADMAP §2d item 8).
//--- a_clashing_type_name_is_judged_by_its_own_declaration
// FILE: alpha.sv
export struct Word {
    w: Str
}

export type Token = Word | Int
// FILE: beta.sv
export noremote struct Token {
    n: Int
}

export struct Holder {
    t: Token
}

export fn make(n: Int) [] -> Holder {
    return Holder { t: Token { n: n } }
}

export fn read(h: Holder) [] -> Int => h {
    return h.t.n
}
// FILE: main.sv
import alpha.Token
import alpha.Word
import beta.make
import beta.read

fn show(t: Token) [] -> Str => t {
    if t is Word {
        return t.w
    }
    return "${t}"
}

fn main() [use] -> None {
    use StdConsole()
    let t: Token = Word { w: "hi" }
    println("${show(t)} ${read(make(7))}")
}
//= stdout
//= hi 7

// [type-identity] One type name in three modules — a program's own `Token`,
// two libraries' — with an effect named like std's `time.Clock`, an aliased
// import (`import lib.b.Token as BToken`), a union over one library's
// `Token`, a wire codec and auto equality on the other's, and an actor
// protocol over the program's own. Every stage after resolution knows which
// declaration each use means.
//--- type_names_clash_across_modules
// FILE: lib/a.sv
import std.codec
// A `Token` with a wire form and auto equality, and an effect named like
// std's `time.Clock`.
import std.net

export struct Token : Hashed<self> by auto { id: Int }

export effect Clock {
    fn now() -> Long
}

export handler Fixed(at: Long) of Clock {
    fn now() -> Long { return copy(at) }
}

export fn make_a() [] -> Token {
    return Token { id: 1 }
}

export fn show_a(t: Token) [Clock] -> Str {
    let back = decode<Token>(encode(copy(t))) ?: unreachable!()
    let s: Set<Token> = Set(copy(t), back)
    return "a.Token ${t.id}, set of ${size(s)}, at ${now()}"
}
// FILE: lib/b.sv
// A different `Token`: a fn field and a union over it.
export struct Token { name: Str, f: (Int) -> Int }
export struct Circle { r: Int }
export type Shape = Token | Circle

export fn make_b() [] -> Token {
    return Token { name: "bee", f: x -> x + 1 }
}

export fn show_b(t: Token) [] -> Str {
    let g = t.f
    return "b.Token ${t.name} ${g(1)}"
}

export fn describe(s: Shape) [] -> Str {
    when s {
        is Token { return "a token named ${s.name}" }
        is Circle { return "a circle of ${s.r}" }
    }
}
// FILE: main.sv
import lib.a.make_a
import lib.a.show_a
import lib.a.Fixed
import lib.b.Token as BToken
import lib.b.Circle
import lib.b.make_b
import lib.b.show_b
import lib.b.describe

// This module's own `Token`, beside two imported ones.
struct Token { label: Str }

actor effect Counter {
    send fn bump(t: Token) => !t
    send fn total(out: Reply<Str>) => !out
}

handler Counting() of Counter {
    mailbox { capacity: 4 }
    state {
        seen: Str = ""
    }
    send fn bump(t: Token) => !t { seen = "${seen}${t.label}" }
    send fn total(out: Reply<Str>) => !out { out.send(copy(seen)) }
}

fn main() [use, spawn] {
    use StdConsole()
    use Fixed(42L)
    let mine = Token { label: "mine" }
    println(mine.label)
    println(show_a(make_a()))
    let b: BToken = make_b()
    println(b.name)
    println(show_b(b))
    println(describe(BToken { name: "two", f: x -> x }))
    println(describe(Circle { r: 3 }))
    let c = spawn Counting() on pool(1)
    c.bump(Token { label: "x" })
    c.bump(copy(mine))
    let s = waitfor out: Reply<Str> { c.total(out) }
    println(s)
}
//= stdout
//= mine
//= a.Token 1, set of 1, at 42
//= bee
//= b.Token bee 2
//= a token named two
//= a circle of 3
//= xmine

// [type-identity] Two effects named `Log` — a plain one declared first,
// keeping the plain key, and an actor one keyed by its module — and two
// structs named `View`, one borrowing. Every lookup goes by key or through
// the file's scope (PIPELINE.md step 5): the deduction tables, the runtime
// features' actor test, both emitters' interface lookups, which until then
// fell back to the plain name and could answer the other module's `Log`.
//--- two_effects_of_one_name_stay_apart
// FILE: lib/a.sv
// A plain `Log`, declared first, so it keeps the plain key.
export effect Log {
    fn note(s: Str) -> None => !s
}

export handler Lines() [Console] of Log {
    state {
        all: Str = ""
    }
    fn note(s: Str) -> None => !s {
        all = "${all}${s};"
        println(all)
    }
}

export struct View { label: Str }
// FILE: lib/b.sv
// An actor `Log` of the same name, keyed by its module.
export actor effect Log {
    send fn add(n: Int)
    send fn total(out: Reply<Int>) => !out
}

export handler Summing() of Log {
    mailbox { capacity: 4 }
    state {
        sum: Int = 0
    }
    send fn add(n: Int) { sum = sum + n }
    send fn total(out: Reply<Int>) => !out { out.send(copy(sum)) }
}

// A `View` that borrows.
export struct View { items: proj List<Int>, at: Int }

export fn view(xs: List<Int>) [] -> View => xs {
    return View { items: xs, at: 0 }
}

export fn first(v: View) [] -> Int => v {
    return get(v.items, v.at) ?: -1
}
// FILE: main.sv
import lib.a.Lines
import lib.a.Log
import lib.b.Summing
import lib.b.view
import lib.b.first

fn main() [use, spawn] {
    use StdConsole()
    use Lines()
    note("one")
    note("two")
    let xs = List(7, 8)
    println("first ${first(view(xs))}")
    let c = spawn Summing() on pool(1)
    c.add(2)
    c.add(3)
    let s = waitfor out: Reply<Int> { c.total(out) }
    println("total ${s}")
}
//= stdout
//= one;
//= one;two;
//= first 7
//= total 5

// [name-dot] [kt-nested-dot-name] A `type` namespacing its members — the
// shape the aws generator uses for Smithy enums: Rust concatenates the
// names, Kotlin nests the members in an `object` named for the type.
//--- a_type_namespace
// FILE: sc.sv
export type StorageClass = StorageClass.Standard | StorageClass.Glacier | StorageClass.Unknown
export struct StorageClass.Standard {}
export struct StorageClass.Glacier {}
export struct StorageClass.Unknown { value: Str }

export fn describe(c: StorageClass) [] -> Str => c {
    when c {
        is StorageClass.Standard { return "standard" }
        is StorageClass.Glacier { return "glacier" }
        is StorageClass.Unknown { return "unknown: ${c.value}" }
    }
}
// FILE: main.sv
import sc.StorageClass
import sc.describe

fn main() [use] {
    use StdConsole()
    let one: StorageClass = StorageClass.Glacier {}
    println(describe(one))
    println(describe(StorageClass.Standard {}))
    println(describe(StorageClass.Unknown { value: "DEEP" }))
}
//= stdout
//= glacier
//= standard
//= unknown: DEEP
