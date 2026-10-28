// Driving a generic pass [iter-generic-drive]: in place, its casts suppressed,
// discharged by the program's own callback; and the generated-pass form.

//--- a_kept_pass_is_driven_in_place
// [iter-drive-in-place] A *kept* pass is advanced where it lives — no local —
// so the caller's next drive continues from where this one stopped. Kotlin's
// local aliased the same object and behaved this way already; naming the
// subject directly is what makes the two backends *say* so identically.
// [iter-generic-drive] [iter-drive-in-place] The three things the generic drive
// buys, in one program (`GENERIC_DRIVE_DEMO` in codegen_tests.rs).
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
    //~ emits: = next(it)
    // Precisely: no local for *this* loop: the kept pass is the subject.
    //~ !emits: : It = it
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

//--- a_payload_cast_gets_the_suppression
// [kt-suppress-cast] A generic drive reads the union payload through an
// erased cast (`step.value as T`), which kotlinc reports as an unchecked
// cast — in code the author cannot edit. The emitted function carries
// `@Suppress("UNCHECKED_CAST")`; a function whose payload reads cast to
// concrete types stays unannotated (those casts are checked at run time
// and draw no warning). (`SUPPRESS_DEMO` in codegen_tests.rs.)
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

// The element type is the combinator's own `T`: the payload read casts to a
// type variable, so the emitted fn needs the suppression.
// A generic payload is an *unchecked* cast ...
//~ emits: @Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
//~| fun<It, T> countAll
fn count_all<It, T>(it: Mut It, ?Next<It, T>) [] -> Int => it: Mut {
    let n = 0
    for x in it {
        n = n + 1
    }
    return n
}

// The element type is concrete (`Int`): the payload read casts to `Int`,
// which the JVM checks at run time — no warning, no annotation.
// ... and a concrete one is a *useless* cast, which kotlinc warns about
// just as loudly (found 2026-09-14 in std's `fs`, whose `to_str` reads a
// concrete error arm): one annotation covers both, since neither warning
// is the author's to silence.
//~ emits: @Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
//~| fun<It> sumInts
fn sum_ints<It>(it: Mut It, ?Next<It, Int>) [] -> Int => it: Mut {
    let sum = 0
    for n in it {
        sum = sum + n
    }
    return sum
}

fn main() [use] {
    use StdConsole()
    let xs = List(1, 2, 3)
    let p = slice(xs)
    println("count ${count_all(p)}")
    let ys = List(4, 5)
    let q = slice(ys)
    println("sum ${sum_ints(q)}")
}

//--- an_owned_generic_pass_is_discharged_by_the_callback
// [linear-generics] The discharge is the program's own: the pass drives in
//~ emits: end(it)
// place and `end(it)` is the explicit terminal — no finally splice (user
// decision 2026-09-12).
// [linear-group] No implicit discharge sites (user decision
// 2026-09-12): no finally splice — `end(it)` is the program's own
// terminal, an ordinary call through the fn-typed parameter.
// [iter-generic-drive] [linear-generics] A combinator that **owns** a possibly
// linear pass releases it through the `?Linear<It>` spread
// (`GENERIC_CLOSE_DEMO` in codegen_tests.rs).
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
//~ !emits: } finally {
fn drain<It canbe linear>(it: Mut It, stop: Int, end: (x: It) -> None, ?Next<It, Int>) [] -> Int => !it =>[end] !x {
    let sum = 0
    for n in it {
        sum = sum + n
        if sum > stop {
            break
        }
    }
    end(it)
    return sum
}

fn main() [use] {
    use StdConsole()
    println("all ${drain(open_handle(4), 100, close)}")
    println("cut ${drain(open_handle(4), 5, close)}")
}

//--- an_iter_fn_emits_a_plain_class_and_next
// [iter-fn] The generated declarations are ordinary Kotlin: a data class for
// the pass, a function that mints one, and the author's body as a function.
// Nothing suspends, so there is no `iterator {}` builder and no state number.
// [iter-fn] A hand-written `next` whose **pass struct is generated** (user
// decision 2026-09-09): the subject stays ordinary data, the `state { … }`
// block is the pass's own fields, and the compiler writes the struct plus the
// `iter` that mints it. (`ITER_FN_DEMO` in codegen_tests.rs.)
struct Countdown {
    from: Int
}

// [iter-fn] The struct its `state` block names holds every parameter,
// then the state; an `iter fn` needs no state machine.
//~ emits: data class CountdownIter(
//~|     var c: Countdown,
//~|     var at: Int,
//~| )
//~ emits: CountdownIter(c = c, at = c.from)
//~ !emits: __advance
//~ !emits: iterator {
iter fn iter(c: Countdown) -> Emitted Int | Finished {
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

//~ emits: data class FibsIter(
//~|     var f: Fibs,
//~ emits: FibsIter(f = f, a = 0, b = 1, made = 0)
iter fn iter(f: Fibs) -> Emitted Int | Finished {
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

//~ emits: data class RowIter(
//~|     var r: Row,
//~ emits: RowIter(r = r, left = r.times)
iter fn iter(r: Row) -> Emitted Str | Finished {
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
    // [fn-effects] The effectful `next` gets its handler per turn. The
    // suffix is the overload's mangle, which moves as std grows.
    //~ emits: = salvo.main.next__NoisyIter(__handle_
    for v in noisy {
        println("v ${v}")
    }
    let row = Row { label: "hey", times: 2 }
    for s in row {
        println("s ${s}")
    }
}
