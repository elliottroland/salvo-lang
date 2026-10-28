// Passes on the Rust backend: hand-written passes, their driving loops, the
// generic drive, and the `iter fn` form.

//--- a_function_in_a_struct_field_is_an_arc_dyn_fn
// [rs-fn-field] A **composed pass, hand-written**: it stores both its source
// and its callback. Storing a function in a struct field used to be a Rust
// codegen error while Kotlin accepted it (a live backend divergence, recorded
// as R0 finding 5); the field is now an `Rc<dyn Fn…>`, the same
// representation a generated pass has always used for a callback, so the
// program builds on both targets — which is what makes user-written
// combinators possible at all, not just std's generated ones.
//
// [rs-fn-field] The representation: an `Rc<dyn Fn…>` field, `#[derive(Clone)]`
// without `Debug` (a `dyn Fn` has none), a hand-written `Debug` printing the
// callback as `<fn>`, and `Arc::new` at the store ([rs-handle]: `Arc<dyn Fn
// + Send + Sync>` since 2026-09-28, so a callback may be handler state).
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

//~ emits: impl std::fmt::Debug for Doubling
//~ emits: .field("f", &"<fn>")
struct Doubling : Next<self, Int> canbe Mut {
    src: Mut Countdown,
    //~ emits: pub f: std::sync::Arc<dyn Fn(i32) -> i32 + Send + Sync>,
    f: (Int) -> Int
}

fn next(d: Mut Doubling) -> Emitted Int | Finished => d: Mut {
    let step = next(d.src)
    when step {
        is Emitted {
            let g = d.f
            return emitted(g(step))
        }
        is Finished {
            return finished()
        }
    }
}

fn twice(n: Int) -> Int {
    return n * 2
}

fn main() [use] {
    use StdConsole()
    //~ emits: f: std::sync::Arc::new(move |__a0: i32| crate::twice(__a0))
    let doubled = Mut Doubling { src: Mut Countdown { at: 3 }, f: twice }
    for n in doubled {
        println("n ${n}")
    }
    println("done")
}

//--- a_pass_lowers_to_a_driving_loop
// [iter-protocol] A hand-written **pass**: `zip`, which `yield` cannot
// express because it reads two sources at once. A struct, a `next`, and
// nothing else — no state machine, no compiler support beyond driving it.
//
// The driving loop: the subject is moved into a mutable local and each turn
// calls the resolved `next`, testing for the `Emitted` arm the *checker*
// chose [type-union-arm-identity]; any other arm ends the loop.
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
    //~ emits: let mut __pass_3: crate::Countdown = crate::countdown(3i32);
    //~ emits: crate::next__Countdown(&mut __pass_3);
    //~|         if matches!(__step_4, crate::unions::Union2::U1(_)) {
    //~ !emits: for mut n in countdown
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
// [iter-protocol] [throw] A **fallible** pass: the producer yields a result
// and the *consumer* throws. This is the answer to "does the protocol need
// `Throw` support" — it does not, so `Emitted T | Finished` stays exactly two
// arms and a `yield` fn never has to declare an effect it cannot perform
// while suspended. Also the regression test for the element being a union
// under a qualifier [qual-group]: `emitted(err(...))` is a qualifier applied
// to an already-qualified value, so the arm it lands in is a qualified union
// *group* and the value needs two wraps — inner union first. That needed an
// intermediate annotated `let` until 2026-09-10.
//
// [qual-group] The nested wrap, read off the generated source rather than
// only from the program's output: `emitted(err(...))` lands in the group arm
// `Emitted (Ok Str | Err Str)`, whose inner union is a wrapper of its own, so
// the value takes *two* wraps — inner arm first, and the arm index comes
// from the qualifier left over after the group's own is removed.
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
            //~ emits: crate::unions::Union2::U1(crate::unions::Union2::U2(crate::std_core_iterator::emitted::<String>(
            return emitted(err("bad line at ${r.at}"))
        }
        //~ emits: crate::unions::Union2::U1(crate::unions::Union2::U1(crate::std_core_iterator::emitted::<String>(
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

//--- a_kept_pass_is_driven_in_place
// [iter-generic-drive] [iter-drive-in-place] The three things the generic drive
// buys, in one program: a combinator driving its pass with an ordinary `for`,
// the caller **carrying that pass on** afterwards (the parity case — Rust used
// to bind the `&mut` parameter into a local, so the caller never saw the
// position the loop reached), and a `yield fn` performing a **generic** effect.
//
// [iter-drive-in-place] The lowering a *kept* pass gets: no local, no clone —
// the loop advances the parameter where it lives, which is what makes the
// caller's next drive continue from where this one stopped. Binding it into a
// local cloned the `&mut`, and the caller never saw the position (Kotlin
// aliased it and did, so the two backends disagreed [backend-parity]). That
// `take` binds no `__pass` local is asserted in `codegen_tests.rs` (a
// needle's absence from one function).
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

// The pass is *kept*, so the loop advances it where it lives and the caller may
// drive it further.
fn take<It>(it: Mut It, count: Int, ?Next<It, Int>) [] -> Int => it: Mut {
    let sum = 0
    let seen = 0
    //~ emits: = next(&mut *it);
    for n in it {
        sum = sum + n
        seen = seen + 1
        if seen == count {
            break
        }
    }
    return sum
}

effect Random<T> {
    fn next_random() -> T
}

handler CyclicRandom<T>(values: T[]) of Random<T> {
    state {
        i: Int = 0
    }

    fn next_random() -> T {
        i = (i + 1) % size(values)
        return values[i]
    }
}

struct Rolls {
    count: Int
}

fn rolls(count: Int) -> Rolls {
    return Rolls {count: count}
}

iter fn iter(r: Rolls) [Random<Int>] -> Emitted Int | Finished {
    state RollsIter {
        made: Int = 0
    }
    if made >= r.count {
        return finished()
    }
    made = made + 1
    return emitted(next_random())
}

fn main() [use] {
    use StdConsole()
    use CyclicRandom(array_of(7, 8, 9))
    let xs = List(1, 2, 3, 4)
    let p = slice(xs)
    println("first ${take(p, 2)}")
    println("rest ${take(p, 9)}")
    for v in rolls(3) {
        println("v ${v}")
    }
}

//--- an_owned_generic_pass_is_discharged_by_the_callback
// [iter-generic-drive] [linear-generics] A combinator that **owns** a possibly
// linear pass releases it through the `?Linear<It>` spread. The release cannot
// print — an implicitly resolved fn is effect-free [implicit-resolve] — so the
// call is asserted in the emitted code and the program is run to prove it
// builds.
//
// [linear-generics] The discharge is the program's: the pass is driven in
// place and the consuming callback `end(it)` is the explicit terminal —
// no loop-local, no deferred splice (user decision 2026-09-12).
linear struct Handle : Next<self, Int> canbe Mut {
    at: Int
}

fn open_handle(from: Int) -> Mut Handle {
    return Mut Handle { at: from }
}

fn next(h: Mut Handle) -> Emitted Int | Finished => h: Mut {
    if h.at <= 0 {
        return finished()
    }
    let v = copy(h.at)
    h.at = h.at - 1
    return emitted(v)
}

fn close(h: Handle) -> None => !h { discard(h) }

// Owns the pass and takes its discharge as a consuming callback (user
// decision 2026-09-12: no implicit discharge sites — the loop drives in
// place, and `end(it)` is the explicit terminal on the one path out).
fn drain<It canbe linear>(it: Mut It, stop: Int, end: (x: It) -> None, ?Next<It, Int>) [] -> Int => !it =>[end] !x {
    let sum = 0
    //~ !emits: __loop1_pass
    for n in it {
        sum = sum + n
        if sum > stop {
            break
        }
    }
    //~ emits: end(it)
    end(it)
    return sum
}

fn main() [use] {
    use StdConsole()
    println("all ${drain(open_handle(4), 100, close)}")
    println("cut ${drain(open_handle(4), 5, close)}")
}

//--- an_iter_fn_emits_a_plain_struct_and_next
// [iter-fn] A hand-written `next` whose **pass struct is generated** (user
// decision 2026-09-09): the subject stays ordinary data, the `state { … }`
// block is the pass's own fields, and the compiler writes the struct plus the
// `iter` that mints it.
//
// Six things in one program, and the same source and stdout on both backends:
// a `for` over a subject, the subject **replayed** (driving copied it), a pass
// **held** and driven by hand then finished by a `for` — the capability the
// `yield fn` origin lacks, since each loop re-mints — several `state` fields,
// a subject field read on every turn, and an **effectful** `next`, whose
// handlers are threaded into every turn of the loop.
//
// [iter-fn] What the expansion emits, and what it does *not*: a plain struct
// borrowing the parameters beside the `state` fields, a plain minter, and a
// plain `next` — no state machine, no `__advance`, no state
// number, and no per-`defer` flag.
struct Countdown {
    from: Int
}

//~ !emits: __advance
//~ !emits: __state
iter fn iter(c: Countdown) -> Emitted Int | Finished {
    // [iter-fn] The struct its `state` block names holds every parameter —
    // **borrowed** whole, as a `proj` field [proj-field], so it carries a
    // lifetime and the mint clones nothing [copy-opt-in] — then the state.
    //~ emits: pub struct CountdownIter<'s> {
    //~|     pub c: &'s crate::Countdown,
    //~|     pub at: i32,
    //~| }
    //~ emits: crate::CountdownIter { c: c, at: c.from }
    state CountdownIter {
        at: Int = c.from
    }
    if at <= 0 {
        return finished()
    }
    at = at - 1
    return emitted(at + 1)
}

struct Fibs {
    count: Int
}

iter fn iter(f: Fibs) -> Emitted Int | Finished {
    //~ emits: pub struct FibsIter<'s> {
    //~|     pub f: &'s crate::Fibs,
    //~ emits: crate::FibsIter { f: f, a: 0i32, b: 1i32, made: 0i32 }
    state FibsIter {
        a: Int = 0,
        b: Int = 1,
        made: Int = 0
    }
    if made >= f.count {
        return finished()
    }
    let now = copy(a)
    let sum = a + b
    a = copy(b)
    b = copy(sum)
    made = made + 1
    return emitted(now)
}

struct Noisy {
    limit: Int
}

iter fn iter(n: Noisy) [Console] -> Emitted Int | Finished {
    state NoisyIter {
        at: Int = 0
    }
    if at >= n.limit {
        println("  done")
        return finished()
    }
    println("  turn ${at}")
    at = at + 1
    return emitted(copy(at))
}

struct Row {
    label: Str,
    times: Int
}

fn describe(r: Row) -> Str => r {
    return "${r.label}!"
}

iter fn iter(r: Row) -> Emitted Str | Finished {
    //~ emits: pub struct RowIter<'s> {
    //~|     pub r: &'s crate::Row,
    //~ emits: crate::RowIter { r: r, left: r.times }
    state RowIter {
        left: Int = r.times
    }
    if left <= 0 {
        return finished()
    }
    left = left - 1
    return emitted(describe(r))
}

fn main() [use] {
    use StdConsole()

    let c = Countdown { from: 3 }
    for n in c {
        println("n ${n}")
    }
    for n in c {
        println("again ${n}")
    }

    let p = iter(c)
    let first = next(p)
    when first {
        is Emitted { println("first ${first}") }
        is Finished { println("empty") }
    }
    for n in p {
        println("rest ${n}")
    }

    println("fib total ${reduce(iter(Fibs { count: 7 }), 0, (acc: Int, n: Int) -> acc + n)}")

    let noisy = Noisy { limit: 2 }
    // [fn-effects] An effectful `next` takes its handlers as leading arguments,
    // threaded into every turn of the loop.
    //~ emits: crate::next__NoisyIter(&__handle_2, &mut __pass_14);
    for v in noisy {
        println("v ${v}")
    }
    let row = Row { label: "hey", times: 2 }
    for s in row {
        println("s ${s}")
    }
}
