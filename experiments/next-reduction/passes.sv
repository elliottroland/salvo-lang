// R0 prototype — the *reduction to `next`*, in the planned language.
//
// Not compilable today: obligation groups (`: Yield<T>`, `: Linear`), a
// `yield` fn naming the pass struct it declares, and `for` closing a pass
// are all what this prototype is validating. `passes.rs` / `passes.kt` are
// what a correct emitter would produce for it; they compile and run, and
// print the same bytes (`expected.txt`).
//
// Six shapes, one program:
//   1. a hand-written pass, driven by `for`, beside a `for` over a `List`;
//   2. a `yield`-generated pass with a `defer` and an effect, abandoned early
//      (so the release path is observable);
//   3. a COMPOSED pass, rendering (A): the group as a *generic bound*;
//   4. the same, rendering (B): the group's member as an *implicit parameter*
//      — over an EFFECTFUL source, which is what (A) cannot do;
//   5. a linear pass driven to exhaustion;
//   6. the same, abandoned early.

// ===================================================================
// std/core/iterator.sv, as the reduction leaves it
// ===================================================================

// Unchanged: the element arm is a qualifier so the element keeps its own
// type, and `Finished` is a fieldless struct [iter-protocol].
qualifier Emitted<T> of T
struct Finished {}

fn emitted<T canbe Linear>(value: T) [] -> [] T as Emitted {
    return value
}

fn finished() [] -> [] Finished {
    return Finished {}
}

// [yield-group] The one protocol. `Self` is bound to the type that declares
// the group, so there is no state type parameter — and no value may ever
// have `Yield<T>` as its type [group-not-a-value].
params Yield<T> {
    fn next(s: Mut Self) -> [s: Mut] Emitted T | Finished
}

// [linear-group] Declaring `: Linear` is declaring how the obligation is
// discharged. One member, and it consumes.
params Linear {
    fn close(s: Self) -> [] None
}

// ===================================================================
// 1. A hand-written pass
// ===================================================================

// The `: Yield<Int>` clause is the declaration `for` reads: no scan, and a
// misspelled `next` is an error *here* rather than "not iterable" at the loop.
// No `Once`: driving mutates, so `Mut` carries the single-use-ness, and a
// second drive continues rather than restarting.
struct Countdown : Yield<Int> canbe Mut {
    at: Int
}

fn next(c: Mut Countdown) -> [c: Mut] Emitted Int | Finished {
    if c.at <= 0 {
        return finished()
    }
    let v = copy(c.at)
    c.at = c.at - 1
    return emitted(v)
}

// Getting a fresh pass is constructing one. No factory type.
fn countdown(from: Int) [] -> [] Mut Countdown {
    return Mut Countdown { at: from }
}

// ===================================================================
// 2. A `yield`-generated pass, with a `defer` and an effect
// ===================================================================

// UNKNOWN 1, as prototyped: the return type *names* the struct the fn
// declares, and carries the group clause so the element type is written
// where the reader looks. The effect list means "driving performs these" —
// calling `chatty` runs none of the body.
//
// The compiler generates:
//     struct Chatty : Yield<Int> canbe Mut { limit: Int, i: Int, ... }
//     fn next(c: Mut Chatty) [Console] -> [c: Mut] Emitted Int | Finished
//     fn close(c: Chatty) [Console] -> [] None      // the body defers
fn chatty(limit: Int) [Console] -> Chatty : Yield<Int> {
    println("open")
    defer { println("close") }
    let i = 0
    while i < limit {
        println("make ${i}")
        yield copy(i)
        i = i + 1
    }
}

// ===================================================================
// 3. A composed pass — rendering (A), the group as a generic bound
// ===================================================================

// `<P: Yield<T>>` is a bound on an obligation group, not an interface: there
// is still no value whose type is a group [group-not-a-value]. It is what
// makes `next(m.src)` legal inside the generated body — with no bounds,
// nothing about a `P` is knowable.
fn map_a<P: Yield<T>, T, U>(src: Mut P, f: (T) -> U) [] -> [src, f] MapA<P, T, U> : Yield<U> {
    for x in src {
        yield f(x)
    }
}

// ===================================================================
// 4. A composed pass — rendering (B), the member as an implicit parameter
// ===================================================================

// The same shape with no new mechanism: `?next` is `?Iterable`'s move
// applied to the protocol (this is the `params Iterator<St, T>` spelling,
// with the state as a parameter instead of `Self`). Two things fall out that
// (A) cannot do:
//   * the implicit's *type* carries the source's effects, so a composed pass
//     over an effectful source works and `map_b` inherits `Console` from a
//     fn-typed parameter — the rule [fn-effects] already has;
//   * `?close` is resolved at the call site, where whether the source has one
//     is known — a bound would have to be `P: Linear`, which a pass without a
//     `close` cannot satisfy.
fn map_b<P, T, U>(
    src: Mut P,
    f: (T) -> U,
    ?next: (s: Mut P) [Console] -> [s: Mut] Emitted T | Finished,
    ?close: (s: P) [Console] -> [] None
) [] -> [src, f] MapB<P, T, U> : Yield<U> {
    for x in src {
        yield f(x)
    }
}

// ===================================================================
// 5 & 6. A linear pass
// ===================================================================

// `: Linear` obliges every path to `close` it; declaring it without a
// matching `close(Lines) -> [] None` is an error at the struct.
struct Lines : Linear, Yield<Str> canbe Mut {
    name: Str,
    count: Int,
    at: Int
}

fn next(l: Mut Lines) -> [l: Mut] Emitted Str | Finished {
    if l.at >= l.count {
        return finished()
    }
    l.at = l.at + 1
    let row: Str = "line ${l.at} of ${l.name}"
    return emitted(row)
}

// The group declares no effects; each implementation declares its own.
fn close(l: Lines) [Console] -> [] None {
    println("closing ${l.name}")
}

fn open_lines(name: Str, count: Int) [Console] -> [] Mut Lines {
    println("opening ${name}")
    return Mut Lines { name: name, count: count, at: 0 }
}

// ===================================================================
// The program
// ===================================================================

fn main() [use] -> None {
    use StdOutConsole()

    // 1 — a pass, and a List, side by side. The pass drives `next`; the list
    // keeps its native loop.
    for n in countdown(3) {
        println("n ${n}")
    }
    for s in list("ada", "grace") {
        println("s ${s}")
    }

    // 2 — abandoned after two elements: the `for` sugar calls `close`, and
    // the producer's `defer` runs.
    let seen = 0
    for v in chatty(3) {
        println("got ${v}")
        seen = seen + 1
        if seen == 2 {
            break
        }
    }

    // 3 — composition, bound-style, over a pure source.
    for v in map_a(countdown(3), n -> n * 2) {
        println("A ${v}")
    }

    // 4 — composition, implicit-style, over an *effectful* source.
    for v in map_b(chatty(3), n -> n * 2) {
        println("B ${v}")
    }

    // 5 — a linear pass, drained. `for` discharges the obligation.
    for row in open_lines("a.txt", 2) {
        println(row)
    }

    // 6 — the same, abandoned. Same discharge, same output shape.
    for row in open_lines("b.txt", 2) {
        println(row)
        break
    }

    println("done")
}
