// Passes: how a `for` drives a hand-written pass, the protocol's own types,
// and the union wrappers a pass's elements and implicits render to.

//--- a_pass_lowers_to_a_guarded_while_loop
// Kotlin has no pattern-matching loop condition, so the driving loop is
// `while (true)` plus a guard. The arm is spelled with its *real* type
// arguments where it can be (a non-generic `next`), which is what keeps the
// element read free of an unchecked cast — a warning in code the user cannot
// edit.
// [iter-protocol] A hand-written **pass** driven by `for` (`PASS_DEMO` in
// codegen_tests.rs). `zip` is the point — it reads two sources at once,
// which `yield` cannot express.
struct Zip : Next<self, (Str, Int)> canbe Mut {
    left: List<Str>,
    right: List<Int>,
    at: Int
}

fn next(z: Mut Zip) -> Emitted (Str, Int) | Finished => z: Mut {
    let l = get(z.left, z.at)
    let r = get(z.right, z.at)
    if l is Str && r is Int {
        z.at = z.at + 1
        // `get` hands back borrows [copy-opt-in]: building a tuple *stores*
        // the element, so the copy is written where it happens.
        return emitted((copy(l), r))
    }
    return finished()
}

fn zip(left: List<Str>, right: List<Int>) -> Zip => !left, !right {
    return Zip { left: left, right: right, at: 0 }
}

struct Countdown : Next<self, Int> canbe Mut {
    at: Int
}

fn next(c: Mut Countdown) -> Emitted Int | Finished => c: Mut {
    if c.at <= 0 {
        return finished()
    }
    let v = copy(c.at)
    c.at = c.at - 1
    return emitted(v)
}

fn countdown(from: Int) -> Countdown {
    return Countdown { at: from }
}

fn main() [use] {
    use StdConsole()
    //~ emits: : Countdown = salvo.main.countdown(3)
    //~ emits: : Union2<Int, salvo.std.core.iterator.Finished> = salvo.main.next__Countdown(
    // [ir-loop] The pass is driven by one `while (true)` testing the step's
    // arm; `Finished` breaks.
    //~ emits: (__step_4 is Union2.U1<*, *>) ->
    //~ emits: else -> {
    //~|                 break
    for n in countdown(3) {
        println("n ${n}")
    }
    let names = List("ada", "grace", "alan")
    let ages = List(36, 45)
    for pair in zip(names, ages) {
        println("${pair.0} is ${pair.1}")
    }
    println("done")
}

//--- a_fieldless_struct_is_a_plain_class
// [kt-struct-empty] A fieldless struct is a plain class, not a data class:
// Kotlin requires a data class to have at least one primary-constructor
// parameter. `Finished` is the first such struct (std's iterator protocol).
//~ emits(std/core/iterator.kt): class Finished
//~ !emits(std/core/iterator.kt): data class Finished
struct Zip : Next<self, (Str, Int)> canbe Mut {
    left: List<Str>,
    right: List<Int>,
    at: Int
}

fn next(z: Mut Zip) -> Emitted (Str, Int) | Finished => z: Mut {
    let l = get(z.left, z.at)
    let r = get(z.right, z.at)
    if l is Str && r is Int {
        z.at = z.at + 1
        return emitted((copy(l), r))
    }
    return finished()
}

fn zip(left: List<Str>, right: List<Int>) -> Zip => !left, !right {
    return Zip { left: left, right: right, at: 0 }
}

struct Countdown : Next<self, Int> canbe Mut {
    at: Int
}

fn next(c: Mut Countdown) -> Emitted Int | Finished => c: Mut {
    if c.at <= 0 {
        return finished()
    }
    let v = copy(c.at)
    c.at = c.at - 1
    return emitted(v)
}

fn countdown(from: Int) -> Countdown {
    return Countdown { at: from }
}

fn main() [use] {
    use StdConsole()
    for n in countdown(3) {
        println("n ${n}")
    }
    let names = List("ada", "grace", "alan")
    let ages = List(36, 45)
    for pair in zip(names, ages) {
        println("${pair.0} is ${pair.1}")
    }
    println("done")
}

//--- a_flattened_qualifier_wraps_the_inner_union_first
// [qual-group] The nested wrap, read off the generated source rather than
// only from the program's output: `emitted(err(...))` lands in the group arm
// `Emitted (Ok Str | Err Str)`, whose inner union is a wrapper of its own, so
// the value takes *two* wraps — inner arm first, and the arm index comes
// from the qualifier left over after the group's own is removed.
// [iter-protocol] [throw] A **fallible** pass (`FALLIBLE_PASS_DEMO` in
// codegen_tests.rs): the producer yields a result and the *consumer* throws.
// This is the answer to "does the protocol need `Throw` support" — it does
// not, so `Emitted T | Finished` stays exactly two arms and a `yield` fn
// never has to declare an effect it cannot perform while suspended. Also the
// regression test for the element being a union under a qualifier
// [qual-group]: `emitted(err(...))` is a qualifier applied to an
// already-qualified value, so the arm it lands in is a qualified union
// *group* and the value needs two wraps — inner union first. That needed an
// intermediate annotated `let` until 2026-09-10.
import std.throw

struct Reader : Next<self, Ok Str | Err Str> canbe Mut {
    lines: List<Str>,
    at: Int
}

fn next(r: Mut Reader) -> Emitted (Ok Str | Err Str) | Finished => r: Mut {
    let line = get(r.lines, r.at)
    if line is Str {
        r.at = r.at + 1
        if line == "boom" {
            //~ emits: Union2.U1<Union2<String, String>, salvo.std.core.iterator.Finished>(Union2.U2<String, String>(
            return emitted(err("bad line at ${r.at}"))
        }
        // The element is a borrow [copy-opt-in]; wrapping it in a result
        // stores it, so the copy is explicit.
        //~ emits: Union2.U1<Union2<String, String>, salvo.std.core.iterator.Finished>(Union2.U1<String, String>(
        return emitted(ok(copy(line)))
    }
    return finished()
}

fn reader(lines: List<Str>) -> Reader => !lines {
    return Reader { lines: lines, at: 0 }
}

fn read_all(lines: List<Str>) [Throw<Str>, Console] -> Int
=> !lines {
    let count = 0
    for outcome in reader(lines) {
        when outcome {
            is Ok {
                println("line ${outcome}")
                count = count + 1
            }
            is Err {
                return throw("stopped: ${outcome}")
            }
        }
    }
    return count
}

fn main() [use] {
    use StdConsole()
    let good = try { read_all(List("alpha", "beta")) }
    when good {
        is Ok { println("read ${good}") }
        is Thrown { println("failed: ${good}") }
    }
    let bad = try { read_all(List("alpha", "boom", "gamma")) }
    when bad {
        is Ok { println("read ${bad}") }
        is Thrown { println("failed: ${bad}") }
    }
}

//--- a_raw_pass_is_driven_in_place_with_no_finally
// [iter-drive-in-place] [linear-group] No implicit discharge sites
// (user decision 2026-09-12): no finally splice — the program's own
// `close(lines)` is the discharge, an ordinary call.
// (`RAW_CLOSE_DEMO` in codegen_tests.rs.)
//~ !emits: } finally {
//~ !emits: __loop1_pass
linear struct Ticks : Next<self, Int> canbe Mut {
    at: Int
}

fn next(l: Mut Ticks) -> Emitted Int | Finished => l: Mut {
    if l.at <= 0 {
        return finished()
    }
    let v = copy(l.at)
    l.at = l.at - 1
    return emitted(v)
}

fn close(l: Ticks) [Console] -> None => !l {
    discard(l)
    println("closed")
}

fn drained() [Console] -> None {
    let lines = Mut Ticks { at: 2 }
    for n in lines {
        println("n ${n}")
    }
    // Plain: the program's module declares one `close` [fn-emit-name].
    //~ emits: close(console, lines)
    close(lines)
    println("after drain")
}

fn abandoned() [Console] -> None {
    let lines = Mut Ticks { at: 5 }
    for n in lines {
        println("m ${n}")
        break
    }
    close(lines)
    println("after break")
}

fn main() [use] {
    use StdConsole()
    drained()
    abandoned()
}

//--- a_union_returning_implicit_renders_the_wrapper_type
// The spread position renders as a Kotlin function type whose result is the
// union *wrapper*, not the Salvo text.
// (`YIELD_SPREAD_DEMO` in codegen_tests.rs.)
struct Slice<T> : Next<self, proj T> canbe Mut {
    items: proj List<T>,
    at: Int
}

fn slice<T>(items: List<T>) -> Mut Slice<T> => items {
    return Mut Slice<T> { items: items, at: 0 }
}

fn next<T>(p: Mut Slice<T>) -> Emitted (proj(p) T) | Finished => p: Mut {
    let e = get(p.items, p.at)
    if e is None {
        return finished()
    }
    p.at = p.at + 1
    return emitted(e)
}

//~ emits: next: (It) -> Union2<T, salvo.std.core.iterator.Finished>
//~ !emits: Emitted T | Finished
fn map2<It, T, U>(it: Mut It, mapper: (T) -> U, ?Next<It, T>) -> Mut List<U> => it: Mut, mapper {
    let out = Mut List<U>()
    let going = true
    while going {
        let step = next(it)
        when step {
            is Emitted {
                add(out, mapper(step))
            }
            is Finished {
                going = false
            }
        }
    }
    return out
}

fn double(n: Int) -> Int {
    return n * 2
}

fn main() [use] {
    use StdConsole()
    let xs = List(1, 2, 3)
    let doubled = map2(slice(xs), double)
    for d in doubled {
        println("d ${d}")
    }
}

//--- the_combinator_surface
// [seq-into] The combinator surface — same source and stdout as the Rust
// backend's `rustc_compiles_and_runs_the_combinator_surface`, which is the
// parity claim for the eager/into pair. The lazy pair went with the
// 2026-09-10 removal, so the generic bodies are reached by handing a pass in
// explicitly. (`SEQ_SURFACE_DEMO` in codegen_tests.rs.)
// The generic body drives its subject through the `next` the call site
// resolved, which arrives as an ordinary function parameter — no trait, no
// bound [implicit-group].
fn double(n: Int) -> Int {
    return n * 2
}

fn is_even(n: Int) -> Bool {
    return n % 2 == 0
}

fn main() [use] -> None {
    use StdConsole()
    let xs = List(1, 2, 3, 4)
    let doubled = map(xs, double)
    println("eager ${doubled.size()}")
    //~ emits(std/core/seq.kt): next: (It) -> Union2<T, salvo.std.core.iterator.Finished>
    let generic = map(iter(copy(xs)), double)
    println("generic ${generic.size()}")
    let snapshot = copy(xs)
    let kept = filter(iter(snapshot), is_even)
    println("kept ${kept.size()}")
    let out = map_to(Mut List<Int>(), iter(copy(xs)), double)
    println("sink ${out.size()}")
    let chained = filter_to(map_to(Mut List<Int>(), iter(copy(xs)), double), iter(xs), is_even)
    println("chained ${chained.size()}")
}
