// [effect-prereq] Effect prerequisites: `effect Fs [Streams]` — wherever
// `Fs` is, `Streams` is too, bound separately (user decision 2026-09-29, R2:
// a prerequisite, not inheritance).
//
// One program of several modules: `streams` and `files` are what the cases
// share; each other module is a case.

// [effect-prereq] A fn declaring `[Files]` may call `Streams` members — in a
// module that never imports `Streams` — and a handler of `Files` reaches it as
// an unwritten dependency. A fn *type* expands the same way, so a fn declared
// `[Files]` passes where `(Str) [Files] -> Str` is expected.
import files.Files
import files.MemFiles
import streams.Counter

fn work(f: (Str) [Files] -> Str) [Files] -> Str {
    return "${f("a")} ${next_id()}"
}

fn opener(p: Str) [Files] -> Str => p {
    return open(p)
}

fn main() [use] {
    use Counter()
    use MemFiles()
    let _a = opener("x")
    let _b = work(opener)
}

// [effect-prereq] Binding a handler of `Files` needs a `Streams` bound first,
// and the error says it is the effect's prerequisite ("implements `Files`,
// which needs `Streams` in scope") — the reader looking at `MemFiles` will
// not find a dependency written there.
fn without_the_prerequisite() [use] {
    //~ error[effect-prereq:face-needs]
    use MemFiles()
}

// FILE: streams.sv
export effect Streams {
    fn next_id() -> Int
}

export handler Counter() of Streams {
    state {
        n: Int = 0
    }
    fn next_id() -> Int {
        n = n + 1
        return n
    }
}

// FILE: files.sv
import streams.Streams

export effect Files [Streams] {
    fn open(path: Str) -> Str => path
}

// A handler of `Files` reaches `Streams` without writing `[Streams]`.
export handler MemFiles() of Files {
    fn open(path: Str) -> Str => path {
        return "${path}#${next_id()}"
    }
}

// FILE: own_streams.sv
// [effect-prereq] A prerequisite resolves in the file that declares it, not
// where the effect is used (2026-10-20): a module declaring its own
// `Streams` can name `Files`, and the `Streams` that `Files` implies is still
// `streams.Streams`, whose member is callable without an import. Before the
// expansion moved into the checker it was an AST rewrite that injected the
// name into the using file, and this program was a name clash.
import files.Files

effect Streams {
    fn other() -> Int
}

fn f() [Files] -> Str {
    let n = next_id()
    return open("x${n}")
}

// FILE: transitive.sv
// [effect-prereq] Transitive: `A [B]`, `B [C]` puts `C` in scope wherever `A`
// is; a handler implementing the prerequisite itself (`of A, B`) depends on
// nothing for it.
effect C {
    fn c() -> Int
}
effect B [C] {
    fn b() -> Int
}
effect A [B] {
    fn a() -> Int
}
handler HC() of C {
    fn c() -> Int { return 3 }
}
handler HAB() of A, B {
    fn a() -> Int { return c() + 1 }
    fn b() -> Int { return c() }
}

fn uses_c() [A] -> Int {
    return c()
}

fn main() [use] {
    use HC()
    use HAB()
    let _n = uses_c()
}

// FILE: generic.sv
// [effect-prereq] A generic effect's prerequisite takes the site's type
// arguments: `Log<T> [Sink<T>]` at `Log<Str>` implies `Sink<Str>`, so the
// implied member is typed at `Str`: only `g` is wrong.
effect Sink<T> {
    fn put(v: T) -> None => v
}
effect Log<T> [Sink<T>] {
    fn log(v: T) -> None => v
}
fn f() [Log<Str>] {
    put("x")
}
fn g() [Log<Str>] {
    //~ error[call-resolve:no-overload]
    put(3)
}

// FILE: x.sv
// [effect-handler-deps] A handler's dependency list resolves in the
// handler's own file: `use HY()` in a module that never imports `X` is fine
// (`deps.sv`). Before 2026-09-29 the list was lowered in the *using* file's
// scope and this was "unknown effect `X`" — found with [effect-prereq],
// whose implied dependencies are never imported at a use site.
export effect X {
    fn x() -> Int
}
export handler HX() of X {
    fn x() -> Int { return 1 }
}

// FILE: m.sv
import x.X
export effect Y {
    fn y() -> Int
}
export handler HY() [X] of Y {
    fn y() -> Int { return x() }
}

// FILE: deps.sv
import m.HY
import x.HX
fn main() [use] {
    use HX()
    use HY()
}

//--- malformed prerequisites
// [effect-prereq] The declaration's own rules: no cycle ("form a cycle"), no
// actor effect on either side ("`actor effect Pong` cannot have
// prerequisites", "`Ping` is an actor effect"), only plain named entries
// ("`use` and `spawn` are capabilities", "`any` has no meaning in a
// prerequisite"), and an exported effect's prerequisite exported too ("its
// prerequisite `P` is not") — every module naming the effect gets the
// prerequisite. And a prerequisite must name an effect ("unknown effect
// `Nowhere`"). One module per old program.
//~ error[effect-prereq:cycle]
effect A [B] {
    fn a() -> Int
}
//~ error[effect-prereq:cycle]
effect B [A] {
    fn b() -> Int
}

// FILE: actor.sv
actor effect Ping {
    send fn ping(n: Int)
}
effect P {
    fn p() -> Int
}
//~ error[effect-prereq:actor-effect]
actor effect Pong [P] {
    send fn pong(n: Int)
}
//~ error[effect-prereq:actor-prereq]
effect Q [Ping] {
    fn q() -> Int
}

// FILE: kinds.sv
effect P {
    fn p() -> Int
}
//~ error[effect-prereq:capability]
//~ error[effect-prereq:any]
effect A [use, any P] {
    fn a() -> Int
}

// FILE: private.sv
effect P {
    fn p() -> Int
}
//~ error[effect-prereq:not-exported]
export effect A [P] {
    fn a() -> Int
}

// FILE: unknown.sv
//~ error[effect-prereq:unknown]
effect A [Nowhere] {
    fn a() -> Int
}
