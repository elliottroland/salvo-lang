// [iter-generic-drive] driving a generic pass
//
// Compile-and-run data ([test-run-data]), moved from the codegen tests.

//--- a_generic_close

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
//= stdout
//= all 10
//= cut 7

//--- a_generic_drive

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
//= stdout
//= first 3
//= rest 7
//= v 8
//= v 9
//= v 7
